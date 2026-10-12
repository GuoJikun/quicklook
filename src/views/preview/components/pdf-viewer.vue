<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { CollectionTag, RefreshRight, ZoomIn, ZoomOut } from '@element-plus/icons-vue'

const props = defineProps<{
    path: string
}>()

interface RenderedPage {
    page_num: number
    path: string
    width: number
    height: number
}

interface OutlineItem {
    title: string
    page: number
    items: OutlineItem[]
}

/** 缩放 100% 对应的参考 DPI；base 尺寸始终按此归一化，避免 scale 与 dpi 双重计数 */
const REF_DPI = 150
/** PDF point → REF_DPI 像素 */
const PT_TO_REF_PX = REF_DPI / 72
const PAGE_GAP = 16
const CANVAS_PAD_Y = 20
/** 虚拟窗口外额外保留的页数（DOM） */
const WINDOW_PAD = 2
/** 预渲染窗口外额外页数 */
const RENDER_PAD = 3

interface PageSize {
    width: number
    height: number
}

interface PageBaseSize {
    w: number
    h: number
}

const pages = ref<Map<number, RenderedPage>>(new Map())
/** 各页已渲染图对应的 generation，用于判断是否可直接复用 */
const pageGeneration = ref<Map<number, number>>(new Map())
const outline = ref<OutlineItem[]>([])
const pager = ref<{
    current: number
    total: number
    scale: number
    dpi: number
    rotation: number
}>({
    current: 1,
    total: 0,
    scale: 1,
    dpi: REF_DPI,
    rotation: 0,
})
const visible = ref(true)
const pageNum = ref<number>(1)
const scrollContainer = ref<HTMLDivElement>()
const renderGeneration = ref(0)
const renderingPages = ref<Map<number, number>>(new Map())
/** 每页在 REF_DPI、未施加用户旋转时的逻辑尺寸（来自 MediaBox） */
const pageBaseSizes = ref<PageBaseSize[]>([])
const virtualStart = ref(1)
const virtualEnd = ref(5)
let disposed = false
let scrollRaf = 0
let renderTimer: ReturnType<typeof setTimeout> | null = null

interface RenderTask {
    pageIndex: number
    generation: number
}

const renderQueue: RenderTask[] = []
let queueProcessing = false

/** 施加用户旋转后的逻辑尺寸（REF_DPI 像素，scale=1） */
const logicalSize = (idx: number): PageBaseSize => {
    const base = pageBaseSizes.value[idx]
    if (!base) return { w: 0, h: 0 }
    const rot = ((pager.value.rotation % 360) + 360) % 360
    return rot === 90 || rot === 270 ? { w: base.h, h: base.w } : base
}

/** 每页显示尺寸（含 scale） */
const displaySizes = computed(() => {
    const scale = pager.value.scale
    return pageBaseSizes.value.map((_, idx) => {
        const s = logicalSize(idx)
        return { w: s.w * scale, h: s.h * scale }
    })
})

/** 变高虚拟列表：各页 top 与总高 */
const layout = computed(() => {
    const tops: number[] = []
    let y = 0
    for (const s of displaySizes.value) {
        tops.push(y)
        y += s.h + PAGE_GAP
    }
    return { tops, totalHeight: displaySizes.value.length ? Math.max(0, y - PAGE_GAP) : 0 }
})

const pageTops = computed(() => layout.value.tops)
const totalContentHeight = computed(() => layout.value.totalHeight)

/**
 * 滚动内容宽度 = 最宽页显示宽度。
 * 配合 min-width:100% 与 left:50%+translateX(-50%)，页始终不向左溢出，
 * 放大后可通过 overflow-x 横向平移。
 */
const contentWidth = computed(() => {
    let maxW = 0
    for (const s of displaySizes.value) {
        if (s.w > maxW) maxW = s.w
    }
    return maxW
})

const virtualPages = computed(() => {
    const list: number[] = []
    const start = Math.max(1, virtualStart.value)
    const end = Math.min(pager.value.total, virtualEnd.value)
    for (let i = start; i <= end; i++) list.push(i)
    return list
})

/** 该页位图是否对应当前 generation（可直接展示且无需重渲） */
const isFresh = (idx: number) =>
    pages.value.has(idx) && pageGeneration.value.get(idx) === renderGeneration.value

const beginRenderSession = () => {
    renderGeneration.value += 1
    renderQueue.length = 0
    renderingPages.value.clear()
    // 不清空 pages：换 DPI/旋转时保留旧图 CSS 拉伸顶替，新图到达后替换，避免白屏
}

/** 在变高布局中找到 offset 所在页（0-based） */
const pageAtOffset = (y: number) => {
    const tops = pageTops.value
    if (!tops.length) return 0
    let lo = 0
    let hi = tops.length - 1
    let ans = 0
    while (lo <= hi) {
        const mid = (lo + hi) >> 1
        if (tops[mid] <= y) {
            ans = mid
            lo = mid + 1
        } else {
            hi = mid - 1
        }
    }
    return ans
}

const processQueue = async () => {
    if (queueProcessing || disposed) return
    queueProcessing = true
    while (renderQueue.length > 0 && !disposed) {
        const current = pager.value.current - 1
        renderQueue.sort((a, b) => Math.abs(a.pageIndex - current) - Math.abs(b.pageIndex - current))
        const task = renderQueue.shift()!
        if (!isFresh(task.pageIndex) && !renderingPages.value.has(task.pageIndex)) {
            await renderPage(task.pageIndex, task.generation)
        }
    }
    queueProcessing = false
}

const enqueuePages = (pageIndices: number[]) => {
    const generation = renderGeneration.value
    for (const idx of pageIndices) {
        if (
            idx < 0 ||
            idx >= pager.value.total ||
            isFresh(idx) ||
            renderingPages.value.has(idx) ||
            renderQueue.some(task => task.pageIndex === idx)
        ) {
            continue
        }
        renderQueue.push({ pageIndex: idx, generation })
    }
    processQueue()
}

const renderPage = async (pageIndex: number, generation: number) => {
    if (disposed || generation !== renderGeneration.value) return
    if (isFresh(pageIndex) || renderingPages.value.has(pageIndex)) return

    renderingPages.value.set(pageIndex, generation)
    try {
        const result = await invoke<RenderedPage>('render_pdf_page', {
            path: props.path,
            pageIndex,
            dpi: pager.value.dpi,
            rotation: pager.value.rotation,
        })
        if (disposed || generation !== renderGeneration.value) return
        pages.value.set(pageIndex, result)
        pageGeneration.value.set(pageIndex, generation)
    } catch (e) {
        if (!disposed && generation === renderGeneration.value) {
            console.error(`[pdf] page ${pageIndex + 1} render failed`, e)
        }
    } finally {
        if (!disposed && renderingPages.value.get(pageIndex) === generation) {
            renderingPages.value.delete(pageIndex)
        }
    }
}

/** 将第 n 页（1-based）滚到可视区顶部 */
const scrollToPage = (n: number) => {
    const container = scrollContainer.value
    if (!container) return
    const idx = Math.min(Math.max(1, n), Math.max(1, pager.value.total)) - 1
    const top = pageTops.value[idx]
    if (top === undefined) return
    container.scrollTop = CANVAS_PAD_Y + top
}

/** 根据滚动位置更新：虚拟窗口、当前页、预渲染队列 */
const updateFromScroll = () => {
    const container = scrollContainer.value
    if (!container) return

    const total = pager.value.total
    if (total <= 0) return

    const tops = pageTops.value
    if (!tops.length) {
        // 尚未拿到页面尺寸，先保证前几页可渲染
        virtualStart.value = 1
        virtualEnd.value = Math.min(total, 5)
        enqueuePages([0, 1, 2].filter(i => i < total))
        return
    }

    const viewH = container.clientHeight
    const rel = Math.max(0, container.scrollTop - CANVAS_PAD_Y)

    const firstIdx = pageAtOffset(rel)
    const lastIdx = Math.min(total - 1, pageAtOffset(rel + viewH) + 1)
    const centerIdx = pageAtOffset(rel + viewH / 2)

    const current = Math.min(total, Math.max(1, centerIdx + 1))
    if (current !== pager.value.current) {
        pager.value.current = current
        pageNum.value = current
    }

    virtualStart.value = Math.max(1, firstIdx + 1 - WINDOW_PAD)
    virtualEnd.value = Math.min(total, lastIdx + 1 + WINDOW_PAD)

    const renderStart = Math.max(0, firstIdx - RENDER_PAD)
    const renderEnd = Math.min(total - 1, lastIdx + RENDER_PAD)
    const toRender: number[] = []
    for (let i = renderStart; i <= renderEnd; i++) toRender.push(i)
    enqueuePages(toRender)
}

const onScroll = () => {
    if (scrollRaf || disposed) return
    scrollRaf = requestAnimationFrame(() => {
        scrollRaf = 0
        if (!disposed) updateFromScroll()
    })
}

const handleJump = () => {
    const total = pager.value.total
    const n = Math.min(Math.max(1, Math.round(pageNum.value) || 1), Math.max(1, total))
    pageNum.value = n
    scrollToPage(n)
    updateFromScroll()
}

const handleNodeClick = (data: OutlineItem) => {
    if (!data.page || data.page < 1) return
    pageNum.value = data.page
    handleJump()
}

const handleZoomIn = () => {
    pager.value.scale = Math.min(5, pager.value.scale + 0.25)
}

const handleZoomOut = () => {
    pager.value.scale = Math.max(0.25, pager.value.scale - 0.25)
}

const handleRotate = async () => {
    pager.value.rotation = (pager.value.rotation + 90) % 360
    const savedPage = pager.value.current

    beginRenderSession()

    await nextTick()
    scrollToPage(savedPage)
    pager.value.current = savedPage
    pageNum.value = savedPage

    updateFromScroll()
}

const handleFitWidth = () => {
    if (!scrollContainer.value || !pageBaseSizes.value.length) return
    const containerWidth = scrollContainer.value.clientWidth - CANVAS_PAD_Y * 2
    const baseW = logicalSize(0).w
    if (containerWidth <= 0 || baseW <= 0) return
    pager.value.scale = Math.min(5, Math.max(0.25, containerWidth / baseW))
}

const handleWheel = (e: WheelEvent) => {
    if (!e.ctrlKey) return
    e.preventDefault()
    if (e.deltaY < 0) handleZoomIn()
    else handleZoomOut()
}

watch(
    () => pager.value.scale,
    () => {
        if (renderTimer) clearTimeout(renderTimer)
        renderTimer = setTimeout(async () => {
            if (!props.path || disposed) return
            const newDpi = Math.round(REF_DPI * pager.value.scale)
            const clampedDpi = Math.max(72, Math.min(600, newDpi))
            const savedPage = pager.value.current

            if (clampedDpi !== pager.value.dpi) {
                // 保留旧图作为占位，仅提升 generation 触发重渲
                beginRenderSession()
                pager.value.dpi = clampedDpi
            }

            await nextTick()
            if (disposed) return

            scrollToPage(savedPage)
            pager.value.current = savedPage
            pageNum.value = savedPage

            updateFromScroll()
        }, 300)
    },
)

// 布局高度变化（缩放/旋转）后重算窗口
watch(totalContentHeight, () => {
    nextTick(() => updateFromScroll())
})

const loadOutline = async (path: string) => {
    const [sizes, outlineData] = await Promise.all([
        invoke<PageSize[]>('get_pdf_page_sizes', { path }),
        invoke<OutlineItem[]>('get_pdf_outline', { path }),
    ])
    pager.value.total = sizes.length
    outline.value = outlineData
    pageBaseSizes.value = sizes.map(s => ({
        w: s.width * PT_TO_REF_PX,
        h: s.height * PT_TO_REF_PX,
    }))
}

onMounted(async () => {
    console.log('[pdf] onMounted', props.path)
    await loadOutline(props.path)
    console.log('[pdf] total pages:', pager.value.total)

    virtualStart.value = 1
    virtualEnd.value = Math.min(pager.value.total, 5)

    beginRenderSession()
    scrollContainer.value?.addEventListener('scroll', onScroll, { passive: true })
    // 非 passive，否则 preventDefault 无法拦截 Ctrl+滚轮
    scrollContainer.value?.addEventListener('wheel', handleWheel, { passive: false })

    updateFromScroll()
    for (let i = 0; i < Math.min(3, pager.value.total); i++) {
        renderPage(i, renderGeneration.value)
    }
})

onUnmounted(() => {
    disposed = true
    if (scrollRaf) {
        cancelAnimationFrame(scrollRaf)
        scrollRaf = 0
    }
    scrollContainer.value?.removeEventListener('scroll', onScroll)
    scrollContainer.value?.removeEventListener('wheel', handleWheel)
    if (renderTimer) clearTimeout(renderTimer)
    renderQueue.length = 0
    renderingPages.value.clear()
    pageGeneration.value.clear()
})
</script>

<template>
    <div class="pdf-viewer">
        <div class="pdf-viewer-toolbar">
            <div class="pdf-viewer-toolbar__left">
                <el-link :underline="false" @click="visible = !visible">
                    <el-icon size="18px">
                        <CollectionTag />
                    </el-icon>
                </el-link>
            </div>
            <div class="pdf-viewer-toolbar__center">
                <el-button text size="small" @click="handleZoomOut">
                    <el-icon size="18px">
                        <ZoomOut />
                    </el-icon>
                </el-button>
                <span class="pdf-viewer-toolbar__zoom">{{ Math.round(pager.scale * 100) }}%</span>
                <el-button text size="small" @click="handleZoomIn">
                    <el-icon size="18px">
                        <ZoomIn />
                    </el-icon>
                </el-button>
                <el-divider direction="vertical" />
                <el-button text size="small" @click="handleFitWidth">适合宽度</el-button>
                <el-divider direction="vertical" />
                <el-button text size="small" @click="handleRotate">
                    <el-icon size="18px">
                        <RefreshRight />
                    </el-icon>
                </el-button>
                <el-divider direction="vertical" />
                <el-input v-model.number="pageNum" size="small" style="width: 50px" @keydown.enter="handleJump" />
                <span class="pdf-viewer-toolbar__total">/ {{ pager.total }}</span>
            </div>
            <div class="pdf-viewer-toolbar__right"></div>
        </div>

        <div class="pdf-viewer-body">
            <div class="pdf-viewer-outline" v-if="visible">
                <el-scrollbar>
                    <el-tree
                        :data="outline"
                        :props="{ children: 'items', label: 'title' }"
                        :highlight-current="true"
                        @node-click="handleNodeClick"
                    />
                </el-scrollbar>
            </div>

            <div ref="scrollContainer" class="pdf-viewer-canvas" :class="{ 'pdf-viewer-canvas--full': !visible }">
                <div
                    class="pdf-viewer-pages"
                    :style="{
                        height: `${totalContentHeight}px`,
                        width: contentWidth > 0 ? `${contentWidth}px` : undefined,
                    }"
                >
                    <div
                        v-for="n in virtualPages"
                        :id="`page-placeholder-${n}`"
                        :key="n"
                        class="pdf-viewer-page-placeholder"
                        :style="{
                            top: `${pageTops[n - 1] ?? 0}px`,
                            width: `${displaySizes[n - 1]?.w ?? 0}px`,
                            height: `${displaySizes[n - 1]?.h ?? 0}px`,
                        }"
                    >
                        <img
                            v-if="pages.get(n - 1)"
                            :src="convertFileSrc(pages.get(n - 1)!.path)"
                            class="pdf-viewer-page"
                        />
                        <div v-else-if="renderingPages.has(n - 1)" class="pdf-viewer-page-loading">渲染中...</div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="scss">
.pdf-viewer {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;

    &-toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        height: 40px;
        padding: 0 16px;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
        background-color: var(--color-bg);
        color: var(--color-text-primary);
        flex-shrink: 0;

        &__left,
        &__right {
            width: 60px;
        }

        &__center {
            display: flex;
            align-items: center;
            gap: 4px;
            font-size: 13px;
        }

        &__zoom {
            min-width: 40px;
            text-align: center;
        }

        &__total {
            margin-left: 4px;
            color: var(--color-text-secondary);
        }
    }

    &-body {
        display: flex;
        flex: 1;
        overflow: hidden;
    }

    &-outline {
        width: 300px;
        height: 100%;
        overflow: auto;
        box-shadow: 1px 0 2px rgba(0, 0, 0, 0.1);
        background-color: var(--color-bg);
        color: var(--color-text-primary);
        font-size: 14px;
        flex-shrink: 0;
    }

    &-canvas {
        flex: 1;
        overflow: auto;
        padding: 20px;
        background-color: var(--color-bg);
    }

    &-pages {
        position: relative;
        width: 100%;
        min-width: 100%;
    }

    &-page-placeholder {
        position: absolute;
        left: 50%;
        transform: translateX(-50%);
        display: flex;
        align-items: center;
        justify-content: center;
        background: white;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
        overflow: hidden;
        box-sizing: border-box;
    }

    &-page {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    &-page-loading {
        color: var(--color-text-secondary);
        font-size: 14px;
    }
}
</style>

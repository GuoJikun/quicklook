<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { PdfOutlineItem, PdfState } from './pdf-types'

/**
 * 纯画布组件：只负责页面渲染、虚拟滚动与缩放状态。
 * 命令栏与目录抽屉由父级 document.vue 绘制（统一样式与 i18n 都在那一层），
 * 因此这里不出现任何按钮，只通过 expose 暴露动作、通过 emit 回传状态。
 */

const { t } = useI18n()

const props = defineProps<{
    path: string
}>()

const emit = defineEmits<{
    (e: 'state', value: PdfState): void
    (e: 'outline', value: PdfOutlineItem[]): void
    (e: 'error', message: string): void
}>()

interface RenderedPage {
    page_num: number
    path: string
    width: number
    height: number
}

interface PageSize {
    width: number
    height: number
}

interface PageBaseSize {
    w: number
    h: number
}

/** 缩放 100% 对应的参考 DPI；base 尺寸始终按此归一化，避免 scale 与 dpi 双重计数 */
const REF_DPI = 150
/** PDF point → REF_DPI 像素 */
const PT_TO_REF_PX = REF_DPI / 72
/** 页间距（设计稿 §5.4：12px） */
const PAGE_GAP = 12
/** 画布四周留白，需与样式里的 padding 保持一致 */
const CANVAS_PAD = 12
const ZOOM_MIN = 0.1
const ZOOM_MAX = 5
const ZOOM_STEP = 0.1
/** 虚拟窗口外额外保留的页数（DOM） */
const WINDOW_PAD = 2
/** 预渲染窗口外额外页数 */
const RENDER_PAD = 3
/** 同时在飞的渲染请求数（前端并发上限，配合后端锁外编码形成流水线） */
const MAX_CONCURRENT_RENDERS = 3

const pages = ref<Map<number, RenderedPage>>(new Map())
/** 各页已渲染图对应的 generation，用于判断是否可直接复用 */
const pageGeneration = ref<Map<number, number>>(new Map())
const pager = ref<{
    current: number
    total: number
    dpi: number
    rotation: number
}>({
    current: 1,
    total: 0,
    dpi: REF_DPI,
    rotation: 0,
})
/** 相对「适应宽度」的缩放倍数：1 = 100% */
const zoom = ref(1)
/** 适应宽度对应的绝对缩放，随窗口宽度变化 */
const fitScale = ref(1)
/** 真正施加到页面尺寸上的缩放 */
const effScale = computed(() => zoom.value * fitScale.value)

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
let resizeObserver: ResizeObserver | null = null

interface RenderTask {
    pageIndex: number
    generation: number
}

const renderQueue: RenderTask[] = []
let activeRenders = 0

/** 施加用户旋转后的逻辑尺寸（REF_DPI 像素，scale=1） */
const logicalSize = (idx: number): PageBaseSize => {
    const base = pageBaseSizes.value[idx]
    if (!base) return { w: 0, h: 0 }
    const rot = ((pager.value.rotation % 360) + 360) % 360
    return rot === 90 || rot === 270 ? { w: base.h, h: base.w } : base
}

/** 每页显示尺寸（含绝对缩放） */
const displaySizes = computed(() => {
    const scale = effScale.value
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

const emitState = () => {
    emit('state', { current: pager.value.current, total: pager.value.total, zoom: zoom.value })
}

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

const processQueue = () => {
    if (disposed) return
    while (renderQueue.length > 0 && activeRenders < MAX_CONCURRENT_RENDERS) {
        const current = pager.value.current - 1
        renderQueue.sort((a, b) => Math.abs(a.pageIndex - current) - Math.abs(b.pageIndex - current))
        const task = renderQueue.shift()!
        if (isFresh(task.pageIndex) || renderingPages.value.has(task.pageIndex)) {
            continue
        }
        activeRenders++
        renderPage(task.pageIndex, task.generation).finally(() => {
            activeRenders = Math.max(0, activeRenders - 1)
            if (!disposed) processQueue()
        })
    }
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

/** 将第 n 页（1-based）滚到可视区顶部（保留 12px 上留白，与页间距视觉一致） */
const scrollToPage = (n: number) => {
    const container = scrollContainer.value
    if (!container) return
    const idx = Math.min(Math.max(1, n), Math.max(1, pager.value.total)) - 1
    const top = pageTops.value[idx]
    if (top === undefined) return
    container.scrollTop = Math.max(0, top)
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
    // 与 scrollToPage 保持同一基准：scrollTop 即内容坐标，不减去画布上留白，
    // 否则跳到第 n 页时视口顶部落在页间 12px 间距里，当前页会被判成 n-1。
    const rel = Math.max(0, container.scrollTop)

    const firstIdx = pageAtOffset(rel)
    const lastIdx = Math.min(total - 1, pageAtOffset(rel + viewH) + 1)
    const centerIdx = pageAtOffset(rel + viewH / 2)

    const current = Math.min(total, Math.max(1, centerIdx + 1))
    if (current !== pager.value.current) {
        pager.value.current = current
        emitState()
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

/** 适应宽度 = 最宽页铺满可用宽度；混合页面尺寸时也不会溢出 */
const computeFitScale = () => {
    const box = scrollContainer.value
    if (!box || !pageBaseSizes.value.length) return
    let maxW = 0
    for (let i = 0; i < pageBaseSizes.value.length; i++) {
        const s = logicalSize(i)
        if (s.w > maxW) maxW = s.w
    }
    const avail = box.clientWidth - CANVAS_PAD * 2
    if (maxW <= 0 || avail <= 0) return
    fitScale.value = Math.max(0.05, avail / maxW)
}

interface ScrollAnchor {
    /** 视口顶部对应的页（0-based） */
    idx: number
    /** 该页内的相对位置 0-1 */
    inPage: number
    /** 视口中心在内容宽度中的相对位置 0-1 */
    centerX: number
}

/** 缩放/旋转/改窗口前记录阅读点，变更后按同一逻辑位置还原 */
const captureAnchor = (): ScrollAnchor | null => {
    const box = scrollContainer.value
    if (!box || !pageTops.value.length) return null
    const rel = Math.max(0, box.scrollTop)
    const idx = pageAtOffset(rel)
    const top = pageTops.value[idx] ?? 0
    const h = displaySizes.value[idx]?.h ?? 0
    const width = contentWidth.value || 1
    return {
        idx,
        inPage: h > 0 ? Math.min(1, Math.max(0, (rel - top) / h)) : 0,
        centerX: Math.min(1, Math.max(0, (box.scrollLeft + box.clientWidth / 2) / width)),
    }
}

const restoreAnchor = async (anchor: ScrollAnchor | null) => {
    await nextTick()
    const box = scrollContainer.value
    if (!box || !anchor || !pageTops.value.length) return
    const idx = Math.min(anchor.idx, pageTops.value.length - 1)
    const top = pageTops.value[idx] ?? 0
    const h = displaySizes.value[idx]?.h ?? 0
    // 定位基准是 pages 容器顶部（不含画布上留白），第一页顶部对应 scrollTop 0，
    // 否则首屏的 12px 留白会被滚动吃掉，页面看起来贴住命令栏。
    box.scrollTop = Math.max(0, top + anchor.inPage * h)
    // 只有放大到出现横向滚动时才需要还原水平位置
    box.scrollLeft =
        contentWidth.value > box.clientWidth
            ? Math.max(0, anchor.centerX * contentWidth.value - box.clientWidth / 2)
            : 0
}

/** 命令栏的实际入口：设置相对适应宽度的缩放 */
const setZoom = async (next: number) => {
    const clamped = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(next * 100) / 100))
    if (Math.abs(clamped - zoom.value) < 0.001) return
    const anchor = captureAnchor()
    zoom.value = clamped
    await restoreAnchor(anchor)
    updateFromScroll()
    emitState()
}

const zoomIn = () => void setZoom(zoom.value + ZOOM_STEP)
const zoomOut = () => void setZoom(zoom.value - ZOOM_STEP)

/** 回到 100%（即适应宽度） */
const fitWidth = () => void setZoom(1)

/** 重新按当前容器宽度计算适应比例，不改动用户选定的缩放档位 */
const refit = async () => {
    const anchor = captureAnchor()
    computeFitScale()
    await restoreAnchor(anchor)
    updateFromScroll()
}

const goToPage = (n: number) => {
    const total = pager.value.total
    const target = Math.min(Math.max(1, Math.round(n) || 1), Math.max(1, total))
    scrollToPage(target)
    updateFromScroll()
}

const rotate = async () => {
    const anchor = captureAnchor()
    pager.value.rotation = (pager.value.rotation + 90) % 360
    beginRenderSession()
    // 旋转会交换宽高，适应宽度的基准随之改变
    computeFitScale()
    await restoreAnchor(anchor)
    updateFromScroll()
    emitState()
}

/**
 * 宽度变化（改窗口、开关目录抽屉）后重算适应宽度。
 * ResizeObserver 回调本身已按帧合并，这里不再套 rAF：
 * 多一层 rAF 会让变更延后一帧，开关抽屉时能明显看到页面尺寸跳一下。
 */
const onResize = () => {
    if (disposed) return
    const anchor = captureAnchor()
    const prevFit = fitScale.value
    computeFitScale()
    // 100% 时窗口变宽页面应跟着变宽，其他档位按比例保持
    if (Math.abs(prevFit - fitScale.value) > 0.0001) {
        void restoreAnchor(anchor).then(updateFromScroll)
    } else {
        updateFromScroll()
    }
}

/** 缩放/窗口变化后按新 DPI 重渲（保留旧图顶替，避免白屏） */
watch(effScale, () => {
    if (renderTimer) clearTimeout(renderTimer)
    renderTimer = setTimeout(async () => {
        if (!props.path || disposed) return
        const nextDpi = Math.min(600, Math.max(72, Math.round(REF_DPI * effScale.value)))
        if (nextDpi === pager.value.dpi) return
        const anchor = captureAnchor()
        // 保留旧图作为占位，仅提升 generation 触发重渲
        beginRenderSession()
        pager.value.dpi = nextDpi
        await restoreAnchor(anchor)
        updateFromScroll()
    }, 260)
})

// 布局高度变化（缩放/旋转）后重算窗口
watch(totalContentHeight, () => {
    nextTick(() => updateFromScroll())
})

const loadDocument = async (path: string) => {
    const [sizes, outlineData] = await Promise.all([
        invoke<PageSize[]>('get_pdf_page_sizes', { path }),
        invoke<PdfOutlineItem[]>('get_pdf_outline', { path }),
    ])
    pager.value.total = sizes.length
    pageBaseSizes.value = sizes.map(s => ({
        w: s.width * PT_TO_REF_PX,
        h: s.height * PT_TO_REF_PX,
    }))
    emit('outline', outlineData)
}

const reset = () => {
    pages.value.clear()
    pageGeneration.value.clear()
    renderingPages.value.clear()
    renderQueue.length = 0
    pageBaseSizes.value = []
    pager.value = { current: 1, total: 0, dpi: REF_DPI, rotation: 0 }
    zoom.value = 1
    fitScale.value = 1
}

const load = async () => {
    reset()
    try {
        await loadDocument(props.path)
    } catch (e) {
        emit('error', e instanceof Error ? e.message : String(e))
        return
    }

    computeFitScale()
    virtualStart.value = 1
    virtualEnd.value = Math.min(pager.value.total, 5)
    beginRenderSession()
    updateFromScroll()
    emitState()
}

onMounted(async () => {
    const container = scrollContainer.value
    if (container) {
        container.addEventListener('scroll', onScroll, { passive: true })
        // 非 passive，否则 preventDefault 无法拦截 Ctrl+滚轮
        container.addEventListener('wheel', handleWheel, { passive: false })
        resizeObserver = new ResizeObserver(onResize)
        resizeObserver.observe(container)
    }
    await load()
})

onUnmounted(() => {
    disposed = true
    if (scrollRaf) {
        cancelAnimationFrame(scrollRaf)
        scrollRaf = 0
    }
    scrollContainer.value?.removeEventListener('scroll', onScroll)
    scrollContainer.value?.removeEventListener('wheel', handleWheel)
    resizeObserver?.disconnect()
    resizeObserver = null
    if (renderTimer) clearTimeout(renderTimer)
    renderQueue.length = 0
    renderingPages.value.clear()
    pageGeneration.value.clear()
})

const handleWheel = (e: WheelEvent) => {
    if (!e.ctrlKey) return
    e.preventDefault()
    if (e.deltaY < 0) zoomIn()
    else zoomOut()
}

// 同一窗口内换文件时组件不会重建，需要主动重读
watch(
    () => props.path,
    path => {
        if (!path || disposed) return
        void load()
    },
)

defineExpose({ zoomIn, zoomOut, fitWidth, refit, rotate, goToPage })
</script>

<template>
    <div class="pdf-viewer">
        <div ref="scrollContainer" class="pdf-viewer-canvas pv-scroll">
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
                        draggable="false"
                        alt=""
                    />
                    <div v-else-if="renderingPages.has(n - 1)" class="pdf-viewer-page-loading">
                        {{ t('pdf.rendering') }}
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
    position: relative;
    overflow: hidden;

    /* 唯一的滚动容器；padding 需与脚本里的 CANVAS_PAD 一致 */
    &-canvas {
        width: 100%;
        height: 100%;
        padding: 12px;
        box-sizing: border-box;
        background-color: var(--color-bg);
    }

    &-pages {
        position: relative;
        margin: 0 auto;
        min-width: 100%;
    }

    &-page-placeholder {
        position: absolute;
        left: 50%;
        transform: translateX(-50%);
        display: flex;
        align-items: center;
        justify-content: center;
        /* 纸张恒为白底：PDF 内容本身按白纸渲染，跟主题走会糊成一片 */
        background: #fff;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
        overflow: hidden;
        box-sizing: border-box;

        /* 深色下浅色纸张与背景对比太强，补一圈极淡描边标出纸张边界 */
        :global(html.dark) & {
            box-shadow:
                0 1px 2px rgba(0, 0, 0, 0.45),
                0 0 0 1px rgba(255, 255, 255, 0.06);
        }
    }

    &-page {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
        user-select: none;
    }

    &-page-loading {
        color: var(--color-text-disabled);
        font-size: var(--font-xs);
    }
}
</style>

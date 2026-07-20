<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
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

const pages = ref<Map<number, RenderedPage>>(new Map())
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
    dpi: 150,
    rotation: 0,
})
const visible = ref(true)
const pageNum = ref<number>(1)
const scrollContainer = ref<HTMLDivElement>()
let observer: IntersectionObserver | null = null
let scrollHandler: (() => void) | null = null
const renderGeneration = ref(0)
const renderingPages = ref<Map<number, number>>(new Map())
const baseWidth = ref<number>(0)
const baseHeight = ref<number>(0)
let disposed = false

interface RenderTask {
    pageIndex: number
    generation: number
}

const renderQueue: RenderTask[] = []
let queueProcessing = false

const beginRenderSession = () => {
    renderGeneration.value += 1
    pages.value.clear()
    renderQueue.length = 0
    renderingPages.value.clear()
    baseWidth.value = 0
    baseHeight.value = 0
}

const processQueue = async () => {
    if (queueProcessing || disposed) return
    queueProcessing = true
    while (renderQueue.length > 0 && !disposed) {
        const current = pager.value.current - 1
        renderQueue.sort((a, b) => Math.abs(a.pageIndex - current) - Math.abs(b.pageIndex - current))
        const task = renderQueue.shift()!
        if (!pages.value.has(task.pageIndex) && !renderingPages.value.has(task.pageIndex)) {
            await renderPage(task.pageIndex, task.generation)
        }
    }
    queueProcessing = false
}

const enqueuePages = (pageIndices: number[]) => {
    const generation = renderGeneration.value
    for (const idx of pageIndices) {
        if (!pages.value.has(idx) && !renderingPages.value.has(idx) && !renderQueue.some(task => task.pageIndex === idx)) {
            renderQueue.push({ pageIndex: idx, generation })
        }
    }
    processQueue()
}

const renderPage = async (pageIndex: number, generation: number) => {
    if (disposed || generation !== renderGeneration.value) return
    if (pages.value.has(pageIndex) || renderingPages.value.has(pageIndex)) return

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
        if (pageIndex === 0 && baseWidth.value === 0) {
            baseWidth.value = result.width
            baseHeight.value = result.height
            console.log(`[pdf] baseSize: ${baseWidth.value}x${baseHeight.value}`)
        }
        console.log(`[pdf] page ${pageIndex + 1} rendered`, result)
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

/** Track which page is most visible in the scroll container. */
const updateCurrentPage = () => {
    const container = scrollContainer.value
    if (!container) return
    const scrollTop = container.scrollTop
    const viewBottom = scrollTop + container.clientHeight

    let bestPage = 1
    let bestVisiblePx = 0

    for (let i = 1; i <= pager.value.total; i++) {
        const el = document.getElementById(`page-placeholder-${i}`)
        if (!el) continue
        const rect = el.getBoundingClientRect()
        // Page position relative to scroll container
        const pageTop = rect.top + scrollTop
        const pageBottom = pageTop + rect.height

        // Overlap between page and viewport
        const overlapTop = Math.max(pageTop, scrollTop)
        const overlapBottom = Math.min(pageBottom, viewBottom)
        const overlap = Math.max(0, overlapBottom - overlapTop)

        if (overlap > bestVisiblePx) {
            bestVisiblePx = overlap
            bestPage = i
        }
    }

    if (bestPage !== pager.value.current) {
        pager.value.current = bestPage
        pageNum.value = bestPage
    }
}

const initObserver = () => {
    observer?.disconnect()
    if (scrollHandler) {
        scrollContainer.value?.removeEventListener('scroll', scrollHandler)
        scrollHandler = null
    }

    // Observer is only used for pre-rendering (enqueuePages), not for page tracking
    observer = new IntersectionObserver(
        entries => {
            const visiblePages: number[] = []
            for (const entry of entries) {
                if (entry.isIntersecting) {
                    const id = entry.target.id
                    const num = parseInt(id.replace('page-placeholder-', ''), 10)
                    if (!isNaN(num)) {
                        visiblePages.push(num - 1)
                    }
                }
            }
            enqueuePages(visiblePages)
        },
        {
            root: scrollContainer.value,
            rootMargin: '400px 0px',
            threshold: 0,
        },
    )

    nextTick(() => {
        for (let i = 0; i < pager.value.total; i++) {
            const el = document.getElementById(`page-placeholder-${i + 1}`)
            if (el) observer!.observe(el)
        }
    })

    // Scroll handler for page tracking
    scrollHandler = () => updateCurrentPage()
    scrollContainer.value?.addEventListener('scroll', scrollHandler!)
}

const handleJump = () => {
    const el = document.getElementById(`page-placeholder-${pageNum.value}`)
    if (el) {
        el.scrollIntoView({ block: 'start' })
    }
}

const handleNodeClick = (data: OutlineItem) => {
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

    const el = document.getElementById(`page-placeholder-${savedPage}`)
    if (el) el.scrollIntoView({ block: 'start' })
    pager.value.current = savedPage
    pageNum.value = savedPage

    initObserver()
    for (let i = 0; i < Math.min(3, pager.value.total); i++) {
        renderPage(i, renderGeneration.value)
    }
}

const handleFitWidth = () => {
    if (!scrollContainer.value || !pages.value.size) return
    const containerWidth = scrollContainer.value.clientWidth - 40
    const firstPage = pages.value.get(0)
    if (firstPage) {
        pager.value.scale = containerWidth / firstPage.width
    }
}

let renderTimer: ReturnType<typeof setTimeout> | null = null
watch(
    () => pager.value.scale,
    () => {
        if (renderTimer) clearTimeout(renderTimer)
        renderTimer = setTimeout(async () => {
            if (!props.path) return
            const baseDpi = 150
            const newDpi = Math.round(baseDpi * pager.value.scale)
            const clampedDpi = Math.max(72, Math.min(600, newDpi))
            if (clampedDpi !== pager.value.dpi) {
                const savedPage = pager.value.current

                beginRenderSession()
                pager.value.dpi = clampedDpi

                await nextTick()

                // Restore scroll position to keep the same page in view
                const el = document.getElementById(`page-placeholder-${savedPage}`)
                if (el) el.scrollIntoView({ block: 'start' })
                pager.value.current = savedPage
                pageNum.value = savedPage

                initObserver()
                for (let i = 0; i < Math.min(3, pager.value.total); i++) {
                    renderPage(i, renderGeneration.value)
                }
            }
        }, 300)
    },
)

const handleWheel = (e: WheelEvent) => {
    if (e.ctrlKey) {
        e.preventDefault()
        if (e.deltaY < 0) handleZoomIn()
        else handleZoomOut()
    }
}

const loadOutline = async (path: string) => {
    const [count, outlineData] = await Promise.all([
        invoke<number>('get_pdf_page_count', { path }),
        invoke<OutlineItem[]>('get_pdf_outline', { path }),
    ])
    pager.value.total = count
    outline.value = outlineData
}

onMounted(async () => {
    console.log('[pdf] onMounted', props.path)
    await loadOutline(props.path)
    console.log('[pdf] total pages:', pager.value.total)
    beginRenderSession()
    initObserver()
    for (let i = 0; i < Math.min(3, pager.value.total); i++) {
        renderPage(i, renderGeneration.value)
    }
})

onUnmounted(() => {
    disposed = true
    observer?.disconnect()
    if (scrollHandler) {
        scrollContainer.value?.removeEventListener('scroll', scrollHandler)
        scrollHandler = null
    }
    if (renderTimer) clearTimeout(renderTimer)
    renderQueue.length = 0
    renderingPages.value.clear()
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

            <div
                ref="scrollContainer"
                class="pdf-viewer-canvas"
                :class="{ 'pdf-viewer-canvas--full': !visible }"
                @wheel.passive="handleWheel"
            >
                <template v-for="i in pager.total" :key="i">
                    <div
                        :id="`page-placeholder-${i}`"
                        class="pdf-viewer-page-placeholder"
                        :style="{
                            flex: `0 0 ${baseHeight * pager.scale}px`,
                            width: `${baseWidth * pager.scale}px`,
                        }"
                    >
                        <img
                            v-if="pages.get(i - 1)"
                            :src="convertFileSrc(pages.get(i - 1)!.path)"
                            class="pdf-viewer-page"
                            @load="console.log('[pdf] img loaded', i)"
                            @error="console.error('[pdf] img error', i)"
                        />
                        <div v-else-if="renderingPages.has(i - 1)" class="pdf-viewer-page-loading">渲染中...</div>
                    </div>
                </template>
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
        overflow-y: auto;
        padding: 20px;
        background-color: #efefef;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 16px;

        &--full {
            width: 100%;
        }
    }

    &-page-placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        background: white;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
        overflow: hidden;
    }

    &-page {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    &-page-loading {
        color: #999;
        font-size: 14px;
    }
}
</style>

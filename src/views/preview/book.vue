<script setup lang="ts">
import { ref, computed, shallowRef, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import LayoutPreview from '@/components/layout-preview.vue'
import type { FileInfo } from '@/utils/typescript'

defineOptions({
    name: 'BookSupport',
})

interface PdfMeta {
    page_count: number
}

interface PdfPageResult {
    base64: string
    page: number
}

// 初始渲染倍率（2x 高清）；zoom 超过此值时触发重渲染
const BASE_RENDER_SCALE = 2.0
const ZOOM_STEP = 0.25
const ZOOM_MIN = 0.25
const ZOOM_MAX = 4.0

/**
 * 根据当前 zoom 决定渲染倍率：
 * - zoom ≤ 2：直接用 BASE_RENDER_SCALE，CSS 缩放即可
 * - zoom > 2：向上取整到最近 0.5，减少不必要的重渲染次数
 */
const getRenderScale = (z: number): number => {
    if (z <= BASE_RENDER_SCALE) return BASE_RENDER_SCALE
    return Math.ceil(z * 2) / 2
}

const route = useRoute()
const fileInfo = ref<FileInfo>()
const loading = ref(true)
const error = ref<string>()
const pageCount = ref(0)
const currentPage = ref(1)
const pageInput = ref(1)

const zoom = ref(1.0)
// 当前显示图片的渲染倍率（对应 Rust 渲染时传入的 scale 参数）
const imgRenderScale = ref(BASE_RENDER_SCALE)
// 当前 <img> 的自然像素宽度，图片加载后更新
const naturalImgWidth = ref(0)

// 缓存键 = `${0-based页码}-${渲染倍率}`，避免不同 zoom 级别混用缓存
const pageCache = shallowRef<Map<string, string>>(new Map())
const currentImgSrc = ref<string>()

// 根据 naturalImgWidth + zoom + imgRenderScale 动态计算图片显示宽度
// 公式：displayWidth = naturalWidth * (zoom / renderScale)
// 例：scale=2 渲染，zoom=1 → 显示 50% 自然宽度；zoom=2 → 1:1；zoom=3（重渲染 scale=3）→ 1:1
const imgStyle = computed(() => {
    if (!naturalImgWidth.value) return {}
    const w = Math.round(naturalImgWidth.value * zoom.value / imgRenderScale.value)
    return { width: `${w}px` }
})

const zoomText = computed(() => `${Math.round(zoom.value * 100)}%`)

const renderPage = async (pageIndex: number) => {
    const path = fileInfo.value?.path
    if (!path) return

    const renderScale = getRenderScale(zoom.value)
    const cacheKey = `${pageIndex}-${renderScale}`

    // 先更新渲染倍率，再重置宽度（等 img onload 更新）
    imgRenderScale.value = renderScale
    naturalImgWidth.value = 0

    if (pageCache.value.has(cacheKey)) {
        currentImgSrc.value = pageCache.value.get(cacheKey)
        return
    }

    loading.value = true
    try {
        const result = await invoke<PdfPageResult>('pdf_render_page', {
            path,
            page: pageIndex,
            scale: renderScale,
        })
        const dataUrl = `data:image/png;base64,${result.base64}`
        pageCache.value.set(cacheKey, dataUrl)
        currentImgSrc.value = dataUrl
    } catch (e) {
        error.value = String(e)
    } finally {
        loading.value = false
    }
}

const goToPage = async (page: number) => {
    if (page < 1 || page > pageCount.value) return
    currentPage.value = page
    pageInput.value = page
    await renderPage(page - 1)
}

const handlePrev = () => goToPage(currentPage.value - 1)
const handleNext = () => goToPage(currentPage.value + 1)

const handleJump = () => {
    const page = pageInput.value
    if (page >= 1 && page <= pageCount.value) {
        goToPage(page)
    } else {
        pageInput.value = currentPage.value
    }
}

const handleImgLoad = (e: Event) => {
    naturalImgWidth.value = (e.target as HTMLImageElement).naturalWidth
}

// 防抖重渲染：zoom 快速变化时只在停止后 200ms 触发
let zoomTimer: ReturnType<typeof setTimeout> | null = null

const applyZoom = (newZoom: number) => {
    zoom.value = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, +newZoom.toFixed(2)))
    const newRenderScale = getRenderScale(zoom.value)
    if (newRenderScale !== imgRenderScale.value) {
        if (zoomTimer) clearTimeout(zoomTimer)
        zoomTimer = setTimeout(() => renderPage(currentPage.value - 1), 200)
    }
}

const handleZoomIn = () => applyZoom(zoom.value + ZOOM_STEP)
const handleZoomOut = () => applyZoom(zoom.value - ZOOM_STEP)

const handleWheel = (e: WheelEvent) => {
    if (!e.ctrlKey) return
    e.preventDefault()
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP
    applyZoom(zoom.value + delta)
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const path = fileInfo.value?.path
    if (!path) return

    try {
        const meta = await invoke<PdfMeta>('pdf_meta', { path })
        pageCount.value = meta.page_count
        await renderPage(0)
    } catch (e) {
        error.value = String(e)
        loading.value = false
    }
})

onUnmounted(() => {
    if (zoomTimer) clearTimeout(zoomTimer)
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="book">
            <div class="book-toolbar">
                <el-button
                    text
                    :disabled="currentPage <= 1"
                    @click="handlePrev"
                >
                    &lsaquo;
                </el-button>
                <el-input
                    v-model.number="pageInput"
                    size="small"
                    style="width: 54px"
                    @keydown.enter="handleJump"
                />
                <span class="book-toolbar-total">/ {{ pageCount }}</span>
                <el-button
                    text
                    :disabled="currentPage >= pageCount"
                    @click="handleNext"
                >
                    &rsaquo;
                </el-button>
                <el-divider direction="vertical" />
                <el-button text size="small" :disabled="zoom <= ZOOM_MIN" @click="handleZoomOut">－</el-button>
                <span class="book-toolbar-zoom">{{ zoomText }}</span>
                <el-button text size="small" :disabled="zoom >= ZOOM_MAX" @click="handleZoomIn">＋</el-button>
            </div>
            <div class="book-content" @wheel="handleWheel">
                <div v-if="error" class="book-error">{{ error }}</div>
                <div v-else-if="loading && !currentImgSrc" class="book-loading">
                    <el-icon class="is-loading" size="32">
                        <svg viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
                            <path d="M512 64a448 448 0 1 1 0 896A448 448 0 0 1 512 64zm0 64a384 384 0 1 0 0 768A384 384 0 0 0 512 128zm0 80a304 304 0 0 1 0 608A304 304 0 0 1 512 208z" fill="currentColor" opacity=".2" />
                            <path d="M512 208a304 304 0 0 1 304 304h-64a240 240 0 0 0-240-240V208z" fill="currentColor" />
                        </svg>
                    </el-icon>
                </div>
                <el-scrollbar v-else class="book-scroll">
                    <div class="book-page">
                        <img
                            v-if="currentImgSrc"
                            :src="currentImgSrc"
                            :style="imgStyle"
                            class="book-page-img"
                            alt="PDF page"
                            draggable="false"
                            @load="handleImgLoad"
                        />
                        <div v-if="loading" class="book-page-overlay">
                            <el-icon class="is-loading" size="24">
                                <svg viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
                                    <path d="M512 64a448 448 0 1 1 0 896A448 448 0 0 1 512 64zm0 64a384 384 0 1 0 0 768A384 384 0 0 0 512 128zm0 80a304 304 0 0 1 0 608A304 304 0 0 1 512 208z" fill="currentColor" opacity=".2" />
                                    <path d="M512 208a304 304 0 0 1 304 304h-64a240 240 0 0 0-240-240V208z" fill="currentColor" />
                                </svg>
                            </el-icon>
                        </div>
                    </div>
                </el-scrollbar>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.book {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;

    &-toolbar {
        flex: 0 0 40px;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
        background-color: var(--color-bg);
        color: var(--color-text-primary);
        padding: 0 24px;
        font-size: 14px;

        &-total {
            color: var(--color-text-primary);
        }

        &-zoom {
            min-width: 40px;
            text-align: center;
            color: var(--color-text-primary);
            font-variant-numeric: tabular-nums;
        }
    }

    &-content {
        flex: 1;
        min-height: 0;
        display: flex;
        justify-content: center;
        align-items: center;
        background-color: #efefef;
    }

    &-loading {
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--el-color-primary);
    }

    &-error {
        color: var(--el-color-danger);
        padding: 16px;
        font-size: 14px;
    }

    &-scroll {
        width: 100%;
        height: 100%;
    }

    &-page {
        display: flex;
        justify-content: center;
        padding: 16px;
        position: relative;
        min-height: 100%;
        // 允许图片超出容器宽度时横向滚动
        min-width: min-content;

        &-img {
            height: auto;
            box-shadow: 0 2px 12px rgba(0, 0, 0, 0.15);
            background-color: #fff;
            display: block;
            user-select: none;
        }

        &-overlay {
            position: absolute;
            inset: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            background-color: rgba(239, 239, 239, 0.6);
            color: var(--el-color-primary);
        }
    }
}
</style>


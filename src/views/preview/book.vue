<script setup lang="ts">
import { ref, shallowRef, onMounted } from 'vue'
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

const route = useRoute()
const fileInfo = ref<FileInfo>()
const loading = ref(true)
const error = ref<string>()
const pageCount = ref(0)
const currentPage = ref(1)
// 已渲染页的 base64 DataURL 缓存，key 为 0-based 页码
const pageCache = shallowRef<Map<number, string>>(new Map())
const currentImgSrc = ref<string>()
const pageInput = ref(1)

const renderPage = async (pageIndex: number) => {
    const path = fileInfo.value?.path
    if (!path) return

    if (pageCache.value.has(pageIndex)) {
        currentImgSrc.value = pageCache.value.get(pageIndex)
        return
    }

    loading.value = true
    try {
        const result = await invoke<PdfPageResult>('pdf_render_page', {
            path,
            page: pageIndex,
            scale: 2.0,
        })
        const dataUrl = `data:image/png;base64,${result.base64}`
        pageCache.value.set(pageIndex, dataUrl)
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
            </div>
            <div class="book-content">
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
                            class="book-page-img"
                            alt="PDF page"
                            draggable="false"
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

        &-img {
            max-width: 100%;
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


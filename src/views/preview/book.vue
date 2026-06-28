<!--
  book.vue - epub/mobi 电子书预览
  epub: 通过 Rust 解析章节结构，按章节展示 HTML
  mobi: 通过 Rust 解析完整 HTML 内容，单页展示
-->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { Document, Menu } from '@element-plus/icons-vue'

defineOptions({
    name: 'BookSupport',
})

// ── 类型 ──────────────────────────────────────

interface EpubChapter {
    index: number
    title: string
    level: number
}

interface EpubInfo {
    title: string
    author: string
    language: string
    total_chapters: number
    chapters: EpubChapter[]
}

interface MobiInfo {
    title: string
    author: string
    description: string
    has_cover: boolean
}

// ── 状态 ──────────────────────────────────────

const route = useRoute()
const fileInfo = ref<FileInfo>()
const bookType = ref<'epub' | 'mobi'>('epub')

// epub 状态
const epubInfo = ref<EpubInfo | null>(null)
const currentChapterIndex = ref(0)
const chapterHtml = ref('')

// mobi 状态
const mobiInfo = ref<MobiInfo | null>(null)
const mobiHtml = ref('')

// UI 状态
const sidebarVisible = ref(true)
const loading = ref(false)
const error = ref('')
const iframeRef = ref<HTMLIFrameElement>()

// ── 工具函数 ──────────────────────────────────

function detectBookType(path: string): 'epub' | 'mobi' {
    const ext = path.split('.').pop()?.toLowerCase()
    return ext === 'mobi' ? 'mobi' : 'epub'
}

// ── epub 逻辑 ─────────────────────────────────

async function loadEpubInfo(path: string) {
    loading.value = true
    error.value = ''
    try {
        epubInfo.value = await invoke<EpubInfo>('get_epub_info', { path })
        if (epubInfo.value.chapters.length > 0) {
            await loadEpubChapter(path, 0)
        }
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
    }
}

async function loadEpubChapter(path: string, index: number) {
    loading.value = true
    error.value = ''
    try {
        chapterHtml.value = await invoke<string>('get_epub_chapter', {
            path,
            chapterIndex: index,
        })
        console.log('loadEpubChapter', index, chapterHtml.value)

        currentChapterIndex.value = index
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
        await nextTick()
        updateIframeSrcdoc()
    }
}

// ── mobi 逻辑 ─────────────────────────────────

async function loadMobiInfo(path: string) {
    loading.value = true
    error.value = ''
    try {
        mobiInfo.value = await invoke<MobiInfo>('get_mobi_info', { path })
        await loadMobiContent(path)
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
    }
}

async function loadMobiContent(path: string) {
    loading.value = true
    error.value = ''
    try {
        mobiHtml.value = await invoke<string>('get_mobi_content', { path })
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
        await nextTick()
        updateIframeSrcdoc()
    }
}

// ── iframe 渲染 ───────────────────────────────

function updateIframeSrcdoc() {
    const el = iframeRef.value
    if (!el) return
    const html = bookType.value === 'epub' ? chapterHtml.value : mobiHtml.value

    console.log('updateIframeSrcdoc', html)
    el.srcdoc = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <style>
                body { margin: 0; padding: 16px; font-family: serif; line-height: 1.8; color: #333; }
                img { max-width: 100%; height: auto; }
                h1, h2, h3, h4 { margin-top: 1em; }
            </style>
        </head>
        <body>${html || '<p style="color:#999;text-align:center;margin-top:40px">暂无内容</p>'}</body>
        </html>
    `
}

// ── 事件处理 ──────────────────────────────────

function handleNodeClick(data: EpubChapter) {
    if (bookType.value === 'epub' && fileInfo.value) {
        loadEpubChapter(fileInfo.value.path, data.index)
    }
}

// ── 生命周期 ──────────────────────────────────

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    if (!fileInfo.value?.path) {
        error.value = '未指定文件路径'
        return
    }

    bookType.value = detectBookType(fileInfo.value.path)

    if (bookType.value === 'epub') {
        await loadEpubInfo(fileInfo.value.path)
    } else {
        await loadMobiInfo(fileInfo.value.path)
    }
})

onUnmounted(() => {
    // 清理
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="book">
            <!-- 工具栏 -->
            <div class="book-toolbar">
                <div class="book-toolbar__left">
                    <el-link :underline="false" @click="sidebarVisible = !sidebarVisible">
                        <el-icon size="18px"><Menu /></el-icon>
                    </el-link>
                </div>
                <div class="book-toolbar__center">
                    <el-icon><Document /></el-icon>
                    <span class="book-toolbar__title">
                        {{ bookType === 'epub' ? epubInfo?.title : mobiInfo?.title || '电子书' }}
                    </span>
                    <span class="book-toolbar__author" v-if="bookType === 'epub'">
                        — {{ epubInfo?.author }}
                    </span>
                    <span class="book-toolbar__author" v-else>
                        — {{ mobiInfo?.author }}
                    </span>
                </div>
                <div class="book-toolbar__right"></div>
            </div>

            <div class="book-body">
                <!-- 侧边栏：epub 目录 / mobi 无 -->
                <div class="book-sidebar" v-if="sidebarVisible && bookType === 'epub'">
                    <el-scrollbar>
                        <div
                            v-for="ch in epubInfo?.chapters"
                            :key="ch.index"
                            class="book-sidebar__item"
                            :class="{ 'is-active': ch.index === currentChapterIndex }"
                            :style="{ paddingLeft: `${12 + ch.level * 16}px` }"
                            @click="handleNodeClick(ch)"
                        >
                            {{ ch.title }}
                        </div>
                    </el-scrollbar>
                </div>

                <!-- 内容区 -->
                <div class="book-content">
                    <div v-if="loading" class="book-content__loading">
                        加载中...
                    </div>
                    <div v-else-if="error" class="book-content__error">
                        {{ error }}
                    </div>
                    <iframe
                        v-else
                        ref="iframeRef"
                        class="book-content__iframe"
                    ></iframe>
                </div>
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
            gap: 6px;
            font-size: 13px;
        }

        &__title {
            font-weight: 500;
        }

        &__author {
            color: var(--color-text-secondary);
        }
    }

    &-body {
        display: flex;
        flex: 1;
        overflow: hidden;
    }

    &-sidebar {
        width: 260px;
        height: 100%;
        overflow: auto;
        box-shadow: 1px 0 2px rgba(0, 0, 0, 0.1);
        background-color: var(--color-bg);
        color: var(--color-text-primary);
        font-size: 13px;
        flex-shrink: 0;

        &__item {
            padding: 8px 12px;
            cursor: pointer;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            border-left: 3px solid transparent;

            &:hover {
                background-color: var(--color-fill-light);
            }

            &.is-active {
                background-color: var(--color-primary-light-9);
                border-left-color: var(--color-primary);
                color: var(--color-primary);
            }
        }
    }

    &-content {
        flex: 1;
        overflow: hidden;
        display: flex;
        background-color: #f5f5f5;

        &__loading,
        &__error {
            display: flex;
            align-items: center;
            justify-content: center;
            width: 100%;
            color: #999;
            font-size: 14px;
        }

        &__error {
            color: var(--color-danger);
        }

        &__iframe {
            width: 100%;
            height: 100%;
            border: none;
            background: white;
        }
    }
}
</style>

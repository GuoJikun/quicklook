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
import { Document, Menu, Remove, Plus } from '@element-plus/icons-vue'

defineOptions({
    name: 'BookSupport',
})

// ── 类型 ──────────────────────────────────────

interface EpubChapter {
    index: number
    title: string
    file_name: string
    level: number
}

interface EpubInfo {
    title: string
    author: string
    language: string
    total_chapters: number
    chapters: EpubChapter[]
    cover_data?: string
}

interface MobiInfo {
    title: string
    author: string
    description: string
    is_html: boolean
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

// 字体大小调节
const fontSize = ref(16)
const fontSizes = [12, 14, 16, 18, 20, 22, 24]

// 请求序列号，用于丢弃过期响应（防止快速点击竞态）
let chapterLoadSeq = 0

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
            // 默认从第 0 章开始；封面通过侧边栏“封面”入口单独展示
            await loadEpubChapter(path, 0)
        }
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
    }
}

async function loadEpubChapter(path: string, index: number) {
    const seq = ++chapterLoadSeq
    loading.value = true
    error.value = ''
    try {
        const html = await invoke<string>('get_epub_chapter', {
            path,
            chapterIndex: index,
        })
        if (seq !== chapterLoadSeq) return // 丢弃过期响应
        chapterHtml.value = html
        currentChapterIndex.value = index
    } catch (e) {
        if (seq !== chapterLoadSeq) return
        error.value = (e as Error)?.message || String(e)
    } finally {
        if (seq !== chapterLoadSeq) return
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

// ── epub 链接解析 ───────────────────────────────

async function resolveEpubLink(
    path: string,
    currentIndex: number,
    href: string,
): Promise<[number, string | null] | null> {
    try {
        return await invoke<[number, string | null] | null>('resolve_epub_link', {
            path,
            currentChapterIndex: currentIndex,
            href,
        })
    } catch (e) {
        console.error('[book] resolveEpubLink invoke error:', e)
        return null
    }
}

// ── iframe 渲染 ───────────────────────────────

function updateIframeSrcdoc() {
    const el = iframeRef.value
    if (!el) return
    const html = bookType.value === 'epub' ? chapterHtml.value : mobiHtml.value

    const contentHtml = html || '<p style="color:#999;text-align:center;margin-top:40px;text-indent:0">暂无内容</p>'

    el.srcdoc = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <style>
                /* 基础排版 */
                body {
                    margin: 0;
                    padding: 24px 32px;
                    font-family: "Source Han Serif", "Noto Serif CJK", "SimSun", "Times New Roman", serif;
                    line-height: 1.9;
                    color: #2c2c2c;
                    max-width: 720px;
                    margin: 0 auto;
                    font-size: ${fontSize.value}px;
                    -webkit-font-smoothing: antialiased;
                }

                /* 段落 */
                p {
                    text-indent: 2em;
                    margin: 0.6em 0;
                    text-align: justify;
                }

                /* 标题 */
                h1 {
                    font-size: 1.8em;
                    text-align: center;
                    margin: 1.5em 0 0.8em;
                    font-weight: 600;
                    color: #1a1a1a;
                }
                h2 {
                    font-size: 1.4em;
                    margin: 1.2em 0 0.6em;
                    padding-bottom: 0.3em;
                    border-bottom: 1px solid #e8e8e8;
                    font-weight: 600;
                    color: #1a1a1a;
                }
                h3 {
                    font-size: 1.2em;
                    margin: 1em 0 0.5em;
                    font-weight: 600;
                    color: #1a1a1a;
                }

                /* 图片 */
                img {
                    max-width: 100%;
                    height: auto;
                    display: block;
                    margin: 1em auto;
                    border-radius: 4px;
                }

                /* 引用 */
                blockquote {
                    margin: 1em 0;
                    padding: 0.5em 1em;
                    border-left: 3px solid #d0d0d0;
                    color: #666;
                    background: #fafafa;
                    font-style: italic;
                }

                /* 列表 */
                ul, ol {
                    margin: 0.8em 0;
                    padding-left: 2em;
                }
                li {
                    margin: 0.3em 0;
                }

                /* 代码 */
                code {
                    font-family: "Consolas", "Source Code Pro", monospace;
                    background: #f5f5f5;
                    padding: 0.15em 0.4em;
                    border-radius: 3px;
                    font-size: 0.9em;
                }
                pre {
                    background: #f5f5f5;
                    padding: 1em;
                    border-radius: 4px;
                    overflow-x: auto;
                    line-height: 1.5;
                }
                pre code {
                    background: none;
                    padding: 0;
                }

                /* 表格 */
                table {
                    border-collapse: collapse;
                    width: 100%;
                    margin: 1em 0;
                }
                th, td {
                    border: 1px solid #ddd;
                    padding: 0.5em 0.8em;
                    text-align: left;
                }
                th {
                    background: #f5f5f5;
                    font-weight: 600;
                }

                /* 链接 */
                a {
                    color: #1a73e8;
                    text-decoration: none;
                }
                a:hover {
                    text-decoration: underline;
                }

                /* 分隔线 */
                hr {
                    border: none;
                    border-top: 1px solid #e0e0e0;
                    margin: 2em 0;
                }

                /* 强调 */
                strong {
                    font-weight: 600;
                    color: #1a1a1a;
                }
                em {
                    font-style: italic;
                }

                /* 脚注 */
                sup {
                    font-size: 0.75em;
                    vertical-align: super;
                    color: #666;
                }

                /* 清除浮动 */
                .clearfix::after {
                    content: "";
                    display: table;
                    clear: both;
                }
            </style>
        </head>
        <body>${contentHtml}</body>
        </html>
    `

    // iframe 加载完成后，从外部附加链接点击拦截
    el.onload = () => {
        try {
            const doc = el.contentDocument
            if (!doc) return
            doc.addEventListener('click', (e) => {
                const link = (e.target as HTMLElement).closest('a')
                if (!link) return
                const href = link.getAttribute('href')
                if (!href || href.startsWith('data:') || href.startsWith('javascript:')) return
                e.preventDefault()
                e.stopPropagation()
                console.log('[book] iframe link intercepted:', href)
                handleIframeLink(href)
            }, true)
        } catch (err) {
            console.warn('[book] failed to attach iframe click handler:', err)
        }
    }
}

// ── 事件处理 ──────────────────────────────────

function showCoverInContent() {
    if (!epubInfo.value?.cover_data) return
    currentChapterIndex.value = -1
    chapterHtml.value = `<div style="text-align:center; padding: 20px;">
        <img src="${epubInfo.value.cover_data}" alt="封面" style="max-width: 100%; max-height: 80vh; border-radius: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.2);">
    </div>`
    updateIframeSrcdoc()
}

function handleNodeClick(data: EpubChapter) {
    if (bookType.value === 'epub' && fileInfo.value) {
        loadEpubChapter(fileInfo.value.path, data.index)
    }
}

// 章节导航
function prevChapter() {
    if (bookType.value !== 'epub' || !fileInfo.value) return
    if (currentChapterIndex.value > 0) {
        loadEpubChapter(fileInfo.value.path, currentChapterIndex.value - 1)
    }
}

function nextChapter() {
    if (bookType.value !== 'epub' || !fileInfo.value) return
    const total = epubInfo.value?.chapters.length || 0
    if (currentChapterIndex.value < total - 1) {
        loadEpubChapter(fileInfo.value.path, currentChapterIndex.value + 1)
    }
}

// 字体大小调节
function changeFontSize(delta: number) {
    const idx = fontSizes.indexOf(fontSize.value)
    const newIdx = Math.max(0, Math.min(fontSizes.length - 1, idx + delta))
    fontSize.value = fontSizes[newIdx]
    updateIframeFontSize()
}

function updateIframeFontSize() {
    const doc = iframeRef.value?.contentDocument
    if (doc?.body) {
        doc.body.style.fontSize = `${fontSize.value}px`
    }
}

// 键盘导航
function handleKeydown(e: KeyboardEvent) {
    // 忽略在输入框中的按键
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return

    switch (e.key) {
        case 'ArrowLeft':
            e.preventDefault()
            prevChapter()
            break
        case 'ArrowRight':
            e.preventDefault()
            nextChapter()
            break
        case '+':
        case '=':
            if (e.ctrlKey || e.metaKey) {
                e.preventDefault()
                changeFontSize(1)
            }
            break
        case '-':
            if (e.ctrlKey || e.metaKey) {
                e.preventDefault()
                changeFontSize(-1)
            }
            break
    }
}

// ── 生命周期 ──────────────────────────────────

// ── iframe 链接拦截 ──────────────────────────────

function handleIframeLink(href: string) {
    console.log('[book] iframe link intercepted:', href)
    if (!href || !fileInfo.value) return

    // 解析 href: 可能是 "chapter.xhtml" 或 "chapter.xhtml#fragment"
    const [chapterPath] = href.split('#')

    // 如果是当前章节内的锚点（无路径变更），直接滚动
    if (!chapterPath || chapterPath === '') {
        const fragment = href.split('#')[1]
        if (fragment) {
            const doc = iframeRef.value?.contentDocument
            if (doc) {
                const target = doc.getElementById(fragment)
                if (target) {
                    target.scrollIntoView()
                    return
                }
            }
        }
        return
    }

    // 使用 Rust 端精确解析链接
    const currentIndex = currentChapterIndex.value
    resolveEpubLink(fileInfo.value.path, currentIndex, href).then(result => {
        console.log('[book] resolveEpubLink result:', result)
        if (!result) return

        const [targetIndex, fragment] = result
        console.log('[book] targetIndex:', targetIndex, 'fragment:', fragment)

        // 如果是当前章节内的锚点，直接滚动
        if (targetIndex === currentIndex && fragment) {
            const doc = iframeRef.value?.contentDocument
            if (doc) {
                const target = doc.getElementById(fragment)
                if (target) {
                    target.scrollIntoView()
                    return
                }
            }
        }

        // 加载目标章节
        const path = fileInfo.value?.path
        if (!path) return
        loadEpubChapter(path, targetIndex).then(() => {
            if (fragment) {
                // 多次尝试定位，等待 iframe 内容完全渲染
                const tryScroll = (attempts: number) => {
                    const doc = iframeRef.value?.contentDocument
                    if (doc) {
                        const target = doc.getElementById(fragment)
                        if (target) {
                            target.scrollIntoView()
                            return
                        }
                    }
                    if (attempts > 0) {
                        requestAnimationFrame(() => tryScroll(attempts - 1))
                    }
                }
                requestAnimationFrame(() => tryScroll(5))
            }
        })
    }).catch(err => {
        console.error('[book] resolveEpubLink error:', err)
    })
}

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

    // 添加键盘事件监听
    document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
    document.removeEventListener('keydown', handleKeydown)
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
                    <span class="book-toolbar__author" v-if="bookType === 'epub'"> — {{ epubInfo?.author }} </span>
                    <span class="book-toolbar__author" v-else> — {{ mobiInfo?.author }} </span>
                </div>
                <div class="book-toolbar__right">
                    <template v-if="bookType === 'epub'">
                        <el-link :underline="false" @click="changeFontSize(-1)" title="缩小字体 (Ctrl+-)">
                            <el-icon size="16px"><Remove /></el-icon>
                        </el-link>
                        <span class="book-toolbar__font-size">{{ fontSize }}px</span>
                        <el-link :underline="false" @click="changeFontSize(1)" title="放大字体 (Ctrl++)">
                            <el-icon size="16px"><Plus /></el-icon>
                        </el-link>
                    </template>
                </div>
            </div>

            <div class="book-body">
                <!-- 侧边栏：epub 目录 / mobi 无 -->
                <div class="book-sidebar" v-if="sidebarVisible && bookType === 'epub'">
                    <el-scrollbar>
                        <!-- 封面 -->
                        <div
                            v-if="epubInfo?.cover_data"
                            class="book-sidebar__item"
                            :class="{ 'is-active': currentChapterIndex === -1 }"
                            @click="showCoverInContent"
                        >封面</div>
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
                    <!-- 骨架屏 -->
                    <div v-if="loading" class="book-content__skeleton">
                        <div class="skeleton-header"></div>
                        <div class="skeleton-line" style="width: 90%"></div>
                        <div class="skeleton-line" style="width: 100%"></div>
                        <div class="skeleton-line" style="width: 85%"></div>
                        <div class="skeleton-line" style="width: 95%"></div>
                        <div class="skeleton-line" style="width: 70%"></div>
                        <div class="skeleton-line" style="width: 88%"></div>
                    </div>
                    <div v-else-if="error" class="book-content__error">
                        {{ error }}
                    </div>
                    <iframe v-else ref="iframeRef" class="book-content__iframe"></iframe>
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
            display: flex;
            align-items: center;
            justify-content: flex-end;
            gap: 4px;
        }

        &__font-size {
            font-size: 12px;
            color: var(--color-text-secondary);
            min-width: 36px;
            text-align: center;
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

        &__cover {
            padding: 16px 12px;
            border-bottom: 1px solid var(--color-border-light);
            text-align: center;

            img {
                width: 100%;
                max-height: 200px;
                object-fit: contain;
                border-radius: 4px;
                box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
            }

            &-label {
                display: block;
                margin-top: 8px;
                font-size: 12px;
                color: #666;
            }
        }
    }

    &-content {
        flex: 1;
        overflow: hidden;
        display: flex;
        background-color: #f5f5f5;

        &__loading,
        &__skeleton {
            padding: 32px;
            width: 100%;

            .skeleton-header {
                width: 60%;
                height: 24px;
                background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
                background-size: 200% 100%;
                animation: skeleton-pulse 1.5s ease-in-out infinite;
                border-radius: 4px;
                margin-bottom: 24px;
            }

            .skeleton-line {
                height: 14px;
                background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
                background-size: 200% 100%;
                animation: skeleton-pulse 1.5s ease-in-out infinite;
                border-radius: 3px;
                margin-bottom: 12px;
            }
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

@keyframes skeleton-pulse {
    0% {
        background-position: 200% 0;
    }
    100% {
        background-position: -200% 0;
    }
}
</style>

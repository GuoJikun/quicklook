<!--
  book.vue - epub 电子书预览
  epub: 通过 Rust 解析章节结构，按章节展示 HTML

  三层结构与其余预览统一：标题行由 LayoutPreview 提供，命令栏放
  「目录 │ 字号 │ 底色」，右区只放阅读进度；正文为 680px 单列 iframe，
  底部 3px 横向进度条。原 book-toolbar 里的书名/作者移到目录抽屉顶部。
 -->
<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { List16Regular } from '@vicons/fluent'
import LayoutPreview from '@/components/layout-preview.vue'
import { useTheme } from '@/hooks/theme'
import type { FileInfo } from '@/utils/typescript'
import { getConfigStore } from '@/utils/store'
// 命令栏控件、目录抽屉与错态的样式独立成文件，便于与 Markdown / PDF 预览对齐同一套规格
import './book-doc.scss'

defineOptions({
    name: 'BookSupport',
})

const { t } = useI18n()
const route = useRoute()
const { isDark } = useTheme()

// ── 类型 ──────────────────────────────────────

interface EpubChapter {
    index: number
    title: string
    file_name: string
    level: number
    fragment?: string | null
}

interface EpubInfo {
    title: string
    author: string
    language: string
    total_chapters: number
    chapters: EpubChapter[]
    cover_data?: string
}

// ── 常量 ──────────────────────────────────────

const STORE_TOC = 'previewBookTocOpen'
const STORE_FONT = 'previewBookFontSize'
const STORE_PAPER = 'previewBookPaper'

/** 字号六档，与设计稿 14–24px 一致；与整体 UI 字号解耦，只作用于阅读区 */
const FONT_STEPS = [14, 16, 18, 20, 22, 24]
const FONT_MIN = FONT_STEPS[0]
const FONT_MAX = FONT_STEPS[FONT_STEPS.length - 1]

type PaperId = 'light' | 'sepia' | 'dark'

/**
 * 阅读底色的三档纸面。取值写死而非取 CSS 变量：这些颜色要注入 iframe 的
 * srcdoc 里，iframe 拿不到父窗口的 --color-*，且纸面是「纸张」而非「界面」，
 * 不应跟随主题变量变化。
 */
const PAPERS: Record<PaperId, { bg: string; fg: string; muted: string; border: string; soft: string; link: string; quote: string; swatch: string }> = {
    light: {
        bg: '#ffffff',
        fg: '#2c2c2c',
        muted: '#6b6b6b',
        border: '#e6e6e6',
        soft: '#f5f5f5',
        link: '#1a73e8',
        quote: '#fafafa',
        swatch: '#ffffff',
    },
    sepia: {
        bg: '#f2e6cf',
        fg: '#3a3327',
        muted: '#7d7057',
        border: '#ddcfb2',
        soft: '#eadfc6',
        link: '#8a5a2b',
        quote: '#eee1c6',
        swatch: '#f2e6cf',
    },
    dark: {
        bg: '#1d1d1f',
        fg: '#d9d9de',
        muted: '#9a9aa4',
        border: '#35353c',
        soft: '#26262b',
        link: '#7ab0ff',
        quote: '#242429',
        swatch: '#3a3a41',
    },
}

const PAPER_LIST: Array<{ id: PaperId; label: string }> = [
    { id: 'light', label: 'book.paperLight' },
    { id: 'sepia', label: 'book.paperSepia' },
    { id: 'dark', label: 'book.paperDark' },
]

// ── 状态 ──────────────────────────────────────

const fileInfo = ref<FileInfo>()

const epubInfo = ref<EpubInfo | null>(null)
const currentChapterIndex = ref(0)
const currentFragment = ref<string | null>(null)
const chapterHtml = ref('')

const tocOpen = ref(true)
const loading = ref(false)
const error = ref('')
const iframeRef = ref<HTMLIFrameElement>()

const fontSize = ref(16)
// 首次进入跟随系统主题；用户选过底色后由存储值覆盖并停止跟随
const paper = ref<PaperId>(isDark.value ? 'dark' : 'light')
/** 用户显式选过底色后就不再跟随系统主题 */
const paperChosen = ref(false)

/** 全书进度 0–1：按 (章序号 + 章内滚动比例) / 总章数 估算 */
const progress = ref(0)
let scrollRatio = 0

type PendingScrollTarget = { kind: 'top' } | { kind: 'fragment'; fragment: string }
const pendingScrollTarget = ref<PendingScrollTarget | null>(null)

// 请求序列号，用于丢弃过期响应（防止快速点击竞态）
let chapterLoadSeq = 0
// 组件卸载标志，用于中断 rAF 递归链
let disposed = false

// ── 计算属性 ──────────────────────────────────

const hasTocEntries = computed(() => (epubInfo.value?.chapters.length ?? 0) > 0)

/** 右区状态：「第 N 章 · 全书 42%」，封面视图只显示「封面」 */
const progressText = computed(() => {
    if (currentChapterIndex.value < 0) return t('book.cover')
    return t('book.chapterProgress', {
        n: currentChapterIndex.value + 1,
        pct: Math.round(progress.value * 100),
    })
})

const paperId = computed<PaperId>(() => paper.value)

// ── 偏好读写 ──────────────────────────────────

const persist = async (key: string, value: unknown) => {
    try {
        const store = await getConfigStore()
        await store.set(key, value)
        await store.save()
    } catch (e) {
        // 记忆项失败不影响预览本身
        console.warn(`persist book preference failed: ${key}`, e)
    }
}

const restorePreferences = async () => {
    try {
        const store = await getConfigStore()
        const size = await store.get<number>(STORE_FONT)
        if (typeof size === 'number' && size >= FONT_MIN && size <= FONT_MAX) fontSize.value = size
        const storedPaper = await store.get<string>(STORE_PAPER)
        if (storedPaper === 'light' || storedPaper === 'sepia' || storedPaper === 'dark') {
            paper.value = storedPaper
            paperChosen.value = true
        }
        const toc = await store.get<boolean>(STORE_TOC)
        if (typeof toc === 'boolean') tocOpen.value = toc
    } catch (e) {
        console.warn('read book preference failed:', e)
    }
}

// ── 命令栏动作 ────────────────────────────────

const toggleToc = () => {
    tocOpen.value = !tocOpen.value
    void persist(STORE_TOC, tocOpen.value)
}

const stepFont = (delta: number) => {
    const idx = FONT_STEPS.indexOf(fontSize.value)
    // 存量配置可能不是标准档位，先归到最近的档再走一步
    const base = idx === -1 ? FONT_STEPS.findIndex(s => s >= fontSize.value) : idx
    const next = FONT_STEPS[Math.min(FONT_STEPS.length - 1, Math.max(0, base + delta))]
    if (next === fontSize.value) return
    fontSize.value = next
    void persist(STORE_FONT, next)
}

const setPaper = (next: PaperId) => {
    if (next === paper.value) return
    paper.value = next
    paperChosen.value = true
    void persist(STORE_PAPER, next)
}

// ── epub 逻辑 ─────────────────────────────────

async function loadBook() {
    const path = fileInfo.value?.path
    if (!path) return
    loading.value = true
    error.value = ''
    try {
        epubInfo.value = await invoke<EpubInfo>('get_epub_info', { path })
        if (epubInfo.value.chapters.length > 0) {
            // 默认从第 0 章开始；封面通过目录里的「封面」入口单独展示
            await loadEpubChapter(path, 0)
        } else {
            chapterHtml.value = ''
            updateIframeSrcdoc()
        }
    } catch (e) {
        error.value = (e as Error)?.message || String(e)
    } finally {
        loading.value = false
    }
}

async function loadEpubChapter(path: string, index: number, fragment: string | null = null) {
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
        currentFragment.value = fragment
        // 换章后阅读位置回到顶部，进度也回到该章起点
        scrollRatio = 0
        updateProgress()
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

// ── 进度 ──────────────────────────────────────

const updateProgress = () => {
    const total = epubInfo.value?.total_chapters || 0
    if (!total || currentChapterIndex.value < 0) {
        progress.value = 0
        return
    }
    progress.value = Math.min(1, Math.max(0, (currentChapterIndex.value + scrollRatio) / total))
}

function onIframeScroll() {
    const doc = iframeRef.value?.contentDocument
    if (!doc) return
    const el = doc.scrollingElement || doc.documentElement
    if (!el) return
    const max = el.scrollHeight - el.clientHeight
    scrollRatio = max > 0 ? Math.min(1, Math.max(0, el.scrollTop / max)) : 0
    updateProgress()
}

// ── iframe 渲染 ───────────────────────────────

function scrollIframeToTarget(target: PendingScrollTarget | null, attempts = 8) {
    const doc = iframeRef.value?.contentDocument
    if (!doc) {
        if (attempts > 0) {
            requestAnimationFrame(() => {
                if (disposed) return
                scrollIframeToTarget(target, attempts - 1)
            })
        }
        return
    }

    if (!target) return

    if (target.kind === 'top') {
        const container = doc.documentElement || doc.body
        if (container) {
            const start = container.scrollTop || 0
            const distance = -start
            const duration = 320
            const startTime = performance.now()
            const step = (now: number) => {
                if (disposed) return
                const elapsed = now - startTime
                const progress = Math.min(elapsed / duration, 1)
                const eased = 1 - Math.pow(1 - progress, 3)
                const current = start + distance * eased
                if (doc.documentElement) doc.documentElement.scrollTop = current
                if (doc.body) doc.body.scrollTop = current
                if (progress < 1) {
                    requestAnimationFrame(step)
                }
            }
            requestAnimationFrame(step)
        }
        return
    }

    const targetEl = doc.getElementById(target.fragment)
    if (targetEl) {
        // 章内锚点用即时跳转：平滑滚动在部分渲染管线里会被跳过，
        // 跨章跳转本来也是「换页」语义，直接定位更符合预期
        targetEl.scrollIntoView({ block: 'start' })
    } else if (attempts > 0) {
        requestAnimationFrame(() => {
            if (disposed) return
            scrollIframeToTarget(target, attempts - 1)
        })
    }
}

/**
 * 整页只有一张图/一个 svg 的章节（封面、插图页）：按可视区等比缩放并居中。
 * Calibre 的封面写成 <svg width="100%" height="100%" viewBox="0 0 768 1024">，
 * 浏览器会把高度按 viewBox 比例撑到列宽的 1.33 倍，封面因此被裁掉一截还要竖着滚。
 */
function fitMediaPage(doc: Document) {
    const body = doc.body
    if (!body) return

    const media = body.querySelectorAll('img, svg, video')
    if (media.length !== 1) return
    const inner = media[0]
    if (Array.from(body.childNodes).some(node => hasTextOutside(node, inner))) return

    // 分隔符、小花之类的装饰图也算「单独一张图」，但它们该保持原尺寸，不该铺满整页
    if (mediaLongestSide(inner) < 320) return

    // preserveAspectRatio="none" 会让盒子比例一变就把图拉伸，改回等比
    if (inner.tagName.toLowerCase() === 'svg') inner.setAttribute('preserveAspectRatio', 'xMidYMid meet')
    body.classList.add('is-cover')
}

/**
 * 媒体之外是否还有正文文字。
 * Calibre 会在 <svg> 里塞 <title>/<desc> 做无障碍说明（实测 126 字符），
 * 那是文字但不是正文，直接看 body.textContent 会把封面误判成普通章节。
 */
function hasTextOutside(node: Node, media: Element): boolean {
    if (node.nodeType === 3) return (node.nodeValue ?? '').trim().length > 0
    if (node.nodeType !== 1) return false
    if (node === media) return false
    const tag = (node as Element).tagName.toLowerCase()
    if (tag === 'title' || tag === 'desc' || tag === 'metadata' || tag === 'style' || tag === 'script') return false
    return Array.from(node.childNodes).some(child => hasTextOutside(child, media))
}

/** 取媒体的原始最长边；拿不到尺寸（还没解码）时返回 0，调用方按「不放宽」处理 */
function mediaLongestSide(el: Element) {
    const tag = el.tagName.toLowerCase()
    if (tag === 'img') {
        const img = el as HTMLImageElement
        return Math.max(img.naturalWidth, img.naturalHeight)
    }
    if (tag === 'video') {
        const video = el as HTMLVideoElement
        return Math.max(video.videoWidth, video.videoHeight)
    }
    const svg = el as SVGSVGElement
    const vb = svg.viewBox?.baseVal
    if (vb && vb.width > 0 && vb.height > 0) return Math.max(vb.width, vb.height)
    return Math.max(parseFloat(svg.getAttribute('width') ?? '0') || 0, parseFloat(svg.getAttribute('height') ?? '0') || 0)
}

/** 把当前底色与字号写进 iframe 的 CSS 变量：不重载文档，阅读位置不丢 */
function applyReadingVars() {
    const doc = iframeRef.value?.contentDocument
    if (!doc) return
    const p = PAPERS[paper.value]
    const root = doc.documentElement
    root.style.setProperty('--bk-bg', p.bg)
    root.style.setProperty('--bk-fg', p.fg)
    root.style.setProperty('--bk-muted', p.muted)
    root.style.setProperty('--bk-border', p.border)
    root.style.setProperty('--bk-soft', p.soft)
    root.style.setProperty('--bk-link', p.link)
    root.style.setProperty('--bk-quote', p.quote)
    root.style.setProperty('--bk-fs', `${fontSize.value}px`)
}

function buildSrcdoc() {
    const p = PAPERS[paper.value]
    const contentHtml =
        chapterHtml.value ||
        `<p style="color:${p.muted};text-align:center;margin-top:40px;text-indent:0">${t('book.empty')}</p>`

    // 颜色与字号全部走变量，切换底色/字号时由 applyReadingVars 直接改变量
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
    :root {
        --bk-bg: ${p.bg};
        --bk-fg: ${p.fg};
        --bk-muted: ${p.muted};
        --bk-border: ${p.border};
        --bk-soft: ${p.soft};
        --bk-link: ${p.link};
        --bk-quote: ${p.quote};
        --bk-fs: ${fontSize.value}px;
    }

    html, body {
        background: var(--bk-bg);
        color: var(--bk-fg);
    }

    body {
        box-sizing: border-box;
        /* 正文列固定 680px，居中；两侧 24px 是最窄窗口下的最小留白 */
        max-width: 728px;
        margin: 0 auto;
        padding: 30px 24px 56px;
        font-family: "Helvetica Neue", Helvetica, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "微软雅黑", Arial, sans-serif;
        font-size: var(--bk-fs);
        line-height: 1.8;
        -webkit-font-smoothing: antialiased;
    }

    /* 整页只有一张图/一个 svg 的章节（封面、插图页）：把媒体塞进可视区并等比留白，
       否则 Calibre 那种 <svg width="100%" height="100%" viewBox=768x1024> 会按
       列宽撑出 1.33 倍高度，封面被裁掉一截还要竖着滚。
       不用 flex 居中，是因为中间常裹着 <div class="calibre"> 之类的外壳，
       flex 下外壳高度是 auto，里面的 100% 高度就没有参照了。 */
    body.is-cover {
        box-sizing: border-box;
        max-width: none;
        height: 100vh;
        margin: 0;
        padding: 16px;
        overflow: hidden;
    }

    /* 先让外壳铺满视口，媒体才能按 100% 拿到明确的高度 */
    body.is-cover > *,
    body.is-cover div,
    body.is-cover figure,
    body.is-cover section {
        width: 100%;
        height: 100%;
        margin: 0;
        padding: 0;
    }

    body.is-cover img,
    body.is-cover video {
        display: block;
        width: 100%;
        height: 100%;
        margin: 0;
        object-fit: contain;
    }

    /* preserveAspectRatio 由 fitMediaPage 改成 xMidYMid meet，这里给满视口即可 */
    body.is-cover svg {
        display: block;
        width: 100%;
        height: 100%;
    }

    p {
        text-indent: 2em;
        margin: 0 0 14px;
        text-align: justify;
    }

    h1 {
        font-size: 1.6em;
        text-align: center;
        margin: 0.6em 0 1em;
        font-weight: 600;
    }
    h2 {
        font-size: 1.3em;
        margin: 1.4em 0 0.7em;
        padding-bottom: 0.3em;
        border-bottom: 1px solid var(--bk-border);
        font-weight: 600;
    }
    h3 {
        font-size: 1.15em;
        margin: 1.2em 0 0.6em;
        font-weight: 600;
    }

    img {
        max-width: 100%;
        height: auto;
        display: block;
        margin: 1em auto;
        border-radius: 4px;
    }

    /* 正文里的内联 svg 同样不能撑破列宽 */
    svg {
        max-width: 100%;
    }

    blockquote {
        margin: 1em 0;
        padding: 0.5em 1em;
        border-left: 3px solid var(--bk-border);
        color: var(--bk-muted);
        background: var(--bk-quote);
        font-style: italic;
    }

    ul, ol {
        margin: 0.8em 0;
        padding-left: 2em;
    }
    li {
        margin: 0.3em 0;
    }

    code {
        font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", "Courier New", monospace;
        background: var(--bk-soft);
        padding: 0.15em 0.4em;
        border-radius: 3px;
        font-size: 0.9em;
    }
    pre {
        background: var(--bk-soft);
        padding: 1em;
        border-radius: 4px;
        overflow-x: auto;
        line-height: 1.5;
    }
    pre code {
        background: none;
        padding: 0;
    }

    table {
        border-collapse: collapse;
        width: 100%;
        max-width: 100%;
        margin: 1em 0;
    }
    th, td {
        border: 1px solid var(--bk-border);
        padding: 0.5em 0.8em;
        text-align: left;
    }
    th {
        background: var(--bk-soft);
        font-weight: 600;
    }

    a {
        color: var(--bk-link);
        text-decoration: none;
    }
    a:hover {
        text-decoration: underline;
    }

    hr {
        border: none;
        border-top: 1px solid var(--bk-border);
        margin: 2em 0;
    }

    strong {
        font-weight: 600;
    }
    em {
        font-style: italic;
    }

    sup {
        font-size: 0.75em;
        vertical-align: super;
        color: var(--bk-muted);
    }

    /* 封面已经铺满视口，投影改成内侧的一圈描边感，避免大盒子投出奇怪的阴影 */
    body.is-cover img {
        border-radius: 0;
        box-shadow: none;
    }

    .clearfix::after {
        content: "";
        display: table;
        clear: both;
    }
</style>
</head>
<body>${contentHtml}</body>
</html>`
}

function updateIframeSrcdoc() {
    const el = iframeRef.value
    if (!el) return

    el.onload = () => {
        try {
            const doc = el.contentDocument
            if (!doc) return
            // 换文档后变量归零，先把当前底色/字号写回去
            applyReadingVars()
            // 封面/插图这类整页媒体按可视区等比缩放，别让它们被裁掉还要竖着滚
            fitMediaPage(doc)
            doc.addEventListener('scroll', onIframeScroll, { passive: true, capture: true })
            doc.addEventListener('click', (e) => {
                const link = (e.target as HTMLElement).closest('a')
                if (!link) return
                const href = link.getAttribute('href')
                if (!href || href.startsWith('data:') || href.startsWith('javascript:')) return
                e.preventDefault()
                e.stopPropagation()
                handleIframeLink(href)
            }, true)
        } catch (err) {
            console.warn('[book] failed to attach iframe handlers:', err)
        }

        requestAnimationFrame(() => {
            if (disposed) return
            scrollIframeToTarget(pendingScrollTarget.value)
            pendingScrollTarget.value = null
            onIframeScroll()
        })
    }

    el.srcdoc = buildSrcdoc()
}

// ── 事件处理 ──────────────────────────────────

function showCoverInContent() {
    if (!epubInfo.value?.cover_data) return
    currentChapterIndex.value = -1
    currentFragment.value = null
    scrollRatio = 0
    updateProgress()
    chapterHtml.value = `<img src="${epubInfo.value.cover_data}" alt="${t('book.cover')}">`
    updateIframeSrcdoc()
}

async function handleNodeClick(data: EpubChapter) {
    if (!fileInfo.value) return

    const targetIndex = data.index
    const targetFragment = data.fragment

    if (targetFragment) {
        currentFragment.value = targetFragment
        pendingScrollTarget.value = { kind: 'fragment', fragment: targetFragment }
    } else {
        currentFragment.value = null
        pendingScrollTarget.value = { kind: 'top' }
    }

    // 窄窗口下抽屉会压住正文，选完章节直接收起
    if (tocOpen.value && window.innerWidth < 900) toggleToc()

    if (currentChapterIndex.value === targetIndex) {
        if (iframeRef.value?.contentDocument) {
            scrollIframeToTarget(pendingScrollTarget.value)
            pendingScrollTarget.value = null
        }
        return
    }

    await loadEpubChapter(fileInfo.value.path, targetIndex, targetFragment)
}

// 章节导航
function prevChapter() {
    if (!fileInfo.value) return
    if (currentChapterIndex.value > 0) {
        void loadEpubChapter(fileInfo.value.path, currentChapterIndex.value - 1)
    } else if (currentChapterIndex.value === -1) {
        // 封面视图：上一章回到第 0 章
        void loadEpubChapter(fileInfo.value.path, 0)
    }
}

function nextChapter() {
    if (!fileInfo.value) return
    // currentChapterIndex 是 spine 索引，边界必须用 total_chapters（spine 数量），
    // 不能用 chapters.length（TOC 条目数，可含同文件多锚点）
    const total = epubInfo.value?.total_chapters || 0
    if (currentChapterIndex.value >= 0 && currentChapterIndex.value < total - 1) {
        void loadEpubChapter(fileInfo.value.path, currentChapterIndex.value + 1)
    }
}

// 键盘导航
function handleKeydown(e: KeyboardEvent) {
    // 抽屉打开时 Esc 先关抽屉，别把预览窗口关掉
    if (e.key === 'Escape') {
        if (!tocOpen.value) return
        e.stopPropagation()
        toggleToc()
        return
    }

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
                stepFont(1)
            }
            break
        case '-':
            if (e.ctrlKey || e.metaKey) {
                e.preventDefault()
                stepFont(-1)
            }
            break
    }
}

// ── iframe 链接拦截 ──────────────────────────────

function handleIframeLink(href: string) {
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
    const filePath = fileInfo.value.path
    void resolveEpubLink(filePath, currentIndex, href)
        .then(result => {
            if (!result) return

            const [targetIndex, fragment] = result

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
            return loadEpubChapter(filePath, targetIndex).then(() => {
                if (!fragment) return
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
                        requestAnimationFrame(() => {
                            if (disposed) return
                            tryScroll(attempts - 1)
                        })
                    }
                }
                requestAnimationFrame(() => tryScroll(5))
            })
        })
        .catch(err => {
            console.error('[book] resolveEpubLink error:', err)
        })
}

// ── 生命周期 ──────────────────────────────────

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    if (!fileInfo.value?.path) {
        error.value = t('book.noPath')
        return
    }

    document.addEventListener('keydown', handleKeydown, true)
    await restorePreferences()
    await loadBook()
})

onBeforeUnmount(() => {
    disposed = true
    document.removeEventListener('keydown', handleKeydown, true)
})

// 底色/字号改变只改变量，不重载 iframe，阅读位置不丢
watch(paper, applyReadingVars)
watch(fontSize, applyReadingVars)

// 用户没选过底色时跟随系统主题，选过就尊重用户
watch(isDark, dark => {
    if (paperChosen.value) return
    paper.value = dark ? 'dark' : 'light'
})

// 同一窗口内换文件时路由 query 变化，组件不会重建，需要主动重读
watch(
    () => route.query.path,
    path => {
        if (typeof path === 'string' && path !== fileInfo.value?.path) {
            fileInfo.value = route.query as unknown as FileInfo
            chapterHtml.value = ''
            epubInfo.value = null
            void loadBook()
        }
    },
)
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading" :hide-toolbar="Boolean(error)">
        <template #toolbar>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="tocOpen"
                :title="t('book.toc')"
                @click="toggleToc"
            >
                <el-icon :size="15"><List16Regular /></el-icon>
                {{ t('book.toc') }}
            </button>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('book.fontSmaller')"
                :disabled="fontSize <= FONT_MIN"
                @click="stepFont(-1)"
            >
                A−
            </button>
            <span class="pv-tb-num">{{ fontSize }}</span>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('book.fontLarger')"
                :disabled="fontSize >= FONT_MAX"
                @click="stepFont(1)"
            >
                A+
            </button>

            <i class="pv-tb-sep"></i>

            <button
                v-for="item in PAPER_LIST"
                :key="item.id"
                class="pv-tb-btn bk-paper"
                type="button"
                :aria-pressed="paperId === item.id"
                :title="t(item.label)"
                :aria-label="t(item.label)"
                @click="setPaper(item.id)"
            >
                <i class="swatch" :style="{ background: PAPERS[item.id].swatch }"></i>
            </button>

            <div class="pv-tb-grow"></div>
            <span class="pv-tb-info">{{ progressText }}</span>
        </template>

        <div class="bk-pv" :class="{ 'is-toc': tocOpen && !error }">
            <div v-if="error" class="bk-pv-state">
                <p class="bk-pv-state-title">{{ t('preview.loadFailed') }}</p>
                <p class="bk-pv-state-detail">{{ error }}</p>
                <button class="bk-pv-state-btn" type="button" @click="loadBook">{{ t('preview.retry') }}</button>
            </div>

            <template v-else>
                <nav v-if="tocOpen" class="bk-pv-toc pv-scroll">
                    <div class="bk-pv-toc-head">
                        <p class="bk-pv-toc-title">{{ epubInfo?.title || t('book.ebook') }}</p>
                        <p v-if="epubInfo?.author" class="bk-pv-toc-author">{{ epubInfo.author }}</p>
                    </div>

                    <p v-if="!hasTocEntries" class="bk-pv-toc-empty">{{ t('book.tocEmpty') }}</p>
                    <template v-else>
                        <a
                            v-if="epubInfo?.cover_data"
                            class="bk-pv-toc-item"
                            :class="{ 'is-active': currentChapterIndex === -1 }"
                            href="#"
                            @click.prevent="showCoverInContent"
                        >
                            {{ t('book.cover') }}
                        </a>
                        <a
                            v-for="ch in epubInfo?.chapters"
                            :key="`${ch.index}-${ch.fragment || 'root'}`"
                            class="bk-pv-toc-item"
                            :class="[
                                `is-lv${Math.min(ch.level + 1, 3)}`,
                                {
                                    // 有锚点的条目按锚点高亮，无锚点的章条目只在未定位到锚点时高亮，
                                    // 否则点进小节后父级与子级会同时点亮
                                    'is-active':
                                        ch.index === currentChapterIndex &&
                                        (ch.fragment ? ch.fragment === currentFragment : !currentFragment),
                                },
                            ]"
                            href="#"
                            :title="ch.title"
                            @click.prevent="handleNodeClick(ch)"
                        >
                            {{ ch.title }}
                        </a>
                    </template>
                </nav>

                <div class="bk-pv-read">
                    <iframe ref="iframeRef" class="bk-pv-iframe"></iframe>
                    <div class="bk-pv-bar">
                        <i :style="{ width: `${progress * 100}%` }"></i>
                    </div>
                </div>
            </template>
        </div>
    </LayoutPreview>
</template>

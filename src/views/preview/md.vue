<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { Copy16Regular, List16Regular } from '@vicons/fluent'
import type { Store } from '@tauri-apps/plugin-store'
import type MarkdownIt from 'markdown-it'
import LayoutPreview from '@/components/layout-preview.vue'
import MdViewer from '@/components/md-viewer/index.vue'
import type { FileInfo } from '@/utils/typescript'
import { readTextFile } from '@/utils'
import { createMd } from '@/utils/markdown/index'
import { getConfigStore } from '@/utils/store'
// 阅读排版与抽屉样式独立成文件：md-viewer 的样式是 px 硬编码的 VitePress 移植版，
// 覆盖层单独维护便于对照调整，也不影响其他使用方。
import './md-doc.scss'

defineOptions({
    name: 'MdSupport',
})

type ViewMode = 'preview' | 'source'

interface TocItem {
    id: string
    text: string
    level: number
}

const FONT_MIN = 13
const FONT_MAX = 20
const FONT_DEFAULT = 15
const STORE_TOC = 'previewMdTocOpen'
const STORE_FONT = 'previewMdFontSize'

/* 代码块右上角的复制按钮：只用图标，避免在注入的 DOM 里放界面文案。
   路径与 @vicons/fluent 的 Copy16Regular / Checkmark16Regular 同源——这里的内联
   字符串是塞进 markdown 渲染结果的，拿不到 Vue 组件，只能内联 path；填充式、12px
   （--icon-xs）与命令栏里的 Fluent 图标保持同一套语言。 */
const ICON_COPY =
    '<svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor"><path d="M4 4.085V10.5a2.5 2.5 0 0 0 2.336 2.495L6.5 13h4.414A1.5 1.5 0 0 1 9.5 14H6a3 3 0 0 1-3-3V5.5a1.5 1.5 0 0 1 1-1.415zM11.5 2A1.5 1.5 0 0 1 13 3.5v7a1.5 1.5 0 0 1-1.5 1.5h-5A1.5 1.5 0 0 1 5 10.5v-7A1.5 1.5 0 0 1 6.5 2h5zm0 1h-5a.5.5 0 0 0-.5.5v7a.5.5 0 0 0 .5.5h5a.5.5 0 0 0 .5-.5v-7a.5.5 0 0 0-.5-.5z"/></svg>'
const ICON_CHECK =
    '<svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor"><path d="M13.864 3.655a.5.5 0 0 1-.021.707l-7.93 7.474a.6.6 0 0 1-.839-.016L2.394 9.1a.5.5 0 0 1 .712-.702l2.406 2.442l7.645-7.206a.5.5 0 0 1 .707.021z"/></svg>'

const route = useRoute()
const { t } = useI18n()

let md: MarkdownIt | null = null
let store: Store | null = null
let rafId = 0
let copiedTimer = 0

const file = ref<FileInfo>()
const loading = ref<boolean>(true)
const error = ref<string>('')
const raw = ref<string>('')
const content = ref<string>('')

const mode = ref<ViewMode>('preview')
const tocOpen = ref<boolean>(false)
const tocItems = ref<TocItem[]>([])
const activeId = ref<string>('')
const fontSize = ref<number>(FONT_DEFAULT)
const copied = ref<boolean>(false)

const docRef = ref<HTMLElement>()
const scrollerRef = ref<HTMLElement>()

const showToc = computed(() => tocOpen.value && mode.value === 'preview' && !error.value)
/** 中文按字符计，英文按字符计也比按词更接近「全文长度」的直觉 */
const wordCount = computed(() => raw.value.replace(/\s+/g, '').length)

const persist = async (key: string, value: unknown) => {
    const target = store
    if (!target) return
    try {
        await target.set(key, value)
        await target.save()
    } catch (e) {
        console.warn('persist markdown preference failed:', e)
    }
}

const findHeading = (id: string) => docRef.value?.querySelector<HTMLElement>(`[id="${CSS.escape(id)}"]`) ?? null

/** 从渲染结果里收回标题清单，id 由 markdown-it-toc-and-anchor 写入 */
const buildToc = () => {
    const root = docRef.value
    if (!root) {
        tocItems.value = []
        return
    }
    tocItems.value = Array.from(root.querySelectorAll<HTMLElement>('h1, h2, h3'))
        .map((el, index) => {
            if (!el.id) el.id = `md-heading-${index}`
            return { id: el.id, text: (el.textContent ?? '').trim(), level: Number(el.tagName.slice(1)) }
        })
        .filter(item => item.text.length > 0)
}

/**
 * 表格包一层滚动容器：宽表横向滚动，表头 sticky 于内容区顶部。
 * 直接给 table 设 overflow 会退化成 block 布局，列宽随之失效。
 */
const wrapTables = () => {
    const root = docRef.value
    if (!root) return
    root.querySelectorAll('table').forEach(table => {
        if (table.parentElement?.classList.contains('md-table-wrap')) return
        const wrap = document.createElement('div')
        wrap.className = 'md-table-wrap'
        table.replaceWith(wrap)
        wrap.appendChild(table)
    })
}

/** 代码块注入复制按钮：v-html 每次重渲染都会丢，所以在渲染完成后重建 */
const decorateCodeBlocks = () => {
    const root = docRef.value
    if (!root) return
    root.querySelectorAll<HTMLElement>("div[class*='language-']").forEach(block => {
        if (block.querySelector(':scope > button.md-copy')) return
        const button = document.createElement('button')
        button.type = 'button'
        button.className = 'md-copy'
        button.title = t('md.copyCode')
        button.innerHTML = ICON_COPY
        button.addEventListener('click', async () => {
            const code = block.querySelector('pre')?.textContent ?? ''
            if (!code) return
            try {
                await navigator.clipboard.writeText(code)
                button.innerHTML = ICON_CHECK
                button.classList.add('is-copied')
                button.title = t('md.copied')
                window.setTimeout(() => {
                    button.innerHTML = ICON_COPY
                    button.classList.remove('is-copied')
                    button.title = t('md.copyCode')
                }, 1600)
            } catch (e) {
                ElMessage.error(t('md.copyFailed', { error: String(e) }))
            }
        })
        block.appendChild(button)
    })
}

const updateActive = () => {
    const scroller = scrollerRef.value
    if (!scroller || mode.value !== 'preview' || tocItems.value.length === 0) return
    const top = scroller.getBoundingClientRect().top + 12
    let current = tocItems.value[0].id
    for (const item of tocItems.value) {
        const el = findHeading(item.id)
        if (!el) continue
        if (el.getBoundingClientRect().top - top <= 8) current = item.id
        else break
    }
    activeId.value = current
}

const onScroll = () => {
    if (rafId) return
    rafId = window.requestAnimationFrame(() => {
        rafId = 0
        updateActive()
    })
}

const enhance = async () => {
    await nextTick()
    if (mode.value !== 'preview') return
    wrapTables()
    decorateCodeBlocks()
    buildToc()
    updateActive()
}

const render = async () => {
    if (!md) return
    content.value = md.render(raw.value)
    await enhance()
}

const load = async () => {
    loading.value = true
    error.value = ''
    raw.value = ''
    content.value = ''
    tocItems.value = []
    activeId.value = ''
    copied.value = false
    file.value = route?.query as unknown as FileInfo
    const path = file.value.path as string

    try {
        if (md === null) {
            md = await createMd()
        }
    } catch (e) {
        error.value = t('preview.mdParserFailed', { error: e instanceof Error ? e.message : String(e) })
        loading.value = false
        return
    }

    try {
        raw.value = await readTextFile(path)
        await render()
    } catch (e) {
        error.value = t('preview.mdReadFailed', { error: e instanceof Error ? e.message : String(e) })
    } finally {
        loading.value = false
    }
}

const restorePreferences = async () => {
    try {
        store = await getConfigStore()
        const size = await store.get<number>(STORE_FONT)
        if (typeof size === 'number' && size >= FONT_MIN && size <= FONT_MAX) fontSize.value = size
        const toc = await store.get<boolean>(STORE_TOC)
        if (typeof toc === 'boolean') tocOpen.value = toc
    } catch (e) {
        // 读取失败只影响记忆项，不应挡住预览
        console.warn('read markdown preference failed:', e)
    }
}

const toggleToc = () => {
    tocOpen.value = !tocOpen.value
    void persist(STORE_TOC, tocOpen.value)
    if (tocOpen.value) void nextTick().then(updateActive)
}

/**
 * 预览与源码共用同一个滚动容器，切换时按比例还原阅读点，
 * 避免两种形态高度不同导致直接跳回文首。
 */
const switchMode = (next: ViewMode) => {
    if (next === mode.value) return
    const box = scrollerRef.value
    const ratio =
        box && box.scrollHeight > box.clientHeight ? box.scrollTop / (box.scrollHeight - box.clientHeight) : 0
    mode.value = next
    void nextTick().then(() => {
        const el = scrollerRef.value
        if (el) el.scrollTop = ratio * Math.max(0, el.scrollHeight - el.clientHeight)
        if (next === 'preview') void enhance()
    })
}

const jumpTo = (id: string) => {
    const scroller = scrollerRef.value
    if (!scroller || !findHeading(id)) return
    activeId.value = id
    // 抽屉是覆盖式的：窄窗口下会压住正文，选中后直接收起
    if (tocOpen.value && scroller.clientWidth < 900) {
        tocOpen.value = false
        void persist(STORE_TOC, false)
    }
    // 抽屉开合会改变正文可用宽度，等布局稳定再按最终位置定位
    void nextTick().then(() => {
        const box = scrollerRef.value
        const target = findHeading(id)
        if (!box || !target) return
        const delta = target.getBoundingClientRect().top - box.getBoundingClientRect().top
        box.scrollTo({ top: box.scrollTop + delta - 12, behavior: 'smooth' })
    })
}

const zoomFont = (delta: number) => {
    const next = Math.min(FONT_MAX, Math.max(FONT_MIN, fontSize.value + delta))
    if (next === fontSize.value) return
    fontSize.value = next
    void persist(STORE_FONT, next)
}

const copyAll = async () => {
    if (!raw.value) return
    try {
        await navigator.clipboard.writeText(raw.value)
        copied.value = true
        window.clearTimeout(copiedTimer)
        copiedTimer = window.setTimeout(() => (copied.value = false), 1600)
    } catch (e) {
        ElMessage.error(t('md.copyFailed', { error: String(e) }))
    }
}

const onKeydown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape' || !tocOpen.value) return
    // 预览窗口的 Esc 会关闭窗口，抽屉打开时先拦下这一次
    event.stopPropagation()
    tocOpen.value = false
    void persist(STORE_TOC, false)
}

onMounted(async () => {
    window.addEventListener('keydown', onKeydown, true)
    void restorePreferences()
    await load()
})

onBeforeUnmount(() => {
    window.removeEventListener('keydown', onKeydown, true)
    window.clearTimeout(copiedTimer)
    if (rafId) window.cancelAnimationFrame(rafId)
})

// 同一窗口内换文件时路由 query 变化，组件不会重建，需要主动重读
watch(
    () => route.query.path,
    path => {
        if (typeof path === 'string' && path !== file.value?.path) void load()
    },
)
</script>

<template>
    <LayoutPreview :file="file" :loading="loading" :hide-toolbar="Boolean(error)">
        <template #toolbar>
            <button class="pv-tb-btn is-text" type="button" :aria-pressed="tocOpen" :title="t('md.toc')" @click="toggleToc">
                <el-icon><List16Regular /></el-icon>
                {{ t('md.toc') }}
            </button>
            <i class="pv-tb-sep"></i>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="mode === 'preview'"
                @click="switchMode('preview')"
            >
                {{ t('md.preview') }}
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="mode === 'source'"
                @click="switchMode('source')"
            >
                {{ t('md.source') }}
            </button>
            <i class="pv-tb-sep"></i>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('md.fontSmaller')"
                :disabled="fontSize <= FONT_MIN"
                @click="zoomFont(-1)"
            >
                A−
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('md.fontLarger')"
                :disabled="fontSize >= FONT_MAX"
                @click="zoomFont(1)"
            >
                A+
            </button>
            <div class="pv-tb-grow"></div>
            <span class="pv-tb-info">{{ t('md.wordCount', { count: wordCount }) }}</span>
            <i class="pv-tb-sep"></i>
            <button class="pv-tb-btn is-text" type="button" :title="t('md.copyAll')" :disabled="!raw" @click="copyAll">
                <el-icon><Copy16Regular /></el-icon>
                {{ copied ? t('md.copied') : t('md.copyAll') }}
            </button>
        </template>

        <div class="md-pv">
            <div v-if="error" class="md-pv-state">
                <p class="md-pv-state-title">{{ t('preview.loadFailed') }}</p>
                <p class="md-pv-state-detail">{{ error }}</p>
                <button class="md-pv-state-btn" type="button" @click="load">{{ t('preview.retry') }}</button>
            </div>

            <template v-else>
                <nav v-if="showToc" class="md-pv-toc pv-scroll">
                    <p v-if="tocItems.length === 0" class="md-pv-toc-empty">{{ t('md.tocEmpty') }}</p>
                    <template v-else>
                        <a
                            v-for="item in tocItems"
                            :key="item.id"
                            class="md-pv-toc-item"
                            :class="[`is-lv${item.level}`, { 'is-active': item.id === activeId }]"
                            href="#"
                            :title="item.text"
                            @click.prevent="jumpTo(item.id)"
                        >
                            {{ item.text }}
                        </a>
                    </template>
                </nav>

                <div ref="scrollerRef" class="md-pv-doc pv-scroll" @scroll.passive="onScroll">
                    <div
                        ref="docRef"
                        class="md-pv-inner"
                        :class="{ 'is-long': wordCount > 80000 }"
                        :style="{ '--md-fs': `${fontSize}px` }"
                    >
                        <MdViewer v-if="mode === 'preview'" :content="content" />
                        <pre v-else class="md-pv-source">{{ raw }}</pre>
                    </div>
                </div>
            </template>
        </div>
    </LayoutPreview>
</template>

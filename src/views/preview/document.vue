<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    ArrowRotateClockwise16Regular,
    ChevronLeft16Regular,
    ChevronRight16Regular,
    List16Regular,
    ZoomIn16Regular,
    ZoomOut16Regular,
} from '@vicons/fluent'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { Store } from '@tauri-apps/plugin-store'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { readFile } from '@tauri-apps/plugin-fs'
import { getConfigStore } from '@/utils/store'
// 命令栏控件、目录抽屉与错态的样式独立成文件，便于与 Markdown 预览对齐同一套规格
import './pdf-doc.scss'

import Excel from '@/components/document/excel.vue'
import PdfViewer from './components/pdf-viewer.vue'
import type { PdfOutlineItem, PdfState } from './components/pdf-types'
import { renderAsync } from 'docx-preview'

const route = useRoute()
const { t } = useI18n()

defineOptions({
    name: 'DocumentSupport',
})

interface Sheet {
    name: string
    rows: string[][]
}

enum DocType {
    Excel = 'Excel',
    Docx = 'Docx',
    Pdf = 'Pdf',
    Pptx = 'Pptx',
}

interface Docs {
    Excel?: Array<Sheet>
    Docx?: string
}

interface FlatTocItem {
    title: string
    page: number
    level: number
}

const STORE_TOC = 'previewPdfTocOpen'
/** 窄于此宽度时抽屉会明显压住页面，跳转后自动收起 */
const DRAWER_AUTO_CLOSE_W = 900

const loading = ref(true)
const fileInfo = ref<FileInfo>()
const content = ref<Docs[keyof Docs]>()
const type = ref<DocType>()

const docsRef = ref<HTMLElement>()
const docsStyleRef = ref<HTMLElement>()

const pdfRef = ref<InstanceType<typeof PdfViewer>>()
const pdfState = ref<PdfState>({ current: 1, total: 0, zoom: 1 })
const outline = ref<PdfOutlineItem[]>([])
const tocOpen = ref(false)
const pdfError = ref('')
/** 变更即重建画布，用于「重试」 */
const pdfKey = ref(0)
const pageDraft = ref('1')
const pageFocused = ref(false)

let store: Store | null = null

const isPdf = computed(() => type.value === DocType.Pdf)
const pdfPath = computed(() => (fileInfo.value?.path as string) ?? '')
const zoomPct = computed(() => Math.round(pdfState.value.zoom * 100))

/** 后端 outline 是树，抽屉用扁平行 + level 缩进，避免再引入 el-tree */
const flatOutline = computed<FlatTocItem[]>(() => {
    const list: FlatTocItem[] = []
    const walk = (items: PdfOutlineItem[], level: number) => {
        for (const item of items) {
            list.push({ title: item.title, page: item.page, level })
            if (item.items?.length) walk(item.items, level + 1)
        }
    }
    walk(outline.value, 1)
    return list
})

/** 当前节 = 最后一个页码不超过当前页的条目 */
const activeTocIndex = computed(() => {
    let index = -1
    flatOutline.value.forEach((item, i) => {
        if (item.page >= 1 && item.page <= pdfState.value.current) index = i
    })
    return index
})

const persist = async (key: string, value: unknown) => {
    const target = store
    if (!target) return
    try {
        await target.set(key, value)
        await target.save()
    } catch (e) {
        console.warn('persist pdf preference failed:', e)
    }
}

const onPdfState = (value: PdfState) => {
    pdfState.value = value
    if (!pageFocused.value) pageDraft.value = String(value.current || 1)
}

const onOutline = (items: PdfOutlineItem[]) => {
    outline.value = items
}

const onPdfError = (message: string) => {
    pdfError.value = message || t('preview.loadFailed')
}

const retryPdf = () => {
    pdfError.value = ''
    pdfKey.value += 1
}

const toggleToc = () => {
    tocOpen.value = !tocOpen.value
    void persist(STORE_TOC, tocOpen.value)
}

const closeToc = () => {
    if (!tocOpen.value) return
    tocOpen.value = false
    void persist(STORE_TOC, false)
}

const stepPage = (delta: number) => {
    pdfRef.value?.goToPage(pdfState.value.current + delta)
}

/** 页码框失焦/回车时提交；非法输入回落到当前页 */
const commitPage = () => {
    pageFocused.value = false
    const target = Number(pageDraft.value)
    if (Number.isFinite(target)) pdfRef.value?.goToPage(target)
    pageDraft.value = String(pdfState.value.current)
}

const jumpToOutline = (item: FlatTocItem) => {
    if (item.page < 1) return
    pdfRef.value?.goToPage(item.page)
    // 抽屉是覆盖式的，窄窗口下压住页面左缘，跳转后直接收起
    if (tocOpen.value && window.innerWidth < DRAWER_AUTO_CLOSE_W) closeToc()
}

const onKeydown = (event: KeyboardEvent) => {
    if (event.key !== 'Escape' || !tocOpen.value) return
    // 预览窗口的 Esc 会关闭窗口，抽屉打开时先拦下这一次
    event.stopPropagation()
    closeToc()
}

const getDocxContent = async (path: string) => {
    const buffer = await readFile(path)
    const docs = await renderAsync(buffer, docsRef.value as HTMLElement, docsStyleRef.value)

    return docs as string
}

const restorePreferences = async () => {
    try {
        store = await getConfigStore()
        const toc = await store.get<boolean>(STORE_TOC)
        if (typeof toc === 'boolean') tocOpen.value = toc
    } catch (e) {
        // 读取失败只影响记忆项，不应挡住预览
        console.warn('read pdf preference failed:', e)
    }
}

const load = async () => {
    loading.value = true
    fileInfo.value = route?.query as unknown as FileInfo

    // PDF 直接走 PdfViewer 组件
    if (fileInfo.value.extension === 'pdf') {
        type.value = DocType.Pdf
        pdfError.value = ''
        pdfState.value = { current: 1, total: 0, zoom: 1 }
        outline.value = []
        pageDraft.value = '1'
        loading.value = false
        return
    }

    const val = fileInfo.value.path as string
    const docs: Docs = await invoke('document', { path: val, mode: fileInfo.value.extension })
    type.value = docs.Excel ? DocType.Excel : docs.Docx ? DocType.Docx : DocType.Pptx
    switch (type.value) {
        case DocType.Excel:
            content.value = docs.Excel as Array<Sheet>
            break
        case DocType.Docx:
            content.value = await getDocxContent(docs.Docx as string)
            break
        case DocType.Pptx:
            content.value = '' as string
    }
    loading.value = false
}

onMounted(async () => {
    window.addEventListener('keydown', onKeydown, true)
    void restorePreferences()
    await load()
})

onBeforeUnmount(() => {
    window.removeEventListener('keydown', onKeydown, true)
})

// 抽屉开合改变画布可用宽度：外层已按 flex 重排，这里让画布重算一次适应宽度
watch(tocOpen, () => {
    void nextTick().then(() => pdfRef.value?.refit())
})

// 同一窗口内换文件时路由 query 变化，组件不会重建
watch(
    () => route.query.path,
    path => {
        if (typeof path === 'string' && path !== fileInfo.value?.path) void load()
    },
)
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading" :hide-toolbar="!isPdf || Boolean(pdfError)">
        <template #toolbar>
            <template v-if="isPdf">
                <!-- 顺序与 Markdown 预览对齐：左区依次为「目录 │ 视图方式 │ 缩放」，右区只放页码状态 -->
                <button
                    class="pv-tb-btn is-text"
                    type="button"
                    :aria-pressed="tocOpen"
                    :title="t('pdf.toc')"
                    @click="toggleToc"
                >
                    <el-icon><List16Regular /></el-icon>
                    {{ t('pdf.toc') }}
                </button>

                <i class="pv-tb-sep"></i>

                <button class="pv-tb-btn is-text" type="button" @click="pdfRef?.fitWidth()">
                    {{ t('pdf.fitWidth') }}
                </button>
                <button class="pv-tb-btn" type="button" :title="t('pdf.rotate')" @click="pdfRef?.rotate()">
                    <ArrowRotateClockwise16Regular />
                </button>

                <i class="pv-tb-sep"></i>

                <button
                    class="pv-tb-btn"
                    type="button"
                    :title="t('pdf.zoomOut')"
                    :disabled="zoomPct <= 10"
                    @click="pdfRef?.zoomOut()"
                >
                    <ZoomOut16Regular />
                </button>
                <span class="pv-tb-num">{{ zoomPct }}%</span>
                <button
                    class="pv-tb-btn"
                    type="button"
                    :title="t('pdf.zoomIn')"
                    :disabled="zoomPct >= 500"
                    @click="pdfRef?.zoomIn()"
                >
                    <ZoomIn16Regular />
                </button>

                <div class="pv-tb-grow"></div>

                <button
                    class="pv-tb-btn"
                    type="button"
                    :title="t('pdf.prevPage')"
                    :disabled="pdfState.current <= 1"
                    @click="stepPage(-1)"
                >
                    <ChevronLeft16Regular />
                </button>
                <span class="pdf-tb-page">
                    <input
                        v-model="pageDraft"
                        class="pdf-tb-page-input"
                        inputmode="numeric"
                        :aria-label="t('pdf.goToPage')"
                        @focus="pageFocused = true"
                        @blur="commitPage"
                        @keydown.enter.prevent="commitPage"
                    />
                </span>
                <span class="pv-tb-info pdf-tb-total">/ {{ pdfState.total || '—' }}</span>
                <button
                    class="pv-tb-btn"
                    type="button"
                    :title="t('pdf.nextPage')"
                    :disabled="pdfState.total > 0 && pdfState.current >= pdfState.total"
                    @click="stepPage(1)"
                >
                    <ChevronRight16Regular />
                </button>
            </template>
        </template>

        <div class="text-support" :class="{ 'is-pdf': isPdf }">
            <div class="text-support-inner" :class="{ 'is-pdf': isPdf }" v-loading="!isPdf && loading">
                <template v-if="isPdf">
                    <div v-if="pdfError" class="pdf-state">
                        <p class="pdf-state-title">{{ t('preview.loadFailed') }}</p>
                        <p class="pdf-state-detail">{{ pdfError }}</p>
                        <button class="pdf-state-btn" type="button" @click="retryPdf">{{ t('preview.retry') }}</button>
                    </div>
                    <template v-else>
                        <nav v-if="tocOpen" class="pdf-toc pv-scroll">
                            <p v-if="flatOutline.length === 0" class="pdf-toc-empty">{{ t('pdf.tocEmpty') }}</p>
                            <template v-else>
                                <a
                                    v-for="(item, index) in flatOutline"
                                    :key="`${index}-${item.page}`"
                                    class="pdf-toc-item"
                                    :class="[`is-lv${Math.min(item.level, 3)}`, { 'is-active': index === activeTocIndex }]"
                                    href="#"
                                    :title="item.title"
                                    @click.prevent="jumpToOutline(item)"
                                >
                                    {{ item.title }}
                                </a>
                            </template>
                        </nav>

                        <PdfViewer
                            ref="pdfRef"
                            :key="pdfKey"
                            :path="pdfPath"
                            class="text-support-pdf"
                            @state="onPdfState"
                            @outline="onOutline"
                            @error="onPdfError"
                        />
                    </template>
                </template>

                <Excel v-else-if="type === DocType.Excel" :data="content as Array<Sheet>" />
                <div v-else-if="type === DocType.Docx">
                    <div ref="docsRef"></div>
                    <div class="docx-style" ref="docsStyleRef"></div>
                </div>
                <div v-else-if="!isPdf">{{ t('preview.notSupportedShort') }}</div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.text-support {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;

    &-inner {
        position: relative;
        width: 100%;
        height: 100%;
        overflow: auto;
        padding: 0;
        font-size: var(--font-sm);
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
    }

    /* PDF 自带滚动容器，外层不能再滚，否则出现双滚动条；
       抽屉与画布并排，抽屉开合即改变画布宽度，适应宽度随之重算 */
    &-inner.is-pdf {
        display: flex;
        overflow: hidden;
    }

    &-pdf {
        flex: 1;
        min-width: 0;
        height: 100%;
    }
}

</style>

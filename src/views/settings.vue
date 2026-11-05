<script lang="ts" setup>
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { app } from '@tauri-apps/api'
import { invoke } from '@tauri-apps/api/core'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
import { LogLevel } from '@tauri-apps/plugin-log'
import type { Store } from '@tauri-apps/plugin-store'
import { getConfigStore } from '@/utils/store'
import { readTextFile } from '@/utils'
import { LOCALE_OPTIONS, currentLocale, setLocale, type LocaleCode } from '@/locales'
import { useTheme, type ThemePreference } from '@/hooks/theme'

const { t } = useI18n()
// 设置窗口不加载预览窗口的 header，这里必须自己初始化 useColorMode，
// 否则 html.dark 不会被写入，深色主题在设置窗口里失效。
const { preference, isDark } = useTheme()

/* ---------------- 左导航 ---------------- */

type SectionKey = 'language' | 'theme' | 'support' | 'code' | 'video' | 'cache' | 'log' | 'about'

const ICONS: Record<SectionKey, string> = {
    language: 'M8 1.8a6.2 6.2 0 1 0 0 12.4A6.2 6.2 0 0 0 8 1.8ZM2 8h12M8 1.8c1.8 2 1.8 10.4 0 12.4M8 1.8c-1.8 2-1.8 10.4 0 12.4',
    theme: 'M13.2 10.4A5.6 5.6 0 0 1 5.6 2.8 5.6 5.6 0 1 0 13.2 10.4Z',
    support: 'M2 3.6h4.4l1.6 2H14v8H2zM2 6h12',
    code: 'M5.6 4 2 8l3.6 4M10.4 4 14 8l-3.6 4M9.2 2.8 6.8 13.2',
    video: 'M2 3.6h9v8.8H2zM11 6.2l3-1.6v6.8l-3-1.6',
    cache: 'M3 4h10M6 4V2.6h4V4M4.6 4l.6 9h5.6l.6-9',
    log: 'M4 2.4h6l2.4 2.4v8.8H4zM6.2 7h4.8M6.2 9.6h4.8M6.2 12.2h3',
    about: 'M8 1.8a6.2 6.2 0 1 0 0 12.4A6.2 6.2 0 0 0 8 1.8ZM8 7.2v4M8 5h.01',
}

const searchQuery = ref('')
const activeSection = ref<SectionKey>('language')
const contentRef = ref<HTMLElement | null>(null)

const navGroups = computed(() => {
    const q = searchQuery.value.trim().toLowerCase()
    const groups: { label: string; items: { key: SectionKey; label: string }[] }[] = [
        {
            label: t('settings.groups.general'),
            items: [
                { key: 'language', label: t('settings.nav.language') },
                { key: 'theme', label: t('settings.nav.theme') },
            ],
        },
        {
            label: t('settings.groups.content'),
            items: [
                { key: 'support', label: t('settings.nav.support') },
                { key: 'code', label: t('settings.nav.code') },
                { key: 'video', label: t('settings.nav.video') },
            ],
        },
        {
            label: t('settings.groups.maintenance'),
            items: [
                { key: 'cache', label: t('settings.nav.cache') },
                { key: 'log', label: t('settings.nav.log') },
            ],
        },
        {
            label: t('settings.groups.about'),
            items: [{ key: 'about', label: t('settings.nav.about') }],
        },
    ]
    return groups
        .map(group => ({
            ...group,
            items: q ? group.items.filter(item => item.label.toLowerCase().includes(q)) : group.items,
        }))
        .filter(group => group.items.length > 0)
})

const visibleKeys = computed(() => navGroups.value.flatMap(group => group.items.map(item => item.key)))

const pageTitle = computed(() => {
    const titles: Record<SectionKey, string> = {
        language: t('settings.nav.language'),
        theme: t('settings.nav.theme'),
        support: t('settings.support.title'),
        code: t('settings.customCode.title'),
        video: t('settings.video.title'),
        cache: t('settings.cache.title'),
        log: t('settings.log.title'),
        about: t('settings.about.title'),
    }
    return titles[activeSection.value]
})

function go(key: SectionKey) {
    activeSection.value = key
    nextTick(() => contentRef.value?.scrollTo({ top: 0 }))
}

/** ↑/↓ 在可见项之间移动；输入框 / 下拉获得焦点时不抢按键 */
function onGlobalKey(event: KeyboardEvent) {
    if (event.metaKey || event.ctrlKey || event.altKey) return
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return
    const el = document.activeElement
    if (
        el instanceof HTMLElement &&
        (['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName) || el.closest('.el-input, .el-select, [role="combobox"]'))
    ) {
        return
    }
    const keys = visibleKeys.value
    const index = keys.indexOf(activeSection.value)
    if (index < 0 || keys.length < 2) return
    event.preventDefault()
    const next = Math.min(keys.length - 1, Math.max(0, index + (event.key === 'ArrowDown' ? 1 : -1)))
    go(keys[next] as SectionKey)
    nextTick(() => {
        document.querySelector<HTMLElement>(`[data-nav-key="${keys[next]}"]`)?.scrollIntoView({ block: 'nearest' })
    })
}

/* ---------------- 轻量反馈 ---------------- */

const savedVisible = ref(false)
let savedTimer: number | undefined
function flashSaved() {
    savedVisible.value = true
    window.clearTimeout(savedTimer)
    savedTimer = window.setTimeout(() => (savedVisible.value = false), 2000)
}

/* 行内反馈：成功/错误写在控件左侧，不用悬浮弹层 */
const codeFeedbackText = ref('')
const codeFeedbackError = ref(false)
const videoFeedbackText = ref('')
const videoFeedbackError = ref(false)
const feedbackTimers: { code?: number; video?: number } = {}

function showFeedback(kind: 'code' | 'video', text: string, error = false) {
    const textRef = kind === 'code' ? codeFeedbackText : videoFeedbackText
    const errorRef = kind === 'code' ? codeFeedbackError : videoFeedbackError
    textRef.value = text
    errorRef.value = error
    window.clearTimeout(feedbackTimers[kind])
    feedbackTimers[kind] = window.setTimeout(() => (textRef.value = ''), 2400)
}

/* ---------------- 配置与数据 ---------------- */

let localStore: Store | null = null

interface FormatSupport {
    nameKey: string
    code: string
    data: string[]
}

async function getConfig(): Promise<FormatSupport[]> {
    const config = await readTextFile('config.json', { baseDir: BaseDirectory.Resource })
    const data = JSON.parse(config)
    return [
        { nameKey: 'settings.formats.markdown', code: 'Md', data: data['preview.markdown.checked'] },
        { nameKey: 'settings.formats.image', code: 'Image', data: data['preview.image.checked'] },
        { nameKey: 'settings.formats.video', code: 'Video', data: data['preview.video.checked'] },
        { nameKey: 'settings.formats.audio', code: 'Audio', data: data['preview.audio.checked'] },
        { nameKey: 'settings.formats.doc', code: 'Doc', data: data['preview.doc.checked'] },
        { nameKey: 'settings.formats.code', code: 'Code', data: data['preview.code.checked'] },
        { nameKey: 'settings.formats.font', code: 'Font', data: data['preview.font.checked'] },
        { nameKey: 'settings.formats.archive', code: 'Archive', data: data['preview.archive.checked'] },
        { nameKey: 'settings.formats.book', code: 'Book', data: data['preview.book.checked'] },
        { nameKey: 'settings.formats.model', code: 'Model', data: data['preview.model.checked'] },
    ]
}

const config = ref<FormatSupport[]>([])
const version = ref('')

/* ---------------- 语言 ---------------- */

const localeOptions = LOCALE_OPTIONS
const switchingLocale = ref(false)
async function chooseLocale(value: LocaleCode) {
    if (switchingLocale.value || value === currentLocale.value) return
    switchingLocale.value = true
    try {
        await setLocale(value)
        flashSaved()
    } finally {
        switchingLocale.value = false
    }
}

/* ---------------- 主题 ---------------- */

const themeModes = computed<{ value: ThemePreference; label: string }[]>(() => [
    { value: 'auto', label: t('settings.theme.modeAuto') },
    { value: 'light', label: t('settings.theme.modeLight') },
    { value: 'dark', label: t('settings.theme.modeDark') },
])

function chooseTheme(value: ThemePreference) {
    if (preference.value === value) return
    preference.value = value
}

const effectiveTheme = computed(
    () =>
        (isDark.value ? t('settings.theme.modeDark') : t('settings.theme.modeLight')) +
        (preference.value === 'auto' ? t('settings.theme.followSystem') : ''),
)

/* ---------------- 日志级别 ---------------- */

const logLevel = ref<number>(LogLevel.Warn)
const logLevelList = [
    { label: 'Error', value: LogLevel.Error },
    { label: 'Warn', value: LogLevel.Warn },
    { label: 'Info', value: LogLevel.Info },
    { label: 'Debug', value: LogLevel.Debug },
    { label: 'Trace', value: LogLevel.Trace },
    { label: 'Off', value: 0 },
]
async function updateLogLevel(level: number) {
    logLevel.value = level
    await localStore?.set('logLevel', level)
    await localStore?.save()
    await invoke('set_log_level', { level })
    flashSaved()
}

/* ---------------- 视频 / ffmpeg ---------------- */

const useLocalFfmpeg = ref(false)
const ffmpegAvailable = ref<boolean | null>(null)

const ffmpegStatus = computed<{ type: 'info' | 'success' | 'error'; title: string; description: string }>(() => {
    if (ffmpegAvailable.value === null) {
        return {
            type: 'info',
            title: t('settings.video.checking'),
            description: t('settings.video.checkingDesc'),
        }
    }
    if (ffmpegAvailable.value) {
        return {
            type: 'success',
            title: t('settings.video.available'),
            description: t('settings.video.availableDesc'),
        }
    }
    return {
        type: 'error',
        title: t('settings.video.unavailable'),
        description: t('settings.video.unavailableDesc'),
    }
})

async function toggleFfmpeg() {
    const next = !useLocalFfmpeg.value
    useLocalFfmpeg.value = next
    await localStore?.set('useLocalFfmpeg', next)
    await localStore?.save()
    ffmpegAvailable.value = next ? await invoke<boolean>('check_ffmpeg') : null
    flashSaved()
}

/* ---------------- 自定义扩展名 ---------------- */

const customCodeExts = ref<string[]>([])
const newCodeExt = ref('')

async function addCodeExt() {
    const ext = newCodeExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customCodeExts.value.includes(ext)) {
        showFeedback('code', t('settings.customCode.exists', { ext }), true)
        return
    }
    customCodeExts.value.push(ext)
    newCodeExt.value = ''
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
    showFeedback('code', t('settings.customCode.added', { ext }))
    flashSaved()
}

async function removeCodeExt(ext: string) {
    customCodeExts.value = customCodeExts.value.filter(item => item !== ext)
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
    flashSaved()
}

const customVideoExts = ref<string[]>([])
const newVideoExt = ref('')

async function addVideoExt() {
    if (!useLocalFfmpeg.value) return
    const ext = newVideoExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customVideoExts.value.includes(ext)) {
        showFeedback('video', t('settings.customCode.exists', { ext }), true)
        return
    }
    customVideoExts.value.push(ext)
    newVideoExt.value = ''
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
    showFeedback('video', t('settings.video.added', { ext }))
    flashSaved()
}

async function removeVideoExt(ext: string) {
    customVideoExts.value = customVideoExts.value.filter(item => item !== ext)
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
    flashSaved()
}

/* ---------------- 缓存 ---------------- */

type CacheKind = 'ffmpeg' | 'image' | 'pdf'
const cacheBusy = reactive<Record<CacheKind, boolean>>({ ffmpeg: false, image: false, pdf: false })
const cacheText = reactive<Record<CacheKind, string>>({ ffmpeg: '', image: '', pdf: '' })
const cacheError = reactive<Record<CacheKind, boolean>>({ ffmpeg: false, image: false, pdf: false })
const cacheTimers: Partial<Record<CacheKind, number>> = {}

function showCache(kind: CacheKind, text: string, error = false) {
    cacheText[kind] = text
    cacheError[kind] = error
    window.clearTimeout(cacheTimers[kind])
    cacheTimers[kind] = window.setTimeout(() => (cacheText[kind] = ''), 2400)
}

async function clearCache(kind: CacheKind) {
    if (cacheBusy[kind]) return
    cacheBusy[kind] = true
    try {
        if (kind === 'ffmpeg') {
            const count = await invoke<number>('clear_cache')
            showCache(kind, t('settings.cache.cleared', { count }))
        } else if (kind === 'image') {
            await invoke('clear_image_cache')
            showCache(kind, t('settings.cache.imageCleared'))
        } else {
            const count = await invoke<number>('clear_pdf_cache')
            showCache(kind, t('settings.cache.pdfCleared', { count }))
        }
        flashSaved()
    } catch (e) {
        const message =
            kind === 'ffmpeg'
                ? t('settings.cache.clearFailed', { error: String(e) })
                : kind === 'image'
                  ? t('settings.cache.imageClearFailed', { error: String(e) })
                  : t('settings.cache.pdfClearFailed', { error: String(e) })
        showCache(kind, message, true)
    } finally {
        cacheBusy[kind] = false
    }
}

/* ---------------- 生命周期 ---------------- */

onMounted(async () => {
    window.addEventListener('keydown', onGlobalKey)
    localStore = await getConfigStore()
    try {
        config.value = await getConfig()
    } catch {
        config.value = []
    }
    version.value = await app.getVersion()

    const storedLevel = await localStore?.get<number | string>('logLevel')
    const numericLevel = Number(storedLevel ?? LogLevel.Warn)
    logLevel.value = Number.isNaN(numericLevel) ? LogLevel.Warn : numericLevel

    useLocalFfmpeg.value = (await localStore?.get<boolean>('useLocalFfmpeg')) ?? false
    if (useLocalFfmpeg.value) ffmpegAvailable.value = await invoke<boolean>('check_ffmpeg')

    customCodeExts.value = (await localStore?.get<string[]>('customCodeExtensions')) ?? []
    customVideoExts.value = (await localStore?.get<string[]>('customVideoExtensions')) ?? []
})

onUnmounted(() => {
    window.removeEventListener('keydown', onGlobalKey)
    window.clearTimeout(savedTimer)
    Object.values(feedbackTimers).forEach(timer => window.clearTimeout(timer))
    Object.values(cacheTimers).forEach(timer => window.clearTimeout(timer))
})
</script>

<template>
    <div class="st-window">
        <div class="st-body">
            <!-- 左导航：唯一分组入口 -->
            <nav class="st-navpane" :aria-label="t('settings.navLabel')">
                <el-input v-model="searchQuery" class="st-search" :placeholder="t('settings.search')" :spellcheck="false">
                    <template #prefix>
                        <svg class="st-search-icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                            <circle cx="7" cy="7" r="4.2" />
                            <path d="M10.2 10.2 13.5 13.5" />
                        </svg>
                    </template>
                </el-input>
                <template v-for="group in navGroups" :key="group.label">
                    <div class="st-navgroup">{{ group.label }}</div>
                    <button
                        v-for="item in group.items"
                        :key="item.key"
                        :data-nav-key="item.key"
                        type="button"
                        class="st-navitem"
                        :class="{ 'is-active': activeSection === item.key }"
                        :aria-current="activeSection === item.key"
                        @click="go(item.key)"
                    >
                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                            <path :d="ICONS[item.key]" />
                        </svg>
                        <span>{{ item.label }}</span>
                    </button>
                </template>
                <div v-if="!navGroups.length" class="st-navempty">{{ t('settings.searchEmpty') }}</div>
                <div class="st-navfoot">QuickLook v{{ version || '—' }}</div>
            </nav>

            <!-- 右侧内容：唯一滚动容器 -->
            <main ref="contentRef" class="st-content">
                <div class="st-pane">
                    <div class="st-pagehead">
                        <h2>{{ pageTitle }}</h2>
                        <span v-if="savedVisible" class="st-saved">{{ t('settings.feedback.saved') }}</span>
                    </div>

                    <!-- 1 语言 -->
                    <template v-if="activeSection === 'language'">
                        <p class="st-subtitle">{{ t('language.description') }}</p>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('language.title') }}</div>
                                    <div class="st-row-d">{{ t('settings.language.switchDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <div class="st-seg" role="group">
                                        <button
                                            v-for="option in localeOptions"
                                            :key="option.value"
                                            type="button"
                                            :aria-pressed="currentLocale === option.value"
                                            :disabled="switchingLocale"
                                            @click="chooseLocale(option.value)"
                                        >
                                            {{ t(option.labelKey) }}
                                        </button>
                                    </div>
                                </div>
                            </div>
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.language.syncTitle') }}</div>
                                    <div class="st-row-d">{{ t('settings.language.syncDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <span v-if="switchingLocale" class="st-dot" aria-hidden="true"></span>
                                    <span v-else class="st-muted">{{ t('settings.language.synced') }}</span>
                                </div>
                            </div>
                        </div>
                        <p class="st-note">{{ t('settings.common.instant') }}</p>
                    </template>

                    <!-- 2 主题 -->
                    <template v-else-if="activeSection === 'theme'">
                        <p class="st-subtitle">{{ t('settings.theme.subtitle') }}</p>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.theme.appTheme') }}</div>
                                    <div class="st-row-d">{{ t('settings.theme.appThemeDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <div class="st-seg is-3" role="group">
                                        <button
                                            v-for="mode in themeModes"
                                            :key="mode.value"
                                            type="button"
                                            :aria-pressed="preference === mode.value"
                                            @click="chooseTheme(mode.value)"
                                        >
                                            {{ mode.label }}
                                        </button>
                                    </div>
                                </div>
                            </div>
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.theme.current') }}</div>
                                    <div class="st-row-d">{{ t('settings.theme.currentDesc') }}</div>
                                </div>
                                <div class="st-row-ctl"><span class="st-muted">{{ effectiveTheme }}</span></div>
                            </div>
                        </div>
                        <p class="st-note">{{ t('settings.theme.note') }}</p>
                    </template>

                    <!-- 3 支持的格式 -->
                    <template v-else-if="activeSection === 'support'">
                        <p class="st-subtitle st-subtitle-row">
                            <span>{{ t('settings.support.description') }}</span>
                            <span class="st-stat">{{ t('settings.support.total', { count: config.length }) }}</span>
                        </p>
                        <div class="st-card">
                            <div v-if="!config.length" class="st-row">
                                <div class="st-row-txt"><span class="st-empty">{{ t('settings.support.empty') }}</span></div>
                            </div>
                            <div v-for="type in config" :key="type.code" class="st-row">
                                <div class="st-row-txt"><div class="st-row-t">{{ t(type.nameKey) }}</div></div>
                                <div class="st-row-ctl">
                                    <div class="st-pills">
                                        <span v-for="ext in type.data || []" :key="ext" class="st-chip is-plain">{{ ext }}</span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </template>

                    <!-- 4 自定义代码格式 -->
                    <template v-else-if="activeSection === 'code'">
                        <p class="st-subtitle">{{ t('settings.customCode.description') }}</p>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.customCode.existing') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <div v-if="customCodeExts.length" class="st-pills">
                                        <span v-for="ext in customCodeExts" :key="ext" class="st-chip">
                                            {{ ext }}
                                            <button type="button" class="st-chip-x" :aria-label="t('settings.common.remove')" @click="removeCodeExt(ext)">×</button>
                                        </span>
                                    </div>
                                    <span v-else class="st-empty">{{ t('settings.customCode.empty') }}</span>
                                </div>
                            </div>
                            <div class="st-row is-input">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.customCode.addExt') }}</div>
                                    <div v-if="codeFeedbackText" class="st-inline" :class="{ 'is-error': codeFeedbackError }">
                                        {{ codeFeedbackText }}
                                    </div>
                                </div>
                                <div class="st-row-ctl">
                                    <el-input v-model="newCodeExt" class="st-textbox" :placeholder="t('settings.customCode.placeholder')" @keyup.enter="addCodeExt" />
                                    <button type="button" class="st-iconbtn" :title="t('common.add')" :aria-label="t('common.add')" @click="addCodeExt">
                                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
                                    </button>
                                </div>
                            </div>
                        </div>
                        <p class="st-note">{{ t('settings.common.instant') }}</p>
                    </template>

                    <!-- 5 视频 -->
                    <template v-else-if="activeSection === 'video'">
                        <p class="st-subtitle">{{ t('settings.video.description') }}</p>
                        <div v-if="useLocalFfmpeg" class="st-infobar">
                            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
                                <circle cx="8" cy="8" r="6.2" /><path d="M8 7.2v4M8 5h.01" />
                            </svg>
                            <div>
                                <div class="st-info-title">{{ ffmpegStatus.title }}</div>
                                <div class="st-info-desc">{{ ffmpegStatus.description }}</div>
                            </div>
                        </div>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.video.toggle') }}</div>
                                    <div class="st-row-d">{{ t('settings.video.toggleDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <button type="button" class="st-switch" role="switch" :aria-checked="useLocalFfmpeg" :aria-label="t('settings.video.toggle')" @click="toggleFfmpeg"></button>
                                </div>
                            </div>
                            <div class="st-row" :class="{ 'is-dep': !useLocalFfmpeg }">
                                <div class="st-row-txt">
                                    <div class="st-row-t">
                                        {{ t('settings.video.customTitle') }}
                                        <span v-if="!useLocalFfmpeg" class="st-row-tip">{{ t('settings.video.customTip') }}</span>
                                    </div>
                                </div>
                                <div class="st-row-ctl">
                                    <div v-if="customVideoExts.length" class="st-pills">
                                        <span v-for="ext in customVideoExts" :key="ext" class="st-chip">
                                            {{ ext }}
                                            <button type="button" class="st-chip-x" :aria-label="t('settings.common.remove')" :disabled="!useLocalFfmpeg" @click="removeVideoExt(ext)">×</button>
                                        </span>
                                    </div>
                                    <span v-else-if="useLocalFfmpeg" class="st-empty">{{ t('settings.video.customEmpty') }}</span>
                                </div>
                            </div>
                            <div class="st-row is-input" :class="{ 'is-dep': !useLocalFfmpeg }">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.customCode.addExt') }}</div>
                                    <div v-if="videoFeedbackText" class="st-inline" :class="{ 'is-error': videoFeedbackError }">
                                        {{ videoFeedbackText }}
                                    </div>
                                </div>
                                <div class="st-row-ctl">
                                    <el-input v-model="newVideoExt" class="st-textbox" :placeholder="t('settings.video.customPlaceholder')" :disabled="!useLocalFfmpeg" @keyup.enter="addVideoExt" />
                                    <button type="button" class="st-iconbtn" :title="t('common.add')" :aria-label="t('common.add')" :disabled="!useLocalFfmpeg" @click="addVideoExt">
                                        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
                                    </button>
                                </div>
                            </div>
                        </div>
                        <p class="st-note">{{ t('settings.common.instant') }}</p>
                    </template>

                    <!-- 6 缓存 -->
                    <template v-else-if="activeSection === 'cache'">
                        <p class="st-subtitle">{{ t('settings.cache.description') }}</p>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.cache.ffmpegTitle') }}</div>
                                    <div class="st-row-d">{{ t('settings.cache.ffmpegDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <span v-if="cacheText.ffmpeg" class="st-inline" :class="{ 'is-error': cacheError.ffmpeg }">{{ cacheText.ffmpeg }}</span>
                                    <button type="button" class="st-subtle is-danger" :disabled="cacheBusy.ffmpeg" @click="clearCache('ffmpeg')">
                                        {{ cacheBusy.ffmpeg ? t('settings.cache.clearing') : t('settings.cache.clear') }}
                                    </button>
                                </div>
                            </div>
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.cache.imageTitle') }}</div>
                                    <div class="st-row-d">{{ t('settings.cache.imageDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <span v-if="cacheText.image" class="st-inline" :class="{ 'is-error': cacheError.image }">{{ cacheText.image }}</span>
                                    <button type="button" class="st-subtle is-danger" :disabled="cacheBusy.image" @click="clearCache('image')">
                                        {{ cacheBusy.image ? t('settings.cache.clearing') : t('settings.cache.clear') }}
                                    </button>
                                </div>
                            </div>
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.cache.pdfTitle') }}</div>
                                    <div class="st-row-d">{{ t('settings.cache.pdfDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <span v-if="cacheText.pdf" class="st-inline" :class="{ 'is-error': cacheError.pdf }">{{ cacheText.pdf }}</span>
                                    <button type="button" class="st-subtle is-danger" :disabled="cacheBusy.pdf" @click="clearCache('pdf')">
                                        {{ cacheBusy.pdf ? t('settings.cache.clearing') : t('settings.cache.clear') }}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </template>

                    <!-- 7 日志 -->
                    <template v-else-if="activeSection === 'log'">
                        <p class="st-subtitle">{{ t('settings.log.description') }}</p>
                        <div class="st-card">
                            <div class="st-row">
                                <div class="st-row-txt">
                                    <div class="st-row-t">{{ t('settings.log.level') }}</div>
                                    <div class="st-row-d">{{ t('settings.log.levelDesc') }}</div>
                                </div>
                                <div class="st-row-ctl">
                                    <el-select
                                        v-model="logLevel"
                                        class="st-combo"
                                        popper-class="st-select-popper"
                                        :aria-label="t('settings.log.level')"
                                        @change="updateLogLevel"
                                    >
                                        <el-option v-for="item in logLevelList" :key="item.value" :label="item.label" :value="item.value" />
                                    </el-select>
                                </div>
                            </div>
                        </div>
                        <p class="st-note">{{ t('settings.common.instant') }}</p>
                    </template>

                    <!-- 8 关于 -->
                    <template v-else>
                        <p class="st-subtitle">{{ t('settings.about.description') }}</p>
                        <div class="st-card st-identity">
                            <div class="st-logo" aria-hidden="true">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4"><rect x="4" y="3" width="16" height="18" rx="2" /><path d="M8 8h8M8 12h8M8 16h5" /></svg>
                            </div>
                            <div class="st-id-info">
                                <div class="st-id-name">QuickLook <small>v{{ version || '—' }}</small></div>
                                <div class="st-id-desc">{{ t('settings.about.appDesc') }}</div>
                                <div class="st-id-links">
                                    <a class="st-link" href="https://github.com/GuoJikun/quicklook/releases/latest" target="_blank" rel="noopener noreferrer">GitHub Releases</a>
                                    <a class="st-link" href="https://gitee.com/guojikun/quicklook/releases/latest" target="_blank" rel="noopener noreferrer">Gitee Releases</a>
                                </div>
                            </div>
                        </div>
                    </template>
                </div>
            </main>
        </div>
    </div>
</template>

<style lang="scss" scoped>
/* 设置窗口保留系统标题栏，因此不自绘 44px 标题行：左导航与内容区占满窗口高度。
   只有右侧内容区可滚动，避免与 #app 形成双滚动条。 */
.st-window {
    --st-nav-w: 240px;
    --st-content-w: 720px;
    /* Element Plus 的禁用态也走项目主题，避免出现组件自带的灰底 */
    --el-disabled-bg-color: transparent;
    --el-disabled-border-color: var(--color-border);
    --el-disabled-text-color: var(--color-text-disabled);
    height: 100vh;
    overflow: hidden;
    background: var(--color-bg);
    color: var(--color-text-primary);
    font-family: inherit;
}

.st-body {
    display: flex;
    height: 100%;
    min-height: 0;
}

/* ---------- 左导航 ---------- */
.st-navpane {
    flex: none;
    width: var(--st-nav-w);
    min-width: 0;
    padding: 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: auto;
    background: color-mix(in srgb, var(--color-surface) 60%, transparent);
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
    border-right: 1px solid var(--color-divider);

    &::-webkit-scrollbar {
        width: var(--preview-scrollbar-w);
    }
    &::-webkit-scrollbar-thumb {
        background: var(--color-border);
        border-radius: 4px;
    }
}

/* 搜索、扩展名、日志级别都用 Element Plus 的 el-input / el-select；
   这里只把 --el-* 映射到项目主题变量，保留 §4.4 的 Fluent 规格（32px 高、1px 边、透明底）。
   scoped 规则带 data-v 属性，优先级高于组件自带样式。 */
.st-search {
    flex: none;
    height: 32px;
    margin-bottom: 8px;
    color: var(--color-text-disabled);
    --el-input-bg-color: var(--color-chip-bg);
    --el-input-border-color: transparent;
    --el-input-hover-border-color: transparent;
    --el-input-focus-border-color: var(--color-brand-fg);
    --el-input-border-radius: var(--radius-md);
    --el-input-text-color: var(--color-text-primary);
    --el-input-placeholder-color: var(--color-text-disabled);

    :deep(.el-input__wrapper) {
        min-height: 32px;
        padding: 0 10px;
    }

    :deep(.el-input__inner) {
        font: inherit;
        font-size: var(--font-xs);
    }

    .st-search-icon {
        width: 14px;
        height: 14px;
        flex: none;
        color: var(--color-text-disabled);
    }
}

.st-navgroup {
    padding: 12px 12px 4px;
    font-size: var(--font-2xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--color-text-disabled);
}

.st-navitem {
    position: relative;
    flex: none;
    height: 36px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    font-size: var(--font-sm);
    text-align: left;
    cursor: pointer;
    transition:
        background-color var(--transition-fast),
        color var(--transition-fast);

    svg {
        width: 16px;
        height: 16px;
        flex: none;
    }

    &:hover {
        background: var(--elevation-hover);
        color: var(--color-text-primary);
    }

    &.is-active {
        background: var(--color-selected-bg);
        color: var(--color-text-primary);
        font-weight: 600;

        &::before {
            content: '';
            position: absolute;
            left: 0;
            top: 9px;
            bottom: 9px;
            width: 3px;
            border-radius: 2px;
            background: var(--color-brand-fg);
        }
    }
}

.st-navempty {
    padding: 8px 12px;
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
}

.st-navfoot {
    margin-top: auto;
    padding: 12px;
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
    font-variant-numeric: tabular-nums;
}

/* ---------- 右侧内容 ---------- */
.st-content {
    flex: 1;
    min-width: 0;
    overflow: auto;
    padding: 24px 32px 40px;

    &::-webkit-scrollbar {
        width: var(--preview-scrollbar-w);
    }
    &::-webkit-scrollbar-thumb {
        background: var(--color-border);
        border-radius: 4px;
    }
}

.st-pane {
    max-width: var(--st-content-w);
    margin: 0 auto;
}

.st-pagehead {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;

    h2 {
        margin: 0;
        font-size: var(--font-3xl);
        font-weight: 600;
        letter-spacing: -0.01em;
        line-height: 1.3;
    }
}

.st-saved {
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
    white-space: nowrap;
}

.st-subtitle {
    margin: 6px 0 24px;
    font-size: var(--font-sm);
    line-height: 1.5;
    color: var(--color-text-secondary);

    &-row {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 16px;
    }
}

.st-stat {
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
}

/* ---------- 卡片与设置行 ---------- */
.st-card {
    padding: 4px 16px;
    margin-bottom: 16px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
}

.st-row {
    min-height: 52px;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 0;
    border-bottom: 1px solid var(--color-divider);

    &:last-child {
        border-bottom: 0;
    }

    &.is-dep {
        opacity: 0.45;
        pointer-events: none;
    }
}

.st-row-txt {
    flex: 1;
    min-width: 0;
}

.st-row-t {
    font-size: var(--font-md);
    font-weight: 600;
    color: var(--color-text-primary);
}

.st-row-tip {
    margin-left: 6px;
    font-size: var(--font-xs);
    font-weight: 400;
    color: var(--color-text-disabled);
}

.st-row-d {
    margin-top: 3px;
    font-size: var(--font-xs);
    line-height: 1.5;
    color: var(--color-text-secondary);
}

.st-row-ctl {
    margin-left: auto;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
}

.st-note {
    margin: -6px 0 16px;
    padding-left: 2px;
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
}

.st-muted,
.st-empty {
    font-size: var(--font-xs);
    color: var(--color-text-disabled);
    font-variant-numeric: tabular-nums;
}

.st-inline {
    margin-top: 3px;
    font-size: var(--font-xs);
    color: var(--color-text-secondary);

    &.is-error {
        color: var(--color-danger-fg);
    }
}

.st-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-accent);
    animation: st-pulse 1s ease-in-out infinite;
}

@keyframes st-pulse {
    50% {
        opacity: 0.3;
    }
}

/* ---------- Fluent 控件 ---------- */
.st-seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border-radius: var(--radius-md);
    background: var(--color-chip-bg);

    button {
        appearance: none;
        border: 0;
        background: transparent;
        color: var(--color-text-secondary);
        font: inherit;
        font-size: var(--font-xs);
        padding: 6px 14px;
        border-radius: 6px;
        cursor: pointer;
        transition:
            background-color var(--transition-fast),
            color var(--transition-fast);

        &:hover:not(:disabled) {
            color: var(--color-text-primary);
        }

        &[aria-pressed='true'] {
            background: var(--color-surface);
            color: var(--color-text-primary);
            font-weight: 600;
        }

        &:disabled {
            cursor: default;
            opacity: 0.6;
        }
    }

    &.is-3 button {
        padding: 6px 10px;
    }
}

.st-switch {
    appearance: none;
    position: relative;
    flex: none;
    width: 40px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 10px;
    background: var(--color-chip-bg);
    cursor: pointer;
    transition: background-color var(--transition-fast);

    &::after {
        content: '';
        position: absolute;
        left: 4px;
        top: 4px;
        width: 12px;
        height: 12px;
        border-radius: 50%;
        background: var(--color-text-disabled);
        transition:
            transform var(--transition-fast),
            background-color var(--transition-fast);
    }

    &[aria-checked='true'] {
        background: var(--color-accent);

        &::after {
            transform: translateX(20px);
            background: var(--btn-primary-text);
        }
    }
}

.st-combo {
    width: 120px;
    --el-fill-color-blank: transparent;
    --el-border-color: var(--color-border);
    --el-border-color-hover: var(--color-accent);
    --el-border-radius-base: var(--radius-sm);
    --el-color-primary: var(--color-brand-fg);
    --el-text-color-regular: var(--color-text-primary);
    --el-select-input-color: var(--color-text-disabled);

    :deep(.el-select__wrapper) {
        min-height: 32px;
        padding: 4px 10px;
        gap: 4px;
        font-size: var(--font-xs);
        line-height: 22px;
    }

    :deep(.el-select__placeholder),
    :deep(.el-select__selected-item) {
        font-size: var(--font-xs);
    }

    :deep(.el-select__caret) {
        font-size: var(--font-xs);
    }
}

.st-textbox {
    box-sizing: border-box;
    /* 定宽：不能写成 max-width，否则 EP 的清空/后缀图标会把盒子撑长 */
    width: 200px;
    flex: none;
    --el-input-bg-color: transparent;
    --el-input-border-color: var(--color-border);
    --el-input-hover-border-color: var(--color-brand-fg);
    --el-input-focus-border-color: var(--color-brand-fg);
    --el-input-border-radius: var(--radius-sm);
    --el-input-text-color: var(--color-text-primary);
    --el-input-placeholder-color: var(--color-text-disabled);

    :deep(.el-input__wrapper) {
        min-height: 32px;
        padding: 0 10px;
    }

    :deep(.el-input__wrapper.is-focus) {
        background: var(--color-surface);
    }

    :deep(.el-input__inner) {
        font: inherit;
        font-size: var(--font-xs);
    }
}

.st-iconbtn {
    appearance: none;
    flex: none;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-chip-bg);
    color: var(--color-text-secondary);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition:
        background-color var(--transition-fast),
        color var(--transition-fast);

    svg {
        width: 14px;
        height: 14px;
    }

    &:hover:not(:disabled) {
        background: var(--color-accent-soft);
        color: var(--color-brand-fg);
    }

    &:disabled {
        cursor: default;
    }
}

.st-pills {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
    max-width: 420px;
}

.st-chip {
    height: 22px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px 0 9px;
    border-radius: 11px;
    background: var(--color-chip-bg);
    color: var(--color-text-secondary);
    font-size: var(--font-xs);
    font-variant-numeric: tabular-nums;

    &.is-plain {
        border-radius: var(--radius-sm);
        padding: 0 8px;
    }
}

.st-chip-x {
    appearance: none;
    width: 14px;
    height: 14px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--color-text-disabled);
    display: grid;
    place-items: center;
    font: inherit;
    font-size: var(--font-xs);
    line-height: 1;
    cursor: pointer;

    &:hover:not(:disabled) {
        background: var(--elevation-hover);
        color: var(--color-text-primary);
    }

    &:disabled {
        cursor: default;
    }
}

.st-subtle {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--color-brand-fg);
    font: inherit;
    font-size: var(--font-xs);
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    white-space: nowrap;
    cursor: pointer;
    transition: background-color var(--transition-fast);

    &:hover:not(:disabled) {
        background: var(--elevation-hover);
    }

    &.is-danger {
        color: var(--color-danger-fg);
    }

    &:disabled {
        opacity: 0.55;
        cursor: default;
    }
}

.st-infobar {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 14px;
    margin-bottom: 16px;
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);

    svg {
        width: 16px;
        height: 16px;
        flex: none;
        margin-top: 1px;
        color: var(--color-brand-fg);
    }
}

.st-info-title {
    font-size: var(--font-sm);
    font-weight: 600;
}

.st-info-desc {
    margin-top: 2px;
    font-size: var(--font-xs);
    line-height: 1.5;
    color: var(--color-text-secondary);
}

/* ---------- 关于：身份卡 ---------- */
.st-identity {
    padding: 20px;
    display: flex;
    align-items: flex-start;
    gap: 16px;
}

.st-logo {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--color-accent-soft);
    color: var(--color-brand-fg);
    display: grid;
    place-items: center;

    svg {
        width: 22px;
        height: 22px;
    }
}

.st-id-info {
    flex: 1;
    min-width: 0;
}

.st-id-name {
    font-size: var(--font-2xl);
    font-weight: 600;
    display: flex;
    align-items: baseline;
    gap: 8px;

    small {
        font-size: var(--font-xs);
        font-weight: 500;
        color: var(--color-text-disabled);
        font-variant-numeric: tabular-nums;
    }
}

.st-id-desc {
    margin-top: 4px;
    font-size: var(--font-xs);
    color: var(--color-text-secondary);
}

.st-id-links {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 14px;
}

.st-link {
    color: var(--color-brand-fg);
    font-size: var(--font-xs);
    text-decoration: none;

    &:hover {
        text-decoration: underline;
    }
}

/* ---------- 响应式 ---------- */
@media (max-width: 999px) {
    .st-window {
        --st-nav-w: 200px;
    }
    .st-content {
        padding: 20px 20px 32px;
    }
    .st-card {
        padding-left: 12px;
        padding-right: 12px;
    }
}

@media (max-width: 759px) {
    .st-body {
        flex-direction: column;
    }
    .st-navpane {
        width: 100%;
        height: 44px;
        flex: none;
        flex-direction: row;
        align-items: center;
        gap: 4px;
        padding: 6px 10px;
        overflow-x: auto;
        overflow-y: hidden;
        border-right: 0;
        border-bottom: 1px solid var(--color-divider);
    }
    .st-search {
        width: 140px;
        flex: none;
        margin: 0 8px 0 0;
    }
    .st-navgroup,
    .st-navfoot,
    .st-navempty {
        display: none;
    }
    .st-navitem {
        flex: none;
        height: 32px;
        padding: 0 10px;

        &.is-active::before {
            top: auto;
            bottom: 0;
            left: 10px;
            right: 10px;
            width: auto;
            height: 2px;
        }
    }
    .st-pagehead h2 {
        font-size: var(--font-2xl);
    }
    .st-row {
        flex-wrap: wrap;
    }
    .st-row-ctl {
        width: 100%;
        justify-content: flex-end;
    }
}

@media (max-width: 520px) {
    .st-row.is-input .st-row-ctl {
        flex-wrap: wrap;
    }
    .st-textbox {
        width: 100%;
    }
    .st-subtitle-row {
        flex-wrap: wrap;
    }
}
</style>

<!-- el-select 的下拉由 Teleport 挂到 body，拿不到 scoped 的 data-v，
     因此用独立的 popper-class 在非 scoped 样式里映射主题变量。 -->
<style lang="scss">
.st-select-popper.el-popper {
    --el-bg-color-overlay: var(--color-surface);
    --el-border-color-light: var(--color-border);
    --el-text-color-regular: var(--color-text-primary);
    --el-text-color-placeholder: var(--color-text-disabled);
    --el-fill-color-light: var(--color-chip-bg);
    --el-color-primary: var(--color-brand-fg);
    --el-box-shadow-light: 0 6px 20px rgba(0, 0, 0, 0.14);
    border-radius: var(--radius-md);

    .el-select-dropdown__list {
        padding: 4px 0;
    }

    .el-select-dropdown__item {
        height: 30px;
        line-height: 30px;
        padding: 0 10px;
        font-size: var(--font-xs);
    }
}

html.dark .st-select-popper.el-popper {
    --el-box-shadow-light: 0 6px 20px rgba(0, 0, 0, 0.45);
}
</style>

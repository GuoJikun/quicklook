<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
import { readTextFile } from '@/utils'
import { app } from '@tauri-apps/api'
import type { Store } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'
import { LogLevel } from '@tauri-apps/plugin-log'
import { ChatDotRound, Delete, EditPen, Files, InfoFilled, VideoCamera } from '@element-plus/icons-vue'

import SettingItem from '@/components/setting-item.vue'
import { getConfigStore } from '@/utils/store'
import { LOCALE_OPTIONS, currentLocale, setLocale, type LocaleCode } from '@/locales'

const { t } = useI18n()

let localStore: Store | null = null
const loadStore = async () => {
    localStore = await getConfigStore()
}

interface FormatSupport {
    nameKey: string
    code: string
    data: string[]
}

const getConfig = async (): Promise<FormatSupport[]> => {
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
const version = ref<string>('')

const localeOptions = LOCALE_OPTIONS
const switchingLocale = ref(false)
const handleLocaleChange = async (value: unknown) => {
    if (typeof value !== 'string') return
    switchingLocale.value = true
    try {
        await setLocale(value as LocaleCode)
    } finally {
        switchingLocale.value = false
    }
}

const logLevel = ref<string | number>(LogLevel.Warn)
const logLevelList = [
    { label: 'Error', value: LogLevel.Error },
    { label: 'Warn', value: LogLevel.Warn },
    { label: 'Info', value: LogLevel.Info },
    { label: 'Debug', value: LogLevel.Debug },
    { label: 'Trace', value: LogLevel.Trace },
    { label: 'Off', value: 0 },
]
const updateLogLevel = async (level: number) => {
    await localStore?.set('logLevel', level)
    await localStore?.save()
    await invoke('set_log_level', { level })
}
const handleLogLevelChange = (level: unknown) => {
    if (typeof level === 'number' || typeof level === 'string') {
        const numeric = typeof level === 'string' ? Number(level) : level
        if (!Number.isNaN(numeric)) {
            logLevel.value = numeric
            updateLogLevel(numeric)
        }
    }
}

const useLocalFfmpeg = ref<boolean>(false)
const ffmpegAvailable = ref<boolean | null>(null)

type AlertType = 'info' | 'success' | 'error'
const ffmpegStatus = computed<{ type: AlertType; title: string; description: string }>(() => {
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

const handleUseLocalFfmpegChange = async (val: unknown) => {
    if (typeof val !== 'boolean') return
    useLocalFfmpeg.value = val
    await localStore?.set('useLocalFfmpeg', val)
    await localStore?.save()
    if (val) {
        ffmpegAvailable.value = await invoke<boolean>('check_ffmpeg')
    } else {
        ffmpegAvailable.value = null
    }
}

const customCodeExts = ref<string[]>([])
const newCodeExt = ref<string>('')

const addCodeExt = async () => {
    const ext = newCodeExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customCodeExts.value.includes(ext)) {
        ElMessage.warning(t('settings.customCode.exists', { ext }))
        return
    }
    customCodeExts.value.push(ext)
    newCodeExt.value = ''
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
    ElMessage.success(t('settings.customCode.added', { ext }))
}

const removeCodeExt = async (ext: string) => {
    customCodeExts.value = customCodeExts.value.filter(e => e !== ext)
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
}

const customVideoExts = ref<string[]>([])
const newVideoExt = ref<string>('')

const addVideoExt = async () => {
    if (!useLocalFfmpeg.value) return
    const ext = newVideoExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customVideoExts.value.includes(ext)) {
        ElMessage.warning(t('settings.customCode.exists', { ext }))
        return
    }
    customVideoExts.value.push(ext)
    newVideoExt.value = ''
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
    ElMessage.success(t('settings.video.added', { ext }))
}

const removeVideoExt = async (ext: string) => {
    customVideoExts.value = customVideoExts.value.filter(e => e !== ext)
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
}

onMounted(async () => {
    await loadStore()
    config.value = await getConfig()
    version.value = await app.getVersion()

    const tmpLogLevel: string = (await localStore?.get<string>('logLevel')) || ''
    logLevel.value = tmpLogLevel || String(LogLevel.Warn)

    const storedFfmpeg = await localStore?.get<boolean>('useLocalFfmpeg')
    useLocalFfmpeg.value = storedFfmpeg ?? false
    if (useLocalFfmpeg.value) {
        ffmpegAvailable.value = await invoke<boolean>('check_ffmpeg')
    }

    customCodeExts.value = (await localStore?.get<string[]>('customCodeExtensions')) ?? []
    customVideoExts.value = (await localStore?.get<string[]>('customVideoExtensions')) ?? []
})

const clearingCache = ref<boolean>(false)
const handleClearCache = async () => {
    clearingCache.value = true
    try {
        const removed = await invoke<number>('clear_cache')
        ElMessage.success(t('settings.cache.cleared', { count: removed }))
    } catch (e) {
        ElMessage.error(t('settings.cache.clearFailed', { error: String(e) }))
    } finally {
        clearingCache.value = false
    }
}

const clearingImageCache = ref<boolean>(false)
const handleClearImageCache = async () => {
    clearingImageCache.value = true
    try {
        await invoke('clear_image_cache')
        ElMessage.success(t('settings.cache.imageCleared'))
    } catch (e) {
        ElMessage.error(t('settings.cache.imageClearFailed', { error: String(e) }))
    } finally {
        clearingImageCache.value = false
    }
}

const clearingPdfCache = ref<boolean>(false)
const handleClearPdfCache = async () => {
    clearingPdfCache.value = true
    try {
        const removed = await invoke<number>('clear_pdf_cache')
        ElMessage.success(t('settings.cache.pdfCleared', { count: removed }))
    } catch (e) {
        ElMessage.error(t('settings.cache.pdfClearFailed', { error: String(e) }))
    } finally {
        clearingPdfCache.value = false
    }
}
</script>

<template>
    <div class="settings">
        <div class="settings-inner">
            <nav class="settings-nav">
                <el-anchor direction="horizontal" container="#app" :offset="100" :bound="30">
                    <el-anchor-link href="#language">{{ t('settings.nav.language') }}</el-anchor-link>
                    <el-anchor-link href="#support">{{ t('settings.nav.support') }}</el-anchor-link>
                    <el-anchor-link href="#custom-code">{{ t('settings.nav.code') }}</el-anchor-link>
                    <el-anchor-link href="#video">{{ t('settings.nav.video') }}</el-anchor-link>
                    <el-anchor-link href="#cache">{{ t('settings.nav.cache') }}</el-anchor-link>
                    <el-anchor-link href="#log">{{ t('settings.nav.log') }}</el-anchor-link>
                    <el-anchor-link href="#version">{{ t('settings.nav.about') }}</el-anchor-link>
                </el-anchor>
            </nav>
            <div class="settings-content">
                <SettingItem
                    id="language"
                    :title="t('language.title')"
                    :description="t('language.description')"
                    :icon="ChatDotRound"
                >
                    <el-radio-group
                        :model-value="currentLocale"
                        :disabled="switchingLocale"
                        @change="handleLocaleChange"
                    >
                        <el-radio-button v-for="item in localeOptions" :key="item.value" :value="item.value">
                            {{ t(item.labelKey) }}
                        </el-radio-button>
                    </el-radio-group>
                </SettingItem>

                <SettingItem
                    id="support"
                    :title="t('settings.support.title')"
                    :description="t('settings.support.description')"
                    :icon="Files"
                >
                    <div class="format-grid">
                        <div v-for="type in config" :key="type.code" class="format-card">
                            <div class="format-card-head">
                                <span class="format-card-code">{{ type.code }}</span>
                                <span class="format-card-name">{{ t(type.nameKey) }}</span>
                            </div>
                            <div class="format-card-exts">
                                <el-tag v-for="ext in type.data" :key="ext" size="small" effect="plain" type="info">
                                    {{ ext }}
                                </el-tag>
                            </div>
                        </div>
                    </div>
                </SettingItem>

                <SettingItem
                    id="custom-code"
                    :title="t('settings.customCode.title')"
                    :description="t('settings.customCode.description')"
                    :icon="EditPen"
                >
                    <div v-if="customCodeExts.length > 0" class="ext-tags">
                        <el-tag
                            v-for="ext in customCodeExts"
                            :key="ext"
                            closable
                            type="primary"
                            effect="light"
                            @close="removeCodeExt(ext)"
                        >
                            {{ ext }}
                        </el-tag>
                    </div>
                    <div v-else class="ext-empty">{{ t('settings.customCode.empty') }}</div>
                    <div class="ext-input-row">
                        <el-input
                            v-model="newCodeExt"
                            :placeholder="t('settings.customCode.placeholder')"
                            size="default"
                            class="ext-input"
                            @keyup.enter="addCodeExt"
                        />
                        <el-button type="primary" @click="addCodeExt">{{ t('common.add') }}</el-button>
                    </div>
                </SettingItem>

                <SettingItem
                    id="video"
                    :title="t('settings.video.title')"
                    :description="t('settings.video.description')"
                    :icon="VideoCamera"
                >
                    <div class="field-row">
                        <div class="field-row-label">
                            <span class="field-row-title">{{ t('settings.video.toggle') }}</span>
                            <span class="field-row-desc">{{ t('settings.video.toggleDesc') }}</span>
                        </div>
                        <el-switch
                            v-model="useLocalFfmpeg"
                            class="field-row-control"
                            @change="handleUseLocalFfmpegChange"
                        />
                    </div>

                    <el-alert
                        v-if="useLocalFfmpeg"
                        :type="ffmpegStatus.type"
                        :title="ffmpegStatus.title"
                        :description="ffmpegStatus.description"
                        :closable="false"
                        show-icon
                    />

                    <div class="ext-sub">
                        <div class="ext-sub-title">
                            <span>{{ t('settings.video.customTitle') }}</span>
                            <span v-if="!useLocalFfmpeg" class="ext-sub-tip">{{ t('settings.video.customTip') }}</span>
                        </div>
                        <div v-if="customVideoExts.length > 0" class="ext-tags">
                            <el-tag
                                v-for="ext in customVideoExts"
                                :key="ext"
                                closable
                                type="primary"
                                effect="light"
                                :disabled="!useLocalFfmpeg"
                                @close="useLocalFfmpeg && removeVideoExt(ext)"
                            >
                                {{ ext }}
                            </el-tag>
                        </div>
                        <div v-else-if="useLocalFfmpeg" class="ext-empty">{{ t('settings.video.customEmpty') }}</div>
                        <div class="ext-input-row">
                            <el-input
                                v-model="newVideoExt"
                                :placeholder="t('settings.video.customPlaceholder')"
                                size="default"
                                class="ext-input"
                                :disabled="!useLocalFfmpeg"
                                @keyup.enter="addVideoExt"
                            />
                            <el-button type="primary" :disabled="!useLocalFfmpeg" @click="addVideoExt">
                                {{ t('common.add') }}
                            </el-button>
                        </div>
                    </div>
                </SettingItem>

                <SettingItem
                    id="cache"
                    :title="t('settings.cache.title')"
                    :description="t('settings.cache.description')"
                    :icon="Delete"
                >
                    <div class="cache-list">
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">{{ t('settings.cache.ffmpegTitle') }}</div>
                                <div class="cache-item-desc">{{ t('settings.cache.ffmpegDesc') }}</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingCache" @click="handleClearCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">{{ t('settings.cache.clear') }}</span>
                            </el-button>
                        </div>
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">{{ t('settings.cache.imageTitle') }}</div>
                                <div class="cache-item-desc">{{ t('settings.cache.imageDesc') }}</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingImageCache" @click="handleClearImageCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">{{ t('settings.cache.clear') }}</span>
                            </el-button>
                        </div>
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">{{ t('settings.cache.pdfTitle') }}</div>
                                <div class="cache-item-desc">{{ t('settings.cache.pdfDesc') }}</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingPdfCache" @click="handleClearPdfCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">{{ t('settings.cache.clear') }}</span>
                            </el-button>
                        </div>
                    </div>
                </SettingItem>

                <SettingItem
                    id="log"
                    :title="t('settings.log.title')"
                    :description="t('settings.log.description')"
                    :icon="Files"
                >
                    <div class="log-row">
                        <span class="log-label">{{ t('settings.log.level') }}</span>
                        <el-radio-group v-model="logLevel" @change="handleLogLevelChange">
                            <el-radio-button v-for="item in logLevelList" :key="item.value" :value="item.value">
                                {{ item.label }}
                            </el-radio-button>
                        </el-radio-group>
                    </div>
                </SettingItem>

                <SettingItem
                    id="version"
                    :title="t('settings.about.title')"
                    :description="t('settings.about.description')"
                    :icon="InfoFilled"
                >
                    <div class="about-row">
                        <div class="about-info">
                            <div class="about-version-label">{{ t('settings.about.currentVersion') }}</div>
                            <div class="about-version-value">v{{ version || '—' }}</div>
                        </div>
                        <div class="about-links">
                            <el-link
                                type="primary"
                                href="https://github.com/GuoJikun/quicklook/releases/latest"
                                target="_blank"
                                :underline="false"
                            >
                                GitHub Releases
                            </el-link>
                            <el-divider direction="vertical" />
                            <el-link
                                type="primary"
                                href="https://gitee.com/guojikun/quicklook/releases/latest"
                                target="_blank"
                                :underline="false"
                            >
                                Gitee Releases
                            </el-link>
                        </div>
                    </div>
                </SettingItem>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.settings {
    width: 100%;
    min-height: 100vh;
    background-color: var(--color-bg);
    box-sizing: border-box;
    padding: var(--space-4) var(--space-5) var(--space-6);
    overflow-x: clip;
}

.settings-inner {
    max-width: 880px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
}

.settings-nav {
    position: sticky;
    top: 0;
    z-index: var(--z-sticky);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-2) var(--space-4);
    overflow-x: auto;
    :deep(.el-anchor) {
        display: flex;
        flex-wrap: nowrap;
        justify-content: center;
    }
    :deep(.el-anchor__list) {
        display: flex;
        flex-wrap: nowrap;
        gap: var(--space-2);
    }
    :deep(.el-anchor__item) {
        padding: 0;
    }
    :deep(.el-anchor-link__title) {
        font-size: var(--font-sm);
        padding: 4px 8px;
        border-radius: 6px;
        transition: background-color var(--transition-base);
    }
    :deep(.el-anchor-link__title:hover) {
        background-color: var(--color-hover-bg);
    }
    :deep(.el-anchor-link--active .el-anchor-link__title) {
        color: var(--color-accent);
        background-color: var(--color-hover-bg);
        font-weight: 600;
    }
}

.settings-content {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.format-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-3);
}

.format-card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    background: var(--color-bg);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    transition: border-color var(--transition-base);

    &:hover {
        border-color: var(--color-accent);
    }

    &-head {
        display: flex;
        align-items: center;
        gap: var(--space-2);
    }

    &-code {
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: var(--font-xs);
        font-weight: 600;
        padding: 2px 8px;
        border-radius: var(--radius-sm);
        background: var(--color-accent);
        color: #fff;
    }

    &-name {
        font-size: var(--font-sm);
        color: var(--color-text-secondary);
    }

    &-exts {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        :deep(.el-tag) {
            margin: 0;
        }
    }
}

.ext-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    :deep(.el-tag) {
        margin: 0;
    }
}

.ext-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    .ext-input {
        flex: 0 0 200px;
    }
}

.ext-empty {
    font-size: var(--font-sm);
    color: var(--color-text-disabled);
    padding: 4px 0;
}

.ext-sub {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: var(--space-3) 14px;
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);

    &-title {
        font-size: var(--font-sm);
        font-weight: 600;
        color: var(--color-text-primary);
    }

    &-tip {
        margin-left: 6px;
        font-weight: normal;
        font-size: var(--font-xs);
        color: var(--el-color-warning);
    }
}

.field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 4px 0;

    &-label {
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
        flex: 1;
    }

    &-title {
        font-size: var(--font-md);
        font-weight: 500;
        color: var(--color-text-primary);
    }

    &-desc {
        font-size: var(--font-xs);
        color: var(--color-text-secondary);
        line-height: 1.5;
    }

    &-control {
        flex: 0 0 auto;
    }
}

.cache-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
}

.cache-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) 14px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);

    &-info {
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
        flex: 1;
    }

    &-title {
        font-size: var(--font-md);
        font-weight: 500;
        color: var(--color-text-primary);
    }

    &-desc {
        font-size: var(--font-xs);
        color: var(--color-text-secondary);
        line-height: 1.5;
    }
}

.log-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
}

.log-label {
    font-size: var(--font-md);
    font-weight: 500;
    color: var(--color-text-primary);
}

.about-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) 14px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    flex-wrap: wrap;
}

.about-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.about-version-label {
    font-size: var(--font-xs);
    color: var(--color-text-secondary);
}

.about-version-value {
    font-size: var(--font-xl);
    font-weight: 600;
    color: var(--color-text-primary);
    font-variant-numeric: tabular-nums;
}

.about-links {
    display: flex;
    align-items: center;
    gap: 4px;
}

@media (max-width: 640px) {
    .settings {
        padding: 12px 12px 24px;
    }
    .format-grid {
        grid-template-columns: 1fr;
    }
    .field-row,
    .cache-item,
    .about-row {
        flex-direction: column;
        align-items: flex-start;
    }
    .ext-input-row .ext-input {
        flex: 1 1 auto;
        width: 100%;
    }
    .ext-input-row {
        flex-wrap: wrap;
    }
}
</style>

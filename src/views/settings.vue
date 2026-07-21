<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
import { readTextFile } from '@/utils'
import { app } from '@tauri-apps/api'
import { load, type Store } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'
import { LogLevel } from '@tauri-apps/plugin-log'
import { Delete, EditPen, Files, InfoFilled, VideoCamera } from '@element-plus/icons-vue'

import SettingItem from '@/components/setting-item.vue'

let localStore: Store | null = null
const loadStore = async () => {
    localStore = await load('config.data', { autoSave: false, defaults: {} })
}

interface FormatSupport {
    name: string
    code: string
    data: string[]
}

const getConfig = async (): Promise<FormatSupport[]> => {
    const config = await readTextFile('config.json', { baseDir: BaseDirectory.Resource })
    const data = JSON.parse(config)
    return [
        { name: 'Markdown', code: 'Md', data: data['preview.markdown.checked'] },
        { name: '图片', code: 'Image', data: data['preview.image.checked'] },
        { name: '视频', code: 'Video', data: data['preview.video.checked'] },
        { name: '文档', code: 'Doc', data: data['preview.doc.checked'] },
        { name: '代码', code: 'Code', data: data['preview.code.checked'] },
        { name: '字体', code: 'Font', data: data['preview.font.checked'] },
        { name: '压缩包', code: 'Archive', data: data['preview.archive.checked'] },
        { name: '书籍', code: 'Book', data: data['preview.book.checked'] },
        { name: '3D模型', code: 'Model', data: data['preview.model.checked'] },
    ]
}

const config = ref<FormatSupport[]>([])
const version = ref<string>('')

const logLevel = ref<string | number>('info')
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
        ElMessage.warning(`扩展名 "${ext}" 已存在`)
        return
    }
    customCodeExts.value.push(ext)
    newCodeExt.value = ''
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
    ElMessage.success(`已添加代码格式：${ext}`)
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
        ElMessage.warning(`扩展名 "${ext}" 已存在`)
        return
    }
    customVideoExts.value.push(ext)
    newVideoExt.value = ''
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
    ElMessage.success(`已添加视频格式：${ext}`)
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
    logLevel.value = tmpLogLevel || 'info'

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
        ElMessage.success(`已清理 ${removed} 个缓存目录`)
    } catch (e) {
        ElMessage.error(`清理缓存失败：${e}`)
    } finally {
        clearingCache.value = false
    }
}

const clearingImageCache = ref<boolean>(false)
const handleClearImageCache = async () => {
    clearingImageCache.value = true
    try {
        await invoke('clear_image_cache')
        ElMessage.success('图片缓存已清理')
    } catch (e) {
        ElMessage.error(`清理图片缓存失败：${e}`)
    } finally {
        clearingImageCache.value = false
    }
}

const clearingPdfCache = ref<boolean>(false)
const handleClearPdfCache = async () => {
    clearingPdfCache.value = true
    try {
        const removed = await invoke<number>('clear_pdf_cache')
        ElMessage.success(`已清理 ${removed} 个 PDF 缓存文件`)
    } catch (e) {
        ElMessage.error(`清理 PDF 缓存失败：${e}`)
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
                    <el-anchor-link href="#support">支持的格式</el-anchor-link>
                    <el-anchor-link href="#custom-code">代码格式</el-anchor-link>
                    <el-anchor-link href="#video">视频</el-anchor-link>
                    <el-anchor-link href="#cache">缓存</el-anchor-link>
                    <el-anchor-link href="#log">日志</el-anchor-link>
                    <el-anchor-link href="#version">关于</el-anchor-link>
                </el-anchor>
            </nav>
            <div class="settings-content">
                <SettingItem
                    id="support"
                    title="支持的格式"
                    description="QuickLook 已识别的文件类型与对应扩展名。"
                    :icon="Files"
                >
                    <div class="format-grid">
                        <div v-for="type in config" :key="type.code" class="format-card">
                            <div class="format-card-head">
                                <span class="format-card-code">{{ type.code }}</span>
                                <span class="format-card-name">{{ type.name }}</span>
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
                    title="自定义代码格式"
                    description="添加额外的代码文件扩展名（如 rb、lua），添加后即可预览对应文件。"
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
                    <div v-else class="ext-empty">尚未添加自定义代码格式</div>
                    <div class="ext-input-row">
                        <el-input
                            v-model="newCodeExt"
                            placeholder="输入扩展名，如 rb"
                            size="default"
                            class="ext-input"
                            @keyup.enter="addCodeExt"
                        />
                        <el-button type="primary" @click="addCodeExt">添加</el-button>
                    </div>
                </SettingItem>

                <SettingItem
                    id="video"
                    title="视频"
                    description="启用本机 ffmpeg 后，将优先直接播放兼容格式/编码的视频，并对不兼容内容自动转码。"
                    :icon="VideoCamera"
                >
                    <div class="field-row">
                        <div class="field-row-label">
                            <span class="field-row-title">使用本机 ffmpeg 解析视频</span>
                            <span class="field-row-desc">兼容格式/编码的视频将直接播放，不兼容时才自动转码。</span>
                        </div>
                        <el-switch
                            v-model="useLocalFfmpeg"
                            class="field-row-control"
                            @change="handleUseLocalFfmpegChange"
                        />
                    </div>

                    <el-alert
                        v-if="useLocalFfmpeg"
                        :type="ffmpegAvailable === false ? 'error' : ffmpegAvailable ? 'success' : 'info'"
                        :title="
                            ffmpegAvailable === null
                                ? '正在检测 ffmpeg…'
                                : ffmpegAvailable
                                  ? '已检测到本机 ffmpeg'
                                  : '未检测到 ffmpeg'
                        "
                        :description="
                            ffmpegAvailable === false
                                ? '请安装 ffmpeg 并将其加入系统 PATH 后重启应用。'
                                : ffmpegAvailable
                                  ? '所有视频转码都将通过本机 ffmpeg 完成。'
                                  : '正在检测本机 ffmpeg 是否可用…'
                        "
                        :closable="false"
                        show-icon
                    />

                    <div class="ext-sub">
                        <div class="ext-sub-title">
                            <span>自定义视频格式扩展名</span>
                            <span v-if="!useLocalFfmpeg" class="ext-sub-tip">（需先启用 ffmpeg 解析）</span>
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
                        <div v-else-if="useLocalFfmpeg" class="ext-empty">尚未添加自定义视频格式</div>
                        <div class="ext-input-row">
                            <el-input
                                v-model="newVideoExt"
                                placeholder="输入扩展名，如 ts"
                                size="default"
                                class="ext-input"
                                :disabled="!useLocalFfmpeg"
                                @keyup.enter="addVideoExt"
                            />
                            <el-button type="primary" :disabled="!useLocalFfmpeg" @click="addVideoExt">
                                添加
                            </el-button>
                        </div>
                    </div>
                </SettingItem>

                <SettingItem
                    id="cache"
                    title="缓存"
                    description="管理由预览生成的临时缓存文件，释放磁盘空间。"
                    :icon="Delete"
                >
                    <div class="cache-list">
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">ffmpeg 转码缓存</div>
                                <div class="cache-item-desc">由视频转码生成的临时 HLS 缓存文件。</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingCache" @click="handleClearCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">清理缓存</span>
                            </el-button>
                        </div>
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">图片预览缓存</div>
                                <div class="cache-item-desc">由图片预览（含 PSD）转码生成的临时 PNG 缓存文件。</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingImageCache" @click="handleClearImageCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">清理缓存</span>
                            </el-button>
                        </div>
                        <div class="cache-item">
                            <div class="cache-item-info">
                                <div class="cache-item-title">PDF 渲染缓存</div>
                                <div class="cache-item-desc">由 PDF 预览渲染生成的临时 PNG 缓存文件。</div>
                            </div>
                            <el-button type="danger" plain :loading="clearingPdfCache" @click="handleClearPdfCache">
                                <el-icon><Delete /></el-icon>
                                <span style="margin-left: 4px">清理缓存</span>
                            </el-button>
                        </div>
                    </div>
                </SettingItem>

                <SettingItem
                    id="log"
                    title="日志"
                    description="调整应用日志的输出级别。级别越详细，输出越多。"
                    :icon="Files"
                >
                    <div class="log-row">
                        <span class="log-label">日志级别</span>
                        <el-radio-group v-model="logLevel" @change="handleLogLevelChange">
                            <el-radio-button v-for="item in logLevelList" :key="item.value" :value="item.value">
                                {{ item.label }}
                            </el-radio-button>
                        </el-radio-group>
                    </div>
                </SettingItem>

                <SettingItem id="version" title="关于" description="版本信息与最新发布渠道。" :icon="InfoFilled">
                    <div class="about-row">
                        <div class="about-info">
                            <div class="about-version-label">当前版本</div>
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

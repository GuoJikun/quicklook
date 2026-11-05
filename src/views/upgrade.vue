<script lang="ts" setup>
import { check, type Update } from '@tauri-apps/plugin-updater'
import { useI18n } from 'vue-i18n'
import { currentLocale } from '@/locales'
import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { error as logError, info } from '@tauri-apps/plugin-log'
import { Check, CircleClose, Download, Refresh, Warning } from '@element-plus/icons-vue'
import { app } from '@tauri-apps/api'
import MdViewer from '@/components/md-viewer/index.vue'
import { createMd } from '@/utils/markdown/index'
import { formatBytes } from '@/utils/index'
import type MarkdownIt from 'markdown-it'

const { t } = useI18n()

type Phase = 'idle' | 'downloading' | 'finished' | 'cancelled'

interface DownloadProgress {
    downloaded: number
    total: number
    lastTick: number
    lastDownloaded: number
    speed: number
}

const checking = ref(true)
const upgradeAvailable = ref(false)
const releaseNotes = ref('')
const currentVersion = ref('')
const newVersion = ref('')
const releaseDate = ref('')
const errMsg = ref('')

const phase = ref<Phase>('idle')
const progress = ref<DownloadProgress>({
    downloaded: 0,
    total: 0,
    lastTick: 0,
    lastDownloaded: 0,
    speed: 0,
})
const mdLoading = ref(false)

const updater = shallowRef<Update | null>(null)
let md: MarkdownIt | null = null
let speedTimer: number | null = null
let downloadCancelled = false

const percentage = computed(() => {
    if (!progress.value.total) return 0
    return Math.min(100, parseFloat(((progress.value.downloaded / progress.value.total) * 100).toFixed(2)))
})

const formatSpeed = (bytesPerSec: number): string => `${formatBytes(bytesPerSec)}/s`

const formatDate = (iso: string): string => {
    if (!iso) return ''
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    return d.toLocaleString(currentLocale.value, { dateStyle: 'long', timeStyle: 'short' })
}

const renderNotes = async (txt: string) => {
    mdLoading.value = true
    try {
        if (md === null) {
            md = await createMd()
        }
        const cleaned = txt
            .replace(/^"|"$/, '')
            .replace(/\\r\\n/g, '\n')
            .replace(/\\n/g, '\n')
        releaseNotes.value = md.render(cleaned)
    } finally {
        mdLoading.value = false
    }
}

const doCheck = async () => {
    checking.value = true
    errMsg.value = ''
    try {
        if (!currentVersion.value) {
            currentVersion.value = await app.getVersion()
        }
        const result = await check()
        if (result) {
            updater.value = result
            upgradeAvailable.value = result.available !== false
            newVersion.value = result.version ?? ''
            releaseDate.value = result.date ?? ''
            await renderNotes(result.body ?? '')
        } else {
            upgradeAvailable.value = false
            newVersion.value = ''
            releaseDate.value = ''
            releaseNotes.value = ''
        }
    } catch (e) {
        errMsg.value = String((e as Error)?.message ?? e)
        logError(`check upgrade failed: ${e}`)
    } finally {
        checking.value = false
    }
}

const resetProgress = () => {
    progress.value = {
        downloaded: 0,
        total: 0,
        lastTick: Date.now(),
        lastDownloaded: 0,
        speed: 0,
    }
}

const startDownload = async () => {
    let u = updater.value
    if (phase.value === 'cancelled') {
        try {
            const fresh = await check()
            if (!fresh) {
                ElMessage.warning(t('upgrade.noNewVersion'))
                phase.value = 'idle'
                return
            }
            updater.value = fresh
            u = fresh
        } catch (e) {
            errMsg.value = t('upgrade.checkError', { error: String((e as Error)?.message ?? e) })
            ElMessage.error(errMsg.value)
            logError(`recheck after cancel failed: ${e}`)
            return
        }
    }
    if (!u || !upgradeAvailable.value) return
    downloadCancelled = false
    phase.value = 'downloading'
    resetProgress()
    info('开始下载更新')
    speedTimer = window.setInterval(() => {
        if (phase.value !== 'downloading') return
        const now = Date.now()
        const dt = (now - progress.value.lastTick) / 1000
        if (dt >= 0.25) {
            const dBytes = progress.value.downloaded - progress.value.lastDownloaded
            progress.value.speed = dBytes / dt
            progress.value.lastTick = now
            progress.value.lastDownloaded = progress.value.downloaded
        }
    }, 500)
    try {
        await u.downloadAndInstall(payload => {
            const { event } = payload
            if (event === 'Started') {
                progress.value.total = payload.data?.contentLength ?? 0
                progress.value.lastTick = Date.now()
                progress.value.lastDownloaded = 0
                info(`started downloading ${progress.value.total} bytes`)
            } else if (event === 'Progress') {
                const chunk = payload.data?.chunkLength ?? 0
                progress.value.downloaded += chunk
            } else if (event === 'Finished') {
                phase.value = 'finished'
                progress.value.downloaded = progress.value.total
                progress.value.speed = 0
                info('download finished')
            }
        })
    } catch (e) {
        if (!downloadCancelled) {
            phase.value = 'idle'
            errMsg.value = t('upgrade.downloadError', { error: String((e as Error)?.message ?? e) })
            ElMessage.error(errMsg.value)
            logError(`upgrade failed: ${e}`)
        }
    } finally {
        if (speedTimer !== null) {
            clearInterval(speedTimer)
            speedTimer = null
        }
    }
}

const cancelDownload = async () => {
    downloadCancelled = true
    const u = updater.value
    updater.value = null
    phase.value = 'cancelled'
    progress.value.speed = 0
    ElMessage.info(t('upgrade.cancelled'))
    if (u) {
        try {
            await u.close()
        } catch (e) {
            logError(`close updater failed: ${e}`)
        }
    }
}

onMounted(doCheck)

onUnmounted(() => {
    if (speedTimer !== null) {
        clearInterval(speedTimer)
        speedTimer = null
    }
})
</script>

<template>
    <div class="upgrade">
        <div class="upgrade-card" v-loading="checking" :element-loading-text="t('upgrade.checking')">
            <header class="upgrade-header">
                <div class="upgrade-header-left">
                    <h2 class="upgrade-title">{{ t('upgrade.title') }}</h2>
                    <span v-if="currentVersion && !checking" class="upgrade-version">v{{ currentVersion }}</span>
                </div>
            </header>

            <template v-if="!checking">
                <section v-if="errMsg" class="state state-error">
                    <el-icon class="state-icon" :size="56"><Warning /></el-icon>
                    <p class="state-title">{{ t('upgrade.checkFailed') }}</p>
                    <p class="state-desc">{{ errMsg }}</p>
                </section>

                <section v-else-if="!upgradeAvailable" class="state state-latest">
                    <div class="state-icon-wrap success">
                        <el-icon :size="32"><Check /></el-icon>
                    </div>
                    <p class="state-title">{{ t('upgrade.upToDate') }}</p>
                    <p class="state-desc" v-if="currentVersion">v{{ currentVersion }}</p>
                </section>

                <section v-else class="state state-upgrade">
                    <div class="upgrade-banner">
                        <div class="banner-row">
                            <span class="banner-label">{{ t('upgrade.latestVersion') }}</span>
                            <span class="banner-version">v{{ newVersion || '—' }}</span>
                        </div>
                        <div class="banner-row" v-if="releaseDate">
                            <span class="banner-label">{{ t('upgrade.releaseDate') }}</span>
                            <span class="banner-value">{{ formatDate(releaseDate) }}</span>
                        </div>
                    </div>

                    <div class="section-title">{{ t('upgrade.releaseNotes') }}</div>
                    <div class="notes-wrapper" v-loading="mdLoading">
                        <MdViewer :content="releaseNotes" />
                    </div>

                    <div v-if="phase === 'downloading' || phase === 'finished'" class="progress-block">
                        <div class="progress-header">
                            <span class="progress-label">
                                {{ phase === 'finished' ? t('upgrade.downloadFinished') : t('upgrade.downloading') }}
                            </span>
                            <span class="progress-stats">
                                {{ formatBytes(progress.downloaded) }} / {{ formatBytes(progress.total) }}
                                <span v-if="phase === 'downloading' && progress.speed > 0" class="progress-speed">
                                    · {{ formatSpeed(progress.speed) }}
                                </span>
                            </span>
                        </div>
                        <el-progress
                            :percentage="percentage"
                            :stroke-width="6"
                            :show-text="true"
                            :status="phase === 'finished' ? 'success' : undefined"
                        />
                    </div>
                </section>

                <footer class="upgrade-footer">
                    <el-button @click="doCheck" :disabled="phase === 'downloading'">
                        <el-icon><Refresh /></el-icon>
                        <span>{{ t('upgrade.recheck') }}</span>
                    </el-button>
                    <template v-if="errMsg">
                        <el-button type="primary" @click="doCheck">{{ t('upgrade.retry') }}</el-button>
                    </template>
                    <template v-else-if="!upgradeAvailable">
                        <el-button type="primary" @click="doCheck">{{ t('upgrade.checkAgain') }}</el-button>
                    </template>
                    <template v-else>
                        <!-- 取消不是破坏性操作，Fluent 2 里不用警示色：降为次级按钮即可 -->
                        <el-button v-if="phase === 'downloading'" :icon="CircleClose" @click="cancelDownload">
                            {{ t('upgrade.cancelDownload') }}
                        </el-button>
                        <el-button
                            v-else
                            type="primary"
                            :icon="Download"
                            :disabled="phase === 'finished'"
                            @click="startDownload"
                        >
                            {{ phase === 'finished' ? t('upgrade.downloadFinished') : t('upgrade.updateNow') }}
                        </el-button>
                    </template>
                </footer>
            </template>
        </div>
    </div>
</template>

<style scoped lang="scss">
.upgrade {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    /* 原先是一条硬编码的浅蓝渐变：深色模式下整页发白，且 Fluent 2 不用装饰性渐变 */
    background: var(--color-bg);
    padding: var(--space-4);
    box-sizing: border-box;
    overflow: auto;

    &-card {
        width: 100%;
        background: var(--color-surface);
        /* 边框而非投影：与设置页卡片、空态卡片保持同一种「浮起」语言 */
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xl);
        padding: var(--space-5);
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: var(--space-4);
        min-height: 320px;
    }

    &-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-bottom: var(--space-3);
        border-bottom: 1px solid var(--color-border);

        &-left {
            display: flex;
            align-items: center;
            gap: var(--space-3);
        }
    }

    &-title {
        margin: 0;
        font-size: var(--font-2xl);
        font-weight: 600;
        color: var(--color-text-primary);
    }

    &-footer {
        display: flex;
        justify-content: flex-end;
        gap: var(--space-2);
        padding-top: var(--space-3);
        border-top: 1px solid var(--color-border);
        margin-top: auto;
    }
}

/* 版本号胶囊：取代 el-tag，避免 Element 自带的 info 蓝底与项目 chip 语言冲突 */
.upgrade-version {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
    background: var(--color-chip-bg);
    color: var(--color-text-secondary);
    font-size: var(--font-xs);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
}

.state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-start;
    padding: 20px 0;
    text-align: left;
    gap: var(--space-2);
    flex: 1;
    width: 100%;
    &-icon {
        color: var(--color-danger-fg);
    }

    /* 状态图标容器：中性底 + 语义前景色。
       原先是绿色实心圆加绿色投影 —— Fluent 2 的状态色只作前景，不做彩色底块。 */
    &-icon-wrap {
        width: 56px;
        height: 56px;
        border-radius: var(--radius-xl);
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--color-chip-bg);
        color: var(--color-text-primary);

        &.success {
            color: var(--color-success-fg);
        }
    }

    &-title {
        margin: 4px 0 0;
        font-size: var(--font-xl);
        font-weight: 600;
        color: var(--color-text-primary);
    }

    &-desc {
        margin: 0;
        font-size: var(--font-sm);
        color: var(--color-text-secondary);
        word-break: break-word;
        max-width: 480px;
    }

    &-latest {
        /* 状态色只用于图标，标题保持主文字色 —— 标题染绿会与「可点击」混淆 */
        .state-title {
            color: var(--color-text-primary);
        }
    }
}

/* 版本信息条：品牌浅底 + 3px 品牌指示条，与导航选中态同源 */
.upgrade-banner {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--space-3) var(--space-4);
    background: var(--color-accent-soft);
    border-radius: var(--radius-md);
    border-left: 3px solid var(--color-brand-fg);
    width: 100%;
    .banner-row {
        display: flex;
        align-items: center;
        gap: var(--space-3);
        font-size: var(--font-sm);
    }

    .banner-label {
        color: var(--color-text-secondary);
        min-width: 64px;
    }

    .banner-version {
        font-size: var(--font-2xl);
        font-weight: 600;
        color: var(--color-brand-fg);
    }

    .banner-value {
        color: var(--color-text-primary);
    }
}

.section-title {
    font-size: var(--font-sm);
    font-weight: 600;
    color: var(--color-text-primary);
    margin: 4px 0 4px;
}

.notes-wrapper {
    min-height: 160px;
    max-height: 320px;
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-sizing: border-box;
}

.progress-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg);
    border-radius: var(--radius-md);
    width: 100%;
    :deep(.el-progress) {
        width: 100%;
    }
}

.progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--font-sm);
    color: var(--color-text-primary);
}

.progress-label {
    font-weight: 500;
}

.progress-speed {
    margin-left: 6px;
    color: var(--color-brand-fg);
    font-variant-numeric: tabular-nums;
}

.progress-stats {
    font-variant-numeric: tabular-nums;
    color: var(--color-text-secondary);
}

/* ---------- Element 按钮与进度条接入项目 token ----------
   el-button 默认自带 Element 的蓝色、500 字重与外阴影，与命令栏、设置页的按钮
   是两套语言。这里只在本窗口内映射：32px 高、4px 圆角、600 字重、无阴影。 */
.upgrade :deep(.el-button) {
    height: var(--control-md);
    padding: 0 14px;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-size: var(--font-sm);
    font-weight: 600;
    box-shadow: none;
    --el-button-bg-color: transparent;
    --el-button-border-color: var(--color-border);
    --el-button-text-color: var(--color-text-primary);
    --el-button-hover-bg-color: var(--elevation-hover);
    --el-button-hover-border-color: var(--color-border);
    --el-button-hover-text-color: var(--color-text-primary);
    --el-button-active-bg-color: var(--elevation-pressed);
    --el-button-active-border-color: var(--color-border);
    --el-button-active-text-color: var(--color-text-primary);
}

.upgrade :deep(.el-button--primary) {
    --el-button-bg-color: var(--btn-primary-bg);
    --el-button-border-color: var(--btn-primary-bg);
    --el-button-text-color: var(--btn-primary-text);
    --el-button-hover-bg-color: var(--btn-primary-hover-bg);
    --el-button-hover-border-color: var(--btn-primary-hover-bg);
    --el-button-hover-text-color: var(--btn-primary-text);
    --el-button-active-bg-color: var(--btn-primary-active-bg);
    --el-button-active-border-color: var(--btn-primary-active-bg);
    --el-button-active-text-color: var(--btn-primary-text);
}

.upgrade :deep(.el-button.is-disabled),
.upgrade :deep(.el-button.is-disabled:hover) {
    box-shadow: none;
    --el-button-disabled-bg-color: var(--color-chip-bg);
    --el-button-disabled-border-color: var(--color-border);
    --el-button-disabled-text-color: var(--color-text-disabled);
}

.upgrade :deep(.el-button .el-icon) {
    font-size: var(--icon-sm);
}

/* 图标与文字的间距交给外边距，取代原先写在模板里的内联 style */
.upgrade :deep(.el-button .el-icon + span) {
    margin-left: 6px;
}

/* 进度条：Fluent 2 是细条 + 右侧百分比，不用条纹流动动画与条内文字 */
.upgrade :deep(.el-progress-bar__outer) {
    border-radius: 3px;
    background-color: var(--color-chip-bg);
}

.upgrade :deep(.el-progress__text) {
    font-size: var(--font-xs);
    color: var(--color-text-secondary);
}
</style>

<script lang="ts" setup>
import { check, type Update } from '@tauri-apps/plugin-updater'
import { computed, onMounted, ref, shallowRef } from 'vue'
import { error as logError, info } from '@tauri-apps/plugin-log'
import { ElMessage } from 'element-plus'
import { Check, CircleClose, Download, Refresh, Warning } from '@element-plus/icons-vue'
import { app } from '@tauri-apps/api'
import MdViewer from '@/components/md-viewer/index.vue'
import { createMd } from '@/utils/markdown/index'
import type MarkdownIt from 'markdown-it'

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

const formatBytes = (bytes: number): string => {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
    const units = ['B', 'KB', 'MB', 'GB']
    let n = bytes
    let i = 0
    while (n >= 1024 && i < units.length - 1) {
        n /= 1024
        i++
    }
    const fixed = n >= 100 || i === 0 ? 0 : n >= 10 ? 1 : 2
    return `${n.toFixed(fixed)} ${units[i]}`
}

const formatSpeed = (bytesPerSec: number): string => `${formatBytes(bytesPerSec)}/s`

const formatDate = (iso: string): string => {
    if (!iso) return ''
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    return d.toLocaleString('zh-CN', { dateStyle: 'long', timeStyle: 'short' })
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
                ElMessage.warning('当前已无新版本可更新')
                phase.value = 'idle'
                return
            }
            updater.value = fresh
            u = fresh
        } catch (e) {
            errMsg.value = `检查更新失败：${(e as Error)?.message ?? e}`
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
            errMsg.value = `下载更新失败：${(e as Error)?.message ?? e}`
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
    ElMessage.info('已取消更新下载')
    if (u) {
        try {
            await u.close()
        } catch (e) {
            logError(`close updater failed: ${e}`)
        }
    }
}

onMounted(doCheck)
</script>

<template>
    <div class="upgrade">
        <div class="upgrade-card" v-loading="checking" element-loading-text="正在检查更新…">
            <header class="upgrade-header">
                <div class="upgrade-header-left">
                    <h2 class="upgrade-title">软件更新</h2>
                    <el-tag v-if="currentVersion && !checking" size="small" type="info" effect="plain">
                        v{{ currentVersion }}
                    </el-tag>
                </div>
            </header>

            <template v-if="!checking">
                <section v-if="errMsg" class="state state-error">
                    <el-icon class="state-icon" :size="56"><Warning /></el-icon>
                    <p class="state-title">检查更新失败</p>
                    <p class="state-desc">{{ errMsg }}</p>
                </section>

                <section v-else-if="!upgradeAvailable" class="state state-latest">
                    <div class="state-icon-wrap success">
                        <el-icon :size="36"><Check /></el-icon>
                    </div>
                    <p class="state-title">当前已经是最新版本</p>
                    <p class="state-desc" v-if="currentVersion">v{{ currentVersion }}</p>
                </section>

                <section v-else class="state state-upgrade">
                    <div class="upgrade-banner">
                        <div class="banner-row">
                            <span class="banner-label">最新版本</span>
                            <span class="banner-version">v{{ newVersion || '—' }}</span>
                        </div>
                        <div class="banner-row" v-if="releaseDate">
                            <span class="banner-label">发布日期</span>
                            <span class="banner-value">{{ formatDate(releaseDate) }}</span>
                        </div>
                    </div>

                    <div class="section-title">更新内容</div>
                    <div class="notes-wrapper" v-loading="mdLoading">
                        <MdViewer :content="releaseNotes" />
                    </div>

                    <div v-if="phase === 'downloading' || phase === 'finished'" class="progress-block">
                        <div class="progress-header">
                            <span class="progress-label">
                                {{ phase === 'finished' ? '下载完成' : '正在下载更新…' }}
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
                            :stroke-width="14"
                            :status="phase === 'finished' ? 'success' : undefined"
                            text-inside
                            striped
                            striped-flow
                        />
                    </div>
                </section>

                <footer class="upgrade-footer">
                    <el-button @click="doCheck" :disabled="phase === 'downloading'">
                        <el-icon><Refresh /></el-icon>
                        <span style="margin-left: 4px">重新检查</span>
                    </el-button>
                    <template v-if="errMsg">
                        <el-button type="primary" @click="doCheck">重试</el-button>
                    </template>
                    <template v-else-if="!upgradeAvailable">
                        <el-button type="primary" plain @click="doCheck">再次检查</el-button>
                    </template>
                    <template v-else>
                        <el-button
                            v-if="phase === 'downloading'"
                            type="warning"
                            :icon="CircleClose"
                            @click="cancelDownload"
                        >
                            取消下载
                        </el-button>
                        <el-button
                            v-else
                            type="primary"
                            :icon="Download"
                            :disabled="phase === 'finished'"
                            @click="startDownload"
                        >
                            {{ phase === 'finished' ? '下载完成' : '立即更新' }}
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
    background: linear-gradient(135deg, #f5f7fa 0%, #e4ecf7 100%);
    padding: 16px;
    box-sizing: border-box;
    overflow: auto;

    &-card {
        width: 100%;
        background: #fff;
        border-radius: 12px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.08);
        padding: 24px;
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 16px;
        min-height: 320px;
    }

    &-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-bottom: 12px;
        border-bottom: 1px solid var(--el-border-color-lighter, #ebeef5);

        &-left {
            display: flex;
            align-items: center;
            gap: 12px;
        }
    }

    &-title {
        margin: 0;
        font-size: 18px;
        font-weight: 600;
        color: var(--el-text-color-primary, #303133);
    }

    &-footer {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        padding-top: 12px;
        border-top: 1px solid var(--el-border-color-lighter, #ebeef5);
        margin-top: auto;
    }
}

.state {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: flex-start;
    padding: 20px 0;
    text-align: left;
    gap: 8px;
    flex: 1;
    width: 100%;
    &-icon {
        color: var(--el-color-danger, #f56c6c);
    }

    &-icon-wrap {
        width: 64px;
        height: 64px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #fff;

        &.success {
            background: var(--el-color-success, #67c23a);
            box-shadow: 0 4px 16px rgba(103, 194, 58, 0.35);
        }
    }

    &-title {
        margin: 4px 0 0;
        font-size: 16px;
        font-weight: 600;
        color: var(--el-text-color-primary, #303133);
    }

    &-desc {
        margin: 0;
        font-size: 13px;
        color: var(--el-text-color-secondary, #909399);
        word-break: break-word;
        max-width: 480px;
    }

    &-latest {
        .state-title {
            color: var(--el-color-success, #67c23a);
        }
    }
}

.upgrade-banner {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 16px;
    background: linear-gradient(135deg, #ecf5ff 0%, #f0f9ff 100%);
    border-radius: 8px;
    border-left: 3px solid var(--el-color-primary, #409eff);
    width: 100%;
    .banner-row {
        display: flex;
        align-items: center;
        gap: 12px;
        font-size: 13px;
    }

    .banner-label {
        color: var(--el-text-color-secondary, #909399);
        min-width: 64px;
    }

    .banner-version {
        font-size: 18px;
        font-weight: 600;
        color: var(--el-color-primary, #409eff);
    }

    .banner-value {
        color: var(--el-text-color-regular, #606266);
    }
}

.section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--el-text-color-regular, #606266);
    margin: 4px 0 4px;
}

.notes-wrapper {
    min-height: 160px;
    max-height: 320px;
    overflow-y: auto;
    padding: 12px 16px;
    background: var(--el-fill-color-blank, #fafafa);
    border: 1px solid var(--el-border-color-lighter, #ebeef5);
    border-radius: 8px;
    box-sizing: border-box;
}

.progress-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    background: var(--el-fill-color-light, #f5f7fa);
    border-radius: 8px;
    width: 100%;
    :deep(.el-progress) {
        width: 100%;
    }
}

.progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 13px;
    color: var(--el-text-color-regular, #606266);
}

.progress-label {
    font-weight: 500;
}

.progress-speed {
    margin-left: 6px;
    color: var(--el-color-primary, #409eff);
    font-variant-numeric: tabular-nums;
}

.progress-stats {
    font-variant-numeric: tabular-nums;
    color: var(--el-text-color-secondary, #909399);
}
</style>

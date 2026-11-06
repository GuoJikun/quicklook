<script lang="ts" setup>
import { check, type Update } from '@tauri-apps/plugin-updater'
import { useI18n } from 'vue-i18n'
import { currentLocale } from '@/locales'
import { computed, onMounted, onUnmounted, ref, shallowRef } from 'vue'
import { error as logError, info } from '@tauri-apps/plugin-log'
import { Checkmark32Regular, DismissCircle32Regular } from '@vicons/fluent'
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

/** 进度条右侧的整数百分比：小数位在 12px 字号下只会变成噪点 */
const percentText = computed(() => `${Math.round(percentage.value)}%`)

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
    <!-- 640×600 的对话框窗口。结构照 Fluent 2 的 ContentDialog：标题区 / 内容区 / 页脚
         直接铺在整窗 surface 上，不再「灰底 + 白卡片」—— 那是网页的分层方式，
         而 Windows 的原生对话框客户区就是一整块 surface（标题栏的灰由 OS 绘制）。 -->
    <div class="uq-page" :aria-busy="checking">
        <header class="uq-head">
            <h1 class="uq-title">{{ t('upgrade.title') }}</h1>
            <span v-if="currentVersion && !checking" class="uq-chip">v{{ currentVersion }}</span>
        </header>

        <div class="uq-main">
            <!-- 检查中：Fluent 的 ProgressRing 是内联的细环，不是盖住整页的遮罩 -->
            <div v-if="checking" class="uq-status" role="status" aria-live="polite">
                <span class="uq-ring" aria-hidden="true"></span>
                <p class="uq-status-desc">{{ t('upgrade.checking') }}</p>
            </div>

            <!-- 检查失败：详情可能很长，整块左对齐，不做居中 -->
            <div v-else-if="errMsg" class="uq-status">
                <DismissCircle32Regular class="uq-status-icon is-danger" aria-hidden="true" />
                <p class="uq-status-title">{{ t('upgrade.checkFailed') }}</p>
                <p class="uq-status-desc">{{ errMsg }}</p>
            </div>

            <!-- 已是最新 -->
            <div v-else-if="!upgradeAvailable" class="uq-status">
                <Checkmark32Regular class="uq-status-icon is-success" aria-hidden="true" />
                <p class="uq-status-title">{{ t('upgrade.upToDate') }}</p>
                <p v-if="currentVersion" class="uq-status-desc">v{{ currentVersion }}</p>
            </div>

            <!-- 有更新 -->
            <template v-else>
                <!-- 版本信息用键值表，不用品牌浅底信息条：
                     accent-soft 上放不了品牌色文字（4.07:1），而纯中性排版本身就是 Fluent 的语气 -->
                <dl class="uq-meta">
                    <div class="uq-meta-row">
                        <dt>{{ t('upgrade.latestVersion') }}</dt>
                        <dd class="is-version">v{{ newVersion || '—' }}</dd>
                    </div>
                    <div v-if="releaseDate" class="uq-meta-row">
                        <dt>{{ t('upgrade.releaseDate') }}</dt>
                        <dd>{{ formatDate(releaseDate) }}</dd>
                    </div>
                </dl>

                <div v-if="releaseNotes || mdLoading" class="uq-notes">
                    <div class="uq-notes-head">
                        <h2>{{ t('upgrade.releaseNotes') }}</h2>
                        <span v-if="mdLoading" class="uq-ring is-sm" aria-hidden="true"></span>
                    </div>
                    <div class="uq-notes-body">
                        <MdViewer :content="releaseNotes" />
                    </div>
                </div>

                <div v-if="phase === 'downloading' || phase === 'finished'" class="uq-progress">
                    <div class="uq-progress-head">
                        <span class="uq-progress-label">
                            {{ phase === 'finished' ? t('upgrade.downloadFinished') : t('upgrade.downloading') }}
                        </span>
                        <span class="uq-progress-pct">{{ percentText }}</span>
                    </div>
                    <div
                        class="uq-progress-track"
                        role="progressbar"
                        :aria-valuenow="Math.round(percentage)"
                        aria-valuemin="0"
                        aria-valuemax="100"
                        :aria-label="t('upgrade.downloading')"
                    >
                        <div
                            class="uq-progress-bar"
                            :class="{ 'is-done': phase === 'finished' }"
                            :style="{ width: `${percentage}%` }"
                        />
                    </div>
                    <div class="uq-progress-meta">
                        {{ formatBytes(progress.downloaded) }} / {{ formatBytes(progress.total) }}
                        <template v-if="phase === 'downloading' && progress.speed > 0">
                            · {{ formatSpeed(progress.speed) }}
                        </template>
                    </div>
                </div>
            </template>
        </div>

        <footer v-if="!checking" class="uq-foot">
            <!-- 页脚按钮不带图标：Fluent 2 的 Dialog footer 就是纯文字，
                 混入图标会把「次要 / 主要」两级在视觉重量上拉平 -->
            <button
                v-if="upgradeAvailable && !errMsg && phase !== 'downloading'"
                type="button"
                class="uq-btn"
                @click="doCheck"
            >
                {{ t('upgrade.recheck') }}
            </button>

            <button v-if="errMsg" type="button" class="uq-btn is-primary" @click="doCheck">
                {{ t('upgrade.retry') }}
            </button>
            <button v-else-if="!upgradeAvailable" type="button" class="uq-btn" @click="doCheck">
                {{ t('upgrade.checkAgain') }}
            </button>
            <button v-else-if="phase === 'downloading'" type="button" class="uq-btn" @click="cancelDownload">
                {{ t('upgrade.cancelDownload') }}
            </button>
            <button v-else type="button" class="uq-btn is-primary" :disabled="phase === 'finished'" @click="startDownload">
                {{ phase === 'finished' ? t('upgrade.downloadFinished') : t('upgrade.updateNow') }}
            </button>
        </footer>
    </div>
</template>

<style scoped lang="scss">
/* 整窗 surface：客户区不留灰边，圆角与描边由系统窗口绘制。
   这一页是对话框形态，不是「页面里放一张卡片」。 */
.uq-page {
    height: 100vh;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    color: var(--color-text-primary);
    font-family: inherit;
    overflow: hidden;
}

/* ---------- 标题区 ---------- */
.uq-head {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-5) var(--space-5) var(--space-3);
}

.uq-title {
    margin: 0;
    font-size: var(--font-2xl);
    font-weight: 600;
    line-height: var(--line-tight);
    color: var(--color-text-primary);
}

/* 版本号胶囊：与命令栏的数值框同一种语言（chip 底 + 等宽数字） */
.uq-chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: var(--control-sm);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--color-chip-bg);
    color: var(--color-text-secondary);
    font-size: var(--font-xs);
    line-height: 1;
    font-variant-numeric: tabular-nums;
}

/* ---------- 内容区 ---------- */
.uq-main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: 0 var(--space-5) var(--space-5);
    /* 内容超出时整块滚动，页脚始终固定 */
    overflow-y: auto;
}

/* 状态块：图标 / 标题 / 说明三段左对齐，垂直居中于内容区。
   有正文内容时左对齐是 Fluent 2 Dialog 的默认排版，长错误文本也能正常读 */
.uq-status {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: var(--space-2);
}

.uq-status-icon {
    flex: none;
    width: var(--icon-xl);
    height: var(--icon-xl);

    /* 状态色只作前景，不铺彩色底块 */
    &.is-danger {
        color: var(--color-danger-fg);
    }

    &.is-success {
        color: var(--color-success-fg);
    }
}

.uq-status-title {
    margin: var(--space-1) 0 0;
    font-size: var(--font-lg);
    font-weight: 600;
    line-height: var(--line-tight);
    color: var(--color-text-primary);
}

.uq-status-desc {
    margin: 0;
    max-width: 100%;
    font-size: var(--font-sm);
    line-height: var(--line-base);
    color: var(--color-text-secondary);
    overflow-wrap: anywhere;
}

/* ---------- 版本信息 ---------- */
.uq-meta {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
}

.uq-meta-row {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);

    dt {
        flex: none;
        width: 72px;
        font-size: var(--font-sm);
        line-height: var(--line-base);
        color: var(--color-text-secondary);
    }

    dd {
        margin: 0;
        font-size: var(--font-sm);
        line-height: var(--line-base);
        color: var(--color-text-primary);
    }

    dd.is-version {
        font-size: var(--font-lg);
        font-weight: 600;
        line-height: var(--line-tight);
        font-variant-numeric: tabular-nums;
    }
}

/* ---------- 更新内容 ---------- */
/* 不加底色与描边：卡片里再套一张卡片是三层嵌套，改用发丝线分区 */
.uq-notes {
    /* 不撑满内容区：更新说明短时若留出一大片空区，会被误读成「列表没加载出来」。
       长内容交给 .uq-main 整体滚动 —— 500px 高的窗口里嵌套滚动区只会让人滚错层。 */
    flex: 0 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-divider);
}

.uq-notes-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);

    /* 小标签语气：它只是内容的容器标签，不该和 changelog 里的
       「新增 / 修复」抢同一层级（那两级都是 14/600） */
    h2 {
        margin: 0;
        font-size: var(--font-xs);
        font-weight: 500;
        line-height: var(--line-base);
        letter-spacing: 0.02em;
        color: var(--color-text-secondary);
    }
}

.uq-notes-body {
    min-height: 0;
    /* 给滚动条留出通道，避免贴住右边缘 */
    padding-right: var(--space-2);

    /* md-viewer 默认是「阅读器」排版（h3 20px、列表上下各 16px、正文 1.75 行高），
       那是给 760px 阅读列用的。同一份 changelog 放进 600px 的对话框，两条变更
       就会撑满整屏。这里按 UI 正文（Body1 14/1.43）压一档，标题与列表的
       间距收到 4–12px —— 更新说明是清单，不是长文。 */
    :deep(.vp-doc) {
        font-size: var(--font-sm);
        line-height: var(--line-base);
        color: var(--color-text-primary);
        word-break: break-word;

        h1,
        h2,
        h3,
        h4,
        h5,
        h6 {
            margin: var(--space-3) 0 var(--space-1);
            padding: 0;
            border: 0;
            font-size: var(--font-sm);
            font-weight: 600;
            line-height: var(--line-base);
            letter-spacing: normal;
            color: var(--color-text-primary);
        }

        > :first-child {
            margin-top: 0;
        }

        > :last-child {
            margin-bottom: 0;
        }

        p {
            margin: var(--space-1) 0;
        }

        ul,
        ol {
            margin: var(--space-1) 0;
            padding-left: 18px;
        }

        li + li {
            margin-top: 2px;
        }

        li > ol,
        li > ul {
            margin: var(--space-1) 0 0;
        }

        li > p {
            margin: 0;
        }

        a {
            color: var(--color-brand-fg);
            text-decoration: underline;
            text-underline-offset: 2px;
        }

        :not(pre) > code {
            padding: 1px 4px;
            border-radius: var(--radius-xs);
            background: var(--color-chip-bg);
            font-size: var(--font-xs);
        }

        pre {
            margin: var(--space-2) 0;
            padding: var(--space-2) var(--space-3);
            border-radius: var(--radius-sm);
            background: var(--color-bg);
            overflow-x: auto;
        }

        pre code {
            padding: 0;
            background: transparent;
            font-size: var(--font-xs);
            line-height: var(--line-code);
        }

        blockquote {
            margin: var(--space-2) 0;
            padding-left: var(--space-3);
            border-left: 3px solid var(--color-border);
            color: var(--color-text-secondary);
        }

        hr {
            margin: var(--space-3) 0;
            border: 0;
            border-top: 1px solid var(--color-divider);
        }

        table {
            font-size: var(--font-xs);
        }
    }
}

/* ---------- 下载进度 ---------- */
.uq-progress {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-divider);
    /* 贴住页脚：进度是「正在发生的事」，不该悬在内容中间。
       说明区短时，剩余留白落在两者之间，被这条分隔线干净地切开；
       内容长时 auto 没有剩余空间可吸，退回正常流。 */
    margin-top: auto;
}

.uq-progress-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--font-sm);
    line-height: var(--line-base);
}

.uq-progress-label {
    font-weight: 600;
    color: var(--color-text-primary);
}

.uq-progress-pct {
    flex: none;
    font-size: var(--font-xs);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
}

/* 4px 是 Fluent 2 ProgressBar 的标准高度；轨道不做条纹与流动动画 */
.uq-progress-track {
    height: 4px;
    border-radius: var(--radius-xs);
    background: var(--color-chip-bg);
    overflow: hidden;
}

.uq-progress-bar {
    height: 100%;
    border-radius: var(--radius-xs);
    background: var(--color-accent);
    transition: width var(--transition-base);

    &.is-done {
        background: var(--color-success-fg);
    }
}

.uq-progress-meta {
    font-size: var(--font-xs);
    line-height: var(--line-base);
    color: var(--color-text-tertiary);
    font-variant-numeric: tabular-nums;
}

/* ---------- 页脚 ---------- */
.uq-foot {
    flex: none;
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--color-divider);
}

/* 标准按钮（描边）：NeutralStroke1，hover 时描边加深。
   主按钮在最右，次级操作在左 —— 与 Fluent 2 的 Dialog footer 一致 */
.uq-btn {
    appearance: none;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: var(--control-md);
    padding: 0 var(--space-4);
    border: 1px solid var(--color-stroke-control);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--font-sm);
    font-weight: 600;
    line-height: 1;
    cursor: pointer;
    transition:
        background-color var(--transition-fast),
        border-color var(--transition-fast),
        color var(--transition-fast);

    &:hover:not(:disabled) {
        background: var(--elevation-hover);
        border-color: var(--color-text-tertiary);
    }

    &:active:not(:disabled) {
        background: var(--elevation-pressed);
    }

    &.is-primary {
        border-color: transparent;
        background: var(--btn-primary-bg);
        color: var(--btn-primary-text);

        &:hover:not(:disabled) {
            background: var(--btn-primary-hover-bg);
            border-color: transparent;
        }

        &:active:not(:disabled) {
            background: var(--btn-primary-active-bg);
        }
    }

    &:disabled {
        cursor: default;
        border-color: var(--color-border);
        background: var(--color-chip-bg);
        color: var(--color-text-disabled);
    }

    /* 主按钮禁用时不沿用中性底，避免看起来像换成了次级按钮 */
    &.is-primary:disabled {
        border-color: transparent;
        background: var(--btn-primary-disabled-bg);
        color: var(--btn-primary-disabled-text);
    }
}

/* ---------- 进度环 ---------- */
/* Fluent 2 的 ProgressRing：20px 外径、2px 笔画、单边高亮的缺口环。
   这是加载指示器而非图标，故不取自 @vicons（图标库里没有可旋转的进度环） */
.uq-ring {
    flex: none;
    width: var(--icon-md);
    height: var(--icon-md);
    /* 静息段取品牌色 30%（Fluent 的 ProgressRing 就这么画）。
       用 --color-border 会在 surface 上淡到看不见，像卡住 */
    border: 2px solid color-mix(in srgb, var(--color-brand-fg) 30%, transparent);
    border-top-color: var(--color-brand-fg);
    border-radius: 50%;
    animation: uq-spin 0.9s linear infinite;

    &.is-sm {
        width: var(--icon-sm);
        height: var(--icon-sm);
    }
}

@keyframes uq-spin {
    to {
        transform: rotate(360deg);
    }
}

/* 降低动效偏好下不停止旋转，而是放慢：静止的环会被误读为「卡住」 */
@media (prefers-reduced-motion: reduce) {
    .uq-ring {
        animation-duration: 2.4s;
    }

    .uq-progress-bar {
        transition: none;
    }
}
</style>

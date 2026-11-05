<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
    Dismiss16Regular,
    MusicNote216Regular,
    Pause16Filled,
    Play16Filled,
    Speaker016Regular,
    Speaker116Regular,
    Speaker216Regular,
    SpeakerMute16Regular,
} from '@vicons/fluent'

import { useTheme } from '@/hooks/theme'
import { useWindow } from '@/hooks/use-window'

import type { FileInfo } from '@/utils/typescript'

useTheme()

defineOptions({
    name: 'AudioSupport',
})

const route = useRoute()
const { t } = useI18n()
const { handleClose } = useWindow()

const fileInfo = ref<FileInfo>()
const player = useTemplateRef<HTMLAudioElement>('player')
const lyricViewport = useTemplateRef<HTMLElement>('lyricViewport')

const isPlaying = ref(false)
const duration = ref(0)
const currentTime = ref(0)
const volume = ref(1)
const lastVolume = ref(1)
const rate = ref(1)
const poster = ref('')
const bufferedPct = ref(0)
const currentLineIndex = ref(-1)
const isSeeking = ref(false)

const RATE_OPTIONS = [0.5, 0.75, 1, 1.25, 1.5, 2]

/** 后端 lofty 读取失败时整体返回 null，故所有字段均可空 */
interface IAudioInfo {
    title?: string | null
    artist?: string | null
    album?: string | null
    duration?: number | null
    bitrate?: number | null
    cover?: number[] | null
}
const audioInfo = ref<IAudioInfo>({})

interface ILrcLine {
    timestamp: number
    text: string
}
interface ILrc {
    content: ILrcLine[]
    offset?: number | null
    title?: string | null
}
const lrc = ref<ILrc>({ content: [] })

const formatTime = (sec: number) => {
    const safe = Number.isFinite(sec) && sec > 0 ? sec : 0
    const m = Math.floor(safe / 60)
    const s = Math.floor(safe % 60)
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

const title = computed(() => audioInfo.value.title || fileInfo.value?.name || '')

const subtitle = computed(() => {
    return [audioInfo.value.artist, audioInfo.value.album].filter(Boolean).join(' · ')
})

/** 规格胶囊：仅展示后端确实提供的字段，不臆造采样率 */
const pills = computed(() => {
    const list: string[] = []
    const name = fileInfo.value?.name ?? ''
    const dot = name.lastIndexOf('.')
    if (dot > 0) list.push(name.slice(dot + 1).toUpperCase())
    if (audioInfo.value.bitrate) list.push(`${audioInfo.value.bitrate}k`)
    return list
})

const currentTimeStr = computed(() => formatTime(currentTime.value))
const remainStr = computed(() => formatTime(duration.value - currentTime.value))
const playedPct = computed(() => {
    if (!duration.value) return 0
    return Math.min(Math.max((currentTime.value / duration.value) * 100, 0), 100)
})
const volumePct = computed(() => Math.round(volume.value * 100))
const rateLabel = computed(() => `${rate.value}x`)

const volumeIcon = computed(() => {
    if (volume.value <= 0) return SpeakerMute16Regular
    if (volume.value < 0.34) return Speaker016Regular
    if (volume.value < 0.67) return Speaker116Regular
    return Speaker216Regular
})

/** LRC 的 offset 以毫秒计，正值表示歌词整体提前，故生效时间需要减去 offset */
const lyricLines = computed(() => {
    const offset = lrc.value.offset ?? 0
    return (lrc.value.content ?? []).map(line => ({
        time: line.timestamp - offset,
        text: line.text,
    }))
})

const togglePlay = () => {
    const audio = player.value
    if (!audio) return
    if (audio.paused) {
        void audio.play().catch(() => {
            /* 自动播放被拦截或源不可用时保持暂停态 */
        })
    } else {
        audio.pause()
    }
}

const seekBy = (delta: number) => {
    const audio = player.value
    if (!audio || !Number.isFinite(audio.duration)) return
    audio.currentTime = Math.min(Math.max(audio.currentTime + delta, 0), audio.duration)
    currentTime.value = audio.currentTime
}

const forward = () => seekBy(10)
const backward = () => seekBy(-10)

const applyVolume = (next: number) => {
    const audio = player.value
    const safe = Math.min(Math.max(next, 0), 1)
    volume.value = safe
    if (audio) audio.volume = safe
}

const toggleMute = () => {
    if (volume.value > 0) {
        lastVolume.value = volume.value
        applyVolume(0)
    } else {
        applyVolume(lastVolume.value > 0 ? lastVolume.value : 0.6)
    }
}

const nudgeVolume = (delta: number) => {
    applyVolume(volume.value + delta)
}

const cycleRate = () => {
    const index = RATE_OPTIONS.indexOf(rate.value)
    rate.value = RATE_OPTIONS[(index + 1) % RATE_OPTIONS.length]
    if (player.value) player.value.playbackRate = rate.value
}

const updateBuffered = () => {
    const audio = player.value
    if (!audio || !audio.duration) return
    const ranges = audio.buffered
    if (!ranges.length) {
        bufferedPct.value = 0
        return
    }
    bufferedPct.value = Math.min((ranges.end(ranges.length - 1) / audio.duration) * 100, 100)
}

/** 从尾部向前查找首个已到达的行，避免超过最后一句后索引停留在错误位置 */
const syncLyrics = (time: number) => {
    const lines = lyricLines.value
    if (!lines.length) return
    const ms = time * 1000
    let index = -1
    for (let i = lines.length - 1; i >= 0; i -= 1) {
        if (lines[i].time <= ms) {
            index = i
            break
        }
    }
    if (index !== currentLineIndex.value) currentLineIndex.value = index
}

/** 用容器内 scrollTop 居中，避免 scrollIntoView 连带滚动外层容器 */
const scrollToLine = (index: number, smooth = true) => {
    const viewport = lyricViewport.value
    if (!viewport || index < 0) return
    const line = viewport.querySelectorAll<HTMLElement>('.lyric-line')[index]
    if (!line) return
    const top = line.offsetTop - (viewport.clientHeight - line.offsetHeight) / 2
    viewport.scrollTo({ top: Math.max(top, 0), behavior: smooth ? 'smooth' : 'auto' })
}

const seekToTime = (time: number) => {
    const audio = player.value
    if (!audio) return
    audio.currentTime = Math.min(Math.max(time / 1000, 0), audio.duration || 0)
}

let rafId = 0

const stopTick = () => {
    if (rafId) {
        cancelAnimationFrame(rafId)
        rafId = 0
    }
}

const tick = () => {
    rafId = 0
    const audio = player.value
    if (!audio) return
    if (!isSeeking.value) currentTime.value = audio.currentTime
    syncLyrics(audio.currentTime)
    if (!audio.paused && !audio.ended) rafId = requestAnimationFrame(tick)
}

const startTick = () => {
    stopTick()
    rafId = requestAnimationFrame(tick)
}

const seekRatioFromEvent = (event: PointerEvent, element: HTMLElement) => {
    if (!duration.value) return
    const rect = element.getBoundingClientRect()
    const ratio = Math.min(Math.max((event.clientX - rect.left) / rect.width, 0), 1)
    currentTime.value = ratio * duration.value
}

const onProgressDown = (event: PointerEvent) => {
    if (!duration.value) return
    const bar = event.currentTarget as HTMLElement
    isSeeking.value = true
    bar.setPointerCapture(event.pointerId)
    seekRatioFromEvent(event, bar)
}

const onProgressMove = (event: PointerEvent) => {
    if (!isSeeking.value) return
    seekRatioFromEvent(event, event.currentTarget as HTMLElement)
}

const onProgressUp = (event: PointerEvent) => {
    if (!isSeeking.value) return
    const bar = event.currentTarget as HTMLElement
    isSeeking.value = false
    if (bar.hasPointerCapture(event.pointerId)) bar.releasePointerCapture(event.pointerId)
    const audio = player.value
    if (audio) audio.currentTime = currentTime.value
}

const volumeFromEvent = (event: PointerEvent, rail: HTMLElement) => {
    const rect = rail.getBoundingClientRect()
    const ratio = 1 - (event.clientY - rect.top) / rect.height
    applyVolume(ratio)
}

const onVolumeDown = (event: PointerEvent) => {
    const rail = event.currentTarget as HTMLElement
    rail.setPointerCapture(event.pointerId)
    volumeFromEvent(event, rail)
}

const onVolumeMove = (event: PointerEvent) => {
    const rail = event.currentTarget as HTMLElement
    if (!rail.hasPointerCapture(event.pointerId)) return
    volumeFromEvent(event, rail)
}

const onVolumeUp = (event: PointerEvent) => {
    const rail = event.currentTarget as HTMLElement
    if (rail.hasPointerCapture(event.pointerId)) rail.releasePointerCapture(event.pointerId)
}

/// 拖动阈值：小于该距离视为一次点击，避免吞掉歌词的点击跳转
const DRAG_THRESHOLD = 4

/// 需要保留点击语义的子元素：命中时不启动窗口拖动
const DRAG_EXCLUDE_SELECTOR = 'button, a, input, select, textarea, [role="slider"], [tabindex]'

let dragOrigin: { x: number; y: number } | null = null

const stopDragTracking = () => {
    document.removeEventListener('mousemove', trackDrag, true)
    document.removeEventListener('mouseup', stopDragTracking, true)
    dragOrigin = null
}

const trackDrag = (event: MouseEvent) => {
    if (!dragOrigin) return
    const dx = Math.abs(event.clientX - dragOrigin.x)
    const dy = Math.abs(event.clientY - dragOrigin.y)
    if (dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD) return

    stopDragTracking()
    void getCurrentWindow()
        .startDragging()
        .catch(() => {})
}

const onDragMouseDown = (event: MouseEvent) => {
    // 只响应左键；右键与中键保持默认行为
    if (event.button !== 0) return
    if ((event.target as HTMLElement | null)?.closest(DRAG_EXCLUDE_SELECTOR)) return
    dragOrigin = { x: event.clientX, y: event.clientY }
    document.addEventListener('mousemove', trackDrag, true)
    document.addEventListener('mouseup', stopDragTracking, true)
}

const onKeydown = (event: KeyboardEvent) => {
    switch (event.code) {
        case 'Escape':
            event.preventDefault()
            handleClose()
            break
        case 'Space':
            // 按钮聚焦时交给按钮自身处理，避免重复触发
            if ((event.target as HTMLElement | null)?.tagName === 'BUTTON') return
            event.preventDefault()
            togglePlay()
            break
        case 'ArrowRight':
            event.preventDefault()
            forward()
            break
        case 'ArrowLeft':
            event.preventDefault()
            backward()
            break
        case 'ArrowUp':
            event.preventDefault()
            nudgeVolume(0.05)
            break
        case 'ArrowDown':
            event.preventDefault()
            nudgeVolume(-0.05)
            break
        default:
            break
    }
}

/** 无歌词文件或解析失败时降级为空歌词，不能阻塞音频加载 */
const loadLyrics = async (path: string) => {
    try {
        const lrcPath = path.replace(/\.[^/.]+$/, '.lrc')
        lrc.value = await invoke<ILrc>('parse_lrc', { path: lrcPath })
    } catch (error) {
        console.warn('[audio] parse lrc failed', error)
        lrc.value = { content: [] }
    }
}

const loadAudioInfo = async (path: string) => {
    try {
        const info = await invoke<IAudioInfo | null>('read_audio_info', { path })
        audioInfo.value = info ?? {}
        if (Array.isArray(info?.cover) && info.cover.length) {
            poster.value = URL.createObjectURL(new Blob([new Uint8Array(info.cover)]))
        }
    } catch (error) {
        console.error('[audio] read audio info failed', error)
        audioInfo.value = {}
    }
}

const abortController = new AbortController()

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const path = fileInfo.value?.path
    const signal = abortController.signal
    const audio = player.value

    if (audio) {
        audio.addEventListener(
            'play',
            () => {
                isPlaying.value = true
                startTick()
            },
            { signal },
        )
        audio.addEventListener(
            'pause',
            () => {
                isPlaying.value = false
                stopTick()
            },
            { signal },
        )
        audio.addEventListener(
            'ended',
            () => {
                isPlaying.value = false
                stopTick()
            },
            { signal },
        )
        audio.addEventListener(
            'loadedmetadata',
            () => {
                duration.value = Number.isFinite(audio.duration) ? audio.duration : 0
                updateBuffered()
            },
            { signal },
        )
        audio.addEventListener(
            'volumechange',
            () => {
                volume.value = audio.volume
            },
            { signal },
        )
        audio.addEventListener('progress', updateBuffered, { signal })
        audio.addEventListener('timeupdate', updateBuffered, { signal })
        audio.addEventListener(
            'seeked',
            () => {
                currentTime.value = audio.currentTime
                syncLyrics(audio.currentTime)
            },
            { signal },
        )
        audio.addEventListener(
            'error',
            () => {
                console.error('[audio] media load failed', audio.error)
            },
            { signal },
        )
    }

    window.addEventListener('keydown', onKeydown, { signal })

    // 先挂载音源再读元信息，避免标签读取耗时阻塞起播
    if (audio && path) {
        audio.src = convertFileSrc(path)
        audio.volume = volume.value
        void audio.play().catch(() => {
            /* 自动播放被拦截时保持暂停态 */
        })
    }
    if (path) await Promise.all([loadAudioInfo(path), loadLyrics(path)])
})

watch(currentLineIndex, index => {
    scrollToLine(index)
})

onBeforeUnmount(() => {
    stopTick()
    stopDragTracking()
    abortController.abort()
    if (poster.value) {
        URL.revokeObjectURL(poster.value)
        poster.value = ''
    }
    const audio = player.value
    if (audio) {
        audio.pause()
        audio.removeAttribute('src')
        audio.load()
    }
})
</script>

<template>
    <div class="audio-wrapper" data-tauri-drag-region="deep">
        <audio ref="player" class="audio-el" hidden></audio>

        <div v-if="poster" class="audio-bg" :style="{ backgroundImage: `url(${poster})` }"></div>
        <div class="audio-scrim"></div>

        <div class="audio-shell">
            <!-- 封面保留拖动与点击语义，不做原生拖拽区，改用阈值区分“点击”与“拖动” -->
            <div class="audio-cover" data-tauri-drag-region="false" @mousedown="onDragMouseDown">
                <img v-if="poster" :src="poster" :alt="t('audio.cover')" draggable="false" />
                <div v-else class="cover-empty">
                    <el-icon>
                        <MusicNote216Regular />
                    </el-icon>
                </div>
            </div>

            <div class="audio-meta">
                <div class="meta-title-row">
                    <span class="meta-title">{{ title }}</span>
                    <span v-for="pill in pills" :key="pill" class="pill">{{ pill }}</span>
                </div>
                <div v-if="subtitle" class="meta-sub">{{ subtitle }}</div>
            </div>

            <div ref="lyricViewport" class="audio-lyric">
                <template v-if="lyricLines.length">
                    <div
                        v-for="(line, index) in lyricLines"
                        :key="index"
                        class="lyric-line"
                        data-tauri-drag-region="false"
                        :class="{ current: index === currentLineIndex }"
                        :title="line.text"
                        @mousedown="onDragMouseDown"
                        @click="seekToTime(line.time)"
                    >
                        {{ line.text }}
                    </div>
                </template>
                <div v-else class="lyric-empty">{{ t('audio.noLyrics') }}</div>
            </div>

            <div class="audio-progress">
                <span class="progress-time">{{ currentTimeStr }}</span>
                <div
                    class="progress-bar"
                    role="slider"
                    tabindex="0"
                    :aria-label="t('audio.progress')"
                    aria-valuemin="0"
                    :aria-valuemax="Math.round(duration)"
                    :aria-valuenow="Math.round(currentTime)"
                    @pointerdown="onProgressDown"
                    @pointermove="onProgressMove"
                    @pointerup="onProgressUp"
                    @pointercancel="onProgressUp"
                >
                    <div class="bar-buffer" :style="{ width: `${bufferedPct}%` }"></div>
                    <div class="bar-fill" :style="{ width: `${playedPct}%` }"></div>
                    <div class="bar-thumb" :style="{ left: `${playedPct}%` }"></div>
                </div>
                <span class="progress-time">-{{ remainStr }}</span>
            </div>

            <div class="audio-transport">
                <button class="icon-btn" type="button" :title="t('audio.backward')" @click="backward">
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                        <path d="M13 8a5 5 0 1 1-1.5-3.5" />
                        <path d="M13 2.5v3.2h-3.2" />
                    </svg>
                </button>

                <button
                    class="play-btn"
                    type="button"
                    :title="isPlaying ? t('audio.pause') : t('audio.play')"
                    :aria-label="isPlaying ? t('audio.pause') : t('audio.play')"
                    @click="togglePlay"
                >
                    <el-icon>
                        <Pause16Filled v-if="isPlaying" />
                        <Play16Filled v-else />
                    </el-icon>
                </button>

                <button class="icon-btn" type="button" :title="t('audio.forward')" @click="forward">
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                        <path d="M3 8a5 5 0 1 0 1.5-3.5" />
                        <path d="M3 2.5v3.2h3.2" />
                    </svg>
                </button>

                <div class="vol">
                    <button
                        class="icon-btn"
                        type="button"
                        :title="t('audio.volume')"
                        :aria-label="t('audio.volume')"
                        @click="toggleMute"
                    >
                        <el-icon>
                            <component :is="volumeIcon" />
                        </el-icon>
                    </button>
                    <div class="vol-pop">
                        <div
                            class="vol-rail" data-tauri-drag-region="false"
                            @pointerdown="onVolumeDown"
                            @pointermove="onVolumeMove"
                            @pointerup="onVolumeUp"
                            @pointercancel="onVolumeUp"
                        >
                            <div class="vol-fill" :style="{ height: `${volumePct}%` }"></div>
                            <div class="vol-knob" :style="{ bottom: `${volumePct}%` }"></div>
                        </div>
                    </div>
                </div>

                <button class="rate-btn" type="button" :title="t('audio.speed')" @click="cycleRate">
                    {{ rateLabel }}
                </button>
            </div>
        </div>

        <div class="win-actions">
            <button class="win-btn close" type="button" :title="t('header.close')" @click="handleClose">
                <el-icon :size="12">
                    <Dismiss16Regular />
                </el-icon>
            </button>
        </div>
    </div>
</template>

<style scoped lang="scss">
.audio-wrapper {
    /* 品牌色分两支：accent 作实心底（播放键、进度条），hi 作文字与强调图形。
       取值来自全局 token，深色模式自动翻转，见 design/fluent-baseline.md §2.1 */
    --audio-accent: var(--color-accent);
    --audio-accent-hi: var(--color-brand-fg);
    /* 设计稿按 560×360 逻辑像素绘制。窗口尺寸会随系统缩放比缩小（150% 时只剩
       373×240 逻辑像素），故几何尺寸统一按 1 设计像素 = 1/360 视口高换算；
       字号另外设下限，避免高缩放下整块 UI 一起缩到不可读 */
    --u: 0.2778vh;
    --cover-size: min(56px, calc(56 * var(--u)));
    --lyric-h: min(calc(150 * var(--u)), 100%);
    --line-h: max(22px, calc(30 * var(--u)));
    --lyric-font: clamp(12px, calc(14 * var(--u)), 14px);
    --lyric-font-current: clamp(16px, calc(20 * var(--u)), 20px);
    --shell-gap: calc(12 * var(--u));

    position: relative;
    width: 100%;
    height: 100%;
    background-color: var(--color-bg);
    overflow: hidden;
    user-select: none;
}

/* ---------- 展开态背景：封面放大模糊 + 主题蒙版 ---------- */
.audio-bg {
    position: absolute;
    inset: -14%;
    background-size: cover;
    background-position: center;
    filter: blur(34px) saturate(1.35);
    opacity: 0.72;
}

.audio-scrim {
    position: absolute;
    inset: 0;
    background: var(--color-bg);
    opacity: 0.76;
}

/* ---------- 布局 ---------- */
.audio-shell {
    position: relative;
    z-index: 1;
    box-sizing: border-box;
    display: grid;
    height: 100%;
    padding: calc(20 * var(--u)) calc(24 * var(--u)) calc(16 * var(--u));
    grid-template-columns: var(--cover-size) minmax(0, 1fr);
    grid-template-areas:
        'cover meta'
        'lyric lyric'
        'progress progress'
        'transport transport';
    grid-template-rows: auto 1fr auto auto;
    align-content: stretch;
    column-gap: var(--shell-gap);
    row-gap: calc(8 * var(--u));
}

/* ---------- 封面 ---------- */
.audio-cover {
    grid-area: cover;
    position: relative;
    align-self: center;
    width: var(--cover-size);
    height: var(--cover-size);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: linear-gradient(135deg, #3a1f0d, #8a4a13 52%, #d98a35);
    box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.18),
        0 8px 20px rgba(0, 0, 0, 0.16);

    img {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
}

.cover-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    color: rgba(255, 255, 255, 0.5);

    :deep(.el-icon) {
        font-size: min(22px, calc(22 * var(--u)));
    }
}

/* ---------- 元信息 ---------- */
.audio-meta {
    grid-area: meta;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    min-width: 0;
    padding-right: 40px;
}

.meta-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
}

.meta-title {
    font-size: clamp(13px, calc(15 * var(--u)), var(--font-lg));
    font-weight: 600;
    line-height: 1.3;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.pill {
    flex: 0 0 auto;
    padding: 0 4px;
    border-radius: 3px;
    background: var(--elevation-hover);
    font-size: var(--font-2xs);
    font-weight: 600;
    line-height: 14px;
    letter-spacing: 0.2px;
    color: var(--color-text-secondary);
    text-transform: uppercase;
}

.meta-sub {
    font-size: clamp(11px, calc(12 * var(--u)), var(--font-xs));
    line-height: 1.4;
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

/* ---------- 歌词 ---------- */
.audio-lyric {
    grid-area: lyric;
    position: relative;
    align-self: center;
    height: var(--lyric-h);
    overflow-x: hidden;
    overflow-y: auto;
    scrollbar-width: none;
    /* 上下渐隐，避免歌词行在可视区边缘被硬切 */
    -webkit-mask-image: linear-gradient(180deg, transparent 0, #000 9%, #000 91%, transparent 100%);
    mask-image: linear-gradient(180deg, transparent 0, #000 9%, #000 91%, transparent 100%);

    &::-webkit-scrollbar {
        display: none;
    }
}

.lyric-line {
    height: var(--line-h);
    line-height: var(--line-h);
    font-size: var(--lyric-font);
    color: var(--color-text-secondary);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
    transition:
        color var(--transition-base),
        font-size var(--transition-base);

    &.current {
        font-size: var(--lyric-font-current);
        font-weight: 600;
        color: var(--audio-accent-hi);
    }
}

.lyric-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: var(--font-xs);
    color: var(--color-text-tertiary);
}

/* ---------- 进度条 ---------- */
.audio-progress {
    grid-area: progress;
    display: flex;
    align-items: center;
    gap: calc(10 * var(--u));
    min-width: 0;
    height: calc(22 * var(--u));
}

.progress-time {
    flex: 0 0 auto;
    font-size: clamp(10px, calc(11 * var(--u)), 11px);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
}

.progress-bar {
    position: relative;
    flex: 1;
    height: 4px;
    min-width: 0;
    border-radius: 2px;
    background: var(--color-border);
    cursor: pointer;
    transition: height var(--transition-fast);

    /* 扩大可点击热区，不改变视觉高度 */
    &::before {
        content: '';
        position: absolute;
        inset: -8px 0;
    }

    &:hover {
        height: 6px;

        .bar-thumb {
            opacity: 1;
        }
    }
}

.bar-buffer,
.bar-fill {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    border-radius: 2px;
}

.bar-buffer {
    background: var(--elevation-hover);
}

.bar-fill {
    background: var(--audio-accent);
}

.bar-thumb {
    position: absolute;
    top: 50%;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--audio-accent);
    box-shadow: 0 0 0 2px var(--color-bg);
    opacity: 0;
    transform: translate(-50%, -50%);
    transition: opacity var(--transition-fast);
    pointer-events: none;
}

/* ---------- 音量 ---------- */
.vol {
    position: relative;
    display: flex;
    align-items: center;

    &:hover .vol-pop {
        opacity: 1;
        pointer-events: auto;
        transform: translateX(-50%) translateY(0);
    }
}

.vol-pop {
    position: absolute;
    bottom: calc(30 * var(--u));
    left: 50%;
    z-index: 5;
    padding: calc(8 * var(--u)) 0;
    border: 1px solid var(--color-border);
    border-radius: 16px;
    background: var(--color-bg);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.16);
    opacity: 0;
    pointer-events: none;
    transform: translateX(-50%) translateY(4px);
    transition:
        opacity var(--transition-fast),
        transform var(--transition-fast);
}

.vol-rail {
    position: relative;
    width: 4px;
    height: calc(80 * var(--u));
    margin: 0 calc(14 * var(--u));
    border-radius: 2px;
    background: var(--elevation-hover);
    cursor: pointer;
}

.vol-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    border-radius: 2px;
    background: var(--audio-accent);
}

.vol-knob {
    position: absolute;
    left: 50%;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--audio-accent);
    box-shadow: 0 0 0 2px var(--color-bg);
    transform: translate(-50%, 50%);
    pointer-events: none;
}

/* ---------- 传输簇 ---------- */
.audio-transport {
    grid-area: transport;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: calc(14 * var(--u));
    height: calc(46 * var(--u));
}

.icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: min(32px, calc(32 * var(--u)));
    height: min(32px, calc(32 * var(--u)));
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
    transition:
        color var(--transition-fast),
        background var(--transition-fast);

    &:hover {
        color: var(--color-text-primary);
        background: var(--elevation-hover);
    }

    svg {
        width: min(17px, calc(17 * var(--u)));
        height: min(17px, calc(17 * var(--u)));
    }

    :deep(.el-icon) {
        font-size: min(16px, calc(16 * var(--u)));
    }
}

.play-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: min(46px, calc(46 * var(--u)));
    height: min(46px, calc(46 * var(--u)));
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--audio-accent);
    color: #fff;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.22);
    transition:
        transform var(--transition-fast),
        background var(--transition-fast);

    :deep(.el-icon) {
        font-size: min(20px, calc(20 * var(--u)));
    }

    &:hover {
        transform: scale(1.06);
        background: var(--audio-accent-hi);
    }

    &:active {
        transform: scale(0.95);
    }
}

.rate-btn {
    min-width: calc(34 * var(--u));
    height: calc(26 * var(--u));
    padding: 0 4px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: clamp(10px, calc(12 * var(--u)), var(--font-xs));
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition:
        color var(--transition-fast),
        background var(--transition-fast);

    &:hover {
        color: var(--color-text-primary);
        background: var(--elevation-hover);
    }
}

/* ---------- 窗口操作：悬停显现 ---------- */
.win-actions {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 6;
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--transition-fast);
}

.audio-wrapper:hover .win-actions,
.audio-wrapper:focus-within .win-actions {
    opacity: 1;
}

.win-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-bg) 72%, transparent);
    backdrop-filter: blur(8px);
    color: var(--color-text-secondary);
    cursor: pointer;
    transition:
        color var(--transition-fast),
        background var(--transition-fast);

    &:hover {
        color: var(--color-text-primary);
        background: var(--elevation-hover);
    }

    &.close:hover {
        color: #fff;
        background: #e81123;
    }
}
</style>

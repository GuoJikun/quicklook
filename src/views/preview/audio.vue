<script setup lang="ts">
import { nextTick } from 'vue'
import { computed, onMounted, ref, watch, useTemplateRef } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import { useWindow } from '@/hooks/use-window'
import { Dismiss16Regular, PauseCircle20Regular, PlayCircle20Regular } from '@vicons/fluent'
// import { VolumeMediumOutline } from '@vicons/ionicons5'

import type { FileInfo } from '@/utils/typescript'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useTheme } from '@/hooks/theme'
import { useEventListener } from '@vueuse/core'

useTheme()

const route = useRoute()
const { handleClose } = useWindow()

defineOptions({
    name: 'AudioSupport',
})

const fileInfo = ref<FileInfo>()
const player = useTemplateRef<HTMLAudioElement>('player')

const isPlaying = ref(false)
const duration = ref(0)
const currentTime = ref(0)
const volume = ref(1)
const currentTimeStr = ref('00:00')
const durationStr = ref('00:00')
const remainStr = computed(() => {
    return formatTime(Math.max(duration.value - currentTime.value, 0))
})

interface IAudioInfo {
    title: string
    artist: string
    album: string
    duration: number // 秒
    bitrate: number // kbps
    cover: ArrayBuffer | null // 图片二进制
    [x: string]: unknown
}
const audioInfo = ref<IAudioInfo>({
    title: '',
    artist: '',
    album: '',
    duration: 0,
    bitrate: 0,
    cover: null,
})
const getAudioInfo = async (path: string) => {
    const info: IAudioInfo = await invoke('read_audio_info', { path })
    audioInfo.value = info
    console.log(info.cover, 'audio info.cover')
    if (info && info.cover) {
        audioInfo.value.poster = URL.createObjectURL(new Blob([new Uint8Array(info.cover)]))
        console.log(audioInfo.value.poster, 'audioInfo.value.poster')
    }
}
interface ILrcLine {
    timestamp: number
    text: string
}
interface ILrc {
    content: Array<ILrcLine>
    offset: number
    title: string
}
const lrc = ref<ILrc>({
    content: [],
    offset: 0,
    title: '',
})
const getLrc = async (path: string) => {
    const lrc_path = path.replace(/\.[^/.]+$/, '.lrc')

    const lrc_inner: ILrc = await invoke('parse_lrc', { path: lrc_path })
    console.log(lrc_inner, 'lrc_inner')
    lrc.value = lrc_inner
}

fileInfo.value = route.query as unknown as FileInfo

const formatTime = (sec: number) => {
    const m = Math.floor(sec / 60)
    const s = Math.floor(sec % 60)
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

const togglePlay = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    if (audio.paused) {
        audio.play()
    } else {
        audio.pause()
    }
}

const forward = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    audio.currentTime = Math.min(audio.currentTime + 10, audio.duration)
}

const backward = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    audio.currentTime = Math.max(audio.currentTime - 10, 0)
}

const seek = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    audio.currentTime = Number(currentTime.value)
}

// const changeVolume = () => {
//     const audio = player.value as HTMLAudioElement
//     if (!audio) return
//     audio.volume = Number(volume.value)
// }

const currentLine = ref<number>()
const syncLyrics = (audio: HTMLAudioElement) => {
    const currentMs = (audio.currentTime || 0) * 1000 // 秒转毫秒

    // 找到 <= 当前时间 的最后一句歌词
    const len = lrc.value.content.length
    if (len === 0) return
    let _currentLine: null | number = null
    for (let i = 0; i < len; i++) {
        const line = lrc.value.content[i]

        if (line.timestamp <= currentMs) {
            _currentLine = line.timestamp
            continue
        }
        currentLine.value = _currentLine as number

        break
    }
}

const handleCurrentTime = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    currentTime.value = audio.currentTime
    currentTimeStr.value = formatTime(audio.currentTime)
    syncLyrics(audio)
    if (!audio.paused && !audio.ended) {
        requestAnimationFrame(handleCurrentTime)
    }
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    await getAudioInfo(fileInfo.value?.path)

    await getLrc(fileInfo.value?.path)

    const path = convertFileSrc(fileInfo.value.path)

    const audio = player.value as HTMLAudioElement
    audio.src = path
    audio.volume = volume.value
    audio.addEventListener('play', () => {
        isPlaying.value = true
        requestAnimationFrame(handleCurrentTime)
    })
    audio.addEventListener('pause', () => {
        isPlaying.value = false
    })

    audio.addEventListener('loadedmetadata', () => {
        duration.value = audio.duration
        durationStr.value = formatTime(audio.duration)
    })
    audio.addEventListener('volumechange', () => {
        volume.value = audio.volume
    })
    audio.addEventListener('progress', e => {
        console.log(e, 'progress event')
    })

    useEventListener(window, 'keydown', (e: KeyboardEvent) => {
        if (e.code === 'Escape') {
            e.preventDefault()
            handleClose()
        } else if (e.code === 'ArrowRight') {
            e.preventDefault()
            forward()
        } else if (e.code === 'ArrowLeft') {
            e.preventDefault()
            backward()
        }
    })
})

watch(currentTime, val => {
    currentTimeStr.value = formatTime(val)
    // 歌词自动跳转高亮
    if (lrc.value.content.length) {
        // 可选：自动滚动到高亮歌词
        nextTick(() => {
            const el = document.querySelector('.audio-lyric .lrc-active')
            if (el) el.scrollIntoView({ block: 'center', behavior: 'smooth' })
        })
    }
})
watch(duration, val => {
    durationStr.value = formatTime(val)
})
</script>

<template>
    <div class="audio-wrapper">
        <audio ref="player" class="audio-player-hidden" hidden autoplay></audio>
        <div class="audio-player" data-tauri-drag-region>
            <div class="audio-poster" data-tauri-drag-region>
                <img
                    v-if="audioInfo.poster"
                    :src="audioInfo.poster as string"
                    alt="封面"
                />
                <div class="audio-poster-overlay">
                    <el-icon class="audio-poster-btn" @click="togglePlay" size="48">
                        <PauseCircle20Regular v-if="isPlaying" />
                        <PlayCircle20Regular v-else />
                    </el-icon>
                </div>
            </div>
            <div class="audio-ui">
                <div class="audio-info" data-tauri-drag-region>
                    <div class="audio-title" data-tauri-drag-region>
                        {{ audioInfo.title || fileInfo?.name }}
                    </div>
                    <div v-if="audioInfo.artist" class="audio-artist" data-tauri-drag-region>
                        {{ audioInfo.artist }}
                    </div>
                    <el-scrollbar class="audio-lyric">
                        <template v-if="lrc.content.length">
                            <div
                                v-for="(line, index) in lrc.content"
                                :key="index"
                                :class="{ 'lrc-active': currentLine === line.timestamp }"
                            >
                                <span>{{ line.text }}</span>
                            </div>
                        </template>
                        <div v-else class="audio-lyric-empty">
                            <span>暂无歌词</span>
                        </div>
                    </el-scrollbar>
                </div>
                <div class="audio-progress">
                    <span class="audio-progress-time">{{ currentTimeStr }}</span>
                    <el-slider
                        v-model="currentTime"
                        :min="0"
                        :max="duration"
                        @input="seek"
                        :format-tooltip="(val: number) => formatTime(val)"
                        class="audio-slider"
                    />
                    <span class="audio-progress-time">-{{ remainStr }}</span>
                </div>
            </div>
        </div>
        <div class="audio-close" @click="handleClose">
            <el-icon>
                <Dismiss16Regular />
            </el-icon>
        </div>
    </div>
</template>

<style scoped lang="scss">
.audio-wrapper {
    width: 100%;
    height: 100%;
    background-color: var(--color-bg);
    position: relative;
}

.audio-player {
    display: flex;
    align-items: center;
    height: 100%;
    padding: 0 var(--space-5);
    gap: var(--space-5);
}

.audio-poster {
    flex: 0 0 160px;
    height: 160px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    position: relative;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    transition: box-shadow var(--transition-medium);

    img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
    }

    &:hover {
        box-shadow: 0 6px 24px rgba(0, 0, 0, 0.18);
    }
}

.audio-poster-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(2px);
    opacity: 0;
    transition: opacity var(--transition-medium);
}

.audio-poster:hover .audio-poster-overlay {
    opacity: 1;
}

.audio-poster-btn {
    cursor: pointer;
    color: white;
    transition: transform var(--transition-fast);

    &:hover {
        transform: scale(1.1);
    }
}

.audio-ui {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    justify-content: space-between;
    min-width: 0;
}

.audio-info {
    padding-top: var(--space-3);
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

.audio-title {
    font-size: var(--font-xl);
    font-weight: 600;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    line-height: var(--line-tight);
}

.audio-artist {
    font-size: var(--font-sm);
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    margin-top: var(--space-1);
    margin-bottom: var(--space-2);
}

.audio-lyric {
    flex: 1;
    font-size: var(--font-sm);
    color: var(--color-text-secondary);
    text-align: center;
    line-height: var(--line-loose);
    overflow: hidden;

    :deep(.el-scrollbar__view) {
        overflow-x: hidden;
    }

    > div > div {
        padding: var(--space-1) var(--space-2);
        border-radius: var(--radius-sm);
        transition:
            color var(--transition-base),
            background var(--transition-base),
            transform var(--transition-base);
    }

    &-empty {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: var(--color-text-disabled);
    }
}

:deep(.lrc-active) {
    color: var(--el-color-primary) !important;
    font-weight: 600;
    font-size: var(--font-lg);
    background: var(--color-hover-bg);
    transform: scale(1.02);
}

.audio-progress {
    flex: 0 0 40px;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-bottom: var(--space-1);

    --el-slider-button-size: 14px;
    --el-slider-height: 4px;
    --el-slider-runway-bg-color: var(--color-border);
    --el-slider-main-bg-color: var(--el-color-primary);

    &-time {
        flex: 0 0 44px;
        font-size: var(--font-xs);
        color: var(--color-text-secondary);
        font-variant-numeric: tabular-nums;

        &:first-child {
            text-align: right;
        }

        &:last-child {
            text-align: left;
        }
    }
}

:deep(.audio-slider) {
    flex: 1;

    .el-slider__runway {
        height: 4px;
    }

    .el-slider__bar {
        height: 4px;
    }

    .el-slider__button-wrapper {
        top: -16px;
    }

    .el-slider__button {
        width: 14px;
        height: 14px;
        border: 2px solid var(--el-color-primary);
        transition: transform var(--transition-fast);
    }

    &:hover .el-slider__button {
        transform: scale(1.2);
    }
}

.audio-close {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--color-text-secondary);
    transition:
        color var(--transition-fast),
        background var(--transition-fast);

    &:hover {
        color: var(--color-danger);
        background: var(--elevation-hover);
    }
}
</style>

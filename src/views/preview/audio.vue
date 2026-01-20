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
                    style="max-width: 100%; max-height: 100%"
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
                        <div v-else>
                            <span>暂无歌词</span>
                        </div>
                    </el-scrollbar>
                </div>
                <div class="audio-progress">
                    <el-slider
                        v-model="currentTime"
                        :min="0"
                        :max="duration"
                        @input="seek"
                        :format-tooltip="(val: number) => formatTime(val)"
                        class="audio-slider"
                    />
                    <span class="audio-progress-time">- {{ remainStr }}</span>
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
.lrc-active {
    color: var(--el-color-primary);
    font-weight: bold;
    font-size: 15px;
    background: rgba(0, 0, 0, 0.08);
    border-radius: 4px;
    padding: 2px 6px;
    transition:
        background 0.2s,
        color 0.2s;
}
// 封面 hover 显示播放/暂停按钮
.audio-poster {
    position: relative;
}
.audio-poster-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.3);
    z-index: -100;
    opacity: 0;
    transition: opacity 0.3s;
}
.audio-poster:hover .audio-poster-overlay {
    z-index: 0;
    opacity: 1;
    color: var(--color-text-primary);
}
.audio-poster-btn {
    cursor: pointer;
    color: white;
}

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
    padding: 0 16px;
    gap: 16px;
}

.audio-poster {
    flex: 0 0 160px;
    height: 160px;
    border: 1px solid var(--color-border);
}
.audio-ui {
    flex: auto;
    display: flex;
    flex-direction: column;
    height: 100%;
    justify-content: space-between;
}
.audio-info {
    padding: 12px 0 0;
    flex: 0 0 calc(100% - 40px);
    overflow: hidden;
}
.audio-title {
    font-size: 16px;
    font-weight: 500;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    margin-bottom: 8px;
}
.audio-lyric {
    font-size: 13px;
    color: var(--color-text-secondary);
    height: calc(100% - 24px);
    overflow-y: auto;
    text-align: center;
}

.audio-progress {
    flex: 0 0 40px;
    display: flex;
    align-items: center;
    gap: 12px;
    --el-slider-button-size: 16px;
    &-time {
        flex: 0 0 50px;
        text-align: right;
    }
}
.audio-controls {
    display: flex;
    align-items: center;
    padding: 12px;
    font-size: 34px;
    &-btn {
        cursor: pointer;
        margin: 0 8px;
        &:hover {
            color: var(--el-color-primary);
        }
    }
}

.audio-close {
    position: absolute;
    top: 0;
    right: 0;
    padding: 8px;
    cursor: pointer;
    &:hover {
        color: var(--el-color-danger);
    }
}
</style>

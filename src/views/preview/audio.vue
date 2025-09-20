<script setup lang="ts">
import { computed, onMounted, ref, watch, useTemplateRef } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import { useWindow } from '@/hooks/use-window'
import {
    Dismiss16Regular,
    PauseCircle20Regular,
    FastForward20Regular,
    PlayCircle20Regular,
    Rewind20Regular,
} from '@vicons/fluent'
import { VolumeMediumOutline } from '@vicons/ionicons5'

import type { FileInfo } from '@/utils/typescript'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useTheme } from '@/hooks/theme'
import { useEventListener } from '@vueuse/core'

const { isDark } = useTheme()

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

const audioInfo = ref<{
    title: string
    artist: string
    album: string
    duration: number // 秒
    bitrate: number // kbps
    cover: ArrayBuffer | null // 图片二进制
}>({
    title: '',
    artist: '',
    album: '',
    duration: 0,
    bitrate: 0,
    cover: null,
})
const getAudioInfo = async (path: string) => {
    audioInfo.value = await invoke('read_audio_info', { path })
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

const changeVolume = () => {
    const audio = player.value as HTMLAudioElement
    if (!audio) return
    audio.volume = Number(volume.value)
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    await getAudioInfo(fileInfo.value?.path)

    const path = convertFileSrc(fileInfo.value.path)

    const audio = player.value as HTMLAudioElement
    audio.src = path
    audio.volume = volume.value
    audio.addEventListener('play', () => {
        isPlaying.value = true
    })
    audio.addEventListener('pause', () => {
        isPlaying.value = false
    })
    audio.addEventListener('timeupdate', () => {
        currentTime.value = audio.currentTime
        currentTimeStr.value = formatTime(audio.currentTime)
    })
    audio.addEventListener('loadedmetadata', () => {
        duration.value = audio.duration
        durationStr.value = formatTime(audio.duration)
    })
    audio.addEventListener('volumechange', () => {
        volume.value = audio.volume
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
})
watch(duration, val => {
    durationStr.value = formatTime(val)
})
</script>

<template>
    <div class="audio-wrapper">
        <audio ref="player" class="audio-player-hidden" hidden autoplay></audio>
        <div class="audio-player" data-tauri-drag-region>
            <div class="audio-poster" data-tauri-drag-region></div>
            <div class="audio-ui">
                <div class="audio-controls" data-tauri-drag-region>
                    <el-icon class="audio-controls-btn" @click="backward" title="快退">
                        <Rewind20Regular />
                    </el-icon>
                    <el-icon class="audio-controls-btn" @click="togglePlay" title="暂停" v-if="isPlaying">
                        <PauseCircle20Regular />
                    </el-icon>
                    <el-icon class="audio-controls-btn" v-else @click="togglePlay" title="播放">
                        <PlayCircle20Regular />
                    </el-icon>
                    <el-icon class="audio-controls-btn" @click="forward" title="快进">
                        <FastForward20Regular />
                    </el-icon>
                </div>
                <div class="audio-progress">
                    <el-slider
                        v-model="currentTime"
                        :min="0"
                        :max="duration"
                        @input="seek"
                        :format-tooltip="val => formatTime(val)"
                        class="audio-slider"
                    />
                    <span class="audio-progress-time">- {{ remainStr }}</span>
                    <!-- <el-tooltip trigger="click" placement="top" effect="dark" :show-arrow="false">
                        <el-icon size="16">
                            <VolumeMediumOutline />
                        </el-icon>
                        <template #content>
                            <el-slider
                                v-model="volume"
                                :min="0"
                                :max="1"
                                :step="0.01"
                                @input="changeVolume"
                                size="small"
                                vertical
                                height="100px"
                                :show-format="false"
                                class="audio-slider"
                            />
                        </template>
                    </el-tooltip> -->
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
}
.audio-controls {
    flex: auto;
}

.audio-progress {
    flex: 0 0 50px;
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

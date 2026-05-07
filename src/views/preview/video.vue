<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import Player, { I18N } from 'xgplayer'
import 'xgplayer/dist/index.min.css'
import ZH from 'xgplayer/es/lang/zh-cn'

import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { load } from '@tauri-apps/plugin-store'

const route = useRoute()

// 启用中文
I18N.use(ZH)

defineOptions({
    name: 'VideoSupport',
})

const fileInfo = ref<FileInfo>()
let player: Player | null = null
const converting = ref(false)
const convertError = ref<string | null>(null)

const initPlayer = (url: string, isHls = false) => {
    if (player !== null) {
        player.destroy()
        ;(document.querySelector('#videos') as HTMLElement).innerHTML = ''
    }
    if (isHls) {
        // 动态加载 xgplayer-hls 插件以支持 m3u8 播放
        import('xgplayer-hls').then(mod => {
            const HlsPlugin = mod.default
            player = new Player({
                id: 'videos',
                isLive: false,
                url,
                height: '100%',
                width: '100%',
                // @ts-expect-error xgplayer-hls plugin is not reflected in the base Player type definitions
                plugins: [HlsPlugin],
            })
        })
    } else {
        player = new Player({
            id: 'videos',
            isLive: false,
            url,
            height: '100%',
            width: '100%',
        })
    }
}

onUnmounted(() => {
    // 如果窗口关闭或切换文件时转码仍在进行，通知后端终止 ffmpeg 进程
    if (converting.value) {
        invoke('cancel_video_conversion').catch((err) =>
            console.error('停止 ffmpeg 转换失败:', err),
        )
    }
    if (player !== null) {
        player.destroy()
    }
})

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const filePath = fileInfo.value.path

    // 检查是否启用了本机 ffmpeg 解析
    const store = await load('config.data', { autoSave: false })
    const useLocalFfmpeg = (await store.get<boolean>('useLocalFfmpeg')) ?? false

    if (useLocalFfmpeg) {
        converting.value = true
        convertError.value = null
        try {
            const m3u8Path = await invoke<string>('convert_video_to_hls', { path: filePath })
            const m3u8Url = convertFileSrc(m3u8Path)
            initPlayer(m3u8Url, true)
        } catch (e: unknown) {
            convertError.value = e instanceof Error ? e.message : String(e)
            // 回退到直接播放
            initPlayer(convertFileSrc(filePath), false)
        } finally {
            converting.value = false
        }
    } else {
        initPlayer(convertFileSrc(filePath), false)
    }
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="video-support">
            <div v-if="converting" class="video-converting">
                <el-text>正在使用 ffmpeg 转换视频，请稍候…</el-text>
            </div>
            <el-alert
                v-if="convertError"
                :title="'ffmpeg 转换失败，已回退到直接播放：' + convertError"
                type="warning"
                :closable="false"
                style="margin-bottom: 8px"
            />
            <div class="video-support-inner">
                <div id="videos"></div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.video-support {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    align-content: center;
    &-inner {
        width: 100%;
        height: 100%;
    }
}
.video-converting {
    padding: 8px 0;
    text-align: center;
}
</style>

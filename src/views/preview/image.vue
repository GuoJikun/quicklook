<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
    ArrowClockwise16Regular,
    ArrowFit16Regular,
    ArrowMinimize16Regular,
    ArrowRotateCounterclockwise20Regular,
    FullScreenMaximize16Regular,
    ZoomIn16Regular,
    ZoomOut16Regular,
} from '@vicons/fluent'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import PreviewImage from './components/preview-image.vue'

defineOptions({
    name: 'ImageSupport',
})

const { t } = useI18n()
const route = useRoute()

const fileInfo = ref<FileInfo>()

const loading = ref(false)
const imgPath = ref<string>()
// 图片全屏时隐藏标题行与命令栏，进入沉浸模式
const isFullscreen = ref(false)

const convertFormats = ['psd', 'tiff', 'tif', 'tga', 'pbm', 'pgm', 'ppm', 'qoi', 'exr', 'heic', 'heif', 'jxl']

const previewRef = ref<InstanceType<typeof PreviewImage> | null>(null)
const ZOOM_STEP = 1.2

const zoomPercent = computed(() => previewRef.value?.percent ?? 100)
const viewMode = computed(() => previewRef.value?.mode ?? 'fit')
const resolution = computed(() => {
    const w = previewRef.value?.naturalW ?? 0
    const h = previewRef.value?.naturalH ?? 0
    return w && h ? `${w} × ${h}` : ''
})
const fullscreen = computed(() => previewRef.value?.isFullscreen ?? false)

const init = async () => {
    loading.value = true
    let path = fileInfo.value?.path as string
    const ext = fileInfo.value?.extension as string
    if (convertFormats.includes(ext)) {
        path = await invoke('convert_to_png', { path })
    }
    imgPath.value = convertFileSrc(path) as string
    loading.value = false
}
onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    await init()
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading" :immersive="isFullscreen">
        <template #toolbar>
            <button class="pv-tb-btn" type="button" :title="t('image.zoomOut')" @click="previewRef?.zoomBy(1 / ZOOM_STEP)">
                <el-icon :size="15"><ZoomOut16Regular /></el-icon>
            </button>
            <span class="pv-tb-num" :title="t('image.zoomPercent')">{{ zoomPercent }}%</span>
            <button class="pv-tb-btn" type="button" :title="t('image.zoomIn')" @click="previewRef?.zoomBy(ZOOM_STEP)">
                <el-icon :size="15"><ZoomIn16Regular /></el-icon>
            </button>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="viewMode === 'fit'"
                :title="t('image.fit')"
                @click="previewRef?.applyFit()"
            >
                <el-icon :size="15"><ArrowFit16Regular /></el-icon>
                {{ t('image.fitShort') }}
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="viewMode === 'actual'"
                :title="t('image.actual')"
                @click="previewRef?.setActualSize()"
            >
                1:1
            </button>

            <i class="pv-tb-sep"></i>

            <button class="pv-tb-btn" type="button" :title="t('image.rotateLeft')" @click="previewRef?.rotateBy(-90)">
                <el-icon :size="15"><ArrowRotateCounterclockwise20Regular /></el-icon>
            </button>
            <button class="pv-tb-btn" type="button" :title="t('image.rotateRight')" @click="previewRef?.rotateBy(90)">
                <el-icon :size="15"><ArrowClockwise16Regular /></el-icon>
            </button>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="fullscreen ? t('image.exitFullscreen') : t('image.fullscreen')"
                @click="previewRef?.toggleFullscreen()"
            >
                <el-icon :size="15">
                    <ArrowMinimize16Regular v-if="fullscreen" />
                    <FullScreenMaximize16Regular v-else />
                </el-icon>
                {{ fullscreen ? t('image.exitFullscreenShort') : t('image.fullscreen') }}
            </button>

            <div class="pv-tb-grow"></div>
            <span v-if="resolution" class="pv-tb-info">{{ resolution }}</span>
        </template>

        <div class="image-support">
            <div class="image-support-inner" id="canvas">
                <PreviewImage
                    v-if="!loading"
                    ref="previewRef"
                    :src="imgPath as string"
                    @fullscreen-change="isFullscreen = $event"
                />
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.image-support {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;

    &-inner {
        width: 100%;
        height: 100%;
        overflow: hidden;
    }
}
</style>

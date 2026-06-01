<script setup lang="ts">
import { ref, onMounted } from 'vue'

import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import PreviewImage from './components/preview-image.vue'

defineOptions({
    name: 'ImageSupport',
})

const route = useRoute()

const fileInfo = ref<FileInfo>()

const loading = ref(false)
const imgPath = ref<string>()

const convertFormats = ['psd', 'tiff', 'tif', 'tga', 'pbm', 'pgm', 'ppm', 'qoi', 'exr']

const init = async () => {
    loading.value = true
    let path = fileInfo.value?.path as string
    const ext = fileInfo.value?.extension as string
    if (convertFormats.includes(ext)) {
        console.log('ext', ext)
        if (ext === 'psd') {
            path = await invoke('psd_to_png', { path })
        } else {
            console.log(1)
            path = await invoke('image_to_png', { path })

            console.log(2)
        }
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
    <LayoutPreview :file="fileInfo" :loading="loading">
        <div class="image-support">
            <div class="image-support-inner" id="canvas">
                <PreviewImage v-if="!loading" :src="imgPath as string" />
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
    align-content: center;

    &-inner {
        width: 100%;
        height: 100%;
        overflow: hidden;
    }
}
</style>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { App, Image, ImageEvent } from 'leafer-ui'
import '@leafer-in/view'
import '@leafer-in/viewport'

import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

defineOptions({
    name: 'ImageSupport',
})

const route = useRoute()

let leafer: App | null = null
let image: Image | null = null
const fileInfo = ref<FileInfo>()

const loading = ref(false)
const init = (path: string) => {
    loading.value = true
    if (leafer !== null) {
        leafer?.destroy()
        image?.destroy()
    }
    leafer = new App({
        view: 'canvas', // 支持 window 、div、canvas 标签对象， 可使用id字符串(不用加 # 号)
        tree: {
            type: 'viewport', // 视口类型，支持 viewport、window、div、canvas
        },
    })

    image = new Image({
        url: path,
        draggable: true,
        scale: 1,
        x: 0,
        y: 0,
    })

    image.load()

    image.on(ImageEvent.LOADED, (ev: ImageEvent) => {
        console.log('Image loaded:', ev)
        leafer?.tree.add(image as Image)
        leafer?.tree.zoom('fit', 0, true)
        loading.value = false
    })
    image.on(ImageEvent.ERROR, () => {
        ElMessage.error('图片加载失败')
        loading.value = false
    })
}
onMounted(() => {
    fileInfo.value = route.query as unknown as FileInfo
    init(convertFileSrc(fileInfo.value.path))
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <div class="image-support">
            <div class="image-support-inner" id="canvas"></div>
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
        background-color: #efefef;
    }
}
</style>

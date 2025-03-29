<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Leafer, Image } from 'leafer-ui'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc } from '@tauri-apps/api/core'

defineOptions({
    name: 'ImageSupport',
})

const route = useRoute()

let leafer: Leafer | null = null
let image: Image | null = null
const fileInfo = ref<FileInfo>()

const init = (path: string) => {
    if (leafer !== null) {
        leafer?.destroy()
        image?.destroy()
    }
    leafer = new Leafer({
        view: 'canvas', // 支持 window 、div、canvas 标签对象， 可使用id字符串(不用加 # 号)
        width: 0, // 不能设置为 0， 否则会变成自动布局
        height: 0,
    })

    image = new Image({
        url: path,
        draggable: true,
    })

    leafer.add(image)
}
onMounted(() => {
    fileInfo.value = route.query as unknown as FileInfo
    init(convertFileSrc(fileInfo.value.path))
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
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
    }
}
</style>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import PreviewImage from './components/preview-image.vue'

defineOptions({
    name: 'ImageSupport',
})

const route = useRoute()

const fileInfo = ref<FileInfo>()

const loading = ref(false)
const imgPath = ref<string>()
const init = async () => {
    loading.value = true
    let path = fileInfo.value?.path as string
    if (fileInfo.value?.extension == 'psd') {
        path = await invoke('psd_to_png', { path })
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
                <PreviewImage :src="imgPath as string" />
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
        background-color: #efefef;
        overflow: hidden;
    }
}
</style>

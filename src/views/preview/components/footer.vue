<script setup lang="ts">
import { watch, ref } from 'vue'
import { lstat, type FileInfo } from '@tauri-apps/plugin-fs'

interface File {
    path: string
    file_type: string
    extension: string
}
interface Props {
    file?: File
}
const props = defineProps<Props>()

const fileInfo = ref<FileInfo>()

watch(
    () => props.file,
    async (val, oldVal) => {
        if (val?.path !== oldVal?.path || !oldVal) {
            fileInfo.value = await lstat(val?.path as string)
            // console.log(fileInfo.value, file)
        }
    },
)
</script>

<template>
    <div class="footer">
        <span class="footer-item">类型：{{ props.file?.file_type }}</span>
        <span class="footer-item">格式：{{ props.file?.extension }}</span>
        <span class="footer-item">大小：{{ fileInfo?.size }}</span>
    </div>
</template>

<style scoped lang="scss">
.footer {
    height: 20px;
    padding: 0 8px;
    background: #f5f5f5;
    display: flex;
    align-items: center;
    align-content: center;
    font-size: 12px;
    gap: 12px;
    &-item {
        zoom: 0.8;
    }
}
</style>

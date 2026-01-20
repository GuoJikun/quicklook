<script setup lang="ts">
import type { FileInfo } from '@/utils/typescript'
import { formatBytes } from '@/utils/index'

interface Props {
    file?: FileInfo
}
const { file } = defineProps<Props>()
</script>

<template>
    <div class="footer">
        <span class="footer-item">文件类型：{{ file?.file_type }}</span>
        <span class="footer-item">文件格式：{{ file?.extension }}</span>
        <span class="footer-item">文件大小：{{ formatBytes(file?.size as number) }}</span>
    </div>
</template>

<style scoped lang="scss">
.footer {
    height: 20px;
    padding: 0 8px;
    background-color: var(--color-surface);
    color: var(--color-text-primary);
    display: flex;
    align-items: center;
    align-content: center;
    font-size: 12px;
    gap: 12px;
    user-select: none;
    &-item {
        zoom: 0.8;
    }
}
</style>

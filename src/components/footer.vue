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
    height: var(--layout-footer-h);
    padding: 0 var(--space-2);
    background-color: var(--color-surface);
    color: var(--color-text-primary);
    display: flex;
    align-items: center;
    font-size: var(--font-xs);
    gap: var(--space-3);
    user-select: none;
    &-item {
        zoom: 0.8;
    }
}
</style>

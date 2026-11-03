<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { FileInfo } from '@/utils/typescript'
import { formatBytes } from '@/utils/index'

interface Props {
    file?: FileInfo
}
const { file } = defineProps<Props>()

const { t } = useI18n()
</script>

<template>
    <div class="footer">
        <span class="footer-item">{{ t('footer.type', { value: file?.file_type ?? '—' }) }}</span>
        <span class="footer-item">{{ t('footer.format', { value: file?.extension ?? '—' }) }}</span>
        <span class="footer-item">{{ t('footer.size', { value: formatBytes(file?.size) }) }}</span>
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

<script setup lang="ts">
import Header from './header.vue'
import Footer from './footer.vue'
import type { FileInfo } from '@/utils/typescript'

interface Props {
    file?: FileInfo
    loading?: boolean
}

const props = defineProps<Props>()
</script>

<template>
    <div class="preview">
        <Header class="preview-header" :file="props.file" />
        <div class="preview-body" v-loading="props.loading" element-loading-background="rgba(210, 210, 210, 0.5)">
            <slot></slot>
        </div>
        <Footer class="preview-footer" :file="props.file" />
    </div>
</template>

<style scoped lang="scss">
.preview {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;

    &-header {
        flex: 0 0 var(--layout-header-h);
        height: var(--layout-header-h);
    }

    &-footer {
        flex: 0 0 var(--layout-footer-h);
        height: var(--layout-footer-h);
    }

    &-body {
        flex: 1;
        min-height: 0;
        display: flex;
        justify-content: center;
        align-items: center;
    }
}
</style>

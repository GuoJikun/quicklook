<script setup lang="ts">
import Header from './header.vue'
import Footer from './footer.vue'
import type { FileInfo } from '@/utils/typescript'

interface Props {
    file?: FileInfo
    loading?: boolean
    /** 沉浸模式：隐藏头尾，仅保留内容区，用于图片全屏等场景 */
    immersive?: boolean
}

const props = defineProps<Props>()
</script>

<template>
    <div class="preview">
        <Header v-if="!props.immersive" class="preview-header" :file="props.file" />
        <div class="preview-body" v-loading="props.loading" element-loading-background="rgba(210, 210, 210, 0.5)">
            <slot></slot>
        </div>
        <Footer v-if="!props.immersive" class="preview-footer" :file="props.file" />
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

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
    &-header {
        position: sticky;
        left: 0;
        top: 0;
        width: 100%;
    }
    &-footer {
        position: absolute;
        left: 0;
        bottom: 0;
        width: 100%;
        font-size: 12px;
    }
    &-body {
        height: calc(100% - 48px);
        display: flex;
        justify-content: center;
        align-items: center;
        align-content: center;
    }
}
</style>

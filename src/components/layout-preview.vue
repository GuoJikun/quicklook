<script setup lang="ts">
import Header from './header.vue'
import type { FileInfo } from '@/utils/typescript'

interface Props {
    file?: FileInfo
    loading?: boolean
    /** 沉浸模式：隐藏标题行与命令栏，仅保留内容区，用于图片全屏等场景 */
    immersive?: boolean
    /** 加载失败等场景下命令栏已无可用操作，整条隐藏而不是留一条空栏 */
    hideToolbar?: boolean
}

const props = defineProps<Props>()
</script>

<template>
    <div class="preview">
        <Header v-if="!props.immersive" :file="props.file" />

        <!-- 命令栏：仅当视图提供了工具栏内容时才占位，无操作的类型不留空条 -->
        <div v-if="!props.immersive && !props.hideToolbar && $slots.toolbar" class="preview-toolbar">
            <slot name="toolbar"></slot>
        </div>

        <div class="preview-body" v-loading="props.loading">
            <slot></slot>
        </div>
    </div>
</template>

<style scoped lang="scss">
/* 三层结构：标题行 44 → 命令栏 40 → 内容 1fr。
   三层同为 --color-bg 底色，没有白底横带与 1px 分隔线，
   视觉衔接靠留白与控件本身的圆角，与音频预览保持同一套语言。 */
.preview {
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
    background-color: var(--color-bg);

    &-toolbar {
        flex: none;
        height: var(--layout-toolbar-h);
        display: flex;
        align-items: center;
        gap: 2px;
        padding: 0 10px;
        /* 命令栏高度固定，任何一处折行都会顶破容器并溢出到内容区 */
        overflow: hidden;
        white-space: nowrap;
    }

    &-body {
        flex: 1;
        min-height: 0;
        position: relative;
        display: flex;
        justify-content: center;
        align-items: center;
        overflow: hidden;

        /* loading 蒙版跟随主题，避免暗色下闪一片浅灰 */
        :deep(.el-loading-mask) {
            background-color: color-mix(in srgb, var(--color-bg) 72%, transparent);
        }
    }
}
</style>

<script setup lang="ts">
import { Dismiss20Regular, Maximize20Regular } from '@vicons/fluent'

import { getCurrentWindow } from '@tauri-apps/api/window'

interface LayoutHeaderProps {
    logo?: string
    title?: string
}
const props = withDefaults(defineProps<LayoutHeaderProps>(), {
    title: '文件预览',
})

const handleClose = () => {
    const curWindow = getCurrentWindow()
    curWindow.hide()
}

// const handleMin = () => {
//     const curWindow = getCurrentWindow()
//     curWindow.minimize()
// }

const handleMax = () => {
    const curWindow = getCurrentWindow()
    curWindow.toggleMaximize()
}
</script>

<template>
    <div class="layout-header" data-tauri-drag-region>
        <div class="layout-header-extra">
            <div>
                <slot name="logo">
                    <img v-if="props.logo" :src="logo" alt="logo" />
                </slot>
                <h1 class="layout-header-title">{{ props.title }}</h1>
            </div>
            <div>
                <slot name="menu"></slot>
            </div>
        </div>
        <div class="layout-header-operate">
            <div class="layout-header-operate-item" @click="handleMax">
                <n-icon><Maximize20Regular /></n-icon>
            </div>
            <div class="layout-header-operate-item" @click="handleClose">
                <n-icon><Dismiss20Regular /></n-icon>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.layout-header {
    display: flex;
    justify-content: space-between;
    position: sticky;
    top: 0;
    left: 0;
    height: 28px;
    font-size: 12px;
    background-color: rgb(239, 244, 249);
    gap: 12px;
    &-extra {
        display: flex;
        justify-content: flex-start;
        align-items: center;
        padding: 0 12px;
        height: 100%;
    }

    &-title {
        font-weight: 500;
        font-size: 12px;
    }
    &-operate {
        display: flex;
        justify-content: flex-end;
        align-items: center;
        height: 100%;
        font-size: 20px;
        &-item {
            cursor: pointer;
            width: 2em;
            font-size: 1em;
            color: currentColor;
            display: inline-flex;
            justify-content: center;
            align-items: center;
            height: 100%;
            &:hover {
                background-color: rgba(0, 0, 0, 0.1);
            }
            &:last-child:hover {
                background-color: #f56c6c;
                color: white;
            }
        }
    }
}
</style>

<script setup lang="ts">
import {
    Dismiss16Regular,
    Maximize16Regular,
    Open16Regular,
    Apps16Regular,
    Pin16Regular,
    PinOff16Regular,
    WeatherSunny16Regular,
    WeatherMoon16Regular,
} from '@vicons/fluent'
import { ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { openPath } from '@tauri-apps/plugin-opener'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { isDark, toggleDark } from '@/hooks/theme'

interface LayoutHeaderProps {
    logo?: string
    title?: string
    file?: FileInfo
}
const props = withDefaults(defineProps<LayoutHeaderProps>(), {
    title: '文件预览',
    path: '',
})

const handleClose = () => {
    const curWindow = getCurrentWindow()
    curWindow.close()
}

// const handleMin = () => {
//     const curWindow = getCurrentWindow()
//     curWindow.minimize()
// }

const handleMax = () => {
    const curWindow = getCurrentWindow()
    curWindow.toggleMaximize()
}

const openByDefault = async () => {
    const path = props.file?.path
    if (path) {
        const result = await openPath(path)
        console.log(result)
    }
}

const openWith = async () => {
    const path = props.file?.path
    if (path) {
        await invoke('show_open_with_dialog', { path })
    }
}

const pined = ref<boolean>(false)
const pin = async () => {
    const curWindow = getCurrentWindow()
    if (pined.value) {
        pined.value = false
    } else {
        pined.value = true
    }
    await curWindow.setAlwaysOnTop(pined.value)
}

const defaultProgrameName = ref<string>('使用默认程序打开')
watch(
    () => props.file,
    async val => {
        const path = val?.path
        if (path) {
            const name = (await invoke('get_default_program_name', { path: path })) as string
            defaultProgrameName.value = name ? `使用 ${name} 打开` : '使用默认程序打开'
        } else {
            defaultProgrameName.value = '使用默认程序打开'
        }
    },
)
</script>

<template>
    <div class="layout-header" data-tauri-drag-region>
        <div class="layout-header-extra no-seleced" data-tauri-drag-region>
            <slot name="logo">
                <img v-if="props.logo" :src="logo" alt="logo" data-tauri-drag-region />
            </slot>
            <h1 class="layout-header-title" data-tauri-drag-region>{{ props?.file?.name || props.title }}</h1>
        </div>
        <div class="layout-header-operate no-selected" data-tauri-drag-region>
            <div
                class="layout-header-operate-item"
                @click="toggleDark()"
                :title="isDark ? '切换为明亮模式' : '切换为暗黑模式'"
            >
                <n-icon :size="16"><WeatherMoon16Regular v-if="isDark" /><WeatherSunny16Regular v-else /></n-icon>
            </div>
            <div class="layout-header-operate-item" @click="pin" :title="`${pined ? '取消固定' : '固定'}`">
                <n-icon :size="16"><PinOff16Regular v-if="pined" /><Pin16Regular v-else /></n-icon>
            </div>
            <div class="layout-header-operate-item" @click="openByDefault" :title="defaultProgrameName">
                <n-icon :size="16"><Open16Regular /></n-icon>
            </div>
            <div class="layout-header-operate-item" @click="openWith" title="推荐打开程序列表">
                <n-icon :size="16"><Apps16Regular /></n-icon>
            </div>
            <div class="layout-header-operate-item" @click="handleMax" title="最大化">
                <n-icon :size="16"><Maximize16Regular /></n-icon>
            </div>
            <div class="layout-header-operate-item is-close" @click="handleClose" title="关闭">
                <n-icon :size="16"><Dismiss16Regular /></n-icon>
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
    font-size: 1.2rem;
    background-color: var(--color-surface);
    color: var(--color-text-primary);
    gap: 12px;
    :deep(i.n-icon) {
        cursor: pointer;
        pointer-events: none;
    }
    &-extra {
        display: inline-flex;
        justify-content: flex-start;
        align-items: center;
        padding: 0 12px;
        height: 100%;
        flex: auto;
        max-width: 50%;
    }

    &-title {
        font-weight: 400;
        font-size: 1.2rem;
        text-overflow: ellipsis;
        overflow: hidden;
        white-space: nowrap;
        max-width: 100%;
    }
    &-operate {
        display: inline-flex;
        justify-content: flex-end;
        align-items: center;
        height: 100%;
        font-size: 2rem;
        flex: 0 1 50%;
        &-item {
            cursor: pointer;
            flex: 0 1 4rem;
            min-width: 4rem;
            font-size: inherit;
            color: var(--color-text-primary);
            display: inline-flex;
            justify-content: center;
            align-items: center;
            height: 100%;
            &:hover {
                background-color: var(--color-hover-bg);
            }
            &.is-close {
                &:hover {
                    background-color: #f12724;
                    color: white;
                }
            }
        }
    }
}
</style>

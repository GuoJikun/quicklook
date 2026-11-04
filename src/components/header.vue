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
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { openPath } from '@tauri-apps/plugin-opener'
import { invoke } from '@tauri-apps/api/core'
import type { FileInfo } from '@/utils/typescript'
import { formatBytes } from '@/utils'
import { useTheme } from '@/hooks/theme'
import { useWindow } from '@/hooks/use-window'

const { t } = useI18n()
const { isDark, toggle } = useTheme()
const { handleClose } = useWindow()

interface LayoutHeaderProps {
    title?: string
    file?: FileInfo
}
const props = withDefaults(defineProps<LayoutHeaderProps>(), {
    title: '',
})

/** 标题行右侧的元信息胶囊：格式与体积，取代原状态栏 */
const pills = computed(() => {
    const list: Array<string> = []
    if (props.file?.extension) list.push(props.file.extension)
    // 路由 query 里的 size 是字符串，formatBytes 只接受数字，否则会显示「—」
    const size = Number(props.file?.size)
    if (Number.isFinite(size) && size > 0) list.push(formatBytes(size))
    return list
})

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
    pined.value = !pined.value
    await curWindow.setAlwaysOnTop(pined.value)
}

const defaultProgramName = ref<string>('')
const openWithDefaultLabel = computed(() =>
    defaultProgramName.value
        ? t('header.openWithApp', { name: defaultProgramName.value })
        : t('header.openWithDefault'),
)
watch(
    () => props.file,
    async val => {
        const path = val?.path
        if (path) {
            const name = (await invoke('get_default_program_name', { path: path })) as string
            defaultProgramName.value = name || ''
        } else {
            defaultProgramName.value = ''
        }
    },
)
</script>

<template>
    <div class="layout-header" data-tauri-drag-region>
        <div class="layout-header-main no-selected" data-tauri-drag-region>
            <h1 class="layout-header-title" data-tauri-drag-region>
                {{ props?.file?.name || props.title || t('header.defaultTitle') }}
            </h1>
            <div v-if="pills.length" class="layout-header-pills">
                <span class="pv-pill">{{ pills[0] }}</span>
                <span v-if="pills[1]" class="pv-pill is-muted">{{ pills[1] }}</span>
            </div>
        </div>
        <div class="layout-header-operate no-selected">
            <div
                class="layout-header-operate-item"
                @click="toggle"
                :title="isDark ? t('header.toLight') : t('header.toDark')"
            >
                <el-icon :size="15"><WeatherMoon16Regular v-if="isDark" /><WeatherSunny16Regular v-else /></el-icon>
            </div>
            <div class="layout-header-operate-item" @click="pin" :title="pined ? t('header.unpin') : t('header.pin')">
                <el-icon :size="15"><PinOff16Regular v-if="pined" /><Pin16Regular v-else /></el-icon>
            </div>
            <div class="layout-header-operate-item" @click="openByDefault" :title="openWithDefaultLabel">
                <el-icon :size="15"><Open16Regular /></el-icon>
            </div>
            <div class="layout-header-operate-item" @click="openWith" :title="t('header.openWithList')">
                <el-icon :size="15"><Apps16Regular /></el-icon>
            </div>
            <div class="layout-header-operate-item" @click="handleMax" :title="t('header.maximize')">
                <el-icon :size="15"><Maximize16Regular /></el-icon>
            </div>
            <div class="layout-header-operate-item is-close" @click="handleClose" :title="t('header.close')">
                <el-icon :size="15"><Dismiss16Regular /></el-icon>
            </div>
        </div>
    </div>
</template>

<style lang="scss" scoped>
/* 标题行 44：透明底、无下边框。标题与胶囊直接浮在窗口底色上，
   不再是一条白底横带（那是旧版被指为「不是 Fluent」的主要原因）。 */
.layout-header {
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--layout-header-h);
    padding: 0 6px 0 16px;
    background-color: transparent;
    color: var(--color-text-primary);
    flex: none;
    user-select: none;
}

.layout-header-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 0 1 auto;
}

.layout-header-title {
    font-size: var(--font-lg);
    font-weight: 600;
    letter-spacing: -0.01em;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 0 1 auto;
    min-width: 0;
}

.layout-header-pills {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
}

.layout-header-operate {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    margin-left: auto;
    height: 100%;
    flex: none;

    &-item {
        cursor: pointer;
        width: 34px;
        height: 30px;
        border-radius: var(--radius-sm);
        color: var(--color-text-secondary);
        display: inline-flex;
        justify-content: center;
        align-items: center;
        transition:
            background-color var(--transition-fast),
            color var(--transition-fast);

        &:hover {
            background-color: var(--elevation-hover);
            color: var(--color-text-primary);
        }

        &:active {
            background-color: var(--color-chip-bg);
        }

        &.is-close {
            margin-left: 4px;

            &:hover {
                background-color: var(--color-danger);
                color: #fff;
            }
        }

        :deep(i.el-icon) {
            cursor: pointer;
            pointer-events: none;
        }
    }
}
</style>

<script setup lang="ts">
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useEventListener } from '@vueuse/core'
import { onMounted } from 'vue'

const appWindow = getCurrentWebviewWindow()

onMounted(() => {
    useEventListener('contextmenu', (e: MouseEvent) => {
        e.preventDefault()
    })
    useEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Escape') {
            appWindow.close()
        }
    })
})
</script>

<template>
    <router-view v-slot="{ Component }">
        <template v-if="Component">
            <transition mode="out-in">
                <component :is="Component"></component>
            </transition>
        </template>
    </router-view>
</template>

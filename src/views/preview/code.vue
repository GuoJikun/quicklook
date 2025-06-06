<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { codeToHtml } from 'shiki'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { readTextFile } from '@/utils'

const route = useRoute()

defineOptions({
    name: 'CodeSupport',
})

const fileInfo = ref<FileInfo>()
const content = ref<string>()
const loading = ref<boolean>(false)

const getLanguage = (extension: string) => {
    let ext = extension
    if (['cjs', 'mjs'].includes(extension)) {
        ext = 'js'
    } else if (['cts', 'mts'].includes(extension)) {
        ext = 'ts'
    } else if (['markdown'].includes(extension)) {
        ext = 'md'
    } else if (['json5', 'json'].includes(extension)) {
        ext = 'json'
    } else if (extension === 'ps1') {
        ext = 'powershell'
    }
    return ext
}

onMounted(async () => {
    loading.value = true
    fileInfo.value = route.query as unknown as FileInfo
    const path = fileInfo.value.path as string

    const code = await readTextFile(path)
    const lang = getLanguage(fileInfo.value.extension)
    content.value = await codeToHtml(code, {
        lang: lang,
        themes: {
            light: 'github-light',
            dark: 'github-dark',
        },
    })
    loading.value = false
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <div class="code-support">
            <div class="code-support-inner" v-html="content"></div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.code-support {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    align-content: center;
    &-inner {
        width: 100%;
        height: 100%;
        overflow: auto;
        padding: 12px 16px;
        font-size: 13px;
        & :deep(pre),
        & :deep(pre code) {
            font-family: 'Courier New', Courier, monospace;
            line-height: 1.2em;
        }
    }
}
</style>

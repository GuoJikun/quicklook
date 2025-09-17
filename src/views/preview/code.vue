<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { createHighlighter } from 'shiki'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { readTextFile } from '@/utils'
import { isDark } from '@/hooks/theme'

const route = useRoute()

defineOptions({
    name: 'CodeSupport',
})

const fileInfo = ref<FileInfo>()
const content = ref<string>()
const loading = ref<boolean>(false)
const innerRef = ref<HTMLDivElement | null>(null)

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

const genHtml = async () => {
    loading.value = true
    const path = fileInfo.value?.path as string

    const code = await readTextFile(path)
    const lang = getLanguage(fileInfo.value?.extension as string)
    const highlighter = await createHighlighter({
        langs: [lang],
        themes: ['github-light', 'github-dark'], // 注册主题
    })
    content.value = highlighter.codeToHtml(code, {
        lang: lang,
        theme: isDark.value ? 'github-dark' : 'github-light',
        colorReplacements: {
            'github-dark': {
                '#24292e': 'var(--color-bg)',
            },
            'github-light': {
                '#fff': 'var(--color-bg)',
            },
        },
    })
    loading.value = false
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    await genHtml()
})

watch(isDark, async () => {
    await genHtml()
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <div class="code-support">
            <div class="code-support-inner" ref="innerRef" v-html="content"></div>
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

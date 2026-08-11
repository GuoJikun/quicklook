<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { createHighlighter } from 'shiki'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { readTextFile } from '@/utils'
import { useTheme } from '@/hooks/theme'

const route = useRoute()
const { isDark } = useTheme()

defineOptions({
    name: 'CodeSupport',
})

const fileInfo = ref<FileInfo>()
const content = ref<string>()
const loading = ref<boolean>(false)
const innerRef = ref<HTMLDivElement | null>(null)

const getLanguage = (extension: string): string => {
    // shiki 语言标识符映射表（扩展名 -> shiki 支持的 lang id）
    const langMap: Record<string, string> = {
        cjs: 'javascript',
        mjs: 'javascript',
        cts: 'typescript',
        mts: 'typescript',
        markdown: 'markdown',
        json5: 'json',
        ps1: 'powershell',
        styl: 'stylus',
        h: 'c',
        pas: 'pascal',
        m: 'matlab',
        txt: 'plaintext',
    }
    return langMap[extension] || extension
}

const genHtml = async () => {
    loading.value = true
    content.value = undefined
    const path = fileInfo.value?.path as string

    try {
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
    } catch (error) {
        console.error('Code preview error:', error)
        content.value = `<pre><code>${String(error)}</code></pre>`
    } finally {
        loading.value = false
    }
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

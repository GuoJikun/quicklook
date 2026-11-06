<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { createHighlighter } from 'shiki'
import { ArrowWrap20Regular, Copy16Regular } from '@vicons/fluent'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { readTextFile } from '@/utils'
import { useTheme } from '@/hooks/theme'

const route = useRoute()
const { isDark } = useTheme()
const { t } = useI18n()

defineOptions({
    name: 'CodeSupport',
})

const fileInfo = ref<FileInfo>()
const content = ref<string>()
const rawCode = ref<string>('')
const loading = ref<boolean>(false)

const FONT_MIN = 12
const FONT_MAX = 18
const fontSize = ref(13)
const wrap = ref(false)
const copied = ref(false)
/** 「已复制」的回落计时器：连点复制时要先清掉上一颗，否则提示会提前消失 */
let copyTimer = 0

/** 扩展名 -> shiki 语言 id */
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

/** 语言 id -> 命令栏徽章上显示的名称 */
const LANG_LABELS: Record<string, string> = {
    javascript: 'JavaScript',
    typescript: 'TypeScript',
    rust: 'Rust',
    python: 'Python',
    csharp: 'C#',
    cpp: 'C++',
    c: 'C',
    go: 'Go',
    java: 'Java',
    kotlin: 'Kotlin',
    php: 'PHP',
    ruby: 'Ruby',
    html: 'HTML',
    css: 'CSS',
    scss: 'SCSS',
    less: 'LESS',
    json: 'JSON',
    yaml: 'YAML',
    toml: 'TOML',
    xml: 'XML',
    sql: 'SQL',
    shellscript: 'Shell',
    powershell: 'PowerShell',
    markdown: 'Markdown',
    plaintext: 'Text',
    vue: 'Vue',
    dockerfile: 'Dockerfile',
    lua: 'Lua',
    swift: 'Swift',
    dart: 'Dart',
    zig: 'Zig',
    graphql: 'GraphQL',
    diff: 'Diff',
}

/**
 * 扩展名 -> 展示名。
 * shiki 靠别名就能高亮（rs / ts / py 都在别名表里），但徽章上要显示规范名，
 * 否则会出现「Rs」这种把扩展名首字母大写的错误结果。
 */
const EXT_LABELS: Record<string, string> = {
    rs: 'Rust',
    ts: 'TypeScript',
    tsx: 'TSX',
    js: 'JavaScript',
    jsx: 'JSX',
    mjs: 'JavaScript',
    cjs: 'JavaScript',
    mts: 'TypeScript',
    cts: 'TypeScript',
    py: 'Python',
    rb: 'Ruby',
    kt: 'Kotlin',
    kts: 'Kotlin',
    cs: 'C#',
    fs: 'F#',
    hs: 'Haskell',
    ex: 'Elixir',
    exs: 'Elixir',
    erl: 'Erlang',
    clj: 'Clojure',
    sh: 'Shell',
    bash: 'Shell',
    zsh: 'Shell',
    fish: 'Fish',
    ps1: 'PowerShell',
    bat: 'Batch',
    cmd: 'Batch',
    yml: 'YAML',
    toml: 'TOML',
    ini: 'INI',
    conf: 'INI',
    htm: 'HTML',
    vue: 'Vue',
    svelte: 'Svelte',
    sql: 'SQL',
    gql: 'GraphQL',
    proto: 'Protobuf',
    tex: 'LaTeX',
    patch: 'Diff',
    cc: 'C++',
    cxx: 'C++',
    hpp: 'C++',
    hh: 'C++',
    mdx: 'MDX',
    styl: 'Stylus',
    sass: 'Sass',
    r: 'R',
    jl: 'Julia',
    zig: 'Zig',
    nim: 'Nim',
    sol: 'Solidity',
    prisma: 'Prisma',
    dockerfile: 'Dockerfile',
    env: 'Dotenv',
    csv: 'CSV',
    tf: 'Terraform',
}

const lang = computed(() => getLanguage(fileInfo.value?.extension ?? 'txt'))
const langLabel = computed(
    () => LANG_LABELS[lang.value] ?? EXT_LABELS[lang.value] ?? lang.value.replace(/^\w/, c => c.toUpperCase()),
)

// 文件末尾的换行不算独立一行
const lineCount = computed(() => {
    if (!rawCode.value) return 0
    const lines = rawCode.value.split(/\r?\n/)
    if (lines.length > 1 && lines[lines.length - 1] === '') lines.pop()
    return lines.length
})

const genHtml = async () => {
    loading.value = true
    content.value = undefined
    const path = fileInfo.value?.path as string

    try {
        const code = await readTextFile(path)
        rawCode.value = code
        const highlighter = await createHighlighter({
            langs: [lang.value],
            themes: ['github-light', 'github-dark'],
        })
        content.value = highlighter.codeToHtml(code, {
            lang: lang.value,
            theme: isDark.value ? 'github-dark' : 'github-light',
            // 背景交给外层容器，避免 shiki 的主题底色与窗口底色不一致
            colorReplacements: {
                'github-dark': { '#24292e': 'transparent' },
                'github-light': { '#fff': 'transparent' },
            },
        })
    } catch (error) {
        console.error('Code preview error:', error)
        rawCode.value = ''
        content.value = `<pre><code>${String(error)}</code></pre>`
    } finally {
        loading.value = false
    }
}

const zoomFont = (delta: number) => {
    fontSize.value = Math.min(FONT_MAX, Math.max(FONT_MIN, fontSize.value + delta))
}

const copyCode = async () => {
    if (!rawCode.value) return
    try {
        await navigator.clipboard.writeText(rawCode.value)
        copied.value = true
        window.clearTimeout(copyTimer)
        copyTimer = window.setTimeout(() => (copied.value = false), 1600)
    } catch (error) {
        ElMessage.error(t('code.copyFailed', { error: String(error) }))
    }
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    await genHtml()
})

watch(isDark, async () => {
    await genHtml()
})

onUnmounted(() => {
    window.clearTimeout(copyTimer)
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <template #toolbar>
            <span class="pv-tb-chip">{{ langLabel }}</span>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="wrap"
                :title="t('code.wrap')"
                @click="wrap = !wrap"
            >
                <ArrowWrap20Regular />
                {{ t('code.wrap') }}
            </button>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('code.fontSmaller')"
                :disabled="fontSize <= FONT_MIN"
                @click="zoomFont(-1)"
            >
                A−
            </button>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('code.fontLarger')"
                :disabled="fontSize >= FONT_MAX"
                @click="zoomFont(1)"
            >
                A+
            </button>

            <div class="pv-tb-grow"></div>

            <span class="pv-tb-info">{{ t('code.summary', { lang: langLabel, count: lineCount }) }}</span>
            <i class="pv-tb-sep"></i>
            <button class="pv-tb-btn is-text" type="button" :title="t('code.copy')" :disabled="!rawCode" @click="copyCode">
                <el-icon><Copy16Regular /></el-icon>
                {{ copied ? t('code.copied') : t('code.copy') }}
            </button>
        </template>

        <div class="code-support">
            <div class="code-support-inner pv-scroll" :class="{ 'is-wrap': wrap }" :style="{ fontSize: `${fontSize}px` }">
                <div class="code-support-html" v-html="content"></div>
            </div>
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

    &-inner {
        width: 100%;
        height: 100%;
        padding: 4px 0 40px;
        font-family: 'Cascadia Mono', Consolas, 'Courier New', monospace;
        line-height: var(--preview-code-line-h);
    }

    /* 行号用 CSS 计数器画在每行左侧，不改变 shiki 的输出结构 */
    &-html :deep(pre.shiki) {
        margin: 0;
        padding: 10px 20px 0 0;
        background: transparent !important;
        font-family: inherit;
        font-size: inherit;
        line-height: inherit;
        overflow: visible;
    }

    &-html :deep(pre.shiki code) {
        display: block;
        counter-reset: code-line;
        font-family: inherit;
        font-size: inherit;
        line-height: inherit;
        /* shiki 的 pre 是 white-space: pre，每行 span 之间的换行符会被渲染成空行，
           行距因此翻倍。行内空格由 .line 自己的 white-space: pre 保留，这里归零。 */
        white-space: normal;
    }

    &-html :deep(.line) {
        display: block;
        position: relative;
        min-height: var(--preview-code-line-h);
        padding-left: 60px;
        padding-right: 12px;
        white-space: pre;

        &::before {
            counter-increment: code-line;
            content: counter(code-line);
            position: absolute;
            left: 0;
            top: 0;
            width: 44px;
            text-align: right;
            color: var(--color-text-tertiary);
            font-variant-numeric: tabular-nums;
            user-select: none;
            pointer-events: none;
        }

        &:hover {
            background: var(--color-chip-bg);
        }
    }

    /* 自动换行：行号列仍固定在左侧，长行在代码区内折行 */
    &-inner.is-wrap &-html :deep(.line) {
        white-space: pre-wrap;
        overflow-wrap: anywhere;
    }
}
</style>

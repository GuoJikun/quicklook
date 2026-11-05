<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { Copy16Regular } from '@vicons/fluent'
import { convertFileSrc } from '@tauri-apps/api/core'
import { readFile } from '@tauri-apps/plugin-fs'
import { openPath } from '@tauri-apps/plugin-opener'
import type { FileInfo } from '@/utils/typescript'
import { parseFontMeta, type FontMeta } from '@/utils/font-meta'
import { GLYPHS } from '@/utils/font-samples'
import LayoutPreview from '@/components/layout-preview.vue'

const route = useRoute()
const { t } = useI18n()

defineOptions({
    name: 'FontSupport',
})

/** 注册到 document.fonts 的名字，与窗口内其它字体互不干扰 */
const FONT_FAMILY = 'QLPreviewFont'

/** 字号阶梯相对「最大档」的比例：48 → 12 / 16 / 20 / 28 / 36 / 48 */
const RUNG_RATIOS = [0.25, 1 / 3, 5 / 12, 7 / 12, 0.75, 1]
const SIZE_MIN = 32
const SIZE_MAX = 72
const SIZE_STEP = 8

const fileInfo = ref<FileInfo>()
const fontMeta = ref<FontMeta>()
const family = ref('')
const error = ref('')
const sampleText = ref('')
const baseSize = ref(48)
const inverted = ref(false)
const copied = ref(false)

let curFont: FontFace | null = null
let disposed = false
let copyTimer = 0

/** 文件名去掉扩展名，解析不出元信息时的兜底显示 */
const fileName = computed(() => (fileInfo.value?.name ?? '').replace(/\.[^.\\/]+$/, ''))

const displayName = computed(() => {
    const meta = fontMeta.value
    if (!meta) return fileName.value || t('preview.notSupportedShort')
    return [meta.family, meta.style].filter(Boolean).join(' ')
})

/** 副标题：英文家族名 · 轮廓格式，可变字体再追加轴信息 */
const subtitle = computed(() => {
    const meta = fontMeta.value
    if (!meta) return []
    return [meta.familyEn, meta.outlines].filter(Boolean)
})

const variableTag = computed(() => {
    const axes = fontMeta.value?.axes ?? []
    if (!axes.length) return ''
    const parts = axes.slice(0, 2).map(axis => `${axis.tag} ${axis.min}–${axis.max}`)
    return `${t('font.variable')} · ${parts.join(' ')}`
})

const summary = computed(() => {
    const meta = fontMeta.value
    if (!meta) return ''
    const parts: string[] = []
    if (meta.version) parts.push(`v${meta.version}`)
    if (meta.glyphCount) parts.push(t('font.glyphCount', { count: meta.glyphCount.toLocaleString() }))
    return parts.join(' · ')
})

/** 阶梯按基准字号等比缩放，标签显示的是取整后的真实像素值 */
const rungs = computed(() => [...new Set(RUNG_RATIOS.map(ratio => Math.round(baseSize.value * ratio)))])

function zoom(delta: number) {
    baseSize.value = Math.min(SIZE_MAX, Math.max(SIZE_MIN, baseSize.value + delta))
}

async function copyName() {
    try {
        await navigator.clipboard.writeText(fontMeta.value?.fullName || displayName.value)
        copied.value = true
        window.clearTimeout(copyTimer)
        copyTimer = window.setTimeout(() => (copied.value = false), 1600)
    } catch (e) {
        ElMessage.error(t('font.copyFailed', { error: String(e) }))
    }
}

function openByDefault() {
    const path = fileInfo.value?.path
    if (path) openPath(path)
}

/** 元信息读不出不算失败——woff/woff2 的表是压缩存放的，回退到文件名即可 */
async function loadMeta(path: string) {
    try {
        const bytes = await readFile(path)
        if (disposed) return
        fontMeta.value = parseFontMeta(bytes) ?? undefined
    } catch (e) {
        console.warn('Font metadata unavailable:', e)
    }
}

async function loadFont(path: string) {
    error.value = ''
    try {
        const font = await new FontFace(FONT_FAMILY, `url(${convertFileSrc(path)})`).load()
        if (disposed) return

        if (curFont) document.fonts.delete(curFont)
        curFont = font
        document.fonts.add(font)
        family.value = `${FONT_FAMILY}, sans-serif`
    } catch (e) {
        if (disposed) return
        error.value = e instanceof Error ? e.message : String(e)
        console.error('Font could not be loaded:', e)
    }
}

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    sampleText.value = t('font.defaultSample')

    const path = fileInfo.value.path
    if (!path) {
        error.value = t('book.noPath')
        return
    }

    // 两个任务互不依赖：元信息失败不影响渲染，渲染失败也不影响元信息展示
    await Promise.all([loadMeta(path), loadFont(path)])
})

onBeforeUnmount(() => {
    disposed = true
    window.clearTimeout(copyTimer)
    if (curFont) {
        document.fonts.delete(curFont)
        curFont = null
    }
})
</script>

<template>
    <LayoutPreview :file="fileInfo" :hide-toolbar="!!error">
        <template #toolbar>
            <label class="pv-tb-input ft-sample">
                <input
                    v-model="sampleText"
                    type="text"
                    :placeholder="t('font.samplePlaceholder')"
                    spellcheck="false"
                    @keydown.stop
                />
                <button
                    v-if="sampleText"
                    class="pv-clear"
                    type="button"
                    :title="t('common.cancel')"
                    @click="sampleText = ''"
                >
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" class="is-xs">
                        <path d="M4 4l8 8M12 4l-8 8" />
                    </svg>
                </button>
            </label>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('font.fontSmaller')"
                :disabled="baseSize <= SIZE_MIN"
                @click="zoom(-SIZE_STEP)"
            >
                A−
            </button>
            <span class="pv-tb-num">{{ baseSize }}</span>
            <button
                class="pv-tb-btn is-text"
                type="button"
                :title="t('font.fontLarger')"
                :disabled="baseSize >= SIZE_MAX"
                @click="zoom(SIZE_STEP)"
            >
                A+
            </button>

            <i class="pv-tb-sep"></i>

            <button
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="inverted"
                :title="t('font.invertHint')"
                @click="inverted = !inverted"
            >
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                    <circle cx="8" cy="8" r="5.6" />
                    <path d="M8 2.4v11.2a5.6 5.6 0 0 0 0-11.2z" fill="currentColor" stroke="none" />
                </svg>
                {{ t('font.invert') }}
            </button>

            <div class="pv-tb-grow"></div>

            <span v-if="summary" class="pv-tb-info">{{ summary }}</span>
            <i class="pv-tb-sep"></i>
            <button class="pv-tb-btn is-text" type="button" :title="t('font.copyName')" @click="copyName">
                <el-icon :size="15"><Copy16Regular /></el-icon>
                {{ copied ? t('font.copied') : t('font.copyName') }}
            </button>
        </template>

        <div class="ft-wrap">
            <div class="ft-card pv-scroll" :class="{ 'is-invert': inverted }" :style="{ '--ft-font': family }">
                <!-- 错态：字体文件本身解不开，命令栏的操作全部失去意义 -->
                <div v-if="error" class="ft-error">
                    <svg class="ft-error-icon" viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 8h24v32H12z" />
                        <path d="M18 30V18m12 12V18M18 18h12" />
                        <path d="M8 8h4M8 16h4M8 24h4M8 32h4M8 40h4" stroke-dasharray="1 3" />
                    </svg>
                    <h3>{{ t('font.parseError') }}</h3>
                    <p>{{ error }}</p>
                    <button class="ft-primary" type="button" @click="openByDefault">
                        {{ t('header.openWithDefault') }}
                    </button>
                </div>

                <div v-else class="ft-inner">
                    <h2 class="ft-name">{{ displayName }}</h2>

                    <div class="ft-sub">
                        <template v-for="(item, index) in subtitle" :key="item">
                            <i v-if="index" class="ft-dot"></i>
                            <span>{{ item }}</span>
                        </template>
                        <span v-if="variableTag" class="ft-tag">{{ variableTag }}</span>
                    </div>

                    <div class="ft-scale">
                        <div v-for="size in rungs" :key="size" class="ft-rung">
                            <span class="ft-rung-label">{{ size }}</span>
                            <span class="ft-rung-text" :style="{ fontSize: `${size}px` }">{{ sampleText }}</span>
                        </div>
                    </div>

                    <div class="ft-grid-head">{{ t('font.glyphs') }}</div>
                    <div class="ft-grid">
                        <span v-for="glyph in GLYPHS" :key="glyph">{{ glyph }}</span>
                    </div>
                </div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.ft-sample {
    width: 320px;
    max-width: none;
}

.ft-wrap {
    width: 100%;
    height: 100%;
    padding: 2px 12px 12px;
    box-sizing: border-box;
}

/* 样张纸：内容区显式给一层底色，反色才有明确的作用对象 */
.ft-card {
    --ft-fg: var(--color-text-primary);
    --ft-fg-2: var(--color-text-secondary);
    --ft-fg-3: var(--color-text-disabled);
    --ft-line: var(--color-divider);

    height: 100%;
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow:
        0 1px 2px rgba(0, 0, 0, 0.06),
        0 8px 24px rgba(0, 0, 0, 0.05);
}

.ft-card.is-invert {
    --ft-fg: #f5f5f5;
    --ft-fg-2: #b4b4b4;
    --ft-fg-3: #7d7d7d;
    --ft-line: rgba(255, 255, 255, 0.1);

    background: #111;
}

.ft-inner {
    padding: 26px 32px 36px;
    color: var(--ft-fg);
}

/* 大字家族名用字体本身渲染，这是样张的第一眼 */
.ft-name {
    margin: 0;
    font-family: var(--ft-font, inherit);
    font-size: var(--font-4xl);
    /* 不做合成加粗：文件是哪个字重就显示哪个字重 */
    font-weight: 400;
    line-height: 1.2;
    letter-spacing: -0.01em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.ft-sub {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 26px;
    font-size: var(--font-xs);
    color: var(--ft-fg-3);
    white-space: nowrap;
    overflow: hidden;
}

.ft-dot {
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: currentColor;
    flex: none;
}

.ft-tag {
    height: 18px;
    padding: 0 7px;
    display: inline-flex;
    align-items: center;
    border-radius: 9px;
    background: var(--color-chip-bg);
    color: var(--color-text-secondary);
    font-size: var(--font-2xs);
    font-weight: 600;
    flex: none;
}

.ft-scale {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 20px;
    border-top: 1px solid var(--ft-line);
}

.ft-rung {
    display: grid;
    grid-template-columns: 34px 1fr;
    gap: 14px;
    align-items: baseline;
}

.ft-rung-label {
    font-size: var(--font-2xs);
    color: var(--ft-fg-3);
    font-variant-numeric: tabular-nums;
    text-align: right;
}

.ft-rung-text {
    font-family: var(--ft-font, inherit);
    color: var(--ft-fg);
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.ft-grid-head {
    margin: 28px 0 10px;
    font-size: var(--font-2xs);
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ft-fg-3);
}

.ft-grid {
    display: grid;
    grid-template-columns: repeat(12, 1fr);
    gap: 1px;
    border: 1px solid var(--ft-line);
    border-radius: var(--radius-sm);
    background: var(--ft-line);
    overflow: hidden;

    span {
        display: grid;
        place-items: center;
        aspect-ratio: 1.6;
        background: var(--color-surface);
        font-family: var(--ft-font, inherit);
        font-size: var(--font-2xl);
        color: var(--ft-fg);
        transition:
            background-color var(--transition-fast),
            color var(--transition-fast);

        &:hover {
            background: var(--color-accent-soft);
            color: var(--color-brand-fg);
        }
    }
}

.is-invert .ft-grid span {
    background: #111;
}

/* ── 错态 ── */
.ft-error {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
    color: var(--ft-fg);

    &-icon {
        width: 44px;
        height: 44px;
        color: var(--ft-fg-3);
    }

    h3 {
        margin: 2px 0 0;
        font-size: var(--font-lg);
        font-weight: 600;
    }

    p {
        margin: 0;
        max-width: 460px;
        font-size: var(--font-sm);
        line-height: 1.55;
        color: var(--ft-fg-3);
        word-break: break-word;
    }
}

.ft-primary {
    margin-top: 8px;
    height: 30px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
    font-family: inherit;
    font-size: var(--font-sm);
    cursor: pointer;

    &:hover {
        background: var(--btn-primary-hover-bg);
    }

    &:active {
        background: var(--btn-primary-active-bg);
    }
}
</style>

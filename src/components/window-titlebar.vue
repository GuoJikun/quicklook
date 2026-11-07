<script lang="ts" setup>
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import {
    Dismiss16Regular,
    Square16Regular,
    SquareMultiple16Regular,
    Subtract16Regular,
} from '@vicons/fluent'
import { isMaximized, useWindow } from '@/hooks/use-window'

type TitlebarControl = 'minimize' | 'maximize' | 'close'

interface WindowTitlebarProps {
    /** 左侧那段字。页面型窗口填窗口名；对话框填唯一标题（配合 variant="dialog"） */
    title?: string
    /**
     * window：页面型窗口的身份标签（12px 次要色），标题不参与内容层级；
     * dialog：对话框唯一标题（16/600 主色），与窗口控制同处一行，
     *         下方内容不再重复一个大标题
     */
    variant?: 'window' | 'dialog'
    /** 只画需要的那几颗钮：固定尺寸的对话框给 close 即可 */
    controls?: TitlebarControl[]
}

const props = withDefaults(defineProps<WindowTitlebarProps>(), {
    title: '',
    variant: 'window',
    controls: () => ['minimize', 'maximize', 'close'],
})

const { t } = useI18n()
const { handleClose, handleMin, handleMax, watchMaximized } = useWindow()

const has = (control: TitlebarControl) => props.controls.includes(control)

/* 只有带最大化钮的窗口才查状态：对话框查询会白白多一次 IPC */
onMounted(() => {
    if (has('maximize')) void watchMaximized()
})

const maxLabel = computed(() => (isMaximized.value ? t('header.restore') : t('header.maximize')))
</script>

<template>
    <!-- 自绘标题栏：decorations(false) 的窗口没有系统标题区，拖拽、双击最大化、
         窗口控制全部由这一条承担。底色透明，随宿主窗口的 html.dark 自动明暗。 -->
    <header class="wtb no-selected" :class="`is-${variant}`" data-tauri-drag-region>
        <span v-if="title" class="wtb-title" data-tauri-drag-region>{{ title }}</span>
        <div class="wtb-actions">
            <button
                v-if="has('minimize')"
                type="button"
                class="wtb-btn"
                :title="t('header.minimize')"
                :aria-label="t('header.minimize')"
                @click="handleMin"
            >
                <Subtract16Regular />
            </button>
            <button
                v-if="has('maximize')"
                type="button"
                class="wtb-btn"
                :title="maxLabel"
                :aria-label="maxLabel"
                @click="handleMax"
            >
                <SquareMultiple16Regular v-if="isMaximized" />
                <Square16Regular v-else />
            </button>
            <button
                v-if="has('close')"
                type="button"
                class="wtb-btn is-close"
                :title="t('header.close')"
                :aria-label="t('header.close')"
                @click="handleClose"
            >
                <Dismiss16Regular />
            </button>
        </div>
    </header>
</template>

<style lang="scss" scoped>
/* 规格与预览窗口的 header.vue 对齐：44 高、透明底、32×32 命中区、4px 圆角。
   两处不共用样式表是因为 header 还含文件信息与预览操作，混在一起反而更难读。 */
.wtb {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: var(--layout-header-h);
    padding-left: var(--space-4);
    background: transparent;
    color: var(--color-text-secondary);
    user-select: none;
}

/* 窗口身份只用 12px 次要色：它标明「这是哪个窗口」，不参与内容层级 */
.wtb-title {
    font-size: var(--font-xs);
    line-height: var(--line-xs);
    color: var(--color-text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
}

/* 对话框形态：这条就是窗口唯一的标题。字号提到 16/600、颜色转主色，
   左缩进跟着内容区一起走 24px —— 标题与下方键值表落在同一条左基准线上，
   比「细标签 + 下方再排一个大标题」少一层，也不会在顶部留下一条空带。 */
.wtb.is-dialog {
    padding-left: var(--space-5);

    .wtb-title {
        font-size: var(--font-lg);
        font-weight: 600;
        line-height: var(--line-lg);
        color: var(--color-text-primary);
    }
}

.wtb-actions {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    height: 100%;
    padding-right: var(--space-1);
}

.wtb-btn {
    appearance: none;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-md);
    height: var(--control-md);
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    cursor: pointer;
    transition:
        background-color var(--transition-fast),
        color var(--transition-fast);

    &:hover {
        background-color: var(--elevation-hover);
        color: var(--color-text-primary);
    }

    &:active {
        background-color: var(--elevation-pressed);
    }

    /* 关闭钮沿用 Windows 标题栏惯例：实心红底白图标 */
    &.is-close {
        margin-left: 2px;

        &:hover {
            background-color: var(--color-danger);
            color: var(--btn-primary-text);
        }

        &:active {
            background-color: var(--color-danger-hover);
            color: var(--btn-primary-text);
        }
    }

    :deep(svg) {
        width: var(--icon-sm);
        height: var(--icon-sm);
        pointer-events: none;
    }
}
</style>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useElementSize, useEventListener, useThrottleFn } from '@vueuse/core'
import { getCurrentWindow } from '@tauri-apps/api/window'

const { t } = useI18n()

const props = defineProps<{
    src: string
}>()

const emit = defineEmits<{
    (e: 'fullscreen-change', value: boolean): void
}>()

const containerRef = ref<HTMLDivElement | null>(null)
const imgRef = ref<HTMLImageElement | null>(null)

const MIN_SCALE = 0.05
const MAX_SCALE = 8
const ZOOM_STEP = 1.2

const scale = ref(1)
// 顺时针角度，取值 0 / 90 / 180 / 270
const rotate = ref(0)
// 平移量单位为屏幕像素（transform 中位于 scale / rotate 左侧，不随缩放变化）
const translate = ref({ x: 0, y: 0 })
const mode = ref<'fit' | 'actual' | 'manual'>('fit')
const isFullscreen = ref(false)

// 图片原始尺寸
const naturalW = ref(0)
const naturalH = ref(0)

// 容器尺寸（响应式）
const { width: containerW, height: containerH } = useElementSize(containerRef)

/** 旋转 90° / 270° 时视觉宽高互换，用于计算适应比例与拖动边界 */
const visualSize = computed(() => {
    const swapped = rotate.value % 180 !== 0
    return {
        w: swapped ? naturalH.value : naturalW.value,
        h: swapped ? naturalW.value : naturalH.value,
    }
})

const percent = computed(() => Math.round(scale.value * 100))

const transformStyle = computed(
    () =>
        `translate(${translate.value.x}px, ${translate.value.y}px) scale(${scale.value}) rotate(${rotate.value}deg)`
)

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

/**
 * 适应窗口所需的缩放比例。
 * 上限取 1，与 macOS Quick Look 一致：小于窗口的图片保持原尺寸居中，不做放大以免发虚。
 */
function fitScale(): number {
    const { w, h } = visualSize.value
    const cw = containerW.value
    const ch = containerH.value
    if (!w || !h || !cw || !ch) return 1
    return Math.min(cw / w, ch / h, 1)
}

// 约束 translate 使图片不脱离容器
function clampTranslate(nextX?: number, nextY?: number) {
    const { w, h } = visualSize.value
    const cw = containerW.value
    const ch = containerH.value

    if (!w || !h || !cw || !ch || !scale.value) {
        // 基础信息不足时只设置候选值
        if (typeof nextX === 'number') translate.value.x = nextX
        if (typeof nextY === 'number') translate.value.y = nextY
        return
    }

    const dw = w * scale.value
    const dh = h * scale.value

    // 图大于容器：允许拖到边缘；图小于容器：允许在空隙内移动，但不得出框
    const maxTx = dw > cw ? (dw - cw) / 2 : (cw - dw) / 2
    const maxTy = dh > ch ? (dh - ch) / 2 : (ch - dh) / 2

    translate.value.x = clamp(typeof nextX === 'number' ? nextX : translate.value.x, -maxTx, maxTx)
    translate.value.y = clamp(typeof nextY === 'number' ? nextY : translate.value.y, -maxTy, maxTy)
}

/** 适应窗口：铺满可用区域并回到居中 */
function applyFit() {
    scale.value = clamp(fitScale(), MIN_SCALE, MAX_SCALE)
    translate.value = { x: 0, y: 0 }
    mode.value = 'fit'
}

/** 实际大小：1 图像像素对应 1 CSS 像素 */
function setActualSize() {
    scale.value = clamp(1, MIN_SCALE, MAX_SCALE)
    translate.value = { x: 0, y: 0 }
    mode.value = 'actual'
}

function zoomBy(ratio: number) {
    scale.value = clamp(scale.value * ratio, MIN_SCALE, MAX_SCALE)
    mode.value = 'manual'
    clampTranslate()
}

const handleWheel = useThrottleFn((e: WheelEvent) => {
    e.preventDefault()
    zoomBy(e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP)
}, 10)

function rotateBy(deg: number) {
    rotate.value = (rotate.value + deg + 360) % 360
    // 适应模式下旋转会导致比例变化，需要重新计算；其余模式仅收敛平移
    if (mode.value === 'fit') {
        applyFit()
    } else {
        clampTranslate()
    }
}

/** 双击 / 快捷键在「适应窗口」与「实际大小」之间切换 */
function toggleFit() {
    if (mode.value === 'fit') {
        setActualSize()
    } else {
        applyFit()
    }
}

function onImgLoad() {
    if (!imgRef.value) return
    naturalW.value = imgRef.value.naturalWidth || 0
    naturalH.value = imgRef.value.naturalHeight || 0
    // 载入后默认适应窗口，避免大图只显示左上角
    applyFit()
}

const dragging = ref(false)
let startX = 0
let startY = 0
let originX = 0
let originY = 0

function handleMouseDown(e: MouseEvent) {
    if (e.button !== 0) return
    dragging.value = true
    startX = e.clientX
    startY = e.clientY
    originX = translate.value.x
    originY = translate.value.y
    window.addEventListener('mousemove', handleMouseMove)
    window.addEventListener('mouseup', handleMouseUp)
}

function handleMouseMove(e: MouseEvent) {
    if (!dragging.value) return
    clampTranslate(originX + (e.clientX - startX), originY + (e.clientY - startY))
}

function handleMouseUp() {
    dragging.value = false
    window.removeEventListener('mousemove', handleMouseMove)
    window.removeEventListener('mouseup', handleMouseUp)
}

// 容器尺寸变化：适应模式重新计算，其余模式保持比例并收敛平移
watch([containerW, containerH], () => {
    if (mode.value === 'fit') {
        applyFit()
    } else {
        clampTranslate()
    }
})

async function setWindowFullscreen(value: boolean): Promise<boolean> {
    try {
        await getCurrentWindow().setFullscreen(value)
    } catch (error) {
        console.error('切换全屏失败:', error)
        ElMessage.error(t('image.toggleFailed', { error: String(error) }))
        return false
    }
    isFullscreen.value = value
    emit('fullscreen-change', value)
    return true
}

function toggleFullscreen() {
    void setWindowFullscreen(!isFullscreen.value)
}

// 全屏时 Esc 应先退出全屏；需在父级关闭窗口的监听之前拦截（捕获阶段）
useEventListener(
    window,
    'keydown',
    (e: KeyboardEvent) => {
        if (e.key === 'Escape' && isFullscreen.value) {
            e.preventDefault()
            e.stopImmediatePropagation()
            toggleFullscreen()
        }
    },
    { capture: true }
)

onBeforeUnmount(() => {
    handleMouseUp()
    // 离开图片视图时复位全屏，避免下一个预览继承无边框全屏状态
    if (isFullscreen.value) void setWindowFullscreen(false)
})

// 缩放/旋转/全屏的控件已上移到 LayoutPreview 的命令栏，
// 此处只暴露状态与方法，由父级渲染按钮，避免同一套操作在两处各写一遍。
defineExpose({
    percent,
    mode,
    rotate,
    isFullscreen,
    naturalW,
    naturalH,
    zoomBy,
    applyFit,
    setActualSize,
    rotateBy,
    toggleFullscreen,
})
</script>

<template>
    <div ref="containerRef" class="preview-image" @wheel="handleWheel">
        <img
            ref="imgRef"
            :src="props.src"
            alt="img"
            :style="{ transform: transformStyle }"
            @mousedown="handleMouseDown"
            @dblclick="toggleFit"
            @load="onImgLoad"
            draggable="false"
        />
    </div>
</template>

<style scoped lang="scss">
/* 图片本身不加圆角与阴影：缩放到 800% 时圆角会被 transform 一并放大，
   反而失真。留出内边距让图片与命令栏之间保持呼吸即可。 */
.preview-image {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    align-content: center;
    padding: 4px 16px 16px;
    background-color: var(--color-bg);
    position: relative;
    overflow: hidden; // 防止视觉溢出

    img {
        display: block;
        cursor: grab;
        user-select: none;
        transform-origin: center center;
        will-change: transform;
        /* base.css 的 img { max-width: 100% } 会先把图缩到容器宽，再叠乘 transform，
           令命令栏上的百分比与实际显示尺寸不符。这里按原始像素渲染，缩放只由 scale 决定。 */
        max-width: none;
        max-height: none;
    }

    img:active {
        cursor: grabbing;
    }
}
</style>

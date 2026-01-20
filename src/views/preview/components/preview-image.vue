<script setup lang="ts">
import { ref } from 'vue'
import { useElementSize, useThrottleFn } from '@vueuse/core'

const props = defineProps<{
    src: string
}>()

const containerRef = ref<HTMLDivElement | null>(null)
const imgRef = ref<HTMLImageElement | null>(null)

const scale = ref(1)
const minScale = 0.1
const maxScale = 5
const step = 0.1
// 容器尺寸（响应式）
const { width: containerW, height: containerH } = useElementSize(containerRef)

// 图片原始尺寸
const naturalW = ref(0)
const naturalH = ref(0)

function onImgLoad() {
    if (!imgRef.value) return
    // 记录图片自然尺寸
    naturalW.value = imgRef.value.naturalWidth || 0
    naturalH.value = imgRef.value.naturalHeight || 0
    // 载入后做一次边界收敛
    clampTranslate()
}

const handleWheel = useThrottleFn((e: WheelEvent) => {
    e.preventDefault()
    if (e.deltaY < 0) {
        scale.value = Math.min(maxScale, scale.value + step)
    } else {
        scale.value = Math.max(minScale, scale.value - step)
    }
    // 缩放后根据新比例收敛平移，避免越界
    clampTranslate()
}, 10)

const translate = ref({ x: 0, y: 0 })
const dragging = ref(false)
let startX = 0
let startY = 0
let originX = 0
let originY = 0

function handleMouseDown(e: MouseEvent) {
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
    const scaleValue = scale.value
    const nextX = originX + (e.clientX - startX) / scaleValue
    const nextY = originY + (e.clientY - startY) / scaleValue
    clampTranslate(nextX, nextY)
}

function handleMouseUp() {
    dragging.value = false
    window.removeEventListener('mousemove', handleMouseMove)
    window.removeEventListener('mouseup', handleMouseUp)
}

// 约束 translate 使图片不能拖出容器
function clampTranslate(nextX?: number, nextY?: number) {
    const s = scale.value
    const cw = containerW.value || 0
    const ch = containerH.value || 0
    const iw0 = naturalW.value || 0
    const ih0 = naturalH.value || 0
    if (!cw || !ch || !iw0 || !ih0 || !s) {
        // 基础信息不足时只设置候选值
        if (typeof nextX === 'number') translate.value.x = nextX
        if (typeof nextY === 'number') translate.value.y = nextY
        return
    }

    const iw = iw0 * s
    const ih = ih0 * s

    // 针对 transform: scale(s) translate(tx, ty)
    // 有效屏幕位移为 (tx * s, ty * s)，允许的最大平移（以 tx/ty 表示）为 (尺寸差/2)/s
    let maxTx = 0
    let maxTy = 0

    if (iw > cw) {
        // 图大于容器：允许在超出部分的一半内拖动
        maxTx = (iw - cw) / 2 / s
    } else {
        // 图小于容器：允许在容器“空隙”的一半内拖动，但不得出框
        maxTx = (cw - iw) / 2 / s
    }

    if (ih > ch) {
        maxTy = (ih - ch) / 2 / s
    } else {
        maxTy = (ch - ih) / 2 / s
    }

    const tx = typeof nextX === 'number' ? nextX : translate.value.x
    const ty = typeof nextY === 'number' ? nextY : translate.value.y

    // clamp
    const clampedX = Math.max(-maxTx, Math.min(maxTx, tx))
    const clampedY = Math.max(-maxTy, Math.min(maxTy, ty))

    translate.value.x = clampedX
    translate.value.y = clampedY
}
</script>

<template>
    <div ref="containerRef" class="preview-image" @wheel="handleWheel">
        <img
            ref="imgRef"
            :src="props.src"
            alt="img"
            :style="{ transform: `scale(${scale}) translate(${translate.x}px, ${translate.y}px)` }"
            @mousedown="handleMouseDown"
            @load="onImgLoad"
            draggable="false"
        />
    </div>
</template>

<style scoped lang="scss">
.preview-image {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    align-content: center;
    background-color: var(--color-bg);
    position: relative;
    overflow: hidden; // 防止视觉溢出

    img {
        display: block;
        cursor: grab;
        user-select: none;
    }

    img:active {
        cursor: grabbing;
    }
}
</style>

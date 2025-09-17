<script setup lang="ts">
import { ref } from 'vue'
import { useThrottleFn } from '@vueuse/core'

const props = defineProps<{
    src: string
}>()

const scale = ref(1)
const minScale = 0.1
const maxScale = 5
const step = 0.1
const handleWheel = useThrottleFn((e: WheelEvent) => {
    e.preventDefault()
    if (e.deltaY < 0) {
        scale.value = Math.min(maxScale, scale.value + step)
    } else {
        scale.value = Math.max(minScale, scale.value - step)
    }
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
    translate.value.x = originX + (e.clientX - startX) / scaleValue
    translate.value.y = originY + (e.clientY - startY) / scaleValue
}

function handleMouseUp() {
    dragging.value = false
    window.removeEventListener('mousemove', handleMouseMove)
    window.removeEventListener('mouseup', handleMouseUp)
}
</script>

<template>
    <div class="preview-image" @wheel="handleWheel">
        <img
            :src="props.src"
            alt="img"
            :style="{ transform: `scale(${scale}) translate(${translate.x}px, ${translate.y}px)` }"
            @mousedown="handleMouseDown"
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

    img {
        display: block;
    }
}
</style>

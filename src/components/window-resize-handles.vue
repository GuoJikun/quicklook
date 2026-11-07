<script lang="ts" setup>
import { onMounted } from 'vue'
import { isMaximized, useWindow, type WindowResizeDirection } from '@/hooks/use-window'

const { startResize, watchMaximized } = useWindow()

/** 8 个方向：四条边 4px、四角 6px，角后渲染压过边，角落才能斜向缩放 */
const HANDLES: { dir: WindowResizeDirection; cls: string }[] = [
    { dir: 'North', cls: 'is-n' },
    { dir: 'South', cls: 'is-s' },
    { dir: 'West', cls: 'is-w' },
    { dir: 'East', cls: 'is-e' },
    { dir: 'NorthWest', cls: 'is-nw' },
    { dir: 'NorthEast', cls: 'is-ne' },
    { dir: 'SouthWest', cls: 'is-sw' },
    { dir: 'SouthEast', cls: 'is-se' },
]

/* 最大化状态下热区要失效，否则拖边缘会先取消最大化再缩放，与系统行为不一致 */
onMounted(() => {
    void watchMaximized()
})
</script>

<template>
    <!-- 无边框窗口没有系统边框可拖，这 8 条透明热区补回缩放能力。
         容器自身不吃事件（pointer-events: none），只有条带可点。 -->
    <div class="wrh" :class="{ 'is-static': isMaximized }" aria-hidden="true">
        <div
            v-for="handle in HANDLES"
            :key="handle.dir"
            class="wrh-item"
            :class="handle.cls"
            @mousedown.prevent="startResize(handle.dir)"
        />
    </div>
</template>

<style lang="scss" scoped>
.wrh {
    position: absolute;
    inset: 0;
    z-index: 20;
    pointer-events: none;

    &.is-static .wrh-item {
        pointer-events: none;
    }
}

.wrh-item {
    position: absolute;
    pointer-events: auto;

    &.is-n {
        top: 0;
        left: 6px;
        right: 6px;
        height: 4px;
        cursor: ns-resize;
    }

    &.is-s {
        bottom: 0;
        left: 6px;
        right: 6px;
        height: 4px;
        cursor: ns-resize;
    }

    &.is-w {
        left: 0;
        top: 6px;
        bottom: 6px;
        width: 4px;
        cursor: ew-resize;
    }

    &.is-e {
        right: 0;
        top: 6px;
        bottom: 6px;
        width: 4px;
        cursor: ew-resize;
    }

    &.is-nw {
        top: 0;
        left: 0;
        width: 6px;
        height: 6px;
        cursor: nwse-resize;
    }

    &.is-ne {
        top: 0;
        right: 0;
        width: 6px;
        height: 6px;
        cursor: nesw-resize;
    }

    &.is-sw {
        bottom: 0;
        left: 0;
        width: 6px;
        height: 6px;
        cursor: nesw-resize;
    }

    &.is-se {
        bottom: 0;
        right: 0;
        width: 6px;
        height: 6px;
        cursor: nwse-resize;
    }
}
</style>

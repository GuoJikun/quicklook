<script setup lang="ts">
import LayoutPreview from '@/components/layout-preview.vue'

defineOptions({
    name: 'NotSupport',
})
</script>

<template>
    <LayoutPreview>
        <div class="not-support">
            <div class="font-support-inner">暂不支持此格式的预览</div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.not-support {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
}
</style>

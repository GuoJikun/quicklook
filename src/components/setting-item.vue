<script lang="ts" setup>
const props = defineProps<{
    title: string
    content?: string
}>()
</script>

<template>
    <div class="setting-item">
        <div class="setting-item-header">
            <span>{{ props.title }}：</span>
        </div>
        <div class="setting-item-content">
            <slot>{{ props.content }}</slot>
        </div>
    </div>
</template>

<style scoped lang="scss">
.setting-item {
    line-height: 1.6em;
    padding: 8px 0;
    &-header {
        font-size: 16px;
        font-weight: 700;
        margin-bottom: 4px;
        color: #555;
    }
    &-content {
        padding: 0 12px;
    }
}
</style>

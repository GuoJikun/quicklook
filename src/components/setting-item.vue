<script lang="ts" setup>
import type { Component } from 'vue'

defineProps<{
    id?: string
    title: string
    description?: string
    icon?: Component
}>()
</script>

<template>
    <section :id="id" class="setting-card">
        <header v-if="title || $slots.header" class="setting-card-header">
            <div class="setting-card-title">
                <el-icon v-if="icon" class="setting-card-icon">
                    <component :is="icon" />
                </el-icon>
                <h3 class="setting-card-title-text">{{ title }}</h3>
            </div>
            <p v-if="description" class="setting-card-desc">{{ description }}</p>
            <slot name="header" />
        </header>
        <div class="setting-card-body">
            <slot />
        </div>
    </section>
</template>

<style scoped lang="scss">
.setting-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    padding: 18px 22px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.02);
    scroll-margin-top: 72px;

    &-header {
        margin-bottom: 14px;
        padding-bottom: 12px;
        border-bottom: 1px solid var(--color-border);
    }

    &-title {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 0;
    }

    &-title-text {
        margin: 0;
        font-size: 15px;
        font-weight: 600;
        color: var(--color-text-primary);
    }

    &-icon {
        color: var(--color-accent);
        font-size: 18px;
    }

    &-desc {
        margin: 6px 0 0;
        font-size: 13px;
        line-height: 1.6;
        color: var(--color-text-secondary);
    }

    &-body {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }
}
</style>

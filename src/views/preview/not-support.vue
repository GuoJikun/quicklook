<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import LayoutPreview from '@/components/layout-preview.vue'

const { t } = useI18n()

defineOptions({
    name: 'NotSupport',
})
</script>

<template>
    <LayoutPreview>
        <div class="not-support">
            <div class="font-support-inner">{{ t('preview.notSupported') }}</div>
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

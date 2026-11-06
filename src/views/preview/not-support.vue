<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { DocumentQuestionMark24Regular } from '@vicons/fluent'
import type { FileInfo } from '@/utils/typescript'
import { formatBytes } from '@/utils'
import LayoutPreview from '@/components/layout-preview.vue'

const route = useRoute()
const { t } = useI18n()

defineOptions({
    name: 'NotSupport',
})

const fileInfo = ref<FileInfo>()

/** 副标题：「.XYZ · 2.4 MB」，缺哪项就省哪项 */
const detail = computed(() => {
    const parts: string[] = []
    const ext = fileInfo.value?.extension?.replace(/^\./, '')
    if (ext) parts.push(`.${ext.toUpperCase()}`)
    const size = Number(fileInfo.value?.size)
    if (Number.isFinite(size) && size > 0) parts.push(formatBytes(size))
    return parts.join(' · ')
})

onMounted(() => {
    fileInfo.value = route.query as unknown as FileInfo
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="ns-wrap">
            <div class="ns-card">
                <el-icon :size="48" class="ns-icon"><DocumentQuestionMark24Regular /></el-icon>
                <h2 class="ns-title">{{ t('preview.notSupported') }}</h2>
                <p v-if="detail" class="ns-detail">{{ detail }}</p>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
/* 不支持的类型没有命令栏——不要为了「结构完整」留一条空栏（设计 §5.9） */
.ns-wrap {
    width: 100%;
    height: 100%;
    padding: var(--space-5);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
}

/* 卡片不加重投影，靠 --color-surface 与窗口底色区分。
   宽度撑满再被 max-width 收住，窄窗口下才不会顶出可视区。
   卡片内不放按钮：打开文件、打开所在文件夹在标题行已经有同样的入口，
   这里再放一遍是重复，也会把「仅仅说明状态」的页面变成操作面板。 */
.ns-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    box-sizing: border-box;
    width: 100%;
    max-width: 420px;
    padding: 34px 24px 30px;
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    text-align: center;
}

.ns-icon {
    color: var(--color-text-tertiary);
}

.ns-title {
    margin: 12px 0 0;
    font-size: var(--font-lg);
    font-weight: 600;
    line-height: var(--line-lg);
    color: var(--color-text-primary);
}

.ns-detail {
    margin: 5px 0 0;
    font-size: var(--font-sm);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
}
</style>

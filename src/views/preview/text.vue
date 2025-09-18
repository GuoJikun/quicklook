<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { readTextFile } from '@/utils'

const route = useRoute()

defineOptions({
    name: 'TextSupport',
})

const fileInfo = ref<FileInfo>()
const content = ref<string>()

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    const path = fileInfo.value.path as string

    const txt = await readTextFile(path)
    content.value = txt
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="text-support">
            <div class="text-support-inner">
                <pre>{{ content }}</pre>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.text-support {
    width: 100%;
    height: 100%;
    display: flex;
    &-inner {
        width: 100%;
        height: 100%;
        background-color: var(--color-bg);
        overflow: auto;
        padding: 12px 24px;
        font-size: 14px;
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
        & pre {
            white-space: pre-wrap;
            word-wrap: break-word;
            font-family: inherit;
            font-size: inherit;
            background-color: transparent;
            color: var(--color-text-primary);
        }
    }
}
</style>

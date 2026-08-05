<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { readTextFile } from '@/utils'
import { createMd } from '@/utils/markdown/index'

import MdViewer from '@/components/md-viewer/index.vue'
import type MarkdownIt from 'markdown-it'

const route = useRoute()

let md: MarkdownIt | null = null

defineOptions({
    name: 'MdSupport',
})
const file = ref<FileInfo>()
const content = ref<string>()
const loading = ref<boolean>(true)
const error = ref<string>('')

onMounted(async () => {
    loading.value = true
    error.value = ''
    file.value = route?.query as unknown as FileInfo
    const path = file.value.path as string

    try {
        if (md === null) {
            md = await createMd()
        }
    } catch (e) {
        error.value = `创建md解析器失败：${e instanceof Error ? e.message : String(e)}`
        loading.value = false
        return
    }

    try {
        const txt = await readTextFile(path)
        content.value = (md as MarkdownIt).render(txt)
    } catch (e) {
        error.value = `读取md文件失败：${e instanceof Error ? e.message : String(e)}`
    } finally {
        loading.value = false
    }
})
</script>

<template>
    <LayoutPreview :file="file" :loading="loading">
        <div class="md-support">
            <div v-if="error" class="md-error">
                <p class="md-error-title">文件加载失败</p>
                <p class="md-error-detail">{{ error }}</p>
            </div>
            <div v-else class="md-support-inner" id="markdown-body">
                <MdViewer :key="file?.path" :content="content" />
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.md-support {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    align-content: center;
    &-inner {
        width: 100%;
        height: 100%;
        overflow: hidden auto;
        padding: 12px 24px;
        font-size: 14px;
    }
}

.md-error {
    text-align: center;
    color: var(--color-text-secondary);
    padding: var(--space-5);

    &-title {
        font-size: var(--font-lg);
        color: var(--color-danger);
        margin-bottom: var(--space-2);
    }

    &-detail {
        font-size: var(--font-sm);
        word-break: break-all;
    }
}
</style>

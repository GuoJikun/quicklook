<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { readFile } from '@tauri-apps/plugin-fs'

import Excel from '@/components/document/excel.vue'
import PdfViewer from './components/pdf-viewer.vue'
import { renderAsync } from 'docx-preview'

const route = useRoute()

defineOptions({
    name: 'DocumentSupport',
})

interface Sheet {
    name: string
    rows: string[][]
}

enum DocType {
    Excel = 'Excel',
    Docx = 'Docx',
    Pdf = 'Pdf',
    Pptx = 'Pptx',
}

interface Docs {
    Excel?: Array<Sheet>
    Docx?: string
}

const loading = ref(true)
const fileInfo = ref<FileInfo>()
const content = ref<Docs[keyof Docs]>()
const type = ref<DocType>()

const docsRef = ref<HTMLElement>()
const docsStyleRef = ref<HTMLElement>()

const getDocxContent = async (path: string) => {
    const buffer = await readFile(path)
    const docs = await renderAsync(buffer, docsRef.value as HTMLElement, docsStyleRef.value)

    return docs as string
}

onMounted(async () => {
    loading.value = true
    fileInfo.value = route?.query as unknown as FileInfo

    // PDF 直接走 PdfViewer 组件
    if (fileInfo.value.extension === 'pdf') {
        type.value = DocType.Pdf
        loading.value = false
        return
    }

    const val = fileInfo.value.path as string
    const docs: Docs = await invoke('document', { path: val, mode: fileInfo.value.extension })
    type.value = docs.Excel ? DocType.Excel : docs.Docx ? DocType.Docx : DocType.Pptx
    console.log(docs);
    switch (type.value) {
        case DocType.Excel:
            content.value = docs.Excel as Array<Sheet>
            break
        case DocType.Docx:
            content.value = await getDocxContent(docs.Docx as string)
            break
        case DocType.Pptx:
            content.value = '' as string
    }
    loading.value = false
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="text-support">
            <div class="text-support-inner" v-loading="loading">
                <PdfViewer v-if="type === DocType.Pdf" :path="fileInfo!.path" />
                <Excel v-else-if="type === DocType.Excel" :data="content as Array<Sheet>" />
                <div v-else-if="type === DocType.Docx">
                    <div ref="docsRef"></div>
                    <div class="docx-style" ref="docsStyleRef"></div>
                </div>
                <div v-else>暂不支持</div>
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
        overflow: auto;
        padding: 0;
        font-size: 1.4rem;
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
    }
}
</style>

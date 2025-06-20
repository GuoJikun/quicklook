<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { convertFileSrc } from '@tauri-apps/api/core'
import * as PDFJS from 'pdfjs-dist'
import { CollectionTag } from '@element-plus/icons-vue'
import { info } from '@tauri-apps/plugin-log'
import type { PDFDocumentProxy } from 'pdfjs-dist'

const route = useRoute()

defineOptions({
    name: 'BookSupport',
})

const fileInfo = ref<FileInfo>()

const loadDocument = (url: string): Promise<PDFDocumentProxy> => {
    return new Promise((resolve, reject) => {
        PDFJS.getDocument({
            url,
            cMapUrl: '/pdf/cmaps/',
            cMapPacked: true,
        })
            .promise.then((pdf: PDFDocumentProxy) => {
                resolve(pdf)
            })
            .catch(e => {
                reject(e)
            })
    })
}

const getMeta = async (pdf: PDFDocumentProxy) => {
    const outline = await pdf.getOutline()
    const meta = await pdf.getMetadata()
    const count = pdf.numPages
    return {
        outline,
        meta,
        count,
    }
}
const visible = ref(false)
const showOutline = () => {
    visible.value = !visible.value
}

// 通过大纲跳转到对应页码
const jumpByOutline = async (ev: any) => {
    console.log('jumpByOutline', ev)
    const dest = ev.dest
    console.log('dest', dest)
    if (!dest && dest.length === 0) {
        console.warn('No destination found in outline item')
        return
    }

    if (typeof dest === 'string') {
        // 如果是字符串，直接跳转到对应的页码
        const pageIndex = parseInt(dest, 10) - 1
        pager.value.current = pageIndex + 1
        renderPage(pdf as PDFDocumentProxy, pager.value.current)
        return
    }

    // 如果是数组，获取第一个元素作为目标
    if (Array.isArray(dest)) {
        const ref = dest[0]
        const pageIndex = (await pdf?.getPageIndex(ref)) ?? 0
        pager.value.current = pageIndex + 1
        renderPage(pdf as PDFDocumentProxy, pager.value.current)
    }
}

const renderPage = (pdf: PDFDocumentProxy, num: number) => {
    pdf.getPage(num).then(page => {
        page.cleanup()
        const context = canvasRef.value?.getContext('2d')
        const viewport = page.getViewport({ scale: 1 })
        ;(canvasRef.value as HTMLCanvasElement).height = viewport.height
        ;(canvasRef.value as HTMLCanvasElement).width = viewport.width
        page.render({
            canvasContext: context as CanvasRenderingContext2D,
            viewport,
        })
    })
}

const handlePrev = (pdf: any) => {
    if (pager.value.current <= 1) {
        return
    }
    pager.value.current--
    renderPage(pdf, pager.value.current)
}
const handleNext = (pdf: any) => {
    if (pager.value.current >= pdf.numPages) {
        return
    }
    pager.value.current++
    renderPage(pdf, pager.value.current)
}

const canvasRef = ref<HTMLCanvasElement>()
const pager = ref<{ current: number; total: number }>({
    current: 1,
    total: 0,
})
const outline = ref<any[]>([])
let pdf: PDFDocumentProxy | null = null
onMounted(async () => {
    PDFJS.GlobalWorkerOptions.workerSrc = '/pdf/pdf.worker.mjs'
    pager.value.current = 1
    fileInfo.value = route?.query as unknown as FileInfo
    const path = convertFileSrc(fileInfo.value.path)
    pdf = await loadDocument(path)
    const meta = await getMeta(pdf)
    console.log('meta', meta)
    pager.value.total = meta.count
    outline.value = meta.outline || []
    if (pdf) {
        renderPage(pdf, pager.value.current)
    } else {
        info(pdf)
        console.error(pdf)
    }
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="book">
            <div class="book-utils">
                <div>
                    <el-link :underline="false" @click="showOutline">
                        <el-icon size="18px">
                            <CollectionTag />
                        </el-icon>
                    </el-link>
                </div>
                <div></div>
                <div></div>
            </div>
            <div class="book-wrap">
                <div class="book-outline" v-if="visible">
                    <el-scrollbar class="scrollbar" :always="false">
                        <el-tree
                            :data="outline"
                            :props="{ label: 'title', children: 'items' }"
                            :highlight-current="true"
                            @node-click="jumpByOutline"
                        >
                        </el-tree>
                    </el-scrollbar>
                </div>
                <div class="book-canvas">
                    <canvas ref="canvasRef" class="canvas"></canvas>
                </div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.book {
    width: 100%;
    height: 100%;
    &-wrap {
        width: 100%;
        height: calc(100% - 40px);
        overflow: auto;
        padding-right: 24px;
        font-size: 14px;
        display: flex;
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
        .canvas {
            display: block;
            margin: 0 auto;
        }
    }
    &-utils {
        display: flex;
        align-items: center;
        width: 100%;
        height: 40px;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
        background-color: #fff;
        padding: 0 24px;
    }
    &-outline {
        width: 240px;
        height: 100%;
        overflow: auto;
        position: relative;
        display: flex;
        flex-direction: column;
        padding: 12px;
        box-shadow: 1px 0 2px rgba(0, 0, 0, 0.1);
        background-color: #f9f9f9;
    }
    &-canvas {
        flex: auto;
        height: calc(100% - 24px);
        position: relative;
        top: 12px;
    }
}
</style>

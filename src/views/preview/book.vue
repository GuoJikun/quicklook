div<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { ref, onMounted, nextTick, useTemplateRef } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { convertFileSrc } from '@tauri-apps/api/core'
import * as PDFJS from 'pdfjs-dist'
import { CollectionTag } from '@element-plus/icons-vue'
import type { PDFDocumentProxy } from 'pdfjs-dist'
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-expect-error
import { RecycleScroller as VirtualList } from 'vue-virtual-scroller'
import 'vue-virtual-scroller/dist/vue-virtual-scroller.css'

const route = useRoute()

defineOptions({
    name: 'BookSupport',
})

const fileInfo = ref<FileInfo>()
const canvasMap = new Map()
const pager = ref<{ current: number; total: number; scale: number }>({
    current: 1,
    total: 0,
    scale: 1,
})
const outlineProps = {
    children: 'items',
    label: 'title',
}
const pageHeight = ref(1000) // 预估值，将根据渲染后更新
const virtualListRef = useTemplateRef('virtualListRef')
const outline = ref<any[]>([])
let pdfDoc: PDFDocumentProxy | null = null

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

const parseOutline = async (outlineList: any[]): Promise<any[]> => {
    const result: any[] = []
    for (const item of outlineList) {
        let pageNumber = 1
        if (item.dest) {
            if (typeof item.dest === 'string') {
                // 如果是字符串，直接跳转到对应的页码
                const pageIndex = parseInt(item.dest, 10) - 1
                pageNumber = pageIndex + 1
            }

            // 如果是数组，获取第一个元素作为目标
            if (Array.isArray(item.dest)) {
                const ref = item.dest[0]
                const pageIndex = (await pdfDoc?.getPageIndex(ref)) ?? 0
                pageNumber = pageIndex + 1
            }
        }
        result.push({
            title: item.title,
            page: pageNumber,
            items: item.items ? await parseOutline(item.items) : [],
        })
    }
    return result
}

const getMeta = async (pdf: PDFDocumentProxy) => {
    const outline = (await pdf.getOutline()) ?? []
    const meta = await pdf.getMetadata()
    const count = pdf.numPages
    return {
        outline: await parseOutline(outline),
        meta,
        count,
    }
}
const visible = ref(false)
const showOutline = () => {
    visible.value = !visible.value
}

const goToPage = (page: number) => {
    if (page < 1 || page > pager.value.total) return
    pager.value.current = page
    scrollToPage()
}

// 通过大纲跳转到对应页码
const jumpByOutline = async (ev: any) => {
    const dest = ev.page
    if (!dest) {
        console.warn('No destination found in outline item')
        return
    }
    goToPage(ev.page)
}

const registerCanvas = (pageNum: number, el: HTMLCanvasElement | null) => {
    if (el && !canvasMap.has(pageNum)) {
        canvasMap.set(pageNum, el)
        renderPage(pageNum)
    }
}

const renderPage = (pageNum: number) => {
    const canvas = canvasMap.get(pageNum)
    if (!canvas || !pdfDoc) return
    pdfDoc.getPage(pageNum).then(page => {
        page.cleanup()
        const context = canvas?.getContext('2d')
        const viewport = page.getViewport({ scale: pager.value.scale, offsetX: 0, offsetY: 0 })
        ;(canvas as HTMLCanvasElement).height = viewport.height
        ;(canvas as HTMLCanvasElement).width = viewport.width
        canvas.style.height = viewport.height + 'px'
        canvas.style.width = viewport.width + 'px'
        pageHeight.value = viewport.height

        page.render({
            canvasContext: context as CanvasRenderingContext2D,
            viewport,
        })
    })
}

const rerenderVisiblePages = () => {
    canvasMap.forEach((_, pageNum) => renderPage(pageNum))
}

const scrollToPage = () => {
    virtualListRef.value?.scrollToItem(pager.value.current - 1)
}
const list = ref<{ id: number; pageNum: number }[]>([])
onMounted(async () => {
    PDFJS.GlobalWorkerOptions.workerSrc = '/pdf/pdf.worker.mjs'
    pager.value.current = 1
    fileInfo.value = route?.query as unknown as FileInfo
    const path = convertFileSrc(fileInfo.value.path)
    pdfDoc = await loadDocument(path)
    const meta = await getMeta(pdfDoc)
    console.log('meta', meta)
    pager.value.total = meta.count
    list.value = Array.from({ length: pager.value.total }, (_, i) => {
        return {
            id: i + 1,
            pageNum: i + 1,
        }
    })
    outline.value = meta.outline || []

    nextTick(() => rerenderVisiblePages())
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
                <div>
                    <div>{{ pager.current }} / {{ pager.total }}</div>
                </div>
                <div></div>
            </div>
            <el-container class="book-wrap">
                <el-aside class="book-outline" v-if="visible">
                    <el-scrollbar class="scrollbar" :always="false">
                        <el-tree
                            :data="outline"
                            :props="outlineProps"
                            :highlight-current="true"
                            @node-click="jumpByOutline"
                        >
                        </el-tree>
                    </el-scrollbar>
                </el-aside>
                <el-main class="book-canvas">
                    <virtual-list
                        ref="virtualListRef"
                        list-class="custom-scroll-wrap"
                        item-class="custom-scroll-item"
                        :items="list"
                        :item-size="pageHeight"
                        class="pdf-virtual-list"
                        #default="{ item }"
                    >
                        <div>
                            <canvas
                                :ref="el => registerCanvas(item.pageNum, el as HTMLCanvasElement | null)"
                                :style="{ height: pageHeight + 'px' }"
                            />
                        </div>
                    </virtual-list>
                </el-main>
            </el-container>
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
        justify-content: space-between;
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
    .pdf-virtual-list {
        height: 100%;
    }
    :global(.custom-scroll-item) {
        display: flex;
        justify-content: center;
    }
    :global(.custom-scroll-item:not(:first-child)) {
        border-top: 1px solid #eaeaea;
    }
}
</style>

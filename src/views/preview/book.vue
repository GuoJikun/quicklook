div<!-- eslint-disable @typescript-eslint/no-explicit-any -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { convertFileSrc } from '@tauri-apps/api/core'
import * as PDFJS from 'pdfjs-dist'
import { CollectionTag } from '@element-plus/icons-vue'
import type { PDFDocumentProxy } from 'pdfjs-dist'
import { App, Canvas, PropertyEvent, ResizeEvent } from 'leafer-ui'
import '@leafer-in/view' // 导入视口插件
import '@leafer-in/viewport' // 导入视口插件
import '@leafer-in/animate' // 导入画布插件
import '@leafer-in/resize' // 导入画布插件
import { ScrollBar } from '@leafer-in/scroll' // 导入滚动条插件  //
import { useThrottleFn } from '@vueuse/core'

const route = useRoute()

defineOptions({
    name: 'BookSupport',
})

let leaferInstance: InstanceType<typeof App> | null = null
let scrollBarInstance: InstanceType<typeof ScrollBar> | null = null
const fileInfo = ref<FileInfo>()
const pager = ref<{ current: number; total: number; scale: number; rotation: number }>({
    current: 1,
    total: 0,
    scale: 1,
    rotation: 0,
})
const outlineProps = {
    children: 'items',
    label: 'title',
}

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
const visible = ref(true)
const showOutline = () => {
    visible.value = !visible.value
}

const goPage = (pageNum: number) => {
    pager.value.current = pageNum
    const targetNode = leaferInstance?.tree.children.find((_, i) => i + 1 === pageNum)
    const moveY = targetNode?.y ?? 0

    const currentY = leaferInstance?.tree.y ?? 0

    let move = 0

    if (moveY < Math.abs(currentY)) {
        move = currentY == 0 ? moveY : Math.abs(currentY) - moveY
    } else {
        move = Math.abs(currentY) - moveY
    }

    ;(leaferInstance as App).tree.move(0, move, true)
}

const handleNodeClick = (data: any) => {
    const page = data.page

    pageNum.value = page
    goPage(page)
}

const pageNum = ref<number>(1)
const handleJump = () => {
    const page = pageNum.value
    if (page < 1 || page > pager.value.total) {
        pageNum.value = pager.value.current
        return
    }

    goPage(page)
}

const renderPage = async () => {
    if (!pdfDoc) return
    if (!leaferInstance) return

    const width = leaferInstance.tree.width ?? 0

    let offsetY = 0

    for (let pageNum = 1; pageNum <= pager.value.total; pageNum++) {
        const page = await pdfDoc.getPage(pageNum)

        const viewport = page.getViewport({
            scale: pager.value.scale,
            rotation: pager.value.rotation,
        })

        const x = (width - viewport.width) / 2

        const canvasNode = new Canvas({
            x: x,
            y: offsetY,
            width: viewport.width,
            height: viewport.height,
            scale: pager.value.scale,
        })

        await page.render({
            canvasContext: canvasNode.context as CanvasRenderingContext2D,
            viewport,
        }).promise

        canvasNode.paint()
        leaferInstance.tree.add(canvasNode, pageNum)
        offsetY += viewport.height + 16

        page.cleanup()
    }
}

const initLeader = () => {
    if (leaferInstance) return
    leaferInstance = new App({
        view: 'leafer-canvas',
        tree: { type: 'document' }, // 给 tree 层添加视口
        sky: {},
        fill: '#efefef',
        zoom: {
            min: 1,
            max: 10,
        },
    })
    scrollBarInstance = new ScrollBar(leaferInstance as any, {
        theme: 'light',
        minSize: 24,
    })

    leaferInstance.tree.on(
        PropertyEvent.LEAFER_CHANGE,
        useThrottleFn((ev: PropertyEvent) => {
            const { attrName, newValue } = ev
            if (attrName === 'y') {
                const y = Math.abs(Number(newValue) ?? 0)
                const children = (ev.target as any)?.children || []
                for (let i = 1; i <= children.length; i++) {
                    const prevY = children[i - 1].y ?? 0
                    const currentY = children[i].y ?? 0
                    if (y > prevY && y < currentY) {
                        pager.value.current = i
                        pageNum.value = i
                        break
                    }
                }
            }
        }, 10),
    )

    leaferInstance.on(
        ResizeEvent.RESIZE,
        useThrottleFn((ev: ResizeEvent) => {
            console.log('resize', ev)
            leaferInstance?.zoom('fit', 0, 'x')
            scrollBarInstance?.update()
        }, 80),
    )
}

const initPdf = async (src: any) => {
    if (pdfDoc) {
        return
    }
    PDFJS.GlobalWorkerOptions.workerSrc = '/pdf/pdf.worker.mjs'
    pager.value.current = 1
    pdfDoc = await loadDocument(src)

    const meta = await getMeta(pdfDoc)
    pager.value.total = meta.count

    outline.value = meta.outline || []
}

onMounted(async () => {
    initLeader()

    fileInfo.value = route?.query as unknown as FileInfo
    const path = convertFileSrc(fileInfo.value.path)
    await initPdf(path)
    await renderPage()
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
                <div class="book-utils-operation">
                    <!-- <el-button-group>
                        <el-button text :icon="Minus" size="small" @click="handleZoomOut"></el-button>
                        <el-button text :icon="Plus" size="small" @click="handleZoomIn"></el-button>
                    </el-button-group> -->
                    <el-divider direction="vertical"></el-divider>
                    <div>
                        <el-input
                            v-model.number="pageNum"
                            size="small"
                            style="width: 50px"
                            @keydown.enter="handleJump"
                        ></el-input>
                        /
                        {{ pager.total }}
                    </div>
                    <el-divider direction="vertical"></el-divider>
                    <!-- <el-button text @click="handleRotate" :icon="RefreshLeft" size="small"></el-button> -->
                </div>
                <div></div>
            </div>
            <div class="book-wrap">
                <div class="book-outline" v-if="visible">
                    <el-scrollbar class="scrollbar" :always="false">
                        <el-tree
                            :data="outline"
                            :props="outlineProps"
                            :highlight-current="true"
                            @node-click="handleNodeClick"
                        >
                        </el-tree>
                    </el-scrollbar>
                </div>
                <div class="book-canvas" :style="{ width: visible ? 'calc(100% - 300px)' : '100%' }">
                    <div id="leafer-canvas" style="height: 100%; width: 100%"></div>
                </div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.book {
    width: 100%;
    height: 100%;
    &-utils {
        display: flex;
        align-items: center;
        justify-content: space-between;
        width: 100%;
        height: 40px;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
        background-color: #fff;
        padding: 0 24px;
        &-operation {
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            font-size: 14px;
        }
    }
    &-wrap {
        width: 100%;
        height: calc(100% - 40px);
        overflow: auto;
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
        font-size: 0;
    }
    &-outline {
        width: 300px;
        height: 100%;
        overflow: auto;
        position: relative;
        box-shadow: 1px 0 2px rgba(0, 0, 0, 0.1);
        background-color: #f9f9f9;
        display: inline-block;
        font-size: 14px;
    }
    &-canvas {
        height: 100%;
        display: inline-block;
    }
}
</style>

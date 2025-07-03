<script setup lang="ts">
import { ref, onMounted } from 'vue'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { formatBytes } from '@/utils/index'
import { ArrowRight } from '@element-plus/icons-vue'

const route = useRoute()

defineOptions({
    name: 'ArchiveSupport',
})

interface ExtractedFile {
    name: string
    size: number
    last_modified: string
    dir: boolean
}

class FileNode {
    name: string
    size: number
    last_modified: string
    isDir: boolean
    children: FileNode[]
    constructor(name: string, size: number, last_modified: string, isDir: boolean) {
        this.name = name
        this.size = size
        this.last_modified = last_modified
        this.isDir = isDir
        this.children = [] // 子目录或文件
    }

    addChild(child: FileNode) {
        this.children.push(child)
    }
}

function buildFileTree(files: ExtractedFile[]): FileNode {
    const root = new FileNode('root', 0, '', true) // 根节点是一个虚拟的根目录

    files.forEach(file => {
        const { name = '', size, last_modified = '', dir } = file
        const isDir = dir === true // 'true' or 'false' 字符串转换成布尔值
        const parts = name.split('/') // 按路径分割
        const filename = parts.pop() // 文件名是路径的最后一部分

        let currentNode = root

        parts.forEach(part => {
            // 查找是否已经存在该目录节点
            let dirNode = currentNode.children.find(child => child.name === part && child.isDir)

            // 如果目录节点不存在且不是空目录，则创建并添加
            if (!dirNode) {
                dirNode = new FileNode(part, 0, '', true)
                if (dirNode.name !== '') currentNode.addChild(dirNode)
            }

            // 进入下一级目录
            currentNode = dirNode
        })

        // 创建文件节点并添加到当前目录
        if (filename !== '') {
            const newFile = new FileNode(filename as string, size, last_modified, isDir)
            currentNode.addChild(newFile)
        }
    })

    return root
}

const fileInfo = ref<FileInfo>()
const content = ref<Array<FileNode>>()

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    const val = fileInfo.value.path as string
    const mode = fileInfo.value.extension as string
    const txt: Array<ExtractedFile> = await invoke('archive', { path: val, mode })
    const treeData = buildFileTree(txt)

    content.value = treeData.children as Array<FileNode> // 根节点是虚拟的，所以直接取子节点
})

const treeProps = {
    children: 'children',
    label: 'name',
}
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="text-support">
            <div class="text-support-inner">
                <el-tree
                    :data="content"
                    node-key="name"
                    :props="treeProps"
                    style="width: 100%"
                    :icon="ArrowRight"
                    :indent="12"
                >
                    <template #default="{ node, data }">
                        <div class="custom-row">
                            <span class="custom-row-1">{{ node.label }}</span>
                            <span class="custom-row-2">{{ data.isDir ? '' : formatBytes(data.size) }}</span>
                            <span class="custom-row-3">{{ data.isDir ? '' : data.last_modified }}</span>
                        </div>
                    </template>
                </el-tree>
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
        padding: 12px;
        font-size: 1.4rem;
        font-family: 'Microsoft YaHei', 'PingFang SC', 'Helvetica Neue', 'Helvetica', 'Arial', sans-serif;
        & .custom-row {
            display: flex;
            width: 100%;
            justify-content: space-between;
            align-items: center;
            line-height: 4rem;
            font-size: 12px;
            &-1 {
                flex: auto;
            }
            &-2 {
                flex: 0 0 100px;
            }
            &-3 {
                flex: 0 0 160px;
            }
        }
    }
}
</style>

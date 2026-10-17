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
    children?: ExtractedFile[] | null
}

function displayName(path: string): string {
    const trimmed = path.replace(/\/+$/, '')
    const parts = trimmed.split('/').filter(Boolean)
    return parts[parts.length - 1] ?? path
}

const fileInfo = ref<FileInfo>()
const content = ref<Array<ExtractedFile>>([])
const loading = ref(true)

async function loadArchive(path: string, password?: string) {
    // 后端返回已构建好的嵌套树（name 为完整路径）
    content.value = await invoke<Array<ExtractedFile>>('archive', {
        path,
        password: password ?? null,
    })
}

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    const val = fileInfo.value.path as string

    try {
        const isProtected = await invoke<boolean>('archive_is_password_protected', { path: val })
        if (isProtected) {
            try {
                const { value } = await ElMessageBox.prompt('该压缩文件需要密码才能查看', '输入密码', {
                    confirmButtonText: '确定',
                    cancelButtonText: '取消',
                    inputType: 'password',
                    inputPlaceholder: '请输入密码',
                    closeOnClickModal: false,
                })
                await loadArchive(val, value ?? undefined)
            } catch (err: unknown) {
                // ElMessageBox.prompt 取消/关闭会 reject: 'cancel' | 'close'
                if (err === 'cancel' || err === 'close') {
                    // 用户点击取消，不做任何操作
                    return
                }
                const msg = err instanceof Error ? err.message : String(err)
                await ElMessageBox.alert(msg, '解压失败')
            }
        } else {
            await loadArchive(val)
        }
    } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        await ElMessageBox.alert(msg, '压缩文件读取失败')
    } finally {
        loading.value = false
    }
})

const treeProps = {
    children: 'children',
    label: 'name',
}
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
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
                    <template #default="{ data }">
                        <div class="custom-row">
                            <span class="custom-row-1">{{ displayName(data.name) }}</span>
                            <span class="custom-row-2">{{ data.dir ? '' : formatBytes(data.size) }}</span>
                            <span class="custom-row-3">{{ data.dir ? '' : data.last_modified }}</span>
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

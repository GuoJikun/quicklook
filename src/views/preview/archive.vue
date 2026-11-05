<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronDown16Regular, ChevronUp16Regular } from '@vicons/fluent'
import LayoutPreview from '@/components/layout-preview.vue'
import { useRoute } from 'vue-router'
import type { FileInfo } from '@/utils/typescript'
import { invoke } from '@tauri-apps/api/core'
import { formatBytes } from '@/utils/index'

const route = useRoute()
const { t } = useI18n()

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

type SortKey = 'name' | 'size' | 'time'

const fileInfo = ref<FileInfo>()
const content = ref<Array<ExtractedFile>>([])
const loading = ref(true)

const keyword = ref('')
const sortKey = ref<SortKey>('name')
const sortAsc = ref(true)
const expanded = ref<Set<string>>(new Set())

function displayName(path: string): string {
    const trimmed = path.replace(/\/+$/, '')
    const parts = trimmed.split('/').filter(Boolean)
    return parts[parts.length - 1] ?? path
}

async function loadArchive(path: string, password?: string) {
    // 后端返回已构建好的嵌套树（name 为完整路径）
    content.value = await invoke<Array<ExtractedFile>>('archive', {
        path,
        password: password ?? null,
    })
    expandInitial()
}

/** 小档案默认全展开，大档案只展开根层，避免一次渲染上万行 */
function expandInitial() {
    const total = countNodes(content.value)
    const limit = 300
    const next = new Set<string>()
    const walk = (items: Array<ExtractedFile>, depth: number) => {
        for (const item of items) {
            if (!item.dir || !item.children?.length) continue
            if (total <= limit || depth === 0) next.add(item.name)
            if (total <= limit) walk(item.children, depth + 1)
        }
    }
    walk(content.value, 0)
    expanded.value = next
}

function countNodes(items: Array<ExtractedFile>): number {
    let n = 0
    for (const item of items) {
        n += 1
        if (item.children?.length) n += countNodes(item.children)
    }
    return n
}

/** 弹出密码输入框；用户取消/关闭返回 null */
async function promptPassword(message: string): Promise<string | null> {
    try {
        const { value } = await ElMessageBox.prompt(message, t('archive.inputPassword'), {
            confirmButtonText: t('common.confirm'),
            cancelButtonText: t('common.cancel'),
            inputType: 'password',
            inputPlaceholder: t('archive.passwordPlaceholder'),
            closeOnClickModal: false,
        })
        return value ?? ''
    } catch (err: unknown) {
        // ElMessageBox.prompt 取消/关闭会 reject: 'cancel' | 'close'
        if (err === 'cancel' || err === 'close') {
            return null
        }
        throw err
    }
}

/** 密码输错时允许重新输入，直到成功或用户取消 */
async function loadArchiveWithPasswordRetry(path: string) {
    let message = t('archive.passwordRequired')
    for (;;) {
        const password = await promptPassword(message)
        if (password === null) {
            return
        }
        try {
            await loadArchive(path, password)
            return
        } catch (err: unknown) {
            const msg = err instanceof Error ? err.message : String(err)
            message = t('archive.passwordWrong', { error: msg })
        }
    }
}

onMounted(async () => {
    fileInfo.value = route?.query as unknown as FileInfo
    const val = fileInfo.value.path as string

    try {
        const isProtected = await invoke<boolean>('archive_is_password_protected', { path: val })
        if (isProtected) {
            await loadArchiveWithPasswordRetry(val)
        } else {
            await loadArchive(val)
        }
    } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        await ElMessageBox.alert(msg, t('archive.readFailed'))
    } finally {
        loading.value = false
    }
})

/** 过滤：命中节点或其后代命中即保留；命中目录时保留全部子项 */
function filterTree(items: Array<ExtractedFile>, kw: string): Array<ExtractedFile> {
    if (!kw) return items
    const out: Array<ExtractedFile> = []
    for (const item of items) {
        const selfHit = displayName(item.name).toLowerCase().includes(kw)
        const kids = item.children?.length ? filterTree(item.children, kw) : []
        if (selfHit || kids.length) {
            out.push({ ...item, children: selfHit ? (item.children ?? null) : kids })
        }
    }
    return out
}

function sortTree(items: Array<ExtractedFile>): Array<ExtractedFile> {
    const arr = [...items].sort((a, b) => {
        // 目录恒在文件之前，排序只作用于同层同类
        if (a.dir !== b.dir) return a.dir ? -1 : 1
        let r = 0
        if (sortKey.value === 'name') {
            r = displayName(a.name).localeCompare(displayName(b.name), 'zh-Hans-CN', { numeric: true })
        } else if (sortKey.value === 'size') {
            r = a.size - b.size
        } else {
            r = a.last_modified.localeCompare(b.last_modified)
        }
        return sortAsc.value ? r : -r
    })
    return arr.map(item => (item.children?.length ? { ...item, children: sortTree(item.children) } : item))
}

interface Row {
    node: ExtractedFile
    depth: number
}

const trimming = computed(() => keyword.value.trim().toLowerCase())

const rows = computed<Array<Row>>(() => {
    const tree = sortTree(filterTree(content.value, trimming.value))
    const out: Array<Row> = []
    const walk = (items: Array<ExtractedFile>, depth: number) => {
        for (const item of items) {
            out.push({ node: item, depth })
            const hasChildren = !!item.children?.length
            // 过滤状态下强制展开，否则命中项藏在折叠目录里看不到
            const open = trimming.value ? true : expanded.value.has(item.name)
            if (item.dir && hasChildren && open) walk(item.children as Array<ExtractedFile>, depth + 1)
        }
    }
    walk(tree, 0)
    return out
})

/** 统计基于原始树，不受过滤与排序影响 */
const stats = computed(() => {
    let files = 0
    let bytes = 0
    const walk = (items: Array<ExtractedFile>) => {
        for (const item of items) {
            if (item.dir) {
                if (item.children?.length) walk(item.children)
            } else {
                files += 1
                bytes += item.size
            }
        }
    }
    walk(content.value)
    return { files, bytes }
})

function toggle(node: ExtractedFile) {
    if (!node.dir) return
    const next = new Set(expanded.value)
    if (next.has(node.name)) next.delete(node.name)
    else next.add(node.name)
    expanded.value = next
}

/** 过滤时子项被强制展开，箭头也要跟着指向下方 */
function isOpen(node: ExtractedFile): boolean {
    return !!trimming.value || expanded.value.has(node.name)
}

function expandAll() {
    const next = new Set<string>()
    const walk = (items: Array<ExtractedFile>) => {
        for (const item of items) {
            if (!item.dir) continue
            if (item.children?.length) next.add(item.name)
            if (item.children?.length) walk(item.children)
        }
    }
    walk(content.value)
    expanded.value = next
}

function collapseAll() {
    expanded.value = new Set()
}

function cycleSort(key: SortKey) {
    if (sortKey.value === key) {
        sortAsc.value = !sortAsc.value
    } else {
        sortKey.value = key
        sortAsc.value = true
    }
}

/** 命令栏上的三个排序按钮，当前列额外显示升降序箭头 */
const sortColumns = computed<Array<{ key: SortKey; label: string }>>(() => [
    { key: 'name', label: t('archive.sortName') },
    { key: 'size', label: t('archive.sortSize') },
    { key: 'time', label: t('archive.sortTime') },
])
</script>

<template>
    <LayoutPreview :file="fileInfo" :loading="loading">
        <template #toolbar>
            <label class="pv-tb-input">
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                    <circle cx="7" cy="7" r="4.2" />
                    <path d="M10.2 10.2 13.5 13.5" />
                </svg>
                <input v-model="keyword" type="text" :placeholder="t('archive.filterPlaceholder')" spellcheck="false" />
                <button v-if="keyword" class="pv-clear" type="button" :title="t('common.cancel')" @click="keyword = ''">
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" class="is-xs">
                        <path d="M4 4l8 8M12 4l-8 8" />
                    </svg>
                </button>
            </label>

            <i class="pv-tb-sep"></i>

            <button
                v-for="col in sortColumns"
                :key="col.key"
                class="pv-tb-btn is-text"
                type="button"
                :aria-pressed="sortKey === col.key"
                :title="t('archive.sortHint')"
                @click="cycleSort(col.key)"
            >
                {{ col.label }}
                <svg
                    v-if="sortKey === col.key"
                    class="sort-arrow"
                    :class="{ 'is-desc': !sortAsc }"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.4"
                >
                    <path d="M8 12.5V4M4.5 7.5 8 4l3.5 3.5" />
                </svg>
            </button>

            <i class="pv-tb-sep"></i>

            <button class="pv-tb-btn" type="button" :title="t('archive.expandAll')" @click="expandAll">
                <el-icon :size="15"><ChevronDown16Regular /></el-icon>
            </button>
            <button class="pv-tb-btn" type="button" :title="t('archive.collapseAll')" @click="collapseAll">
                <el-icon :size="15"><ChevronUp16Regular /></el-icon>
            </button>

            <div class="pv-tb-grow"></div>
            <span class="pv-tb-info">{{ t('archive.summary', { count: stats.files, size: formatBytes(stats.bytes) }) }}</span>
        </template>

        <div class="ar-wrap">
            <div class="ar-table pv-scroll">
                <div class="ar-head">
                    <span>{{ t('archive.nameCol') }}</span>
                    <span>{{ t('archive.sizeCol') }}</span>
                    <span>{{ t('archive.timeCol') }}</span>
                </div>

                <div
                    v-for="row in rows"
                    :key="row.node.name"
                    class="ar-row"
                    :class="{ 'is-dir': row.node.dir }"
                    @click="toggle(row.node)"
                >
                    <div class="ar-name" :style="{ paddingLeft: `${row.depth * 16}px` }">
                        <svg
                            v-if="row.node.dir"
                            class="chev"
                            :class="{ 'is-open': isOpen(row.node) }"
                            viewBox="0 0 16 16"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.6"
                        >
                            <path d="M6.5 4 10.5 8l-4 4" />
                        </svg>
                        <span v-else class="chev-placeholder"></span>

                        <svg v-if="row.node.dir" class="ico folder" viewBox="0 0 16 16" fill="currentColor">
                            <path
                                d="M1.8 4.2A1.2 1.2 0 0 1 3 3h3.1l1.3 1.6H13a1.2 1.2 0 0 1 1.2 1.2v5.4A1.2 1.2 0 0 1 13 12.4H3a1.2 1.2 0 0 1-1.2-1.2z"
                            />
                        </svg>
                        <svg v-else class="ico" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.3">
                            <path d="M4 1.8h5.2L13 5.6v8.6H4z" />
                            <path d="M9 1.8v4h4" />
                        </svg>

                        <span class="nm" :title="row.node.name">{{ displayName(row.node.name) }}</span>
                    </div>
                    <span class="sz">{{ row.node.dir ? '—' : formatBytes(row.node.size) }}</span>
                    <span class="tm">{{ row.node.last_modified }}</span>
                </div>

                <div v-if="!rows.length && !loading" class="ar-empty">
                    {{ trimming ? t('archive.noMatch') : t('archive.empty') }}
                </div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.ar-wrap {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding-top: 4px;
}

.ar-table {
    flex: 1;
    min-height: 0;
}

.ar-head,
.ar-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 96px 140px;
    align-items: center;
    gap: 12px;
    padding: 0 14px;
}

.ar-head {
    height: 26px;
    position: sticky;
    top: 0;
    z-index: 1;
    background: color-mix(in srgb, var(--color-bg) 88%, transparent);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--color-divider);
    font-size: var(--font-xs);
    color: var(--color-text-tertiary);
    letter-spacing: 0.04em;
    user-select: none;

    span:not(:first-child) {
        text-align: right;
    }
}

.ar-row {
    height: 28px;
    font-size: var(--font-xs);
    color: var(--color-text-primary);
    border-radius: var(--radius-sm);
    cursor: default;

    &:hover {
        background: var(--elevation-hover);
    }

    &.is-dir {
        cursor: pointer;
        background: color-mix(in srgb, var(--color-chip-bg) 60%, transparent);

        .nm {
            font-weight: 600;
        }
    }
}

.ar-name {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;

    .nm {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .chev {
        width: 14px;
        height: 14px;
        flex: none;
        color: var(--color-text-tertiary);
        transition: transform var(--transition-fast);

        &.is-open {
            transform: rotate(90deg);
        }
    }

    .chev-placeholder {
        width: 14px;
        flex: none;
    }

    .ico {
        width: 14px;
        height: 14px;
        flex: none;
        color: var(--color-text-tertiary);
    }

    .ico.folder {
        color: #e9b44c;
    }
}

.sz,
.tm {
    text-align: right;
    font-size: var(--font-xs);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.ar-empty {
    padding: 40px 14px;
    text-align: center;
    font-size: var(--font-xs);
    color: var(--color-text-tertiary);
}

.sort-arrow {
    width: 12px;
    height: 12px;
    flex: none;
    transition: transform var(--transition-fast);

    &.is-desc {
        transform: rotate(180deg);
    }
}
</style>

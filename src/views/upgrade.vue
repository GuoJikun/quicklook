<script lang="ts" setup>
import { check, Update } from '@tauri-apps/plugin-updater'
import { onMounted, ref } from 'vue'
import { info } from '@tauri-apps/plugin-log'
import MdViewer from '@/components/md-viewer/index.vue'
import { createMd } from '@/utils/markdown/index'
import type MarkdownIt from 'markdown-it'

const checked = ref(false)
const checkUpgrade = async (): Promise<{ updaterInstance: Update | null; isUpgrade: boolean }> => {
    const result = await check()
    return {
        updaterInstance: result,
        isUpgrade: !!result,
    }
}
let md: MarkdownIt | null = null
const mdLoading = ref(false)
const renderNotes = async (txt: string) => {
    mdLoading.value = true
    if (md === null) {
        md = await createMd()
    }

    notes.value = md.render(txt)
    mdLoading.value = false
}
let updaterInstance: Update | null = null
const isUpgrade = ref(false)
const notes = ref<string>('')
onMounted(async () => {
    checked.value = false
    const tmp = await checkUpgrade()
    checked.value = true
    console.log('check upgrade', tmp.updaterInstance)
    updaterInstance = tmp.updaterInstance
    isUpgrade.value = tmp.isUpgrade
    let tmoNotes = tmp.updaterInstance?.body || ''
    tmoNotes = tmoNotes.replace(/^\"|\"$/g, '')
    tmoNotes = tmoNotes.replace(/\\r\\n/g, '\n')
    console.log('tmoNotes', tmoNotes)
    renderNotes(tmoNotes)
})

const calcPercentage = (current: number, total: number) => {
    const tmp = (current / total) * 100
    return parseFloat(tmp.toFixed(2))
}
const isDownloading = ref(false)
const percentage = ref<number>(0)
const upgrade = async () => {
    isDownloading.value = true
    info('开始下载更新')
    let total: number = 0
    let current: number = 0
    if (isUpgrade.value) {
        await updaterInstance?.downloadAndInstall(payload => {
            const { event } = payload
            switch (event) {
                case 'Started':
                    total = payload.data?.contentLength || 0
                    info(`started downloading ${total} bytes`)
                    break
                case 'Progress':
                    current += payload.data?.chunkLength
                    percentage.value = calcPercentage(current, total)
                    info(`downloaded ${current} from ${total}`)
                    break
                case 'Finished':
                    info('download finished')
                    isDownloading.value = false
                    percentage.value = 100
                    break
            }
        })
    }
}

const cancelUpgrade = () => {
    updaterInstance?.close()
}

const handleUpgrade = () => {
    if (isDownloading.value) {
        cancelUpgrade()
    } else {
        upgrade()
    }
}
</script>

<template>
    <div class="upgrade" v-loading="!checked">
        <template v-if="checked">
            <div v-if="isUpgrade" class="upgrade-yes">
                <p>更新信息：</p>
                <MdViewer :content="notes" style="min-height: 200px" v-loading="mdLoading" />
                <template v-if="isDownloading">
                    <p style="margin-bottom: 12px">更新进度：</p>
                    <el-progress :percentage="percentage" text-inside striped striped-flow :stroke-width="15" />
                </template>
                <div style="display: flex; justify-content: center; margin-top: 12px">
                    <el-button @click="handleUpgrade" size="small" type="primary">{{
                        isDownloading ? '取消升级' : '现在升级'
                    }}</el-button>
                </div>
            </div>
            <div v-else class="upgrade-none">
                <p>当前已经是最新版本了</p>
            </div>
        </template>
    </div>
</template>

<style scoped lang="scss">
.upgrade {
    width: 100vw;
    height: 100vh;
    &-none {
        width: 100%;
        height: 100%;
        display: flex;
        justify-content: center;
        align-items: center;
        color: var(--el-color-success);
        box-sizing: border-box;
    }
    &-yes {
        width: 100%;
        height: 100%;
        padding: 12px;
        box-sizing: border-box;
        pre {
            white-space: pre-wrap;
            word-wrap: break-word;
            font-size: 13px;
            font-family: Inter, 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei',
                '微软雅黑', Arial, sans-serif;
        }
    }
}
</style>

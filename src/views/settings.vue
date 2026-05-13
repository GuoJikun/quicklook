<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
// import { emit } from '@tauri-apps/api/event'
import { readTextFile } from '@/utils'
import { app } from '@tauri-apps/api'
import { load, type Store } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'
import { LogLevel } from '@tauri-apps/plugin-log'
import { Check } from '@element-plus/icons-vue'

import SettingItem from '@/components/setting-item.vue'

let localStore: Store | null = null
const loadStore = async () => {
    localStore = await load('config.data', { autoSave: false, defaults: {} })
}

// // const updateConfig = async (val: string) => {
// //     const blob = new Blob([val], { type: 'application/json' })
// //     const arrayBuffer = new Uint8Array(await blob.arrayBuffer())

// //     writeFile('config.json', arrayBuffer, { baseDir: BaseDirectory.Resource })
// //         .then(() => {
// //             console.log('写入成功')
// //             emit('config_updated')
// //         })
// //         .catch(e => {
// //             console.error(e)
// //         })
// // }

const getConfig = async () => {
    const config = await readTextFile('config.json', { baseDir: BaseDirectory.Resource })
    const data = JSON.parse(config)
    const target = [
        { name: 'Markdown', code: 'Md', data: data['preview.markdown.checked'] },
        { name: '图片', code: 'Image', data: data['preview.image.checked'] },
        { name: '视频', code: 'Video', data: data['preview.video.checked'] },
        { name: '文档', code: 'Doc', data: data['preview.doc.checked'] },
        { name: '代码', code: 'Code', data: data['preview.code.checked'] },
        { name: '字体', code: 'Font', data: data['preview.font.checked'] },
        { name: '压缩包', code: 'Archive', data: data['preview.archive.checked'] },
        { name: '书籍', code: 'Book', data: data['preview.book.checked'] },
    ]
    return target
}

interface Config {
    name: string
    code: string
    data: Array<string>
}

const config = ref<Array<Config>>()
const version = ref<string>('')

const logLevel = ref<string>('info')
const logLevelList = [
    { label: 'Error', value: LogLevel.Error },
    { label: 'Warn', value: LogLevel.Warn },
    { label: 'Info', value: LogLevel.Info },
    { label: 'Debug', value: LogLevel.Debug },
    { label: 'Trace', value: LogLevel.Trace },
    { label: 'Off', value: 0 },
]
const updateLogLevel = async (level: number) => {
    await localStore?.set('logLevel', level)
    await localStore?.save()
    await invoke('set_log_level', { level })
}
const handleLogLevelChange = (level: unknown) => {
    updateLogLevel(level as number)
}

// 本机 ffmpeg 视频解析设置
const useLocalFfmpeg = ref<boolean>(false)
const ffmpegAvailable = ref<boolean | null>(null)

const handleUseLocalFfmpegChange = async (val: unknown) => {
    if (typeof val !== 'boolean') return
    await localStore?.set('useLocalFfmpeg', val)
    await localStore?.save()
    if (val) {
        ffmpegAvailable.value = await invoke<boolean>('check_ffmpeg')
    }
}

// 用户自定义代码格式
const customCodeExts = ref<string[]>([])
const newCodeExt = ref<string>('')

const addCodeExt = async () => {
    const ext = newCodeExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customCodeExts.value.includes(ext)) {
        ElMessage.warning(`扩展名 "${ext}" 已存在`)
        return
    }
    customCodeExts.value.push(ext)
    newCodeExt.value = ''
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
    ElMessage.success(`已添加代码格式：${ext}`)
}

const removeCodeExt = async (ext: string) => {
    customCodeExts.value = customCodeExts.value.filter(e => e !== ext)
    await localStore?.set('customCodeExtensions', customCodeExts.value)
    await localStore?.save()
}

// 用户自定义视频格式
const customVideoExts = ref<string[]>([])
const newVideoExt = ref<string>('')

const addVideoExt = async () => {
    const ext = newVideoExt.value.trim().toLowerCase().replace(/^\./, '')
    if (!ext) return
    if (customVideoExts.value.includes(ext)) {
        ElMessage.warning(`扩展名 "${ext}" 已存在`)
        return
    }
    customVideoExts.value.push(ext)
    newVideoExt.value = ''
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
    ElMessage.success(`已添加视频格式：${ext}`)
}

const removeVideoExt = async (ext: string) => {
    customVideoExts.value = customVideoExts.value.filter(e => e !== ext)
    await localStore?.set('customVideoExtensions', customVideoExts.value)
    await localStore?.save()
}

onMounted(async () => {
    await loadStore()
    config.value = await getConfig()
    version.value = await app.getVersion()
    const tmpLogLevel: string = (await localStore?.get<string>('logLevel')) || ''
    console.log('当前日志级别:', tmpLogLevel)
    logLevel.value = tmpLogLevel || 'info'

    const storedFfmpeg = await localStore?.get<boolean>('useLocalFfmpeg')
    useLocalFfmpeg.value = storedFfmpeg ?? false
    if (useLocalFfmpeg.value) {
        ffmpegAvailable.value = await invoke<boolean>('check_ffmpeg')
    }

    customCodeExts.value = (await localStore?.get<string[]>('customCodeExtensions')) ?? []
    customVideoExts.value = (await localStore?.get<string[]>('customVideoExtensions')) ?? []
})

const clearingCache = ref<boolean>(false)
const handleClearCache = async () => {
    clearingCache.value = true
    try {
        const removed = await invoke<number>('clear_cache')
        ElMessage.success(`已清理 ${removed} 个缓存目录`)
    } catch (e) {
        ElMessage.error(`清理缓存失败：${e}`)
    } finally {
        clearingCache.value = false
    }
}
</script>

<template>
    <div class="setting">
        <el-affix>
            <el-anchor direction="horizontal">
                <el-anchor-link href="#support">支持的格式</el-anchor-link>
                <el-anchor-link href="#custom-code">自定义代码格式</el-anchor-link>
                <el-anchor-link href="#video">视频</el-anchor-link>
                <el-anchor-link href="#cache">缓存</el-anchor-link>
                <el-anchor-link href="#log">日志</el-anchor-link>
                <el-anchor-link href="#version">版本</el-anchor-link>
            </el-anchor>
        </el-affix>
        <div class="setting-content">
            <SettingItem title="支持的格式" id="support">
                <div class="support-item" v-for="type in config" :key="type.code">
                    <div class="support-item-header">
                        <span>{{ type.code }}：</span>
                    </div>
                    <div class="support-item-body">
                        {{ type.data.join('、') }}
                    </div>
                </div>
            </SettingItem>
            <SettingItem title="自定义代码格式" id="custom-code">
                <div style="font-size: 13px; color: var(--el-text-color-secondary); margin-bottom: 8px">
                    在此添加额外的代码文件扩展名（如 <code>rb</code>、<code>lua</code>），添加后即可预览对应文件。
                </div>
                <div class="custom-ext-tags" v-if="customCodeExts.length > 0">
                    <el-tag
                        v-for="ext in customCodeExts"
                        :key="ext"
                        closable
                        @close="removeCodeExt(ext)"
                        style="margin: 2px 4px 2px 0"
                    >
                        {{ ext }}
                    </el-tag>
                </div>
                <div class="custom-ext-input flex-col-center" style="margin-top: 8px">
                    <el-input
                        v-model="newCodeExt"
                        placeholder="输入扩展名，如 rb"
                        size="small"
                        style="width: 180px"
                        @keyup.enter="addCodeExt"
                    />
                    <el-button size="small" type="primary" style="margin-left: 8px" @click="addCodeExt">
                        添加
                    </el-button>
                </div>
            </SettingItem>
            <SettingItem title="视频" id="video">
                <div class="flex-col-center">
                    <span>使用本机 ffmpeg 解析视频：</span>
                    <el-switch
                        v-model="useLocalFfmpeg"
                        @change="handleUseLocalFfmpegChange"
                        style="margin-left: 16px"
                    />
                </div>
                <div
                    v-if="useLocalFfmpeg"
                    style="margin-top: 8px; font-size: 13px; color: var(--el-text-color-secondary)"
                >
                    <template v-if="ffmpegAvailable === null">正在检测 ffmpeg…</template>
                    <template v-else-if="ffmpegAvailable">
                        <el-icon style="color: var(--el-color-success); vertical-align: middle"><Check /></el-icon>
                        已检测到本机 ffmpeg，非 h264 格式视频将自动转换后播放。
                    </template>
                    <template v-else>
                        <span style="color: var(--el-color-danger)">
                            未检测到 ffmpeg，请安装 ffmpeg 并确保其在系统 PATH 中。
                        </span>
                    </template>
                </div>
                <div style="margin-top: 12px">
                    <div style="font-size: 13px; color: var(--el-text-color-secondary); margin-bottom: 8px">
                        自定义视频格式扩展名（如 <code>ts</code>、<code>rm</code>）：
                        <span v-if="!useLocalFfmpeg" style="color: var(--el-color-warning); margin-left: 4px">
                            （需先启用 ffmpeg 解析）
                        </span>
                    </div>
                    <div class="custom-ext-tags" v-if="customVideoExts.length > 0">
                        <el-tag
                            v-for="ext in customVideoExts"
                            :key="ext"
                            closable
                            :disabled="!useLocalFfmpeg"
                            @close="useLocalFfmpeg && removeVideoExt(ext)"
                            style="margin: 2px 4px 2px 0"
                        >
                            {{ ext }}
                        </el-tag>
                    </div>
                    <div class="custom-ext-input flex-col-center" style="margin-top: 8px">
                        <el-input
                            v-model="newVideoExt"
                            placeholder="输入扩展名，如 ts"
                            size="small"
                            style="width: 180px"
                            :disabled="!useLocalFfmpeg"
                            @keyup.enter="useLocalFfmpeg && addVideoExt()"
                        />
                        <el-button
                            size="small"
                            type="primary"
                            style="margin-left: 8px"
                            :disabled="!useLocalFfmpeg"
                            @click="addVideoExt"
                        >
                            添加
                        </el-button>
                    </div>
                </div>
            </SettingItem>
            <SettingItem title="缓存" id="cache">
                <div class="flex-col-center">
                    <span>清理 ffmpeg 缓存：</span>
                    <el-button
                        size="small"
                        type="danger"
                        plain
                        :loading="clearingCache"
                        style="margin-left: 16px"
                        @click="handleClearCache"
                        >清理缓存</el-button
                    >
                </div>
                <div style="margin-top: 8px; font-size: 13px; color: var(--el-text-color-secondary)">
                    清理由 ffmpeg 视频转码生成的临时 HLS 缓存文件。
                </div>
            </SettingItem>
            <SettingItem title="日志" id="log">
                <div class="flex-col-center">
                    <span>日志级别：</span>
                    <el-radio-group v-model="logLevel" @change="handleLogLevelChange" style="margin-left: 16px">
                        <el-radio v-for="item in logLevelList" :key="item.value" :value="item.value">{{
                            item.label
                        }}</el-radio>
                    </el-radio-group>
                </div>
            </SettingItem>
            <SettingItem title="版本" id="version">
                <span>app 版本：{{ version }}</span>
            </SettingItem>
        </div>
    </div>
</template>

<style lang="scss" scoped>
.setting {
    padding: 16px;
    width: 100%;
    line-height: 1.6em;
    background-color: #fff;
    &-content {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
}
.support {
    &-item {
        padding: 4px 6px;
        display: flex;
        align-items: flex-start;
        &-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex: 0 0 70px;
            min-width: 70px;
            justify-content: flex-end;
        }
    }
}
.custom-ext-tags {
    display: flex;
    flex-wrap: wrap;
}
</style>

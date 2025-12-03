<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { BaseDirectory } from '@tauri-apps/plugin-fs'
// import { emit } from '@tauri-apps/api/event'
import { readTextFile } from '@/utils'
import { app } from '@tauri-apps/api'
import { load, type Store } from '@tauri-apps/plugin-store'
import { invoke } from '@tauri-apps/api/core'
import { LogLevel } from '@tauri-apps/plugin-log'

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

const officeParser = ref<string>('internal')
const updateOfficeParser = async (val: string) => {
    await localStore?.set('officeParser', val)
    await localStore?.save()
}
const handleOfficeParserChange = async (val: string) => {
    officeParser.value = val
    updateOfficeParser(val)
}

onMounted(async () => {
    await loadStore()
    config.value = await getConfig()
    version.value = await app.getVersion()
    officeParser.value = (await localStore?.get<string>('officeParser')) || 'internal'
    const tmpLogLevel: string = (await localStore?.get<string>('logLevel')) || ''
    console.log('当前日志级别:', tmpLogLevel)
    logLevel.value = tmpLogLevel || 'info'
})
</script>

<template>
    <div class="setting">
        <el-affix>
            <el-anchor direction="horizontal">
                <el-anchor-link href="#office-parser">Office 解析器</el-anchor-link>
                <el-anchor-link href="#support">支持的格式</el-anchor-link>
                <el-anchor-link href="#log">日志</el-anchor-link>
                <el-anchor-link href="#version">版本</el-anchor-link>
            </el-anchor>
        </el-affix>
        <SettingItem title="Office 解析器" id="office-parser">
            <el-radio-group v-model="officeParser" @change="handleOfficeParserChange">
                <el-radio value="office">微软 Office</el-radio>
                <el-radio value="wps">WPS</el-radio>
                <el-radio value="internal">内置解析器</el-radio>
            </el-radio-group>
        </SettingItem>
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
        <SettingItem title="日志设置" id="log">
            <el-radio-group v-model="logLevel" @change="handleLogLevelChange" style="margin-left: 16px">
                <el-radio v-for="item in logLevelList" :key="item.value" :value="item.value">{{ item.label }}</el-radio>
            </el-radio-group>
        </SettingItem>
        <SettingItem title="版本" id="version">
            {{ version }}
        </SettingItem>
    </div>
</template>

<style lang="scss" scoped>
.setting {
    padding: 16px;
    width: 100%;
    line-height: 1.6em;
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
</style>

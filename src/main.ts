import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { error } from '@tauri-apps/plugin-log'

import { NIcon, create } from 'naive-ui'
const naive = create({
    components: [NIcon],
})

import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import zhCn from 'element-plus/es/locale/lang/zh-cn'

import initSentry from './utils/sentry'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(naive)
app.use(ElementPlus, {
    locale: zhCn,
})

app.config.errorHandler = (err, vm, code) => {
    error(`[Vue Error]: Error- ${err?.toString()}；Code- ${code}`)
}
app.config.warnHandler = (msg, vm, trace) => {
    console.groupCollapsed(`%c[Global Warn]`, 'color: red; font-weight: bold;')
    console.warn(trace)
    console.warn(msg)
    console.warn(vm)
    console.groupEnd()
}
if (!import.meta.env.DEV) {
    initSentry({ app, router })
}
app.mount('#app')

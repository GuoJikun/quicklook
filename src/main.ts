import './assets/main.scss'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { error, warn } from '@tauri-apps/plugin-log'

import 'element-plus/theme-chalk/base.css'
import 'element-plus/theme-chalk/dark/css-vars.css'

import initSentry from './utils/sentry'
// 初始化主题（暗黑/明亮），在应用创建前执行以减少闪烁
import './hooks/theme'

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.config.errorHandler = (err, vm, code) => {
    error(`[Vue Error]: Error- ${err?.toString()}；Code- ${code}`)
}
app.config.warnHandler = (msg, vm, trace) => {
    console.groupCollapsed(`%c[Global Warn]`, 'color: red; font-weight: bold;')
    console.warn(trace)
    console.warn(msg)
    console.warn(vm)
    console.groupEnd()
    warn(`[Vue Warn]: Message- ${msg}；Trace- ${trace}`)
}
if (!import.meta.env.DEV) {
    initSentry({ app, router })
}
app.mount('#app')

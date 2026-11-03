import './assets/main.scss'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { error, warn } from '@tauri-apps/plugin-log'

import 'element-plus/theme-chalk/base.css'
import 'element-plus/theme-chalk/dark/css-vars.css'

// 初始化主题（暗黑/明亮），在应用创建前执行以减少闪烁
import './hooks/theme'
import i18n, { initLocale, listenLocaleChange } from './locales'

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(i18n)

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
    void import('./utils/sentry').then(({ default: initSentry }) => {
        initSentry({ app, router })
    })
}
// 语言存在 config.data 里，需先读出再挂载，否则会先渲染一帧中文再跳变
initLocale().finally(() => {
    app.mount('#app')
    // 其他窗口切换语言时同步当前窗口
    void listenLocaleChange()
})

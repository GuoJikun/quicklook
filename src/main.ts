import './assets/main.scss'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { error, warn } from '@tauri-apps/plugin-log'

import 'element-plus/theme-chalk/base.css'
import 'element-plus/theme-chalk/dark/css-vars.css'

// 副作用导入：主题在此完成初始化（写入 html.dark），早于 app.mount 以减少首帧闪烁。
// 所有窗口都会执行到这里，因此各窗口无需各自初始化；页面只按需调用 useTheme() 读取响应式状态。
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

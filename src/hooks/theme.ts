import { useColorMode, type BasicColorSchema } from '@vueuse/core'
import { computed } from 'vue'

/** 设置页可见的三档偏好；auto 时实际明暗由系统决定 */
export type ThemePreference = BasicColorSchema

/** 主题监听是模块级单例，不是每次调用 useTheme() 各建一份。
 *
 * Tauri 的每个窗口都是独立 webview、各有一份 document，`html.dark` 只能由窗口自己写入。
 * 放在模块作用域意味着「谁先 import 本模块，谁就完成初始化」：由 main.ts 副作用导入，
 * 写入时机早于 app.mount，既消除首帧白闪，也让页面里没有调用 useTheme 的窗口
 * （如升级页）拿到正确主题 —— 它此前正是因为完全没引入本模块而始终停在浅色。
 *
 * 跨窗口同步由 useColorMode 内部的 storage 监听完成：任一窗口切换明暗，其余窗口随之更新。 */
const { system, store } = useColorMode({
    selector: document.documentElement,
    attribute: 'class',
    onChanged: (_mode: BasicColorSchema, defaultHandler) => {
        defaultHandler(_mode)
        document.documentElement.classList.toggle('dark', _mode === 'dark')
    },
})

/** 实际生效的明暗（light / dark），供 isDark 与标题栏快捷切换使用 */
const mode = computed({
    get: () => (store.value === 'auto' ? system.value : store.value),
    set: (val: BasicColorSchema) => {
        store.value = val
    },
})

/** 用户选中的偏好（auto / light / dark）。标题栏 toggle 写入 light/dark 后，
    设置页能立刻看到选中态变化，且只有设置页能把状态改回 auto。 */
const preference = computed<ThemePreference>({
    get: () => store.value,
    set: (val: ThemePreference) => {
        store.value = val
    },
})

const toggle = () => {
    mode.value = mode.value === 'dark' ? 'light' : 'dark'
}

const isDark = computed(() => mode.value === 'dark')

export function useTheme() {
    return { mode, preference, isDark, toggle }
}

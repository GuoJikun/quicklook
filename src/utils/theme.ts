import { useDark, useToggle, useColorMode, type BasicColorSchema } from '@vueuse/core'

// 全局暗黑模式控制
// - 使用 data-theme="dark" 来驱动 src/assets/var.css 中的变量
// - 同步在 <html> 上加/去掉 class="dark"，以兼容 markdown 样式与部分三方库
export const isDark = useDark({
    selector: document.documentElement,
    attribute: 'data-theme',
    valueDark: 'dark',
    valueLight: '',
    onChanged(isDark, defaultHandler) {
        // 设置 data-theme 属性
        defaultHandler(isDark ? 'dark' : 'light')
        // 兼容：给 html 添/去除 .dark（如 md-viewer、Element Plus 等）
        document.documentElement.classList.toggle('dark', isDark)
    },
})

export const toggleDark = useToggle(isDark)

// 首次导入时根据当前值立即同步一次 html.dark（避免首屏闪烁）
document.documentElement.classList.toggle('dark', isDark.value)

// 可选：导出一个 useTheme，兼容“useTheme”的命名习惯
export function useTheme() {
    const mode = useColorMode({
        selector: document.documentElement,
        attribute: 'data-theme',
        modes: {
            dark: 'dark',
            light: 'light',
        },
        onChanged: (_mode: BasicColorSchema, defaultHandler) => {
            defaultHandler(_mode)
            document.documentElement.classList.toggle('dark', _mode === 'dark')
        },
    })
    const toggle = () => {
        mode.value = mode.value === 'dark' ? 'light' : 'dark'
    }
    return { mode, isDark, toggle }
}

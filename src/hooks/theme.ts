import { useColorMode, type BasicColorSchema } from '@vueuse/core'
import { computed } from 'vue'

/** 设置页可见的三档偏好；auto 时实际明暗由系统决定 */
export type ThemePreference = BasicColorSchema

export function useTheme() {
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
    return { mode, preference, isDark, toggle }
}

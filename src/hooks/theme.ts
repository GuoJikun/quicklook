import { useColorMode, type BasicColorSchema } from '@vueuse/core'
import { computed } from 'vue'

export function useTheme() {
    const { system, store } = useColorMode({
        selector: document.documentElement,
        attribute: 'class',
        onChanged: (_mode: BasicColorSchema, defaultHandler) => {
            defaultHandler(_mode)
            document.documentElement.classList.toggle('dark', _mode === 'dark')
        },
    })

    const mode = computed({
        get: () => (store.value === 'auto' ? system.value : store.value),
        set: (val: BasicColorSchema) => {
            store.value = val
        },
    })

    const toggle = () => {
        mode.value = mode.value === 'dark' ? 'light' : 'dark'
    }

    const isDark = computed(() => mode.value === 'dark')
    return { mode, isDark, toggle }
}

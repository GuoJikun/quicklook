/**
 * i18n 初始化与语言切换。
 *
 * 设计要点：
 * - 语言持久化在 `config.data` 的 `locale` 键，与其它设置共用同一个 store；
 * - 每个 webview（主窗口 / 预览窗口 / 设置窗口 / 更新窗口）都独立挂载一次 Vue，
 *   所以在 `main.ts` 里先 `initLocale()` 再 mount，避免首帧闪一下中文；
 * - 切换语言时通过 `set_locale` 命令通知 Rust 重建托盘菜单，Rust 再广播
 *   `locale-changed` 事件，让已打开的其他窗口同步（预览窗口复用时会整页
 *   `navigate`，但已开着的窗口不会自己重载）。
 */
import { computed } from 'vue'
import { createI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import zhCnElement from 'element-plus/es/locale/lang/zh-cn'
import enUsElement from 'element-plus/es/locale/lang/en'

import { getConfigStore } from '@/utils/store'
import zhCN from './zh-CN'
import enUS from './en-US'

export type LocaleCode = 'zh-CN' | 'en-US'

/** 默认中文 */
export const DEFAULT_LOCALE: LocaleCode = 'zh-CN'

export const LOCALE_STORAGE_KEY = 'locale'

/** 语言切换器选项；语言名一律用自身语言书写 */
export const LOCALE_OPTIONS: { value: LocaleCode; labelKey: string }[] = [
    { value: 'zh-CN', labelKey: 'language.zhCN' },
    { value: 'en-US', labelKey: 'language.enUS' },
]

export const ELEMENT_LOCALES = {
    'zh-CN': zhCnElement,
    'en-US': enUsElement,
} as const

export const i18n = createI18n({
    legacy: false,
    globalInjection: true,
    locale: DEFAULT_LOCALE,
    fallbackLocale: DEFAULT_LOCALE,
    messages: {
        'zh-CN': zhCN,
        'en-US': enUS,
    },
    // 缺键时在开发环境报警，生产环境静默回退，避免控制台噪音
    missingWarn: import.meta.env.DEV,
    fallbackWarn: false,
})

/** 把任意来源的语言标记归一化为受支持的 LocaleCode */
export function normalizeLocale(value?: string | null): LocaleCode {
    if (!value) return DEFAULT_LOCALE
    const normalized = value.trim().toLowerCase()
    if (normalized.startsWith('zh')) return 'zh-CN'
    if (normalized.startsWith('en')) return 'en-US'
    return DEFAULT_LOCALE
}

export function getLocale(): LocaleCode {
    return i18n.global.locale.value as LocaleCode
}

/** Element Plus 组件语言（跟随当前语言，供 el-config-provider 使用） */
export const elementLocale = computed(() => ELEMENT_LOCALES[getLocale()])

/** 当前语言（响应式） */
export const currentLocale = computed(() => getLocale())

/** 只改内存中的语言，不落盘、不广播 */
export function applyLocale(code: LocaleCode) {
    i18n.global.locale.value = code
    if (typeof document !== 'undefined') {
        document.documentElement.lang = code
    }
}

/** 读取已保存的语言，缺省为中文 */
export async function loadStoredLocale(): Promise<LocaleCode> {
    try {
        const store = await getConfigStore()
        return normalizeLocale(await store.get<string>(LOCALE_STORAGE_KEY))
    } catch (e) {
        // 读不到就退回中文，但必须留下痕迹：窗口漏掉 store 权限时
        // （capabilities 里没登记该窗口）表现就是「界面语言和标题栏不一致」，
        // 静默回退会让这类问题只能靠猜
        console.warn('[i18n] failed to read stored locale, fallback to', DEFAULT_LOCALE, e)
        return DEFAULT_LOCALE
    }
}

/**
 * 启动时调用：读出语言并应用。
 * 每个 webview 各挂载一次 Vue，所以每个窗口都必须能读 store ——
 * 窗口的 capability 里漏掉 `store:allow-load` / `store:allow-get` 时，
 * 这里会退回默认语言，界面语言就与 Rust 画的标题栏对不上。
 */
export async function initLocale(): Promise<LocaleCode> {
    const code = await loadStoredLocale()
    applyLocale(code)
    return code
}

/**
 * 切换语言：立即生效 → 落盘 → 通知 Rust 更新托盘。
 * 任一环节失败都不影响界面切换本身。
 */
export async function setLocale(code: LocaleCode): Promise<void> {
    applyLocale(code)
    try {
        const store = await getConfigStore()
        await store.set(LOCALE_STORAGE_KEY, code)
        await store.save()
    } catch (e) {
        console.warn('[i18n] failed to persist locale:', e)
    }
    try {
        await invoke('set_locale', { locale: code })
    } catch (e) {
        console.warn('[i18n] failed to notify backend:', e)
    }
}

/** 监听 Rust 广播的语言变更，让已打开的其他窗口同步 */
export async function listenLocaleChange(): Promise<void> {
    try {
        await listen<string>('locale-changed', event => {
            applyLocale(normalizeLocale(event.payload))
        })
    } catch (e) {
        console.warn('[i18n] failed to listen locale-changed:', e)
    }
}

export default i18n

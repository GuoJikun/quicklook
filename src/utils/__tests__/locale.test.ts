import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

import { DEFAULT_LOCALE, LOCALE_OPTIONS, i18n, normalizeLocale } from '@/locales'
import zhCN from '@/locales/zh-CN'
import enUS from '@/locales/en-US'

// vitest 的 root 指向项目根目录（见 vitest.config.ts），据此定位源码
const SRC_ROOT = join(process.cwd(), 'src')

/** 把嵌套语言包拍平成 `a.b.c` 形式的键列表 */
function flattenKeys(node: Record<string, unknown>, prefix = ''): string[] {
    return Object.entries(node).flatMap(([key, value]) => {
        const full = prefix ? `${prefix}.${key}` : key
        if (value !== null && typeof value === 'object') {
            return flattenKeys(value as Record<string, unknown>, full)
        }
        return [full]
    })
}

/** 递归收集源码文件 */
function collectSourceFiles(dir: string, acc: string[] = []): string[] {
    for (const entry of readdirSync(dir)) {
        const full = join(dir, entry)
        if (statSync(full).isDirectory()) {
            collectSourceFiles(full, acc)
        } else if (/\.(vue|ts)$/.test(entry)) {
            acc.push(full)
        }
    }
    return acc
}

const zhKeys = flattenKeys(zhCN)
const enKeys = flattenKeys(enUS)

describe('语言包', () => {
    it('默认语言是简体中文', () => {
        expect(DEFAULT_LOCALE).toBe('zh-CN')
    })

    it('中英文键集合完全一致', () => {
        const missingInEn = zhKeys.filter(key => !enKeys.includes(key))
        const missingInZh = enKeys.filter(key => !zhKeys.includes(key))
        expect({ missingInEn, missingInZh }).toEqual({ missingInEn: [], missingInZh: [] })
    })

    it('没有值为空的文案', () => {
        const emptyInZh = zhKeys.filter(key => {
            const value = key.split('.').reduce<unknown>((acc, k) => (acc as Record<string, unknown>)[k], zhCN)
            return typeof value !== 'string' || value.trim() === ''
        })
        expect(emptyInZh).toEqual([])
    })

    it('语言选项与受支持语言一一对应', () => {
        expect(LOCALE_OPTIONS.map(item => item.value)).toEqual(['zh-CN', 'en-US'])
        for (const option of LOCALE_OPTIONS) {
            expect(zhKeys).toContain(option.labelKey)
        }
    })
})

describe('normalizeLocale', () => {
    it('缺省与无法识别时回退中文', () => {
        expect(normalizeLocale(undefined)).toBe('zh-CN')
        expect(normalizeLocale(null)).toBe('zh-CN')
        expect(normalizeLocale('')).toBe('zh-CN')
        expect(normalizeLocale('fr-FR')).toBe('zh-CN')
    })

    it('识别中文的各种写法', () => {
        expect(normalizeLocale('zh-CN')).toBe('zh-CN')
        expect(normalizeLocale('zh')).toBe('zh-CN')
        expect(normalizeLocale('zh-Hans-CN')).toBe('zh-CN')
    })

    it('识别英文并忽略大小写与空白', () => {
        expect(normalizeLocale('en-US')).toBe('en-US')
        expect(normalizeLocale('EN')).toBe('en-US')
        expect(normalizeLocale('  en-GB  ')).toBe('en-US')
    })
})

/**
 * 去掉注释与日志语句，只留下可能进入界面的字符串。
 * 日志（console.* / info / warn / error）沿用仓库既有的中文风格，不参与本项检查。
 */
function stripNonUiText(source: string): string {
    return source
        .replace(/\/\*[\s\S]*?\*\//g, '')
        .replace(/<!--[\s\S]*?-->/g, '')
        .replace(/<style[\s\S]*?<\/style>/g, '')
        .replace(/(^|\s)\/\/.*$/gm, '')
        .replace(/\bconsole\.\w+\([^;]*?\)/g, '')
        .replace(/\b(?:info|warn|error|debug)\(\s*['"`][^)]*\)/g, '')
}

describe('源码中的文案键', () => {
    const files = collectSourceFiles(SRC_ROOT).filter(
        file => !file.includes(join('src', 'locales')) && !file.includes('__tests__'),
    )

    it('所有 t() 引用的键都存在于语言包中', () => {
        const missing: string[] = []
        for (const file of files) {
            const source = readFileSync(file, 'utf-8')
            for (const match of source.matchAll(/\bt\(\s*'([^']+)'/g)) {
                const key = match[1]
                if (!zhKeys.includes(key) && !missing.includes(key)) {
                    missing.push(key)
                }
            }
        }
        expect(missing).toEqual([])
    })

    /**
     * 有一类键不会以 `t('...')` 字面量出现，而是存成字符串字段后动态取值，
     * 例如 `{ nameKey: 'settings.formats.audio' }` + `t(type.nameKey)`。
     * 静态扫描工具（i18n 检查器、本文件上一条用例的正则）都看不到它们，
     * 会误报成「未使用」，所以在这里按运行时解析结果单独校验。
     */
    it('动态引用的键（nameKey / labelKey 等）能在两种语言下正确解析', () => {
        const PATTERN = /\b(?:nameKey|labelKey|titleKey|descKey|descriptionKey|placeholderKey|messageKey):\s*'([^']+)'/g
        // `labelKey` 定义在 src/locales/index.ts，所以这里不能沿用上面过滤掉 locales 的列表
        const keyFiles = collectSourceFiles(SRC_ROOT).filter(file => !file.includes('__tests__'))
        const collected: { file: string; key: string }[] = []
        for (const file of keyFiles) {
            const source = readFileSync(file, 'utf-8')
            for (const match of source.matchAll(PATTERN)) {
                collected.push({ file: file.replace(SRC_ROOT, ''), key: match[1] })
            }
        }

        const original = i18n.global.locale.value
        const broken: string[] = []
        try {
            for (const locale of ['zh-CN', 'en-US'] as const) {
                i18n.global.locale.value = locale
                for (const { file, key } of collected) {
                    // 键不存在时 vue-i18n 原样返回键名
                    if (String(i18n.global.t(key)) === key) {
                        const at = `${file}: ${key}`
                        if (!broken.includes(at)) broken.push(at)
                    }
                }
            }
        } finally {
            i18n.global.locale.value = original
        }

        expect(broken).toEqual([])
        // 防止正则失效导致用例空转：目前至少有 10 个格式名 + 2 个语言名
        expect(collected.length).toBeGreaterThanOrEqual(12)
    })

    it('没有遗留硬编码的中文界面文案', () => {
        // markdown 插件的 codeCopyButtonTitle 目前未被渲染逻辑使用，保留原值
        const allowList = [join('src', 'utils', 'markdown', 'index.ts')]
        const offenders = files
            .filter(file => !allowList.some(allowed => file.endsWith(allowed)))
            .filter(file => /[\u4e00-\u9fa5]/.test(stripNonUiText(readFileSync(file, 'utf-8'))))
        expect(offenders.map(file => file.replace(SRC_ROOT, ''))).toEqual([])
    })
})

import { defineConfig } from '@x18n/core'

// Migrated from i18n-ally by `x18n migrate i18n-ally`.
export default defineConfig({
  sourceLocale: 'en-US',
  // index.ts 是 createI18n 的装配文件，会被 {locale}.{ext} 误当成语言包
  ignoreLocales: ['index'],
  // 这些键通过映射表间接引用（如 nameKey: 'settings.formats.md'），静态扫描看不到
  ignoreKeys: ['language.*', 'settings.formats.*', 'model.bg*'],
  resources: [
    {
      pattern: 'src/locales/{locale}.{ext}'
    },
    {
      pattern: 'src/locales/{locale}/**/*.{ext}'
    }
  ]
})

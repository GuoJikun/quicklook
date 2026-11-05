import { defineConfig } from '@x18n/core'

// Migrated from i18n-ally by `x18n migrate i18n-ally`.
export default defineConfig({
  resources: [
    {
      pattern: 'src/locales/{locale}.{ext}'
    },
    {
      pattern: 'src/locales/{locale}/**/*.{ext}'
    }
  ]
})

import { fileURLToPath } from 'node:url'
import { mergeConfig, defineConfig, configDefaults } from 'vitest/config'
import viteConfig from './vite.config'

export default defineConfig(configEnv =>
    mergeConfig(
        viteConfig(configEnv),
        defineConfig({
            test: {
                environment: 'jsdom',
                exclude: [...configDefaults.exclude, 'e2e/**'],
                include: ['src/utils/**/*.{test,spec}.{js,ts,jsx,tsx}'],
                root: fileURLToPath(new URL('./', import.meta.url)),
            },
        }),
    ),
)

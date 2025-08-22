import { fileURLToPath, URL } from 'node:url'
import process from 'node:process'

import { defineConfig, loadEnv, type PluginOption } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import { sentryVitePlugin } from '@sentry/vite-plugin'
import pkg from './package.json'

// https://vite.dev/config/
export default defineConfig(({ mode, command }) => {
    const env = loadEnv(mode, process.cwd())

    let plugins: PluginOption[] = [vue(), vueJsx()]
    if (command === 'build') {
        plugins = [
            ...plugins,
            sentryVitePlugin({
                org: 'zhiqiu',
                project: 'quicklook-vue',
                authToken: env.VITE_SENTRY_TOKEN,
                sourcemaps: {
                    filesToDeleteAfterUpload: ['dist/**/*.map'],
                },
                release: {
                    name: pkg.version || 'default',
                },
            }),
        ]
    }

    return {
        plugins,
        resolve: {
            alias: {
                '@': fileURLToPath(new URL('./src', import.meta.url)),
            },
        },
        build: {
            sourcemap: true,
            rollupOptions: {
                output: {
                    manualChunks: {
                        vender: ['vue', 'vue-router', 'pinia'],
                    },
                },
            },
        },
        server: {
            host: true,
            port: 6688,
        },
    }
})

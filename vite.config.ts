import { fileURLToPath, URL } from 'node:url'
import process from 'node:process'
import fs from 'node:fs'
import path from 'node:path'

import { defineConfig, loadEnv, type PluginOption } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import { sentryVitePlugin } from '@sentry/vite-plugin'
import pkg from './package.json'

function copyPdfiumDll(): PluginOption {
    function copy() {
        const rootDir = process.cwd()
        const targetDir = path.join(rootDir, 'src-tauri', 'dlls')
        const arch = process.arch === 'arm64' ? 'arm64' : 'x64'
        const srcFile = path.join(rootDir, `pdfium-${arch}.dll`)
        const destFile = path.join(targetDir, 'pdfium.dll')

        if (!fs.existsSync(srcFile)) {
            console.warn(`[copy-pdfium-dll] ${srcFile} not found, skipping`)
            return
        }

        fs.mkdirSync(targetDir, { recursive: true })
        fs.copyFileSync(srcFile, destFile)
        console.log(`[copy-pdfium-dll] ${srcFile} -> ${destFile}`)
    }

    return {
        name: 'copy-pdfium-dll',
        configureServer: {
            handler() {
                copy()
            },
        },
        closeBundle() {
            copy()
        },
    }
}

// https://vite.dev/config/
export default defineConfig(({ mode, command }) => {
    const env = loadEnv(mode, process.cwd())

    let plugins: PluginOption[] = [vue(), vueJsx(), copyPdfiumDll()]
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

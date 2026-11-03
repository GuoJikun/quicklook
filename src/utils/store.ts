/**
 * config.data 单例。
 *
 * `@tauri-apps/plugin-store` 在 Rust 侧按路径复用同一个 Store 实例，
 * 因此重复 `load()` 不会产生数据副本，只是多一次 IPC 往返、多一个前端句柄。
 * 这里收敛成单例的原因是：语言、日志级别、ffmpeg 开关等设置分散在多个模块，
 * 统一入口可以避免各自 `load()` 带来的多余往返，读写顺序也更直观。
 */
import { load, type Store } from '@tauri-apps/plugin-store'

let instance: Promise<Store> | null = null

export function getConfigStore(): Promise<Store> {
    if (!instance) {
        instance = load('config.data', { autoSave: false, defaults: {} })
    }
    return instance
}

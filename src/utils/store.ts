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
        // 缓存的是 Promise 而不是结果，必须显式处理失败：否则一次加载失败
        // 会把 rejected Promise 永久留在 instance 里，此后所有调用都直接
        // 抛错，只有重载页面才能恢复。
        instance = load('config.data', { autoSave: false, defaults: {} }).catch(error => {
            instance = null
            throw error
        })
    }
    return instance
}

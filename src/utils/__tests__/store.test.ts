import { beforeEach, describe, expect, it, vi } from 'vitest'

const { loadMock } = vi.hoisted(() => ({ loadMock: vi.fn() }))

vi.mock('@tauri-apps/plugin-store', () => ({
    load: loadMock,
}))

beforeEach(() => {
    // store.ts 的 instance 是模块级状态，必须重置模块才能隔离用例
    vi.resetModules()
    loadMock.mockReset()
})

describe('getConfigStore', () => {
    it('并发调用只真正 load 一次，复用同一个实例', async () => {
        const store = { get: vi.fn() }
        loadMock.mockResolvedValue(store)

        const { getConfigStore } = await import('../store')
        const [first, second] = await Promise.all([getConfigStore(), getConfigStore()])

        expect(loadMock).toHaveBeenCalledTimes(1)
        expect(first).toBe(store)
        expect(second).toBe(store)
    })

    it('加载失败后可以重试，不会把 rejected Promise 永久缓存', async () => {
        const store = { get: vi.fn() }
        loadMock
            .mockRejectedValueOnce(new Error('ipc failed'))
            .mockResolvedValueOnce(store)
        const { getConfigStore } = await import('../store')

        await expect(getConfigStore()).rejects.toThrow('ipc failed')
        // 关键断言：失败后缓存已清空，第二次调用会重新尝试而不是直接复用旧错误
        await expect(getConfigStore()).resolves.toBe(store)
        expect(loadMock).toHaveBeenCalledTimes(2)
    })
})

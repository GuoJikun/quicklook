import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/api/window', () => ({
    Window: {
        getByLabel: vi.fn(),
    },
}))

vi.mock('@tauri-apps/plugin-fs', () => ({
    readFile: vi.fn(),
}))

import { Window } from '@tauri-apps/api/window'
import { readFile } from '@tauri-apps/plugin-fs'
import { Type, formatBytes, getWindow, readTextFile } from '../index'

const mockedGetByLabel = vi.mocked(Window.getByLabel)
const mockedReadFile = vi.mocked(readFile)

beforeEach(() => {
    vi.clearAllMocks()
})

describe('Type', () => {
    it('get 返回小写类型名', () => {
        expect(Type.get({})).toBe('object')
        expect(Type.get([])).toBe('array')
        expect(Type.get('a')).toBe('string')
        expect(Type.get(1)).toBe('number')
        expect(Type.get(null)).toBe('null')
        expect(Type.get(undefined)).toBe('undefined')
        expect(Type.get(Symbol('s'))).toBe('symbol')
        expect(Type.get(1n)).toBe('bigint')
    })

    it('isObject 仅对普通对象为 true', () => {
        expect(Type.isObject({})).toBe(true)
        expect(Type.isObject([])).toBe(false)
        expect(Type.isObject(null)).toBe(false)
        expect(Type.isObject('x')).toBe(false)
    })

    it('isArray 识别数组', () => {
        expect(Type.isArray([])).toBe(true)
        expect(Type.isArray([1, 2])).toBe(true)
        expect(Type.isArray({})).toBe(false)
        expect(Type.isArray('[]')).toBe(false)
    })

    it('isFunction 识别普通函数，不识别 async/generator', () => {
        expect(Type.isFunction(() => {})).toBe(true)
        expect(Type.isFunction(function () {})).toBe(true)
        expect(Type.get(async () => {})).toBe('asyncfunction')
        expect(Type.isFunction(async () => {})).toBe(false)
        expect(Type.isFunction({})).toBe(false)
    })

    it('isString / isNumber / isBoolean', () => {
        expect(Type.isString('a')).toBe(true)
        expect(Type.isString(1)).toBe(false)
        expect(Type.isNumber(1)).toBe(true)
        expect(Type.isNumber(NaN)).toBe(true)
        expect(Type.isNumber('1')).toBe(false)
        expect(Type.isBoolean(true)).toBe(true)
        expect(Type.isBoolean(0)).toBe(false)
    })

    it('isUndefined / isNull', () => {
        expect(Type.isUndefined(undefined)).toBe(true)
        expect(Type.isUndefined(null)).toBe(false)
        expect(Type.isNull(null)).toBe(true)
        expect(Type.isNull(undefined)).toBe(false)
    })

    it('isSymbol / isBigInt / isUnit8Array', () => {
        expect(Type.isSymbol(Symbol('s'))).toBe(true)
        expect(Type.isSymbol('s')).toBe(false)
        expect(Type.isBigInt(1n)).toBe(true)
        expect(Type.isBigInt(1)).toBe(false)
        expect(Type.isUnit8Array(new Uint8Array([1]))).toBe(true)
        expect(Type.isUnit8Array([1])).toBe(false)
    })
})

describe('formatBytes', () => {
    it('MB 以下按 KB 展示（固定两位小数）', () => {
        expect(formatBytes(1024)).toBe('1.00 KB')
        expect(formatBytes(1536)).toBe('1.50 KB')
        expect(formatBytes(0)).toBe('0.00 KB')
        expect(formatBytes(1024 * 1024 - 1)).toBe('1024.00 KB')
    })

    it('MB 档', () => {
        expect(formatBytes(1024 * 1024)).toBe('1.00 MB')
        expect(formatBytes(1.5 * 1024 * 1024)).toBe('1.50 MB')
        expect(formatBytes(100 * 1024 * 1024)).toBe('100.00 MB')
        expect(formatBytes(1024 * 1024 * 1024 - 1)).toBe('1024.00 MB')
    })

    it('GB 档', () => {
        expect(formatBytes(1024 ** 3)).toBe('1.00 GB')
        expect(formatBytes(2.25 * 1024 ** 3)).toBe('2.25 GB')
        expect(formatBytes(1024 ** 4 - 1)).toBe('1024.00 GB')
    })

    it('TB 档', () => {
        expect(formatBytes(1024 ** 4)).toBe('1.00 TB')
        expect(formatBytes(3 * 1024 ** 4)).toBe('3.00 TB')
    })

    it('边界：刚好达到单位阈值时进位到下一档', () => {
        expect(formatBytes(1024 ** 2)).toBe('1.00 MB')
        expect(formatBytes(1024 ** 3)).toBe('1.00 GB')
        expect(formatBytes(1024 ** 4)).toBe('1.00 TB')
    })
})

describe('readTextFile', () => {
    type ReadFileResult = Awaited<ReturnType<typeof readFile>>

    it('readFile 返回 Uint8Array 时解码为字符串', async () => {
        const text = '你好, QuickLook'
        const bytes = new Uint8Array(new TextEncoder().encode(text)) as ReadFileResult
        mockedReadFile.mockResolvedValueOnce(bytes)

        const result = await readTextFile('/tmp/a.txt')
        expect(result).toBe(text)
        expect(mockedReadFile).toHaveBeenCalledWith('/tmp/a.txt', {})
    })

    it('透传 conf 参数', async () => {
        mockedReadFile.mockResolvedValueOnce(new Uint8Array([65, 66]) as ReadFileResult)

        await readTextFile('/tmp/a.txt', { baseDir: 1 })
        expect(mockedReadFile).toHaveBeenCalledWith('/tmp/a.txt', { baseDir: 1 })
    })

    it('非 Uint8Array 返回空字符串', async () => {
        mockedReadFile.mockResolvedValueOnce({ not: 'a buffer' } as unknown as ReadFileResult)

        await expect(readTextFile('/tmp/a.txt')).resolves.toBe('')
    })

    it('空 buffer 返回空字符串', async () => {
        mockedReadFile.mockResolvedValueOnce(new Uint8Array([]) as ReadFileResult)

        await expect(readTextFile('/tmp/empty.txt')).resolves.toBe('')
    })
})

describe('getWindow', () => {
    it('按 label 调用 Window.getByLabel', async () => {
        const fake = { label: 'preview' }
        mockedGetByLabel.mockResolvedValueOnce(fake as unknown as Awaited<ReturnType<typeof Window.getByLabel>>)

        await expect(getWindow('preview')).resolves.toBe(fake)
        expect(mockedGetByLabel).toHaveBeenCalledWith('preview')
    })

    it('找不到窗口时返回 null', async () => {
        mockedGetByLabel.mockResolvedValueOnce(null)

        await expect(getWindow('missing')).resolves.toBeNull()
    })
})

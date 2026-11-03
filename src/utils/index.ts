import { Window, type WindowLabel } from '@tauri-apps/api/window'
import { readFile } from '@tauri-apps/plugin-fs'

export const getWindow = async (label: WindowLabel) => {
    return await Window.getByLabel(label)
}

export class Type {
    /**
     * 获取类型
     * @param val
     * @returns
     */
    static get = (val: unknown) => {
        return Object.prototype.toString.call(val).slice(8, -1).toLowerCase()
    }
    /**
     * 是否对象
     * @param val
     * @returns
     */
    static isObject = (val: unknown) => {
        return Type.get(val) === 'object'
    }
    /**
     * 是否数组
     * @param val
     * @returns
     */
    static isArray = (val: unknown) => {
        return Type.get(val) === 'array'
    }
    /**
     * 是否函数
     * @param val
     * @returns
     */
    static isFunction = (val: unknown) => {
        return Type.get(val) === 'function'
    }
    /**
     * 是否字符串
     * @param val
     * @returns
     */
    static isString = (val: unknown) => {
        return Type.get(val) === 'string'
    }
    /**
     * 是否数字
     * @param val
     * @returns
     */
    static isNumber = (val: unknown) => {
        return Type.get(val) === 'number'
    }
    /**
     * 是否布尔
     * @param val
     * @returns
     */
    static isBoolean = (val: unknown) => {
        return Type.get(val) === 'boolean'
    }
    /**
     * 是否undefined
     * @param val
     * @returns
     */
    static isUndefined = (val: unknown) => {
        return Type.get(val) === 'undefined'
    }
    /**
     * 是否null
     * @param val
     * @returns
     */
    static isNull = (val: unknown) => {
        return Type.get(val) === 'null'
    }
    /**
     * 是否symbol
     * @param val
     * @returns
     */
    static isSymbol = (val: unknown) => {
        return Type.get(val) === 'symbol'
    }
    /**
     * 是否bigint
     * @param val
     * @returns
     */
    static isBigInt = (val: unknown) => {
        return Type.get(val) === 'bigint'
    }

    /**
     * 是否uint8array
     * @param val
     * @returns
     */
    static isUnit8Array = (val: unknown) => {
        return Type.get(val) === 'uint8array'
    }
}
/**
 * 格式化字节大小
 * @param size 字节大小；缺失或非法（NaN / Infinity）时返回占位符
 * @returns
 */
export const formatBytes = (size: number | null | undefined): string => {
    // `undefined` 参与比较恒为 false，会一路落到 else 分支输出 "NaN TB"，
    // 因此必须先拦截。文件信息缺失（如「暂不支持」页不传 file、预览数据
    // 尚未就绪）都会走到这里。
    if (size === null || size === undefined || !Number.isFinite(size)) {
        return '—'
    }

    const KB = 1024
    const MB = KB * 1024
    const GB = MB * 1024
    const TB = GB * 1024

    if (size < MB) {
        return `${(size / KB).toFixed(2)} KB`
    } else if (size < GB) {
        return `${(size / MB).toFixed(2)} MB`
    } else if (size < TB) {
        return `${(size / GB).toFixed(2)} GB`
    } else {
        return `${(size / TB).toFixed(2)} TB`
    }
}

/**
 * 读取文本文件
 * @param path string
 * @returns
 */
export const readTextFile = async (path: string, conf = {}): Promise<string> => {
    const file = await readFile(path, conf)
    if (file instanceof Uint8Array) {
        return new TextDecoder().decode(file)
    } else {
        return ''
    }
}

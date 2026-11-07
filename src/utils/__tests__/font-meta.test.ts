import { describe, expect, it } from 'vitest'
import { parseFontMeta } from '../font-meta'

/* ── 测试用字体构造器 ──────────────────────────────────────────────────
   真实字体动辄十几 MB，不方便入库；这里按 sfnt 规范拼一个「只含被测表的
   最小字体」，让解析器的每条分支都能被精确覆盖。 */

interface NameEntry {
    nameID: number
    value: string
    platformID?: number
    languageID?: number
}

interface AxisEntry {
    tag: string
    min: number
    def: number
    max: number
}

function u16(value: number): number[] {
    return [(value >> 8) & 0xff, value & 0xff]
}

function u32(value: number): number[] {
    return [(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]
}

function tagBytes(tag: string): number[] {
    return [...tag].map(c => c.charCodeAt(0))
}

function utf16be(text: string): number[] {
    const out: number[] = []
    for (let i = 0; i < text.length; i++) out.push(...u16(text.charCodeAt(i)))
    return out
}

function buildNameTable(entries: NameEntry[]): number[] {
    const records: number[] = []
    const storage: number[] = []
    let storageOffset = 0

    for (const entry of entries) {
        const platformID = entry.platformID ?? 3
        const languageID = entry.languageID ?? 0x409
        const encoded = platformID === 1 ? [...entry.value].map(c => c.charCodeAt(0)) : utf16be(entry.value)

        records.push(
            ...u16(platformID),
            ...u16(1), // encodingID
            ...u16(languageID),
            ...u16(entry.nameID),
            ...u16(encoded.length),
            ...u16(storageOffset),
        )
        storage.push(...encoded)
        storageOffset += encoded.length
    }

    return [...u16(0), ...u16(entries.length), ...u16(6 + entries.length * 12), ...records, ...storage]
}

function buildFvarTable(axes: AxisEntry[]): number[] {
    const axisSize = 20
    const header = [
        ...u32(0x00010000),
        ...u16(16), // axesArrayOffset
        ...u16(2), // reserved
        ...u16(axes.length),
        ...u16(axisSize),
        ...u16(0), // instanceCount
        ...u16(0), // instanceSize
    ]
    const body: number[] = []
    for (const axis of axes) {
        body.push(
            ...tagBytes(axis.tag),
            ...u32(Math.round(axis.min * 65536)),
            ...u32(Math.round(axis.def * 65536)),
            ...u32(Math.round(axis.max * 65536)),
            ...u16(0), // flags
            ...u16(256), // nameID
        )
    }
    return [...header, ...body]
}

function buildHeadTable(unitsPerEm: number): number[] {
    const head = new Array<number>(54).fill(0)
    head.splice(18, 2, ...u16(unitsPerEm))
    return head
}

function buildMaxpTable(numGlyphs: number): number[] {
    return [...u32(0x00010000), ...u16(numGlyphs), ...new Array<number>(26).fill(0)]
}

interface FontOptions {
    names: NameEntry[]
    version?: number
    unitsPerEm?: number
    numGlyphs?: number
    axes?: AxisEntry[]
    extraTables?: Record<string, number[]>
}

function buildFont(options: FontOptions): Uint8Array {
    const tables = new Map<string, number[]>()
    tables.set('name', buildNameTable(options.names))
    tables.set('head', buildHeadTable(options.unitsPerEm ?? 1000))
    tables.set('maxp', buildMaxpTable(options.numGlyphs ?? 1234))
    if (options.axes) tables.set('fvar', buildFvarTable(options.axes))
    for (const [tag, data] of Object.entries(options.extraTables ?? {})) tables.set(tag, data)

    const dirSize = 12 + tables.size * 16
    const placements: Array<{ tag: string; data: number[]; offset: number }> = []
    let cursor = dirSize

    for (const [tag, data] of tables) {
        placements.push({ tag, data, offset: cursor })
        const pad = (4 - (data.length % 4)) % 4
        cursor += data.length + pad
    }

    const out = new Uint8Array(cursor)
    const view = new DataView(out.buffer)

    view.setUint32(0, options.version ?? 0x00010000, false)
    view.setUint16(4, tables.size, false)

    placements.forEach((placement, index) => {
        const rec = 12 + index * 16
        placement.tag.split('').forEach((char, i) => view.setUint8(rec + i, char.charCodeAt(0)))
        view.setUint32(rec + 8, placement.offset, false)
        view.setUint32(rec + 12, placement.data.length, false)
        out.set(placement.data, placement.offset)
    })

    return out
}

/** 把单个字体的 offset table 挪到 16 字节之后，凑成 TTC 容器。
    注意 TTC 里的表偏移是相对整个文件而非相对各自的 offset table，
    搬迁后表目录里的偏移必须一并平移，否则会解析到错误位置。 */
function wrapInCollection(font: Uint8Array): Uint8Array {
    const header = 16
    const out = new Uint8Array(header + font.length)
    const view = new DataView(out.buffer)
    out.set(tagBytes('ttcf'), 0)
    view.setUint32(4, 0x00010000, false)
    view.setUint32(8, 1, false)
    view.setUint32(12, header, false)
    out.set(font, header)

    const source = new DataView(font.buffer, font.byteOffset, font.byteLength)
    const numTables = source.getUint16(4, false)
    for (let i = 0; i < numTables; i++) {
        const rec = header + 12 + i * 16 + 8
        view.setUint32(rec, view.getUint32(rec, false) + header, false)
    }

    return out
}

const CJK_NAMES: NameEntry[] = [
    { nameID: 16, value: '思源宋体', languageID: 0x804 },
    { nameID: 16, value: 'Source Han Serif SC', languageID: 0x409 },
    { nameID: 17, value: 'SemiBold', languageID: 0x409 },
    { nameID: 4, value: '思源宋体 SemiBold', languageID: 0x804 },
    { nameID: 5, value: 'Version 2.001', languageID: 0x409 },
]

describe('parseFontMeta', () => {
    it('提取家族名、风格、版本与字表统计', () => {
        const meta = parseFontMeta(buildFont({ names: CJK_NAMES, unitsPerEm: 1000, numGlyphs: 65535 }))

        expect(meta).not.toBeNull()
        expect(meta!.family).toBe('思源宋体')
        expect(meta!.familyEn).toBe('Source Han Serif SC')
        expect(meta!.style).toBe('SemiBold')
        expect(meta!.fullName).toBe('思源宋体 SemiBold')
        expect(meta!.version).toBe('2.001')
        expect(meta!.glyphCount).toBe(65535)
        expect(meta!.unitsPerEm).toBe(1000)
        expect(meta!.outlines).toBe('TrueType')
        expect(meta!.axes).toEqual([])
    })

    it('Regular 不进入风格位，避免标题出现「字体 Regular」', () => {
        const meta = parseFontMeta(
            buildFont({
                names: [
                    { nameID: 1, value: 'HarmonyOS Sans' },
                    { nameID: 2, value: 'Regular' },
                ],
            }),
        )

        expect(meta!.family).toBe('HarmonyOS Sans')
        expect(meta!.style).toBe('')
        // 家族名只有英文时不再重复出现在副标题
        expect(meta!.familyEn).toBe('')
    })

    it('中文条目缺失时回退到英文家族名', () => {
        const meta = parseFontMeta(
            buildFont({
                names: [
                    { nameID: 1, value: 'JetBrains Mono' },
                    { nameID: 2, value: 'Bold' },
                ],
            }),
        )

        expect(meta!.family).toBe('JetBrains Mono')
        expect(meta!.style).toBe('Bold')
    })

    it('OTTO 版本号识别为 CFF 轮廓', () => {
        const meta = parseFontMeta(buildFont({ names: CJK_NAMES, version: 0x4f54544f }))
        expect(meta!.outlines).toBe('OpenType CFF')
    })

    it('读取 fvar 的可变轴', () => {
        const meta = parseFontMeta(
            buildFont({
                names: CJK_NAMES,
                axes: [
                    { tag: 'wght', min: 200, def: 400, max: 900.5 },
                    { tag: 'wdth', min: 75, def: 100, max: 125 },
                ],
            }),
        )

        expect(meta!.axes).toEqual([
            { tag: 'wght', min: 200, default: 400, max: 900.5 },
            { tag: 'wdth', min: 75, default: 100, max: 125 },
        ])
    })

    it('TTC 容器取第一个 face', () => {
        const meta = parseFontMeta(wrapInCollection(buildFont({ names: CJK_NAMES })))
        expect(meta!.family).toBe('思源宋体')
    })

    it('ArrayBuffer 与 Uint8Array 入参结果一致', () => {
        const bytes = buildFont({ names: CJK_NAMES })
        const fromArrayBuffer = parseFontMeta(bytes.buffer as ArrayBuffer)
        expect(fromArrayBuffer!.family).toBe(parseFontMeta(bytes)!.family)
    })

    it('WOFF/WOFF2 容器返回 null，交由调用方回退文件名', () => {
        const woff = new Uint8Array(64)
        woff.set(tagBytes('wOFF'), 0)
        const woff2 = new Uint8Array(64)
        woff2.set(tagBytes('wOF2'), 0)

        expect(parseFontMeta(woff)).toBeNull()
        expect(parseFontMeta(woff2)).toBeNull()
    })

    it('数据损坏时返回 null 而不是抛错', () => {
        expect(parseFontMeta(new Uint8Array(0))).toBeNull()
        expect(parseFontMeta(new Uint8Array(11))).toBeNull()

        // 头合法但 numTables 巨大，表目录必然越界
        const broken = new Uint8Array(16)
        new DataView(broken.buffer).setUint32(0, 0x00010000, false)
        new DataView(broken.buffer).setUint16(4, 0xffff, false)
        expect(parseFontMeta(broken)).toBeNull()

        // 缺少 head 表
        const noHead = buildFont({ names: CJK_NAMES })
        expect(parseFontMeta(noHead.subarray(0, 12))).toBeNull()
    })
})

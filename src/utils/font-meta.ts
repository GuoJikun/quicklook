/**
 * 字体元信息解析（只读 TrueType / OpenType 的 name、head、maxp、fvar 四张表）
 *
 * 浏览器能把字体渲染出来，却读不出它叫什么——`FontFace` 只认得我们传进去的名字。
 * 而字体预览有一半价值在于「这是什么字体」，所以这里直接从二进制里取。
 *
 * 不做的事：
 * - WOFF / WOFF2：表结构是压缩存放的，要额外接 zlib / brotli，代价与收益不成比例，
 *   交回 `null` 让调用方回退到文件名；
 * - CFF 字形轮廓：只用来区分 OpenType(CFF) 与 TrueType(glyf)，不解析字形。
 */

/** 可变字体的一条设计轴（如 wght 200–900） */
export interface FontAxis {
    /** 四字符标签，如 wght / wdth / slnt */
    tag: string
    min: number
    default: number
    max: number
}

export interface FontMeta {
    /** 家族名，优先取中文本地化条目 */
    family: string
    /** 英文家族名；与 family 相同或缺失时为空串 */
    familyEn: string
    /** 风格名（SemiBold、Italic…）；Regular / 常规 归一化为空串 */
    style: string
    /** 完整名（name ID 4），用于「复制字体名」 */
    fullName: string
    /** 版本串（name ID 5），已去掉前缀 "Version " */
    version: string
    /** 轮廓格式 */
    outlines: 'TrueType' | 'OpenType CFF'
    /** 可变字体轴；非可变字体为空数组 */
    axes: FontAxis[]
    /** maxp.numGlyphs */
    glyphCount: number
    /** head.unitsPerEm */
    unitsPerEm: number
}

/** 读取越界时统一抛 RangeError，由 parseFontMeta 兜底，避免到处写边界判断 */
class Reader {
    private view: DataView

    constructor(private bytes: Uint8Array) {
        this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    }

    get length() {
        return this.view.byteLength
    }

    u8(offset: number) {
        return this.view.getUint8(offset)
    }

    u16(offset: number) {
        return this.view.getUint16(offset, false)
    }

    i32(offset: number) {
        return this.view.getInt32(offset, false)
    }

    u32(offset: number) {
        return this.view.getUint32(offset, false)
    }

    /** 4 字节标签 → 字符串，非法字节用 `?` 占位 */
    tag(offset: number) {
        let out = ''
        for (let i = 0; i < 4; i++) {
            const code = this.u8(offset + i)
            out += code >= 0x20 && code < 0x7f ? String.fromCharCode(code) : '?'
        }
        return out
    }
}

interface NameRecord {
    platformID: number
    languageID: number
    nameID: number
    value: string
}

interface TableRecord {
    offset: number
    length: number
}

/** Windows 平台的本地化语言 ID */
const LANG_ZH_CN = [0x804, 0x404, 0x1004, 0xc04]
const LANG_EN_US = 0x409

/** UTF-16BE，name 表在 platform 0/3 下的编码 */
function decodeUtf16(bytes: Uint8Array, start: number, length: number): string {
    let out = ''
    for (let i = 0; i + 1 < length; i += 2) {
        out += String.fromCharCode((bytes[start + i] << 8) | bytes[start + i + 1])
    }
    return out
}

/** Mac Roman 里的 ASCII 区间与 Latin-1 一致，非 ASCII 字符直接按码位回退 */
function decodeMacRoman(bytes: Uint8Array, start: number, length: number): string {
    let out = ''
    for (let i = 0; i < length; i++) {
        out += String.fromCharCode(bytes[start + i])
    }
    return out
}

function readNameRecords(reader: Reader, table: TableRecord, bytes: Uint8Array): NameRecord[] {
    const base = table.offset
    const count = reader.u16(base + 2)
    const storage = base + reader.u16(base + 4)
    const records: NameRecord[] = []

    for (let i = 0; i < count; i++) {
        const rec = base + 6 + i * 12
        const platformID = reader.u16(rec)
        const languageID = reader.u16(rec + 4)
        const nameID = reader.u16(rec + 6)
        const length = reader.u16(rec + 8)
        const offset = storage + reader.u16(rec + 10)

        // name 表本身可能被截断，越界条目直接跳过而不是让整个解析失败
        if (offset + length > bytes.length) continue

        const value = (
            platformID === 0 || platformID === 3
                ? decodeUtf16(bytes, offset, length)
                : decodeMacRoman(bytes, offset, length)
        )
            .replace(/\u0000/g, '')
            .trim()

        if (value) records.push({ platformID, languageID, nameID, value })
    }

    return records
}

/** 同一条 nameID 可能有十来种语言版本，按「平台可信度 + 语言偏好」打分取最高 */
function rank(record: NameRecord, lang: 'zh' | 'en'): number {
    let score = record.platformID === 3 ? 4 : record.platformID === 0 ? 3 : 2

    const isZh = record.platformID === 3 && LANG_ZH_CN.includes(record.languageID)
    const isEn =
        (record.platformID === 3 && record.languageID === LANG_EN_US) ||
        (record.platformID === 1 && record.languageID === 0)

    if (lang === 'zh') {
        if (isZh) score += 6
        else if (isEn) score += 3
    } else {
        if (isEn) score += 6
        else if (isZh) score += 1
    }

    return score
}

/**
 * 按 nameID 优先级取名字：ID 16（排版家族名）比 ID 1（家族名）更准确，
 * 因为前者在「思源宋体 SemiBold」这类字体上不会把字重塞进家族名里。
 */
function pickName(records: NameRecord[], ids: number[], lang: 'zh' | 'en'): string {
    for (const id of ids) {
        const candidates = records.filter(r => r.nameID === id)
        if (!candidates.length) continue
        candidates.sort((a, b) => rank(b, lang) - rank(a, lang))
        return candidates[0].value
    }
    return ''
}

/** "Version 2.004" → 2.004；部分厂商还会在后面接 `;构建号`，一并去掉 */
function normalizeVersion(raw: string): string {
    return raw
        .replace(/^version\s*/i, '')
        .split(';')[0]
        .trim()
}

/**
 * Regular 是最常见的一档，单独写在标题里没有信息量。
 * 中文条目里对应的是 U+5E38 U+89C4（常规）与 U+6807 U+51C6（标准），
 * 写成码位是为了不和「源码不得硬编码中文文案」的守卫冲突。
 */
const REGULAR_ALIASES = new Set(['regular', 'normal', 'book', '\u5e38\u89c4', '\u6807\u51c6'])

function normalizeStyle(raw: string): string {
    return REGULAR_ALIASES.has(raw.toLowerCase()) ? '' : raw
}

function readAxes(reader: Reader, table: TableRecord): FontAxis[] {
    const base = table.offset
    const axesOffset = base + reader.u16(base + 4)
    const axisCount = reader.u16(base + 8)
    const axisSize = reader.u16(base + 10)

    const axes: FontAxis[] = []
    // 轴数量理论上无上限，这里截断到 4 条，避免命令栏被极端字体撑爆
    for (let i = 0; i < Math.min(axisCount, 4); i++) {
        const offset = axesOffset + i * axisSize
        axes.push({
            tag: reader.tag(offset),
            min: Math.round((reader.i32(offset + 4) / 65536) * 100) / 100,
            default: Math.round((reader.i32(offset + 8) / 65536) * 100) / 100,
            max: Math.round((reader.i32(offset + 12) / 65536) * 100) / 100,
        })
    }
    return axes
}

/**
 * 解析字体元信息。
 *
 * @param source 字体文件的完整字节；TTC 取第一个 face
 * @returns 解析失败（数据损坏、或 WOFF/WOFF2 这类表被压缩的容器）时返回 `null`
 */
export function parseFontMeta(source: Uint8Array | ArrayBuffer): FontMeta | null {
    const bytes = source instanceof Uint8Array ? source : new Uint8Array(source)
    if (bytes.length < 12) return null

    try {
        const reader = new Reader(bytes)

        // TTC 的头 12 字节是 'ttcf' + 版本 + 字体数，真正的 sfnt 偏移在 12
        let start = 0
        if (reader.tag(0) === 'ttcf') {
            if (reader.u32(8) < 1) return null
            start = reader.u32(12)
        }

        // 版本号是 4 字节整数而不是可读标签：0x00010000 / 'true' 是 glyf 轮廓，
        // 'OTTO' 是 CFF 轮廓，'wOFF' / 'wOF2' 会落到这里——容器不同，不做解压
        const version = reader.u32(start)
        const isCff = version === 0x4f54544f // OTTO
        if (version !== 0x00010000 && version !== 0x74727565 && !isCff) return null

        const numTables = reader.u16(start + 4)
        const tables = new Map<string, TableRecord>()
        for (let i = 0; i < numTables; i++) {
            const rec = start + 12 + i * 16
            tables.set(reader.tag(rec), {
                offset: reader.u32(rec + 8),
                length: reader.u32(rec + 12),
            })
        }

        const nameTable = tables.get('name')
        const headTable = tables.get('head')
        if (!nameTable || !headTable) return null

        const records = readNameRecords(reader, nameTable, bytes)
        if (!records.length) return null

        const family = pickName(records, [16, 1], 'zh')
        const familyEn = pickName(records, [16, 1], 'en')
        const fullName = pickName(records, [4, 16, 1], 'zh')

        if (!family && !familyEn) return null

        const maxpTable = tables.get('maxp')
        const fvarTable = tables.get('fvar')

        return {
            family: family || familyEn,
            familyEn: familyEn && familyEn !== family ? familyEn : '',
            style: normalizeStyle(pickName(records, [17, 2], 'en')),
            fullName: fullName || family || familyEn,
            version: normalizeVersion(pickName(records, [5], 'en')),
            outlines: isCff ? 'OpenType CFF' : 'TrueType',
            axes: fvarTable ? readAxes(reader, fvarTable) : [],
            glyphCount: maxpTable ? reader.u16(maxpTable.offset + 4) : 0,
            unitsPerEm: reader.u16(headTable.offset + 18),
        }
    } catch {
        return null
    }
}

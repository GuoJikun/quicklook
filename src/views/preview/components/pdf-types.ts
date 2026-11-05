/** PDF 预览中父子组件共享的类型：目录项与命令栏状态 */

export interface PdfOutlineItem {
    title: string
    page: number
    items: PdfOutlineItem[]
}

export interface PdfState {
    /** 当前可见页（1-based） */
    current: number
    total: number
    /** 相对「适应宽度」的缩放倍数：1 即 100%（设计稿约定 100% = 适应宽度） */
    zoom: number
}

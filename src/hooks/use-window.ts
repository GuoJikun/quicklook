import { getCurrentWindow } from '@tauri-apps/api/window'
import { ref } from 'vue'

/** `startResizeDragging` 的参数类型在 @tauri-apps/api 里没有导出（window.d.ts 内部 type），
 *  这里按它的签名原样声明一份，避免用 any 绕过类型检查 */
export type WindowResizeDirection =
    | 'East'
    | 'North'
    | 'NorthEast'
    | 'NorthWest'
    | 'South'
    | 'SouthEast'
    | 'SouthWest'
    | 'West'

/** 最大化状态：无边框窗口的标题栏据此在「最大化 / 还原」两个图标间切换。
 *  只有真正带最大化钮的窗口才调用 watchMaximized()，固定尺寸的对话框不必多挂一次监听。 */
export const isMaximized = ref(false)

let unlisten: (() => void) | null = null

export const useWindow = () => {
    const handleClose = () => {
        void getCurrentWindow().close()
    }

    const handleMin = () => {
        void getCurrentWindow().minimize()
    }

    const handleMax = () => {
        void getCurrentWindow().toggleMaximize()
    }

    /** 首次调用时挂一次 resize 监听，同窗口内重复调用不会叠加 */
    const watchMaximized = async () => {
        const win = getCurrentWindow()
        isMaximized.value = await win.isMaximized()
        unlisten ??= await win.onResized(async () => {
            isMaximized.value = await win.isMaximized()
        })
    }

    /** 无边框窗口没有系统边框可点，边缘缩放由自绘热区发起 */
    const startResize = (direction: WindowResizeDirection) => {
        void getCurrentWindow().startResizeDragging(direction)
    }

    return {
        handleClose,
        handleMin,
        handleMax,
        watchMaximized,
        startResize,
    }
}

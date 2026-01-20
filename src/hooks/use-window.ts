import { getCurrentWindow } from '@tauri-apps/api/window'

export const useWindow = () => {
    const handleClose = () => {
        const curWindow = getCurrentWindow()
        curWindow.close()
    }

    const handleMin = () => {
        const curWindow = getCurrentWindow()
        curWindow.minimize()
    }

    return {
        handleClose,
        handleMin,
    }
}

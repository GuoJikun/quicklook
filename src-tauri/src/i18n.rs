//! 后端文案的国际化。
//!
//! 前端语言包在 `src/locales/`，后端只负责那些**由 Rust 直接绘制**的界面元素：
//! 托盘菜单、系统对话框标题、窗口标题。两者共用同一个 `locale` 键
//! （存在 `config.data` 里），默认中文。

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// store 文件名，与前端 `getConfigStore()` 使用的一致
pub const CONFIG_STORE: &str = "config.data";

/// store 中保存语言的键，与前端 `LOCALE_STORAGE_KEY` 一致
pub const LOCALE_STORE_KEY: &str = "locale";

/// 默认语言：简体中文
pub const DEFAULT_LOCALE: &str = "zh-CN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    ZhCn,
    EnUs,
}

impl Locale {
    /// 把任意语言标记归一化为受支持的语言，无法识别时回退中文
    pub fn from_tag(raw: Option<&str>) -> Self {
        match raw {
            Some(tag) => {
                let tag = tag.trim().to_ascii_lowercase();
                if tag.starts_with("en") {
                    Self::EnUs
                } else {
                    Self::ZhCn
                }
            },
            None => Self::ZhCn,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::EnUs => "en-US",
        }
    }

    pub fn tray_texts(self) -> TrayTexts {
        match self {
            Self::ZhCn => TrayTexts {
                settings: "设置",
                check_update: "检查更新",
                quit: "退出",
                enable_autostart: "启用开机自启",
                disable_autostart: "禁用开机自启",
                update_failed: "检查更新失败",
                update_none: "没有可用的更新",
                update_window_title: "检查更新",
                update_check_title: "更新检查",
                update_error_title: "警告",
            },
            Self::EnUs => TrayTexts {
                settings: "Settings",
                check_update: "Check for Updates",
                quit: "Quit",
                enable_autostart: "Launch at startup",
                disable_autostart: "Don't launch at startup",
                update_failed: "Failed to check for updates",
                update_none: "No updates available",
                update_window_title: "Software Update",
                update_check_title: "Update Check",
                update_error_title: "Warning",
            },
        }
    }
}

/// 托盘与托盘相关对话框的文案
pub struct TrayTexts {
    pub settings: &'static str,
    pub check_update: &'static str,
    pub quit: &'static str,
    pub enable_autostart: &'static str,
    pub disable_autostart: &'static str,
    pub update_failed: &'static str,
    pub update_none: &'static str,
    pub update_window_title: &'static str,
    pub update_check_title: &'static str,
    pub update_error_title: &'static str,
}

/// 读取当前语言：store 优先，读不到时回退中文。
pub fn current_locale(app: &AppHandle) -> Locale {
    let raw = app
        .store(CONFIG_STORE)
        .inspect_err(|e| log::warn!("读取 store 失败，语言回退到默认值: {:?}", e))
        .ok()
        .and_then(|store| store.get(LOCALE_STORE_KEY))
        .and_then(|value| value.as_str().map(|s| s.to_string()));
    Locale::from_tag(raw.as_deref())
}

/// 持久化语言，供托盘在下次启动时读取。
pub fn save_locale(app: &AppHandle, locale: Locale) -> Result<(), String> {
    let store = app
        .store(CONFIG_STORE)
        .map_err(|e| format!("打开 store 失败: {e}"))?;
    store.set(LOCALE_STORE_KEY, serde_json::Value::from(locale.tag()));
    store.save().map_err(|e| format!("保存语言失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_chinese() {
        assert_eq!(Locale::from_tag(None), Locale::ZhCn);
        assert_eq!(Locale::from_tag(Some("")), Locale::ZhCn);
        assert_eq!(Locale::from_tag(Some("zh-CN")), Locale::ZhCn);
        assert_eq!(Locale::from_tag(Some("zh-Hans")), Locale::ZhCn);
    }

    #[test]
    fn recognises_english() {
        assert_eq!(Locale::from_tag(Some("en-US")), Locale::EnUs);
        assert_eq!(Locale::from_tag(Some("EN")), Locale::EnUs);
        assert_eq!(Locale::from_tag(Some(" en-gb ")), Locale::EnUs);
    }

    #[test]
    fn unknown_falls_back_to_chinese() {
        assert_eq!(Locale::from_tag(Some("fr-FR")), Locale::ZhCn);
        assert_eq!(Locale::from_tag(Some("日本語")), Locale::ZhCn);
    }

    #[test]
    fn tag_roundtrip() {
        for locale in [Locale::ZhCn, Locale::EnUs] {
            assert_eq!(Locale::from_tag(Some(locale.tag())), locale);
        }
    }
}

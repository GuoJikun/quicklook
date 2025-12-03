mod com_utils;
pub mod converter;
pub mod detector;
pub mod error;
mod ms_office;
mod wps_office;

pub use converter::{convert_to_html, convert_to_html_with_options, ConvertOptions};
pub use detector::{detect_office_apps, get_preferred_office, OfficeApp, OfficeInfo};
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_office() {
        let apps = detect_office_apps();
        println!("检测到的办公软件: {:?}", apps);
    }
}

pub mod audio;
pub mod config;
pub mod ffmp;
pub mod image;
pub mod monitor;
pub mod selected_file;
pub mod win;

#[allow(unused)]
pub fn get_scaled_size(size: f64, scale: f64) -> f64 {
    size / scale
}

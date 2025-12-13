//! 示例 1: 检测已安装的办公软件
//!
//! 运行方式:
//! ```bash
//! cargo run --example detect_office
//! ```

use office_converter::{detect_office_apps, detector::get_preferred_office, OfficeApp};

fn main() {
    println!("==========================================");
    println!("  办公软件检测示例");
    println!("==========================================\n");

    // 检测所有已安装的办公软件
    println!("🔍 正在检测已安装的办公软件...\n");
    let apps = detect_office_apps();

    if apps.is_empty() {
        println!("❌ 未检测到任何办公软件!");
        println!("   请确保已安装 Microsoft Office 或 WPS Office\n");
        return;
    }

    println!("✅ 检测到 {} 个办公软件:\n", apps.len());

    for (index, app) in apps.iter().enumerate() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📦 办公软件 #{}", index + 1);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        match app.app {
            OfficeApp::MsOffice => println!("   类型: 🏢 Microsoft Office"),
            OfficeApp::Wps => println!("   类型: 📝 WPS Office"),
        }

        println!("   版本: {}", app.version);
        println!("   路径: {}", app.install_path.display());

        // 检查安装路径是否存在
        if app.install_path.exists() {
            println!("   状态: ✅ 安装有效");
        } else {
            println!("   状态: ⚠️  路径不存在");
        }
        println!();
    }

    // 获取首选的办公软件
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 首选办公软件");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    match get_preferred_office() {
        Ok(office) => {
            match office.app {
                OfficeApp::MsOffice => println!("   推荐使用: 🏢 Microsoft Office"),
                OfficeApp::Wps => println!("   推荐使用: 📝 WPS Office"),
            }
            println!("   版本: {}", office.version);
            println!("   路径: {}", office.install_path.display());
        },
        Err(e) => {
            println!("   ❌ 获取失败: {}", e);
        },
    }

    println!("\n==========================================");
    println!("  检测完成!");
    println!("==========================================\n");
}

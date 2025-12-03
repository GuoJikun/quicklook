//! 示例 2: 将 Office 文档转换为 HTML
//!
//! 运行方式:
//! ```bash
//! cargo run --example convert_document
//! ```
//!
//! 或者指定文件路径:
//! ```bash
//! cargo run --example convert_document -- "C:\path\to\document.docx"
//! ```

use office_converter::{
    convert_to_html, convert_to_html_with_options, detect_office_apps, ConvertOptions, OfficeApp,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("==========================================");
    println!("  Office 文档转 HTML 示例");
    println!("==========================================\n");

    // 检查是否有办公软件
    let apps = detect_office_apps();
    if apps.is_empty() {
        println!("❌ 错误: 未检测到任何办公软件!");
        println!("   请先安装 Microsoft Office 或 WPS Office\n");
        return;
    }

    println!("✅ 检测到办公软件:");
    for app in &apps {
        match app.app {
            OfficeApp::MsOffice => println!("   - Microsoft Office {}", app.version),
            OfficeApp::Wps => println!("   - WPS Office {}", app.version),
        }
    }
    println!();

    // 获取文件路径（从命令行参数或使用默认路径）
    let args: Vec<String> = env::args().collect();
    let input_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        // 默认测试文件路径
        println!("💡 提示: 未指定文件路径，使用默认测试路径");
        println!("   你可以通过命令行参数指定文件:");
        println!("   cargo run --example convert_document -- \"your_file.docx\"\n");

        PathBuf::from("test.docx")
        // PathBuf::from("test.xlsx")
    };

    println!("📄 输入文件: {}", input_path.display());

    // 检查文件是否存在
    if !input_path.exists() {
        println!("❌ 错误: 文件不存在!");
        println!("\n请创建一个测试文件或指定现有文件路径");
        println!("支持的格式: .doc, .docx, .xls, .xlsx, .ppt, .pptx\n");

        // 提供创建测试文件的建议
        println!("建议:");
        println!("1. 在当前目录创建一个名为 'test.docx' 的 Word 文档");
        println!("2. 或者运行: cargo run --example convert_document -- \"path/to/your/file.docx\"");
        println!();
        return;
    }

    // 获取文件扩展名
    let extension = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    println!("📋 文件类型: {}", extension.to_uppercase());

    // 确定使用哪个办公软件（优先使用 Microsoft Office）
    let office_app = apps
        .iter()
        .find(|app| app.app == OfficeApp::MsOffice)
        .or_else(|| apps.first())
        .unwrap();

    println!("🔧 使用软件: {:?}", office_app.app);
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("开始转换...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 方法 1: 简单转换（HTML 字符串）
    println!("方法 1: 转换为 HTML 字符串");
    match convert_to_html(&input_path, office_app.app.clone()) {
        Ok(html) => {
            println!("✅ 转换成功!");
            println!("   HTML 长度: {} 字符", html.len());
            println!("   HTML 大小: {:.2} KB", html.len() as f64 / 1024.0);

            // 显示 HTML 预览
            let preview_len = html.len().min(300);
            println!("\n📝 HTML 预览 (前 {} 字符):", preview_len);
            println!("   {}", "-".repeat(60));
            for line in html[..preview_len].lines().take(10) {
                println!("   {}", line);
            }
            if html.len() > preview_len {
                println!("   ... (还有 {} 字符)", html.len() - preview_len);
            }
            println!("   {}", "-".repeat(60));
            println!();
        },
        Err(e) => {
            println!("❌ 转换失败: {}", e);
            return;
        },
    }

    // 方法 2: 使用自定义选项转换并保存到文件
    println!("\n方法 2: 转换并保存到文件");

    let mut output_path = input_path.clone();
    output_path.set_extension("html");

    let options = ConvertOptions {
        office_app: Some(office_app.app.clone()),
        output_path: Some(output_path.clone()),
        include_styles: true,
        include_images: true,
    };

    println!("💾 输出文件: {}", output_path.display());

    match convert_to_html_with_options(&input_path, options) {
        Ok(html) => {
            println!("✅ 转换并保存成功!");

            // 获取输出文件信息
            if let Ok(metadata) = fs::metadata(&output_path) {
                println!("   文件大小: {:.2} KB", metadata.len() as f64 / 1024.0);
            }

            println!("   保存位置: {}", output_path.display());

            // 统计一些信息
            let line_count = html.lines().count();
            println!("\n📊 文件统计:");
            println!("   总字符数: {}", html.len());
            println!("   总行数: {}", line_count);

            // 检查常见的 HTML 元素
            if html.contains("<table") {
                println!("   包含表格: ✅");
            }
            if html.contains("<img") {
                let img_count = html.matches("<img").count();
                println!("   包含图片: ✅ ({} 个)", img_count);
            }
            if html.contains("<style") || html.contains("style=") {
                println!("   包含样式: ✅");
            }
        },
        Err(e) => {
            println!("❌ 转换失败: {}", e);
            return;
        },
    }

    println!("\n==========================================");
    println!("  转换完成!");
    println!("==========================================");
    println!("\n💡 提示:");
    println!("   你可以用浏览器打开生成的 HTML 文件查看效果");
    println!("   文件路径: {}\n", output_path.display());
}

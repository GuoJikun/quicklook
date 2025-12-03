# Office Converter 示例运行脚本

Write-Host "===========================================" -ForegroundColor Cyan
Write-Host "  Office Converter 示例程序" -ForegroundColor Cyan
Write-Host "===========================================" -ForegroundColor Cyan
Write-Host ""

# 进入 office-converter 目录
Set-Location "e:\private\Rust\quicklook\crates\office-converter"

Write-Host "请选择要运行的示例:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  1. 检测办公软件 (detect_office)" -ForegroundColor Green
Write-Host "  2. 转换文档为 HTML (convert_document)" -ForegroundColor Green
Write-Host "  3. 退出" -ForegroundColor Red
Write-Host ""

$choice = Read-Host "请输入选项 (1-3)"

switch ($choice) {
    "1" {
        Write-Host ""
        Write-Host "运行: 检测办公软件..." -ForegroundColor Cyan
        Write-Host ""
        cargo run --example detect_office
    }
    "2" {
        Write-Host ""
        Write-Host "请选择:" -ForegroundColor Yellow
        Write-Host "  1. 使用默认测试文件" -ForegroundColor Green
        Write-Host "  2. 指定文件路径" -ForegroundColor Green
        Write-Host ""
        $subChoice = Read-Host "请输入选项 (1-2)"
        
        if ($subChoice -eq "1") {
            Write-Host ""
            Write-Host "运行: 转换文档 (使用默认文件)..." -ForegroundColor Cyan
            Write-Host ""
            cargo run --example convert_document
        }
        elseif ($subChoice -eq "2") {
            Write-Host ""
            $filePath = Read-Host "请输入文件路径"
            Write-Host ""
            Write-Host "运行: 转换文档 ($filePath)..." -ForegroundColor Cyan
            Write-Host ""
            cargo run --example convert_document -- "$filePath"
        }
        else {
            Write-Host "无效的选项!" -ForegroundColor Red
        }
    }
    "3" {
        Write-Host ""
        Write-Host "再见!" -ForegroundColor Green
        exit
    }
    default {
        Write-Host ""
        Write-Host "无效的选项!" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "===========================================" -ForegroundColor Cyan
Write-Host "  完成!" -ForegroundColor Cyan
Write-Host "===========================================" -ForegroundColor Cyan
Write-Host ""

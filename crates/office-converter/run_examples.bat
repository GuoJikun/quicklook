@echo off
chcp 65001 >nul
cd /d "%~dp0"

echo ==========================================
echo   Office Converter 示例程序
echo ==========================================
echo.

:menu
echo 请选择要运行的示例:
echo.
echo   1. 检测办公软件
echo   2. 转换文档为 HTML
echo   3. 退出
echo.

set /p choice="请输入选项 (1-3): "

if "%choice%"=="1" goto detect
if "%choice%"=="2" goto convert
if "%choice%"=="3" goto end
echo 无效的选项!
echo.
goto menu

:detect
echo.
echo 运行: 检测办公软件...
echo.
cargo run --example detect_office
goto done

:convert
echo.
echo 请选择:
echo   1. 使用默认测试文件
echo   2. 指定文件路径
echo.
set /p subchoice="请输入选项 (1-2): "

if "%subchoice%"=="1" (
    echo.
    echo 运行: 转换文档 ^(使用默认文件^)...
    echo.
    cargo run --example convert_document
    goto done
)

if "%subchoice%"=="2" (
    echo.
    set /p filepath="请输入文件路径: "
    echo.
    echo 运行: 转换文档...
    echo.
    cargo run --example convert_document -- "!filepath!"
    goto done
)

echo 无效的选项!
goto menu

:done
echo.
echo ==========================================
echo   完成!
echo ==========================================
echo.
pause
goto end

:end

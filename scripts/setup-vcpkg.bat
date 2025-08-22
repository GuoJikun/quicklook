@echo off
REM Setup vcpkg for QuickLook development

echo Setting up vcpkg for QuickLook development...

REM Check if vcpkg directory exists
if not exist "vcpkg" (
    echo Cloning vcpkg repository...
    git clone https://github.com/microsoft/vcpkg.git
    cd vcpkg
    call bootstrap-vcpkg.bat
    cd ..
) else (
    echo vcpkg directory already exists, updating...
    cd vcpkg
    git pull
    cd ..
)

REM Set VCPKG_ROOT environment variable for current session
set VCPKG_ROOT=%CD%\vcpkg

REM Install dependencies based on architecture
echo Installing FFmpeg dependencies...

REM Detect architecture
if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
    echo Detected x64 architecture
    vcpkg\vcpkg install --triplet x64-windows-static-md
) else if "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    echo Detected ARM64 architecture  
    vcpkg\vcpkg install --triplet aarch64-windows-static-md
) else (
    echo Warning: Unknown architecture %PROCESSOR_ARCHITECTURE%, defaulting to x64
    vcpkg\vcpkg install --triplet x64-windows-static-md
)

echo.
echo ============================================
echo vcpkg setup complete!
echo.
echo To use vcpkg in future sessions, run:
echo set VCPKG_ROOT=%CD%\vcpkg
echo.
echo Or add this to your system environment variables.
echo ============================================
pause
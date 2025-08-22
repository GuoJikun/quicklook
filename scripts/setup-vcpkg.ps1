# Setup vcpkg for QuickLook development
# PowerShell script for Windows

Write-Host "Setting up vcpkg for QuickLook development..." -ForegroundColor Green

# Check if vcpkg directory exists
if (-not (Test-Path "vcpkg")) {
    Write-Host "Cloning vcpkg repository..." -ForegroundColor Yellow
    git clone https://github.com/microsoft/vcpkg.git
    Set-Location vcpkg
    .\bootstrap-vcpkg.bat
    Set-Location ..
} else {
    Write-Host "vcpkg directory already exists, updating..." -ForegroundColor Yellow
    Set-Location vcpkg
    git pull
    Set-Location ..
}

# Set VCPKG_ROOT environment variable for current session
$env:VCPKG_ROOT = Join-Path (Get-Location) "vcpkg"

Write-Host "Installing FFmpeg dependencies..." -ForegroundColor Yellow

# Detect architecture
$arch = $env:PROCESSOR_ARCHITECTURE
$triplet = switch ($arch) {
    "AMD64" { 
        Write-Host "Detected x64 architecture" -ForegroundColor Cyan
        "x64-windows-static-md" 
    }
    "ARM64" { 
        Write-Host "Detected ARM64 architecture" -ForegroundColor Cyan
        "aarch64-windows-static-md" 
    }
    default { 
        Write-Host "Warning: Unknown architecture $arch, defaulting to x64" -ForegroundColor Yellow
        "x64-windows-static-md" 
    }
}

Write-Host "Using triplet: $triplet" -ForegroundColor Cyan
& .\vcpkg\vcpkg.exe install --triplet $triplet

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "vcpkg setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To use vcpkg in future sessions, run:" -ForegroundColor Yellow
Write-Host "`$env:VCPKG_ROOT = `"$($env:VCPKG_ROOT)`"" -ForegroundColor Cyan
Write-Host ""
Write-Host "Or add this to your PowerShell profile." -ForegroundColor Yellow
Write-Host "============================================" -ForegroundColor Green

# Ask to set environment variable permanently
$response = Read-Host "Would you like to set VCPKG_ROOT environment variable permanently? (y/N)"
if ($response -match '^[Yy]') {
    [Environment]::SetEnvironmentVariable("VCPKG_ROOT", $env:VCPKG_ROOT, [EnvironmentVariableTarget]::User)
    Write-Host "VCPKG_ROOT has been set permanently for current user." -ForegroundColor Green
} else {
    Write-Host "VCPKG_ROOT is only set for current session." -ForegroundColor Yellow
}
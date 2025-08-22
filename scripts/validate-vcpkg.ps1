# QuickLook vcpkg Setup Validation Script
# PowerShell script to validate vcpkg configuration

Write-Host "Validating vcpkg setup for QuickLook..." -ForegroundColor Green

$exitCode = 0

# Check VCPKG_ROOT environment variable
Write-Host "`n1. Checking VCPKG_ROOT environment variable..." -ForegroundColor Yellow
if ($env:VCPKG_ROOT) {
    Write-Host "   ✓ VCPKG_ROOT is set to: $($env:VCPKG_ROOT)" -ForegroundColor Green
    
    if (Test-Path $env:VCPKG_ROOT) {
        Write-Host "   ✓ VCPKG_ROOT directory exists" -ForegroundColor Green
    } else {
        Write-Host "   ✗ VCPKG_ROOT directory does not exist" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "   ✗ VCPKG_ROOT environment variable is not set" -ForegroundColor Red
    $exitCode = 1
}

# Check vcpkg executable
Write-Host "`n2. Checking vcpkg executable..." -ForegroundColor Yellow
$vcpkgExe = Join-Path $env:VCPKG_ROOT "vcpkg.exe"
if (Test-Path $vcpkgExe) {
    Write-Host "   ✓ vcpkg.exe found" -ForegroundColor Green
    
    # Get vcpkg version
    try {
        $version = & $vcpkgExe version 2>$null
        Write-Host "   ✓ vcpkg version: $($version.Split("`n")[0])" -ForegroundColor Green
    } catch {
        Write-Host "   ⚠ Could not get vcpkg version" -ForegroundColor Yellow
    }
} else {
    Write-Host "   ✗ vcpkg.exe not found" -ForegroundColor Red
    $exitCode = 1
}

# Check project manifest files
Write-Host "`n3. Checking project manifest files..." -ForegroundColor Yellow

if (Test-Path "vcpkg.json") {
    Write-Host "   ✓ vcpkg.json found" -ForegroundColor Green
    
    # Validate JSON syntax
    try {
        $manifest = Get-Content "vcpkg.json" | ConvertFrom-Json
        Write-Host "   ✓ vcpkg.json is valid JSON" -ForegroundColor Green
        Write-Host "   ✓ Project: $($manifest.name) v$($manifest.version)" -ForegroundColor Green
    } catch {
        Write-Host "   ✗ vcpkg.json contains invalid JSON" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "   ✗ vcpkg.json not found" -ForegroundColor Red
    $exitCode = 1
}

if (Test-Path "vcpkg-configuration.json") {
    Write-Host "   ✓ vcpkg-configuration.json found" -ForegroundColor Green
} else {
    Write-Host "   ⚠ vcpkg-configuration.json not found (optional)" -ForegroundColor Yellow
}

# Check installed packages
Write-Host "`n4. Checking installed packages..." -ForegroundColor Yellow
if ($env:VCPKG_ROOT -and (Test-Path $vcpkgExe)) {
    try {
        $packages = & $vcpkgExe list 2>$null | Where-Object { $_ -match "ffmpeg" }
        if ($packages) {
            Write-Host "   ✓ FFmpeg packages found:" -ForegroundColor Green
            foreach ($pkg in $packages) {
                Write-Host "     - $pkg" -ForegroundColor Cyan
            }
        } else {
            Write-Host "   ⚠ No FFmpeg packages installed" -ForegroundColor Yellow
            Write-Host "     Run: vcpkg install --triplet x64-windows-static-md" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "   ✗ Could not check installed packages" -ForegroundColor Red
        $exitCode = 1
    }
}

# Check Rust integration
Write-Host "`n5. Checking Rust integration..." -ForegroundColor Yellow

if (Test-Path "src-tauri/Cargo.toml") {
    $cargoContent = Get-Content "src-tauri/Cargo.toml" -Raw
    if ($cargoContent -match 'vcpkg\s*=') {
        Write-Host "   ✓ vcpkg crate dependency found in Cargo.toml" -ForegroundColor Green
    } else {
        Write-Host "   ✗ vcpkg crate dependency not found in Cargo.toml" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "   ✗ src-tauri/Cargo.toml not found" -ForegroundColor Red
    $exitCode = 1
}

if (Test-Path "src-tauri/build.rs") {
    $buildContent = Get-Content "src-tauri/build.rs" -Raw
    if ($buildContent -match 'vcpkg::') {
        Write-Host "   ✓ vcpkg integration found in build.rs" -ForegroundColor Green
    } else {
        Write-Host "   ✗ vcpkg integration not found in build.rs" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "   ✗ src-tauri/build.rs not found" -ForegroundColor Red
    $exitCode = 1
}

# Summary
Write-Host "`n============================================" -ForegroundColor Green
if ($exitCode -eq 0) {
    Write-Host "✅ vcpkg setup validation passed!" -ForegroundColor Green
    Write-Host "You can now build the project with: pnpm tauri build" -ForegroundColor Cyan
} else {
    Write-Host "❌ vcpkg setup validation failed!" -ForegroundColor Red
    Write-Host "Please fix the issues above before building." -ForegroundColor Yellow
    Write-Host "For detailed setup instructions, see: VCPKG.md" -ForegroundColor Cyan
}
Write-Host "============================================" -ForegroundColor Green

exit $exitCode
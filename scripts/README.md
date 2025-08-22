# QuickLook Development Scripts

This directory contains helpful scripts for setting up and managing the QuickLook development environment.

## Scripts

### vcpkg Setup Scripts

These scripts help set up vcpkg for managing FFmpeg and other C++ dependencies:

- **`setup-vcpkg.ps1`** - PowerShell script for Windows (recommended for Windows)
- **`setup-vcpkg.bat`** - Batch script for Windows (legacy)
- **`setup-vcpkg.sh`** - Shell script for Linux/macOS

#### Usage

```bash
# Windows (PowerShell)
.\scripts\setup-vcpkg.ps1

# Windows (Command Prompt)
.\scripts\setup-vcpkg.bat

# Linux/macOS
./scripts/setup-vcpkg.sh
```

### Validation Scripts

- **`validate-vcpkg.ps1`** - Validates vcpkg configuration and dependencies

#### Usage

```bash
# Windows PowerShell
.\scripts\validate-vcpkg.ps1
```

## What These Scripts Do

1. **Setup Scripts**:
   - Clone the vcpkg repository if not present
   - Bootstrap vcpkg for the current platform
   - Install FFmpeg dependencies with the correct triplet
   - Set up environment variables (optionally permanent)
   - Detect system architecture automatically

2. **Validation Script**:
   - Checks VCPKG_ROOT environment variable
   - Verifies vcpkg executable exists and works
   - Validates project manifest files (vcpkg.json)
   - Lists installed packages
   - Checks Rust/Cargo integration

## Requirements

- **Windows**: PowerShell 5.0+ or Command Prompt
- **Linux/macOS**: Bash shell
- **Git**: Required for cloning vcpkg repository
- **Visual Studio Build Tools** (Windows): Required for building C++ packages

## Troubleshooting

If scripts fail, check:

1. Internet connection for downloading vcpkg
2. Sufficient disk space (vcpkg packages can be large)
3. Required build tools are installed
4. Windows: Execution policy allows running scripts
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

For more detailed information, see [VCPKG.md](../VCPKG.md) in the project root.
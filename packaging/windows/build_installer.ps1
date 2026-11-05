<#
.SYNOPSIS
    Builds the release binary and packages all Windows installer formats.
#>
$ErrorActionPreference = "Stop"

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "   LogicCraft CAD - Windows Installer Build Suite      " -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Resolve-Path "$ScriptDir\..\.."

Push-Location $ProjectRoot
try {
    Write-Host "[1/4] Compiling optimized release binaries (cargo build --release --bins)..." -ForegroundColor Yellow
    cargo build --release --bins

    $OutputDir = "$ProjectRoot\target\installer"
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    }

    Write-Host "[2/4] Checking available installer compilers..." -ForegroundColor Yellow

    # Check Inno Setup Compiler (iscc.exe)
    $iscc = Get-Command "iscc.exe" -ErrorAction SilentlyContinue
    if (-not $iscc) {
        $candidatePaths = @(
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\iscc.exe",
            "C:\Program Files (x86)\Inno Setup 6\iscc.exe",
            "C:\Program Files\Inno Setup 6\iscc.exe"
        )
        foreach ($cp in $candidatePaths) {
            if (Test-Path $cp) {
                $iscc = [PSCustomObject]@{ Source = $cp }
                break
            }
        }
    }
    if ($iscc) {
        Write-Host "[*] Found Inno Setup compiler ($($iscc.Source)). Building EXE Setup..." -ForegroundColor Green
        & $iscc.Source "packaging\windows\installer.iss"
        if ($LASTEXITCODE -eq 0) {
            Write-Host "[SUCCESS] Built target\installer\LogicCraft-Setup-1.0.0-x64.exe" -ForegroundColor Green
        }
    } else {
        Write-Host "[-] Inno Setup (iscc.exe) not in PATH. Skipping .iss compilation." -ForegroundColor Gray
    }

    # Check WiX Toolset (candle.exe and light.exe)
    $candle = Get-Command "candle.exe" -ErrorAction SilentlyContinue
    $light = Get-Command "light.exe" -ErrorAction SilentlyContinue
    if ($candle -and $light) {
        Write-Host "[*] Found WiX Toolset. Building MSI Installer..." -ForegroundColor Green
        Push-Location "packaging\windows"
        try {
            & $candle.Source -arch x64 -out "$OutputDir\installer.wixobj" "installer.wxs"
            if ($LASTEXITCODE -eq 0) {
                & $light.Source -ext WixUIExtension -out "$OutputDir\LogicCraft-1.0.0-x64.msi" "$OutputDir\installer.wixobj"
                if ($LASTEXITCODE -eq 0) {
                    Write-Host "[SUCCESS] Built target\installer\LogicCraft-1.0.0-x64.msi" -ForegroundColor Green
                }
            }
        } catch {
            Write-Host "[-] WiX compilation failed: $_" -ForegroundColor Yellow
        } finally {
            Pop-Location
            $global:LASTEXITCODE = 0
        }
    } else {
        Write-Host "[-] WiX Toolset (candle.exe / light.exe) not in PATH. Skipping .msi compilation." -ForegroundColor Gray
    }

    Write-Host "[3/4] Creating standalone Portable ZIP distribution package..." -ForegroundColor Yellow
    $ZipDest = "$OutputDir\LogicCraft-v1.0.0-windows-x64.zip"
    if (Test-Path $ZipDest) {
        Remove-Item $ZipDest -Force
    }

    $StageDir = "$OutputDir\stage_win"
    if (Test-Path $StageDir) {
        Remove-Item $StageDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

    Copy-Item "target\release\logiccraft.exe" $StageDir -Force
    if (Test-Path "target\release\logiccraft-updater.exe") {
        Copy-Item "target\release\logiccraft-updater.exe" $StageDir -Force
    }
    Copy-Item "packaging\assets\icon.ico" $StageDir -Force
    Copy-Item "packaging\windows\install.ps1" $StageDir -Force
    Copy-Item "packaging\windows\update.ps1" $StageDir -Force
    Copy-Item "packaging\windows\uninstall.ps1" $StageDir -Force
    if (Test-Path "mechanics.json") {
        Copy-Item "mechanics.json" $StageDir -Force
    }
    if (Test-Path "MECHANICS.md") {
        Copy-Item "MECHANICS.md" $StageDir -Force
    }

    Compress-Archive -Path "$StageDir\*" -DestinationPath $ZipDest -Force
    Remove-Item $StageDir -Recurse -Force
    Write-Host "[SUCCESS] Created Portable Package: target\installer\LogicCraft-v1.0.0-windows-x64.zip" -ForegroundColor Green

    Write-Host ""
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host " Windows Packaging Complete!" -ForegroundColor Green
    Write-Host " Output Directory: $OutputDir" -ForegroundColor White
    Write-Host " Scripts Available:" -ForegroundColor White
    Write-Host "   - Inno Setup Script: packaging\windows\installer.iss" -ForegroundColor White
    Write-Host "   - WiX MSI Script:   packaging\windows\installer.wxs" -ForegroundColor White
    Write-Host "   - Native Installer: packaging\windows\install.ps1" -ForegroundColor White
    Write-Host "========================================================" -ForegroundColor Cyan
} finally {
    Pop-Location
}

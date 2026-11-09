<#
.SYNOPSIS
    LogicCraft CAD - Native Windows Application Updater Script
.DESCRIPTION
    Updates an existing LogicCraft CAD installation with the latest release binaries,
    preserves user configuration (%APPDATA%\LogicCraft\settings.json), and restarts the app.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File update.ps1
    powershell -ExecutionPolicy Bypass -File update.ps1 -InstallDir "C:\LogicCraft"
#>
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\LogicCraft",
    [switch]$NoLaunch = $false
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "    LogicCraft CAD - Native Windows Updater Script        " -ForegroundColor Cyan
Write-Host "    Version: 1.0.3 (Latest Release)                       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host ""

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Resolve-Path "$ScriptDir\..\.."
$ReleaseExe = "$ProjectRoot\target\release\logiccraft.exe"
$UpdaterExe = "$ProjectRoot\target\release\logiccraft-updater.exe"
$IconFile = "$ScriptDir\..\assets\icon.ico"

# Step 1: Ensure release binaries are built
if ((-not (Test-Path $ReleaseExe)) -or (-not (Test-Path $UpdaterExe))) {
    Write-Host "[1/5] Compiling latest release binaries (cargo build --release --bins)..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        cargo build --release --bins
    } finally {
        Pop-Location
    }
} else {
    Write-Host "[1/5] Release binaries detected at $ProjectRoot\target\release" -ForegroundColor Green
}

# Step 2: Stop any running LogicCraft CAD process to release file lock
Write-Host "[2/5] Closing running LogicCraft instances..." -ForegroundColor Yellow
Get-Process "logiccraft" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host " [*] Stopping PID $($_.Id)..." -ForegroundColor Yellow
    $_ | Stop-Process -Force
}
Start-Sleep -Milliseconds 400

# Step 3: Backup previous installation if present
Write-Host "[3/5] Verifying target directory and creating backup..." -ForegroundColor Yellow
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

$BackupDir = "$InstallDir\backup"
if (-not (Test-Path $BackupDir)) {
    New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
}

if (Test-Path "$InstallDir\logiccraft.exe") {
    Copy-Item -Path "$InstallDir\logiccraft.exe" -Destination "$BackupDir\logiccraft.exe.bak" -Force
    Write-Host " [OK] Backed up previous executable to $BackupDir\logiccraft.exe.bak" -ForegroundColor Green
}

# Step 4: Copy updated binaries and assets
Write-Host "[4/5] Installing updated application binaries and assets..." -ForegroundColor Yellow
Copy-Item -Path $ReleaseExe -Destination "$InstallDir\logiccraft.exe" -Force
Write-Host " [OK] Copied logiccraft.exe" -ForegroundColor Green

if (Test-Path $UpdaterExe) {
    Copy-Item -Path $UpdaterExe -Destination "$InstallDir\logiccraft-updater.exe" -Force
    Write-Host " [OK] Copied logiccraft-updater.exe" -ForegroundColor Green
}

if (Test-Path $IconFile) {
    Copy-Item -Path $IconFile -Destination "$InstallDir\icon.ico" -Force
}

if (Test-Path "$ProjectRoot\mechanics.json") {
    Copy-Item -Path "$ProjectRoot\mechanics.json" -Destination "$InstallDir\mechanics.json" -Force
}

if (Test-Path "$ProjectRoot\MECHANICS.md") {
    Copy-Item -Path "$ProjectRoot\MECHANICS.md" -Destination "$InstallDir\MECHANICS.md" -Force
}

Copy-Item -Path "$ScriptDir\update.ps1" -Destination "$InstallDir\update.ps1" -Force
Copy-Item -Path "$ScriptDir\uninstall.ps1" -Destination "$InstallDir\uninstall.ps1" -Force

# Step 5: Verify user settings persistence
Write-Host "[5/5] Checking persistent settings..." -ForegroundColor Yellow
$SettingsFile = "$env:APPDATA\LogicCraft\settings.json"
if (Test-Path $SettingsFile) {
    Write-Host " [OK] User settings file preserved intact at: $SettingsFile" -ForegroundColor Green
} else {
    Write-Host " [i] User settings will be generated on first start at: $SettingsFile" -ForegroundColor Gray
}

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " LogicCraft CAD update successfully applied!" -ForegroundColor Green
Write-Host " Target Location: $InstallDir\logiccraft.exe" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Cyan

# Launch application
if (-not $NoLaunch) {
    Write-Host "[*] Launching LogicCraft CAD..." -ForegroundColor Cyan
    Start-Process -FilePath "$InstallDir\logiccraft.exe"
}

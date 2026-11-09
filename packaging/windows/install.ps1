<#
.SYNOPSIS
    LogicCraft CAD - Native Windows Installer Script
.DESCRIPTION
    Installs the LogicCraft release build to the user's application directory
    (%LOCALAPPDATA%\Programs\LogicCraft or Program Files with Admin), creates Start Menu
    and Desktop shortcuts, registers the .cdl file association, and sets up uninstaller.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File install.ps1
    powershell -ExecutionPolicy Bypass -File install.ps1 -DesktopShortcut
#>
param(
    [switch]$DesktopShortcut = $true,
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\LogicCraft"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  LogicCraft CAD - Windows Installer   " -ForegroundColor Cyan
Write-Host "  Version: 1.0.4                       " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ProjectRoot = Resolve-Path "$ScriptDir\..\.."
$ReleaseExe = "$ProjectRoot\target\release\logiccraft.exe"
$IconFile = "$ScriptDir\..\assets\icon.ico"

# Verify or compile release binary
if (-not (Test-Path $ReleaseExe)) {
    Write-Host "[*] Release binary not found at $ReleaseExe" -ForegroundColor Yellow
    Write-Host "[*] Building release binary with Cargo (cargo build --release)..." -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        cargo build --release
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $ReleaseExe)) {
    Write-Error "Failed to locate release executable at $ReleaseExe. Build aborted."
}

Write-Host "[1/5] Creating installation directory at $InstallDir..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "[2/5] Copying application files..." -ForegroundColor Green
Copy-Item -Path $ReleaseExe -Destination "$InstallDir\logiccraft.exe" -Force
$UpdaterExe = "$ProjectRoot\target\release\logiccraft-updater.exe"
if (Test-Path $UpdaterExe) {
    Copy-Item -Path $UpdaterExe -Destination "$InstallDir\logiccraft-updater.exe" -Force
}
if (Test-Path "$ScriptDir\update.ps1") {
    Copy-Item -Path "$ScriptDir\update.ps1" -Destination "$InstallDir\update.ps1" -Force
}
if (Test-Path $IconFile) {
    Copy-Item -Path $IconFile -Destination "$InstallDir\icon.ico" -Force
}
if (Test-Path "$ProjectRoot\mechanics.json") {
    Copy-Item -Path "$ProjectRoot\mechanics.json" -Destination "$InstallDir\mechanics.json" -Force
}

Write-Host "[3/5] Creating Start Menu shortcut..." -ForegroundColor Green
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\LogicCraft CAD"
New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null

$WshShell = New-Object -ComObject WScript.Shell
$StartShortcut = $WshShell.CreateShortcut("$StartMenuDir\LogicCraft CAD.lnk")
$StartShortcut.TargetPath = "$InstallDir\logiccraft.exe"
$StartShortcut.WorkingDirectory = $InstallDir
$StartShortcut.Description = "LogicCraft CAD - Digital Logic Circuit Studio"
if (Test-Path "$InstallDir\icon.ico") {
    $StartShortcut.IconLocation = "$InstallDir\icon.ico,0"
}
$StartShortcut.Save()

if (Test-Path "$InstallDir\logiccraft-updater.exe") {
    $UpdaterShortcut = $WshShell.CreateShortcut("$StartMenuDir\LogicCraft Updater.lnk")
    $UpdaterShortcut.TargetPath = "$InstallDir\logiccraft-updater.exe"
    $UpdaterShortcut.WorkingDirectory = $InstallDir
    $UpdaterShortcut.Description = "LogicCraft CAD - Native Application Updater"
    if (Test-Path "$InstallDir\icon.ico") {
        $UpdaterShortcut.IconLocation = "$InstallDir\icon.ico,0"
    }
    $UpdaterShortcut.Save()
}

if ($DesktopShortcut) {
    Write-Host "[4/5] Creating Desktop shortcut..." -ForegroundColor Green
    $DesktopDir = [Environment]::GetFolderPath("Desktop")
    $DeskShortcut = $WshShell.CreateShortcut("$DesktopDir\LogicCraft CAD.lnk")
    $DeskShortcut.TargetPath = "$InstallDir\logiccraft.exe"
    $DeskShortcut.WorkingDirectory = $InstallDir
    $DeskShortcut.Description = "LogicCraft CAD - Digital Logic Circuit Studio"
    if (Test-Path "$InstallDir\icon.ico") {
        $DeskShortcut.IconLocation = "$InstallDir\icon.ico,0"
    }
    $DeskShortcut.Save()
}

Write-Host "[5/5] Registering .lcc file association and Uninstaller in Windows Registry..." -ForegroundColor Green
# Register .lcc file extension for current user
$RegPath = "HKCU:\Software\Classes"
New-Item -Path "$RegPath\.lcc" -Force | Out-Null
Set-ItemProperty -Path "$RegPath\.lcc" -Name "(Default)" -Value "LogicCraft.Circuit" -Force

New-Item -Path "$RegPath\LogicCraft.Circuit" -Force | Out-Null
Set-ItemProperty -Path "$RegPath\LogicCraft.Circuit" -Name "(Default)" -Value "LogicCraft Circuit File" -Force
if (Test-Path "$InstallDir\icon.ico") {
    New-Item -Path "$RegPath\LogicCraft.Circuit\DefaultIcon" -Force | Out-Null
    Set-ItemProperty -Path "$RegPath\LogicCraft.Circuit\DefaultIcon" -Name "(Default)" -Value "$InstallDir\icon.ico,0" -Force
}
New-Item -Path "$RegPath\LogicCraft.Circuit\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$RegPath\LogicCraft.Circuit\shell\open\command" -Name "(Default)" -Value "`"$InstallDir\logiccraft.exe`" `"%1`"" -Force

# Generate uninstall script in install folder
Copy-Item -Path "$ScriptDir\uninstall.ps1" -Destination "$InstallDir\uninstall.ps1" -Force

# Register in Windows Add/Remove Programs
$UninstallReg = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LogicCraft"
New-Item -Path $UninstallReg -Force | Out-Null
Set-ItemProperty -Path $UninstallReg -Name "DisplayName" -Value "LogicCraft CAD" -Force
Set-ItemProperty -Path $UninstallReg -Name "DisplayVersion" -Value "1.0.2" -Force
Set-ItemProperty -Path $UninstallReg -Name "Publisher" -Value "Craft Anything" -Force
Set-ItemProperty -Path $UninstallReg -Name "InstallLocation" -Value $InstallDir -Force
Set-ItemProperty -Path $UninstallReg -Name "DisplayIcon" -Value "$InstallDir\icon.ico,0" -Force
Set-ItemProperty -Path $UninstallReg -Name "UninstallString" -Value "powershell.exe -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`"" -Force
Set-ItemProperty -Path $UninstallReg -Name "QuietUninstallString" -Value "powershell.exe -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`" -Silent" -Force

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " LogicCraft CAD installed successfully!" -ForegroundColor Green
Write-Host " Application Path: $InstallDir\logiccraft.exe" -ForegroundColor White
Write-Host " Shortcuts: Start Menu$(if ($DesktopShortcut) { ', Desktop' })" -ForegroundColor White
Write-Host " File Association: *.lcc (LogicCraft Circuit)" -ForegroundColor White
Write-Host "==========================================================" -ForegroundColor Cyan

<#
.SYNOPSIS
    LogicCraft CAD - Native Windows Uninstaller Script
#>
param(
    [switch]$Silent = $false
)

$ErrorActionPreference = "SilentlyContinue"

if (-not $Silent) {
    Write-Host "Uninstalling LogicCraft CAD..." -ForegroundColor Yellow
}

$InstallDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

# 1. Remove Start Menu folder
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\LogicCraft CAD"
if (Test-Path $StartMenuDir) {
    Remove-Item -Path $StartMenuDir -Recurse -Force
}

# 2. Remove Desktop shortcut
$DesktopDir = [Environment]::GetFolderPath("Desktop")
if (Test-Path "$DesktopDir\LogicCraft CAD.lnk") {
    Remove-Item -Path "$DesktopDir\LogicCraft CAD.lnk" -Force
}

# 3. Remove Registry associations
Remove-Item -Path "HKCU:\Software\Classes\.cdl" -Recurse -Force
Remove-Item -Path "HKCU:\Software\Classes\LogicCraft.Circuit" -Recurse -Force
Remove-Item -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LogicCraft" -Recurse -Force

# 4. Remove installation files (schedule directory removal after process exits)
if (-not $Silent) {
    Write-Host "LogicCraft CAD uninstalled successfully." -ForegroundColor Green
}

# Remove install directory via detached command
Start-Process cmd.exe -ArgumentList "/c timeout /t 1 /nobreak >nul & rmdir /s /q `"$InstallDir`"" -WindowStyle Hidden

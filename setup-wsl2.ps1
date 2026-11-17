# setup-wsl2.ps1 — Install Ubuntu in WSL2 and set up the ARKON build environment.
# Run this script as Administrator in PowerShell.
#
# Usage (from an elevated PowerShell):
#   Set-ExecutionPolicy Bypass -Scope Process -Force
#   .\setup-wsl2.ps1

#Requires -RunAsAdministrator

Write-Host "==> Installing Ubuntu 24.04 in WSL2..." -ForegroundColor Cyan
wsl --install -d Ubuntu-24.04

Write-Host ""
Write-Host "==> Ubuntu installed. Setting up build environment inside WSL2..." -ForegroundColor Cyan
Write-Host "    This will install podman and run the ARKON ISO build." -ForegroundColor Cyan

# Convert Windows path to WSL path
$repoPath = (Get-Item "$PSScriptRoot").FullName -replace '\\', '/' -replace '^([A-Za-z]):', '/mnt/$1'.ToLower()
$driveLetter = ($PSScriptRoot -split ':')[0].ToLower()
$wslRepoPath = "/mnt/$driveLetter" + ($PSScriptRoot -replace '^[A-Za-z]:', '' -replace '\\', '/')

Write-Host "    Repo path in WSL: $wslRepoPath" -ForegroundColor Gray

wsl -d Ubuntu-24.04 -- bash -c @"
set -e
echo '==> Updating packages...'
sudo apt-get update -qq
sudo apt-get install -y podman qemu-utils

echo '==> Building ARKON ISO...'
cd '$wslRepoPath'
bash build/scripts/build-image.sh arkon
bash build/scripts/build-iso.sh arkon

echo '==> Done. ISO is in artifacts/iso/'
ls -lh artifacts/iso/
"@

Write-Host ""
Write-Host "==> Build complete. ISO is in: $PSScriptRoot\artifacts\iso\" -ForegroundColor Green

# n3v3 Windows Installer
# Usage: irm https://raw.githubusercontent.com/MCB-SMART-BOY/n3v3/master/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "    _   __                " -ForegroundColor Cyan
Write-Host "   / | / /___  _   _____  " -ForegroundColor Cyan
Write-Host "  /  |/ / _ \| | / / _ \ " -ForegroundColor Cyan
Write-Host " / /|  /  __/| |/ /  __/ " -ForegroundColor Cyan
Write-Host "/_/ |_/\___/ |___/\___/  " -ForegroundColor Cyan
Write-Host ""
Write-Host "n3v3 Installer for Windows" -ForegroundColor Green
Write-Host ""

# Get latest release
Write-Host "Fetching latest release..." -ForegroundColor Yellow
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/MCB-SMART-BOY/n3v3/releases/latest"
$version = $release.tag_name
Write-Host "Latest version: $version" -ForegroundColor Green

# Find Windows asset
$asset = $release.assets | Where-Object { $_.name -like "*windows*.zip" }
if (-not $asset) {
    Write-Host "Error: Windows build not found in release" -ForegroundColor Red
    exit 1
}

$downloadUrl = $asset.browser_download_url
$fileName = $asset.name

# Create install directory
$installDir = "$env:LOCALAPPDATA\n3v3"
$binDir = "$installDir\bin"

if (-not (Test-Path $binDir)) {
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
}

# Download
$tempFile = "$env:TEMP\$fileName"
Write-Host "Downloading $fileName..." -ForegroundColor Yellow
Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile

# Extract
Write-Host "Extracting..." -ForegroundColor Yellow
Expand-Archive -Path $tempFile -DestinationPath $binDir -Force
Remove-Item $tempFile

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$binDir*") {
    Write-Host "Adding to PATH..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$binDir", "User")
    $env:Path = "$env:Path;$binDir"
}

# Verify installation
Write-Host ""
Write-Host "Verifying installation..." -ForegroundColor Yellow
$n3v3Path = "$binDir\n3v3.exe"
if (Test-Path $n3v3Path) {
    & $n3v3Path --version
    Write-Host ""
    Write-Host "n3v3 installed successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Installation path: $binDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Quick start:" -ForegroundColor Yellow
    Write-Host "  n3v3 repl          # Start interactive REPL"
    Write-Host "  n3v3 doc           # View documentation"
    Write-Host "  n3v3 eval '1 + 2'  # Evaluate expression"
    Write-Host ""
    Write-Host "NOTE: Restart your terminal to use 'n3v3' command." -ForegroundColor Yellow
} else {
    Write-Host "Error: Installation failed" -ForegroundColor Red
    exit 1
}

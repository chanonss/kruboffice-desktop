# Build KrubOffice for Windows (x64): the launcher and the three programs, and the MSI installer.
#
#   pwsh packaging/windows/package.ps1 -KhianKrub ..\khiankrub-web -KhitKrub ..\khitkrub-web -PhutKrub ..\phutkrub-web
#
# Needs Rust, Python 3 and WiX Toolset 5 with its UI and Util extensions:
#   dotnet tool install --global wix --version 5.0.2
#   wix extension add -g WixToolset.UI.wixext/5.0.2
#   wix extension add -g WixToolset.Util.wixext/5.0.2
# Writes dist\KrubOffice-<version>-windows-x64.msi (unsigned).
param(
    [Parameter(Mandatory = $true)][string]$KhianKrub,
    [Parameter(Mandatory = $true)][string]$KhitKrub,
    [Parameter(Mandatory = $true)][string]$PhutKrub
)
$ErrorActionPreference = 'Stop'
$Root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$Version = (Select-String -Path (Join-Path $Root 'Cargo.toml') -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
if (-not $Version) { throw 'version not found in Cargo.toml' }

function Build($Dir, $Package) {
    Push-Location $Dir
    try {
        cargo build --release -p $Package
        if ($LASTEXITCODE -ne 0) { throw "cargo build -p $Package failed in $Dir" }
    } finally { Pop-Location }
}
Build $Root 'kruboffice'
Build $KhianKrub 'wordcraft'
Build $KhitKrub 'gridcraft'
Build $PhutKrub 'deckcraft'

$Dist = Join-Path $Root 'dist'
$Stage = Join-Path $Dist 'stage'
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Stage | Out-Null
Copy-Item (Join-Path $Root 'target\release\kruboffice.exe') (Join-Path $Stage 'kruboffice.exe')
Copy-Item (Join-Path $KhianKrub 'target\release\wordcraft.exe') (Join-Path $Stage 'khiankrub.exe')
Copy-Item (Join-Path $KhitKrub 'target\release\gridcraft.exe') (Join-Path $Stage 'khitkrub.exe')
Copy-Item (Join-Path $PhutKrub 'target\release\deckcraft.exe') (Join-Path $Stage 'phutkrub.exe')

# The Start Center's font (Sarabun, SIL OFL 1.1) and the apps' icons, next to kruboffice.exe.
New-Item -ItemType Directory -Force (Join-Path $Stage 'fonts'), (Join-Path $Stage 'icons') | Out-Null
foreach ($f in 'Sarabun-Regular.ttf', 'Sarabun-SemiBold.ttf', 'OFL.txt') {
    Invoke-WebRequest -UseBasicParsing "https://raw.githubusercontent.com/google/fonts/main/ofl/sarabun/$f" -OutFile (Join-Path $Stage "fonts\$f")
}
Copy-Item (Join-Path $KhianKrub 'assets\app-icon\khiankrub-192.png') (Join-Path $Stage 'icons')
Copy-Item (Join-Path $KhitKrub 'assets\app-icon\khitkrub-192.png') (Join-Path $Stage 'icons')
Copy-Item (Join-Path $PhutKrub 'assets\app-icon\phutkrub-192.png') (Join-Path $Stage 'icons')

# File types: packaging/types.txt, kept equal to the launcher's table by a unit test.
$Wxs = Join-Path $Dist 'kruboffice.wxs'
Get-Content -Encoding utf8 (Join-Path $Root 'packaging\types.txt') | python -I (Join-Path $Root 'packaging\windows\make_wxs.py') $Wxs
if ($LASTEXITCODE -ne 0) { throw 'make_wxs.py failed' }
$Rtf = Join-Path $Dist 'license.rtf'
python -I (Join-Path $Root 'packaging\windows\license_rtf.py') $Rtf

$Msi = Join-Path $Dist "KrubOffice-$Version-windows-x64.msi"
wix build $Wxs -arch x64 -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -d "Version=$Version" -d "BinDir=$Stage" -d "IconPath=$(Join-Path $Root 'packaging\windows\kruboffice.ico')" -d "LicenseRtf=$Rtf" -o $Msi
if ($LASTEXITCODE -ne 0) { throw 'wix build failed' }
Write-Host "Built $Msi"

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

# The C runtime goes inside each program (+crt-static), so no Visual C++ Redistributable is needed:
# without it a fresh Windows says "VCRUNTIME140.dll was not found". Scoped to the target (hence
# --target below) so build scripts and proc-macros on the host are unaffected.
$Target = 'x86_64-pc-windows-msvc'
[Environment]::SetEnvironmentVariable('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS', '-C target-feature=+crt-static')

# Each program's About shows its source commit (instead of "development build").
$env:WORDCRAFT_BUILD_SHA = (git -C $KhianKrub rev-parse HEAD)
$env:GRIDCRAFT_BUILD_SHA = (git -C $KhitKrub rev-parse HEAD)
$env:DECKCRAFT_BUILD_SHA = (git -C $PhutKrub rev-parse HEAD)

function Build($Dir, $Package) {
    Push-Location $Dir
    try {
        cargo build --release -p $Package --target $Target
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
Copy-Item (Join-Path $Root 'target\x86_64-pc-windows-msvc\release\kruboffice.exe') (Join-Path $Stage 'kruboffice.exe')
Copy-Item (Join-Path $KhianKrub 'target\x86_64-pc-windows-msvc\release\wordcraft.exe') (Join-Path $Stage 'khiankrub.exe')
Copy-Item (Join-Path $KhitKrub 'target\x86_64-pc-windows-msvc\release\gridcraft.exe') (Join-Path $Stage 'khitkrub.exe')
Copy-Item (Join-Path $PhutKrub 'target\x86_64-pc-windows-msvc\release\deckcraft.exe') (Join-Path $Stage 'phutkrub.exe')

# The Start Center's font (Sarabun, SIL OFL 1.1) and the apps' icons, next to kruboffice.exe.
New-Item -ItemType Directory -Force (Join-Path $Stage 'fonts'), (Join-Path $Stage 'icons') | Out-Null
foreach ($f in 'Sarabun-Regular.ttf', 'Sarabun-SemiBold.ttf', 'OFL.txt') {
    Invoke-WebRequest -UseBasicParsing "https://raw.githubusercontent.com/google/fonts/main/ofl/sarabun/$f" -OutFile (Join-Path $Stage "fonts\$f")
}
Copy-Item (Join-Path $KhianKrub 'assets\app-icon\khiankrub-192.png') (Join-Path $Stage 'icons')
Copy-Item (Join-Path $KhitKrub 'assets\app-icon\khitkrub-192.png') (Join-Path $Stage 'icons')
Copy-Item (Join-Path $PhutKrub 'assets\app-icon\phutkrub-192.png') (Join-Path $Stage 'icons')
# Explorer's icons for each program's files (packed from the same artwork; see ATTRIBUTION.md).
foreach ($a in 'khiankrub', 'khitkrub', 'phutkrub') { Copy-Item (Join-Path $Root "packaging\windows\$a.ico") (Join-Path $Stage 'icons') }

# KrubOffice's licence (MIT OR Apache-2.0), and the notices of the open-source parts in this build (the three programs'
# dependencies and the fonts they embed from the craft-fonts checkout next to them).
Copy-Item (Join-Path $Root 'LICENSE-MIT') (Join-Path $Stage 'LICENSE-MIT.txt')
Copy-Item (Join-Path $Root 'LICENSE-APACHE') (Join-Path $Stage 'LICENSE-APACHE.txt')
$Fonts = Join-Path (Split-Path -Parent (Resolve-Path $KhianKrub)) 'craft-fonts\fonts'
$NoticeArgs = @((Join-Path $Stage 'THIRD-PARTY-NOTICES.txt'), 'x86_64-pc-windows-msvc', "$($Root):kruboffice",
    "$((Resolve-Path $KhianKrub)):wordcraft", "$((Resolve-Path $KhitKrub)):gridcraft", "$((Resolve-Path $PhutKrub)):deckcraft")
if (Test-Path $Fonts) { $NoticeArgs += @('--fonts', $Fonts) }
$NoticeArgs += @('--fonts', (Join-Path $Stage 'fonts'))
python -I (Join-Path $Root 'packaging\notices.py') @NoticeArgs
if ($LASTEXITCODE -ne 0) { throw 'notices.py failed' }

# File types: packaging/types.txt, kept equal to the launcher's table by a unit test.
# Check: no program may need the Visual C++ runtime DLLs (a fresh Windows doesn't have them).
foreach ($exe in Get-ChildItem $Stage -Filter *.exe) {
    $text = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($exe.FullName)).ToLowerInvariant()
    foreach ($dll in 'vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'api-ms-win-crt-') {
        if ($text.Contains($dll)) { throw "$($exe.Name) still needs $($dll) (build with +crt-static)" }
    }
}

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

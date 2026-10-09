param(
    [Parameter(Mandatory = $true)][string]$BundleDir,
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$bundlePath = (Resolve-Path -LiteralPath $BundleDir).Path
$testPath = Join-Path $PSScriptRoot 'header-layout.c'
$vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsPath = & $vsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) { throw 'MSVC x86/x64 tools not found' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$outputPath = (Resolve-Path -LiteralPath $OutputDir).Path
$repoPath = Split-Path -Parent $PSScriptRoot
$layoutPath = Join-Path $repoPath 'target/layout-contracts'
$cargoOptions = @('--locked')
if ($Offline) { $cargoOptions += '--offline' }
foreach ($target in @('x86_64-pc-windows-msvc', 'i686-pc-windows-msvc')) {
    & cargo test --manifest-path (Join-Path $repoPath 'Cargo.toml') @cargoOptions --target $target --test layout_contract
    if ($LASTEXITCODE -ne 0) { throw "Rust layout contract failed ($target)" }
}
foreach ($architecture in @('x64', 'x86')) {
    $commandPath = Join-Path $outputPath "header-layout-$architecture.cmd"
    $objectPath = Join-Path $outputPath "header-layout-$architecture.obj"
    $command = @"
@echo off
call "$vsPath\VC\Auxiliary\Build\vcvarsall.bat" $architecture >nul
if errorlevel 1 exit /b 2
cl /nologo /TC /std:c11 /W4 /WX /c /I "$bundlePath" /I "$layoutPath" "$testPath" /Fo"$objectPath"
if errorlevel 1 exit /b 2
cl /nologo /TP /std:c++20 /W4 /WX /c /I "$bundlePath" /I "$layoutPath" "$testPath" /Fo"$objectPath"
exit /b %errorlevel%
"@
    [IO.File]::WriteAllText($commandPath, $command, [Text.Encoding]::ASCII)
    & cmd.exe /d /c $commandPath
    if ($LASTEXITCODE -ne 0) { throw "Header layout check failed ($architecture): $LASTEXITCODE" }
    Write-Output "C and C++ header layout matches Rust and SDK ($architecture)"
}

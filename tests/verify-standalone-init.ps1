param(
    [Parameter(Mandatory = $true)][string]$BundleDir,
    [Parameter(Mandatory = $true)][string]$OutputDir
)
$ErrorActionPreference = 'Stop'
$bundlePath = (Resolve-Path -LiteralPath $BundleDir).Path
$testPath = Join-Path $PSScriptRoot 'standalone-init-failure.c'
$vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsPath = & $vsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) { throw 'MSVC x86/x64 tools not found' }
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$outputPath = (Resolve-Path -LiteralPath $OutputDir).Path
foreach ($architecture in @('x64', 'x86')) {
    $commandPath = Join-Path $outputPath ("verify-init-$architecture.cmd")
    $objectPath = Join-Path $outputPath ("verify-init-$architecture.obj")
    $executablePath = Join-Path $outputPath ("verify-init-$architecture.exe")
    $command = @"
@echo off
call "$vsPath\VC\Auxiliary\Build\vcvarsall.bat" $architecture >nul
if errorlevel 1 exit /b 2
cl /nologo /W4 /WX /O2 /I "$bundlePath" "$testPath" /Fo"$objectPath" /Fe"$executablePath" /link kernel32.lib
if errorlevel 1 exit /b 2
"$executablePath"
if errorlevel 1 exit /b 1
"$executablePath" capacity
if errorlevel 1 exit /b 1
"$executablePath" capacity native
exit /b %errorlevel%
"@
    [System.IO.File]::WriteAllText($commandPath, $command, [System.Text.Encoding]::ASCII)
    & cmd.exe /d /c $commandPath
    if ($LASTEXITCODE -ne 0) { throw "Standalone initialization regression failed ($architecture): $LASTEXITCODE" }
}

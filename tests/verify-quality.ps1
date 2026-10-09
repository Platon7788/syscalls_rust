param([switch]$Offline)
$ErrorActionPreference = 'Stop'
$repoPath = Split-Path -Parent $PSScriptRoot
$cargoOptions = @('--locked')
if ($Offline) { $cargoOptions += '--offline' }
function Invoke-Cargo {
    param([string[]]$CargoArgs)
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo failed ($LASTEXITCODE): $CargoArgs" }
}
Push-Location $repoPath
try {
    Invoke-Cargo -CargoArgs @('fmt', '--all', '--', '--check')
    Invoke-Cargo -CargoArgs (@('clippy', '--workspace', '--all-targets') + $cargoOptions + @('--', '-D', 'warnings'))
    Invoke-Cargo -CargoArgs (@('test', '--workspace') + $cargoOptions)
    $first = Join-Path $repoPath 'target/local-quality/bundle-first'
    $second = Join-Path $repoPath 'target/local-quality/bundle-second'
    foreach ($bundle in @($first, $second)) {
        Invoke-Cargo -CargoArgs (@('run', '-p', 'syscalls-standalone', '--bin', 'syscalls-standalone') + $cargoOptions + @('--', '--lib', 'lib.rs', '--out', $bundle))
    }
    foreach ($name in @('syscalls.h', 'syscalls.c', 'syscallsstubs.x64.asm', 'syscallsstubs.x86.c', 'syscalls.props', 'README.md')) {
        $left = (Get-FileHash -LiteralPath (Join-Path $first $name) -Algorithm SHA256).Hash
        $right = (Get-FileHash -LiteralPath (Join-Path $second $name) -Algorithm SHA256).Hash
        if ($left -ne $right) { throw "Generation is not reproducible: $name" }
    }
    & (Join-Path $PSScriptRoot 'verify-header-layout.ps1') -BundleDir $first -OutputDir (Join-Path $repoPath 'target/local-quality/header-checks') -Offline:$Offline
    & git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
    Write-Output 'All local quality checks passed.'
}
finally {
    Pop-Location
}

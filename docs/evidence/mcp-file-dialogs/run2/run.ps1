$ErrorActionPreference = 'Stop'
$env:CARGO_HOME = Join-Path (Get-Location) '.tmp/cargo-home'
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) '.tmp/target'
$env:TEMP = Join-Path (Get-Location) '.tmp/rust-temp'
$env:TMP = $env:TEMP
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$receipt = Join-Path (Get-Location) '.tmp/file-dialog-build/run2'
$head = (git rev-parse HEAD).Trim()
if ($head -ne 'bf0c571e89567e3b73d13cc4752657514112e51b') { throw 'wrong source head' }
$stages = @(
    @{ name = 'dialogs'; args = @('test','--locked','--offline','-j1','-p','lurq','--features','mcp','--lib','mcp::','--','--test-threads=1') },
    @{ name = 'windows'; args = @('test','--locked','--offline','-j1','-p','lurq','--features','mcp','--lib','app::window','--','--test-threads=1') }
)
$stages += @{ name = 'default'; args = @('check','--locked','--offline','-j1','-p','lurq','--lib') }
foreach ($stage in $stages) {
    $began = [DateTime]::UtcNow
    Write-Output "START $($stage.name) $head"
    & cargo @($stage.args) *> (Join-Path $receipt "$($stage.name).log")
    $result = $LASTEXITCODE
    @{ head=$head; stage=$stage.name; arguments=$stage.args; started=$began.ToString('o'); finished=[DateTime]::UtcNow.ToString('o'); exit_code=$result } |
        ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $receipt "$($stage.name).json") -Encoding utf8
    Get-Content -LiteralPath (Join-Path $receipt "$($stage.name).log") | Select-Object -Last 22
    Write-Output "TERMINAL $($stage.name) $result"
    if ($result -ne 0) { exit $result }
}

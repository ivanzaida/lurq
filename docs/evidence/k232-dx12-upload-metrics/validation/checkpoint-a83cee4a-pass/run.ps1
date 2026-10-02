param([Parameter(Mandatory=$true)][string]$ExpectedHead)
$ErrorActionPreference = 'Stop'
$taskRoot = 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance'
Set-Location -LiteralPath $taskRoot
if ((git rev-parse HEAD).Trim() -ne $ExpectedHead) { throw 'Source head changed' }
if (git status --porcelain --untracked-files=no) { throw 'Tracked source dirty' }
$taskChanged = @(git diff --name-only 9b1affb $ExpectedHead)
if ($taskChanged.Count -ne 1 -or $taskChanged[0] -ne 'crates/lurq/src/app/dx12_render/canvas.rs') { throw 'Repair extends beyond approved import' }
$taskLock = 'f0d263dc9e391de25a5e8299eee8c7a7f312e55841c855ade04a82e6acaa59ae'
if ((Get-FileHash Cargo.lock).Hash.ToLowerInvariant() -ne $taskLock) { throw 'Lock changed' }
$taskOutput = Join-Path $taskRoot '.tmp/dx12-independent-tests/run2'
if (Test-Path -LiteralPath $taskOutput) { throw 'Output exists' }
New-Item -ItemType Directory -Path $taskOutput | Out-Null
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $taskOutput 'run.ps1')
$env:CARGO_TARGET_DIR = 'H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance/.tmp/target'
$env:CARGO_HOME = 'H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces/.tmp/lurq-adoption-cargo-home'
$env:TEMP = Join-Path $taskRoot '.tmp/rust-temp'
$env:TMP = $env:TEMP
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
foreach ($taskKey in @('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_PROFILE_DEV_OPT_LEVEL','CARGO_PROFILE_TEST_OPT_LEVEL')) {
    if ([Environment]::GetEnvironmentVariable($taskKey)) { throw "Unexpected setting: $taskKey" }
}
$taskCases = @(
    @{name='dx12-enabled';features='mcp,canvas,perf_profile,dx12'},
    @{name='dx12-disabled';features='mcp,canvas,dx12'}
)
foreach ($taskCase in $taskCases) {
    $taskArgs = @('check','-p','lurq','--lib','--locked','--offline','-j1','--features',$taskCase.features)
    $taskStart = [DateTime]::UtcNow
    & cargo @taskArgs > (Join-Path $taskOutput ($taskCase.name+'.log')) 2>&1
    $taskExit = $LASTEXITCODE
    [ordered]@{
        head=$ExpectedHead;tree=(git rev-parse 'HEAD^{tree}').Trim();command=@('cargo')+$taskArgs
        exit_code=$taskExit;started_utc=$taskStart.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o')
        lock_sha256=(Get-FileHash Cargo.lock).Hash.ToLowerInvariant()
        target=$env:CARGO_TARGET_DIR;cargo_home=$env:CARGO_HOME;temp=$env:TEMP
        jobs=1;incremental=0;dev_debug=0;test_debug=0
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskOutput ($taskCase.name+'.json')) -Encoding utf8
    Get-Content -LiteralPath (Join-Path $taskOutput ($taskCase.name+'.log')) -Tail 12
    if ($taskExit -ne 0) { exit $taskExit }
}
[ordered]@{head=(git rev-parse HEAD).Trim();tree=(git rev-parse 'HEAD^{tree}').Trim();lock_sha256=(Get-FileHash Cargo.lock).Hash.ToLowerInvariant();tracked_status=@(git status --porcelain --untracked-files=no);exit_code=0} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOutput 'terminal.json') -Encoding utf8

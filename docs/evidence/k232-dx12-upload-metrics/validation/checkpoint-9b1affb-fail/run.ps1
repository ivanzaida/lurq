$ErrorActionPreference = 'Stop'
$taskRoot = 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance'
Set-Location -LiteralPath $taskRoot
$taskOutput = Join-Path $taskRoot '.tmp/dx12-independent-tests/run1'
$taskHead = '9b1affb544572b75be9d6e19cb897146a05db5fa'
$taskLockHash = 'f0d263dc9e391de25a5e8299eee8c7a7f312e55841c855ade04a82e6acaa59ae'
if ((git rev-parse HEAD).Trim() -ne $taskHead) { throw 'Source head changed' }
if (git status --porcelain --untracked-files=no) { throw 'Tracked source changed' }
if ((Get-FileHash Cargo.lock).Hash.ToLowerInvariant() -ne $taskLockHash) { throw 'Source lock changed' }
$env:CARGO_TARGET_DIR = 'H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance/.tmp/target'
$env:CARGO_HOME = 'H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces/.tmp/lurq-adoption-cargo-home'
$env:TEMP = Join-Path $taskRoot '.tmp/rust-temp'
$env:TMP = $env:TEMP
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
foreach ($taskKey in @('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_PROFILE_DEV_OPT_LEVEL','CARGO_PROFILE_TEST_OPT_LEVEL')) {
    if ([Environment]::GetEnvironmentVariable($taskKey)) { throw "Unexpected compiler setting: $taskKey" }
}
if (-not (Test-Path -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'debug/deps'))) { throw 'Warm target missing' }
$taskCases = @(
    @{name='enabled'; args=@('test','-p','lurq','--lib','--locked','--offline','-j1','--features','mcp,canvas,perf_profile','profiling_canvas_upload','--','--test-threads=1');count=4},
    @{name='disabled'; args=@('test','-p','lurq','--lib','--locked','--offline','-j1','--features','mcp,canvas','profiling_canvas_upload','--','--test-threads=1');count=2},
    @{name='dx12-enabled'; args=@('check','-p','lurq','--lib','--locked','--offline','-j1','--features','mcp,canvas,perf_profile,dx12');count=0},
    @{name='dx12-disabled'; args=@('check','-p','lurq','--lib','--locked','--offline','-j1','--features','mcp,canvas,dx12');count=0}
)
$taskInventory = @(Get-CimInstance Win32_Process | Where-Object { $_.Name -in @('cargo.exe','rustc.exe','kontur-desktop.exe') } | Select-Object ProcessId,Name,CommandLine)
$taskInventory | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $taskOutput 'prestart-processes.json') -Encoding utf8
foreach ($taskCase in $taskCases) {
    $taskStart = [DateTime]::UtcNow
    & cargo @($taskCase.args) > (Join-Path $taskOutput ($taskCase.name+'.log')) 2>&1
    $taskExit = $LASTEXITCODE
    $taskActual = 0
    if ($taskExit -eq 0 -and $taskCase.count -gt 0) {
        $taskMatch = [regex]::Match([IO.File]::ReadAllText((Join-Path $taskOutput ($taskCase.name+'.log'))),'test result: ok\. (\d+) passed; 0 failed')
        if ($taskMatch.Success) { $taskActual = [int]$taskMatch.Groups[1].Value }
        if ($taskActual -ne $taskCase.count) { $taskExit = 90 }
    }
    [ordered]@{head=$taskHead;tree=(git rev-parse 'HEAD^{tree}').Trim();command=@('cargo')+$taskCase.args;exit_code=$taskExit;expected_tests=$taskCase.count;passed_tests=$taskActual;started_utc=$taskStart.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o');lock_sha256=(Get-FileHash Cargo.lock).Hash.ToLowerInvariant();target=$env:CARGO_TARGET_DIR;cargo_home=$env:CARGO_HOME;temp=$env:TEMP;jobs=1;incremental=0;dev_debug=0;test_debug=0} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskOutput ($taskCase.name+'.json')) -Encoding utf8
    Get-Content -LiteralPath (Join-Path $taskOutput ($taskCase.name+'.log')) -Tail 16
    if ($taskExit -ne 0) { exit $taskExit }
}
[ordered]@{head=(git rev-parse HEAD).Trim();tree=(git rev-parse 'HEAD^{tree}').Trim();lock_sha256=(Get-FileHash Cargo.lock).Hash.ToLowerInvariant();tracked_status=@(git status --porcelain --untracked-files=no);exit_code=0} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOutput 'terminal.json') -Encoding utf8

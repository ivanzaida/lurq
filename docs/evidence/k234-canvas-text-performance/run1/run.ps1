$ErrorActionPreference = 'Stop'
$workspace = 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance'
Set-Location -LiteralPath $workspace
$expected = '863b23c24b32d48042dfcd44a3f8d350cae7ba05'
if ((git rev-parse HEAD).Trim() -ne $expected) { throw 'Frozen source head changed' }
if (git status --porcelain --untracked-files=no) { throw 'Tracked source changed' }
$env:CARGO_HOME = 'H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces/.tmp/lurq-adoption-cargo-home'
$env:CARGO_TARGET_DIR = 'H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance/.tmp/target'
$env:TEMP = 'H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance/.tmp/rust-temp'
$env:TMP = $env:TEMP
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
if ($env:RUSTFLAGS) { throw 'Unexpected RUSTFLAGS' }
$cases = @(
  @{name='canvas-text-lib'; args=@('--lib','canvas::text')},
  @{name='canvas-text-public'; args=@('--test','canvas_tests','text_')},
  @{name='canvas-text-paint-refusal'; args=@('--test','canvas_tests','a_gradient_is_refused_for_text_rather_than_reduced_to_one_of_its_colours')}
)
foreach ($case in $cases) {
  $arguments = @('test','-p','lurq','--locked','--offline','-j1','--features','canvas,perf_profile') + $case.args + @('--','--test-threads=1')
  $started = [DateTime]::UtcNow.ToString('o')
  & cargo @arguments 2>&1 | Tee-Object -FilePath ".tmp/k234-checks/run1/$($case.name).log"
  $status = $LASTEXITCODE
  @{head=$expected; tree=(git rev-parse 'HEAD^{tree}').Trim(); command=@('cargo')+$arguments; exit_code=$status; started_utc=$started; finished_utc=[DateTime]::UtcNow.ToString('o'); environment=@{CARGO_HOME=$env:CARGO_HOME; CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR; TEMP=$env:TEMP; jobs='1'; incremental='0'; dev_debug='0'; test_debug='0'}} | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 ".tmp/k234-checks/run1/$($case.name).json"
  if ($status -ne 0) { exit $status }
}

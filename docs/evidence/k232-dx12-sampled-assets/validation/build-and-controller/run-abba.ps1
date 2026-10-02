$ErrorActionPreference = 'Stop'
$warm = 'H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces'
$source = 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance'
$candidateBranch = 'codex/k232-dx12-sampled-assets'
$baselineHead = '644ff6613411e28dc3035c0c910df64fcf915b51'
$candidateHead = 'cf0c5d81ddab2c960f1aff34a4b5117a73ade9e5'
$konturHead = '61d73d08b7c047d6c62ad55d6ed127a495881772'
$parent = Join-Path $warm '.tmp/k232-profiler-candidate'
$baseline = Join-Path $parent 'dx12-upload-034'
$candidate = Join-Path $parent 'dx12-sampled-assets'
$output = Join-Path $candidate 'abba-01'
$sdk = 'C:/Users/lurkm/AppData/Local/Packages/OpenAI.Codex_2p2nqsd0c76g0/LocalCache/Local/uv/cache/archive-v0/q-kPMK8-v5Cc871G/Scripts/python.exe'
$env:PYTHONDONTWRITEBYTECODE = '1'
$sdkRaw = & $sdk -c 'import json,importlib.metadata; print(json.dumps({"mcp":importlib.metadata.version("mcp"),"httpx":importlib.metadata.version("httpx")}))'
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$sdkVersions = $sdkRaw | ConvertFrom-Json
if ($sdkVersions.mcp -ne '1.28.1' -or $sdkVersions.httpx -ne '0.28.1') { throw 'SDK versions changed' }
function SourceHead {
  $head = git -C $source rev-parse HEAD
  if ($LASTEXITCODE -ne 0) { throw 'Source Git failed' }
  $status = git -C $source status --porcelain --untracked-files=no
  if ($LASTEXITCODE -ne 0 -or $status) { throw 'Source tracked status changed' }
  return $head.Trim()
}
if ((SourceHead) -ne $candidateHead) { throw 'Candidate source must be frozen before runs' }
if ((git -C $warm rev-parse HEAD).Trim() -ne $konturHead -or $LASTEXITCODE -ne 0) { throw 'Consumer head changed' }
if (git -C $warm status --porcelain --untracked-files=no) { throw 'Consumer tracked source dirty' }
if ((Get-FileHash (Join-Path $warm '.env.desktop')).Hash.ToLowerInvariant() -ne '055b967168508d226e60a53c5fb1cef62966fcd88709209b31c33b7decce7722') { throw 'Configured env changed' }
if (Test-Path -LiteralPath $output) { throw 'Never replace existing ABBA evidence' }
$inputRows = @()
foreach ($item in @(@{root=$baseline;head=$baselineHead;wrapper='2ee4079a6e7af37555902665e3947cda987a574dd822086c29812c0cf79656dd'}, @{root=$candidate;head=$candidateHead;wrapper='e96b6170191b74dfd46786b132ecaa1a626de5129c31e7bd37630ec8dfef899b'})) {
  $identity = Get-Content -LiteralPath (Join-Path $item.root 'identity.json') -Raw | ConvertFrom-Json
  if ($identity.exit_code -ne 0 -or $identity.kontur_head -ne $konturHead -or $identity.lurq_head -ne $item.head) { throw 'Build identity mismatch' }
  if ((Get-FileHash (Join-Path $item.root 'kontur-desktop.exe')).Hash.ToLowerInvariant() -ne $identity.binary_sha256) { throw 'Binary mismatch' }
  if ((Get-FileHash (Join-Path $item.root 'smoke.py')).Hash.ToLowerInvariant() -ne $item.wrapper) { throw 'Frozen wrapper changed' }
  if ((Get-FileHash (Join-Path $warm 'Cargo.lock')).Hash.ToLowerInvariant() -ne $identity.registry_lock_sha256) { throw 'Registry lock not restored' }
  $inputRows += @{identity=$identity;root=$item.root;wrapper_sha256=$item.wrapper}
}
New-Item -ItemType Directory -Path $output | Out-Null
[IO.File]::WriteAllText((Join-Path $output 'frozen-abba-inputs.json'), (@{inputs=$inputRows;sdk_executable=$sdk;sdk_versions=$sdkVersions;order=@('A1','B1','B2','A2');scope='Four fresh owned processes, unchanged bounded wrapper journey; no baseline rebuild or source guard weakening'} | ConvertTo-Json -Depth 10), [Text.UTF8Encoding]::new($false))
$cases = @(@{name='A1';variant='baseline';root=$baseline;head=$baselineHead}, @{name='B1';variant='candidate';root=$candidate;head=$candidateHead}, @{name='B2';variant='candidate';root=$candidate;head=$candidateHead}, @{name='A2';variant='baseline';root=$baseline;head=$baselineHead})
$receipts = @()
$controllerExit = 0
try {
  foreach ($case in $cases) {
    if ((SourceHead) -notin @($baselineHead,$candidateHead)) { throw 'Unexpected source during ABBA' }
    if ($case.variant -eq 'baseline') { git -C $source switch --detach $baselineHead }
    else { git -C $source switch $candidateBranch }
    if ($LASTEXITCODE -ne 0 -or (SourceHead) -ne $case.head) { throw 'Exact source switch failed' }
    $runRoot = Join-Path $case.root ('abba-01-'+$case.name)
    if (Test-Path -LiteralPath $runRoot) { throw 'Never replace prior native run' }
    $started = [DateTime]::UtcNow
    & $sdk (Join-Path $case.root 'smoke.py') --binary (Join-Path $case.root 'kontur-desktop.exe') --build-identity (Join-Path $case.root 'identity.json') --output $runRoot > (Join-Path $output ($case.name+'-console.log')) 2>&1
    $runExit = $LASTEXITCODE
    $receipts += @{name=$case.name;variant=$case.variant;source_head=$case.head;capture_root=$runRoot;exit_code=$runExit;started_utc=$started.ToString('o');finished_utc=[DateTime]::UtcNow.ToString('o')}
    [IO.File]::WriteAllText((Join-Path $output 'run-receipts.json'), ($receipts | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
    Write-Output ('Native '+$case.name+' terminal '+$runExit)
    if ($runExit -ne 0) { $controllerExit=$runExit; break }
  }
} finally {
  if ((SourceHead) -notin @($baselineHead,$candidateHead)) { throw 'Unknown source; refusing blind restore' }
  git -C $source switch $candidateBranch
  if ($LASTEXITCODE -ne 0 -or (SourceHead) -ne $candidateHead) { throw 'Candidate source restore failed' }
}
[IO.File]::WriteAllText((Join-Path $output 'terminal.json'), (@{exit_code=$controllerExit;source_restored=$true;cases_completed=$receipts.Count;source_head=(SourceHead);finished_utc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
exit $controllerExit

param([ValidateSet('baseline','candidate')][string]$Variant)
$ErrorActionPreference = 'Stop'
$root = 'H:/projects/pencil-web/.codex/worktrees/k222-composed-surfaces'
Set-Location -LiteralPath $root
$folder = Join-Path $root '.tmp/k232-profiler-candidate'
$output = Join-Path $folder "comparison/$Variant"
[System.IO.Directory]::CreateDirectory($output) | Out-Null
if (Test-Path -LiteralPath "$output/identity.json") { throw 'Completed comparison build already exists' }
$source = if ($Variant -eq 'baseline') { 'H:/projects/pencil-web/.codex/worktrees/lurq-k232-mcp-performance' } else { 'H:/projects/pencil-web/.codex/worktrees/lurq-canvas-text-performance' }
$expectedSource = if ($Variant -eq 'baseline') { '5e50f862e0e59e10e8778c5b7a61f4488f065a2a' } else { '863b23c24b32d48042dfcd44a3f8d350cae7ba05' }
$konturHead = 'b3076c671ceb069dabe3b869881cb42f86a3ab34'
if ((git rev-parse HEAD).Trim() -ne $konturHead) { throw 'Kontur HEAD changed' }
if (git status --porcelain --untracked-files=no) { throw 'Kontur tracked files changed' }
if (git -C $source status --porcelain --untracked-files=no -- crates Cargo.toml Cargo.lock) { throw 'Lurq source files changed' }
git -C $source diff --exit-code $expectedSource HEAD -- crates Cargo.toml Cargo.lock
if ($LASTEXITCODE -ne 0) { throw 'Lurq workspace differs from reviewed source' }
$sourceTree = (git -C $source rev-parse "${expectedSource}:crates/lurq").Trim()
if ((git -C $source rev-parse 'HEAD:crates/lurq').Trim() -ne $sourceTree) { throw 'Lurq source differs from reviewed source' }
$sourceHead = (git -C $source rev-parse HEAD).Trim()
foreach ($name in @('RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','CARGO_PROFILE_RELEASE_OPT_LEVEL','CARGO_PROFILE_RELEASE_DEBUG','CARGO_PROFILE_RELEASE_LTO','CARGO_PROFILE_RELEASE_CODEGEN_UNITS','CARGO_PROFILE_RELEASE_INCREMENTAL')) {
  if ([Environment]::GetEnvironmentVariable($name)) { throw "Unexpected profile override: $name" }
}
$lock = Join-Path $root 'Cargo.lock'
$original = [System.IO.File]::ReadAllBytes($lock)
$originalHash = '3AD9998913DFB6FCA3BF3F062B74256338135FDD213F7ABC17A096D87CC11887'
$patchedHash = '5A49B46EA09509C8ED1A3BF0A031CB8726C6C9A5D9983115DBE40031452626A7'
if ((Get-FileHash -LiteralPath $lock -Algorithm SHA256).Hash -ne $originalHash) { throw 'Original lock differs' }
$patched = Join-Path $folder 'Cargo.lock.patched'
if ((Get-FileHash -LiteralPath $patched -Algorithm SHA256).Hash -ne $patchedHash) { throw 'Prepared path-only lock differs' }
$started = [DateTime]::UtcNow.ToString('o')
$status = $null
try {
  [System.IO.File]::WriteAllBytes($lock, [System.IO.File]::ReadAllBytes($patched))
  & node .tmp/k232-profiler-candidate/comparison_build.mjs metadata $Variant 1> "$output/metadata.json" 2> "$output/metadata-error.log"
  if ($LASTEXITCODE -ne 0) { throw 'Metadata failed' }
  $metadata = Get-Content -Raw -LiteralPath "$output/metadata.json" | ConvertFrom-Json
  $package = @($metadata.packages | Where-Object { $_.name -eq 'lurq' })
  if ($package.Count -ne 1 -or $package[0].manifest_path.Replace('\','/') -ne "$source/crates/lurq/Cargo.toml") { throw 'Resolved Lurq source mismatch' }
  $features = @($metadata.resolve.nodes | Where-Object { $_.id -eq $package[0].id }).features
  if ('perf_profile' -notin $features -or 'canvas' -notin $features) { throw 'Profiler or Canvas unavailable' }
  @{ source=$source; manifest=$package[0].manifest_path; features=$features; lock_sha256=$patchedHash } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 "$output/metadata-validation.json"
  & node .tmp/k232-profiler-candidate/comparison_build.mjs build $Variant 2>&1 | Tee-Object -FilePath "$output/build.log"
  $status = $LASTEXITCODE
  if ($status -ne 0) { throw "Build exited $status" }
  if ((git -C $source rev-parse 'HEAD:crates/lurq').Trim() -ne $sourceTree -or (git -C $source status --porcelain --untracked-files=no -- crates Cargo.toml Cargo.lock)) { throw 'Lurq source changed during build' }
  $binary = Join-Path $output 'kontur-desktop.exe'
  Copy-Item -LiteralPath (Join-Path $root '.tmp/target/release/kontur-desktop.exe') -Destination $binary
  @{ variant=$Variant; kontur_head=$konturHead; kontur_tree=(git rev-parse 'HEAD^{tree}').Trim(); lurq_head=$sourceHead; lurq_source_commit=$expectedSource; lurq_crate_tree=$sourceTree; binary_path=$binary; binary_sha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant(); source_lock_sha256=$originalHash.ToLowerInvariant(); executed_lock_sha256=$patchedHash.ToLowerInvariant(); build_profile='release optimized'; build_exit_code=$status; started_utc=$started; finished_utc=[DateTime]::UtcNow.ToString('o'); target=(Join-Path $root '.tmp/target'); jobs=1 } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 "$output/identity.json"
} finally {
  if ((Get-FileHash -LiteralPath $lock -Algorithm SHA256).Hash -ne $patchedHash) { throw 'Executed lock changed; refusing blind restore' }
  if ((git rev-parse HEAD).Trim() -ne $konturHead) { throw 'Kontur HEAD changed; refusing blind restore' }
  [System.IO.File]::WriteAllBytes($lock, $original)
  @{ original_lock_restored=((Get-FileHash -LiteralPath $lock -Algorithm SHA256).Hash -eq $originalHash); finished_utc=[DateTime]::UtcNow.ToString('o'); build_exit_code=$status } | ConvertTo-Json | Set-Content -Encoding utf8 "$output/cleanup.json"
}

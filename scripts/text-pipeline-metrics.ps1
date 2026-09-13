# Compare dev-only Cargo package overrides without editing any manifests.
[CmdletBinding()]
param(
  [ValidateSet('workspace', 'unoptimized', 'current', 'five', 'no-cosmic', 'no-swash', 'no-rustybuzz', 'no-ttf-parser', 'no-fontdb', 'zeno', 'raster-deps', 'font-raster-deps', 'skrifa')]
  [string[]]$Variants = @('unoptimized', 'current', 'five', 'no-cosmic', 'no-swash', 'no-rustybuzz', 'no-ttf-parser', 'no-fontdb', 'zeno', 'raster-deps'),
  [ValidateRange(1, 10000)]
  [int]$Samples = 30,
  [switch]$Interactions,
  [string]$OutputDirectory = 'target/text-metrics'
)

$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$previousMetrics = $env:LURQ_TEXT_METRICS
$previousSamples = $env:LURQ_TEXT_METRICS_SAMPLES
$previousInteractions = $env:LURQ_TEXT_INTERACTIONS
Push-Location $workspace
try {
  $outputPath = [IO.Path]::GetFullPath($OutputDirectory, $workspace)
  New-Item -ItemType Directory -Force $outputPath | Out-Null
  $packages = @('cosmic-text', 'swash', 'rustybuzz', 'ttf-parser', 'fontdb', 'unicode-bidi', 'unicode-linebreak', 'unicode-script')
  foreach ($variant in $Variants) {
    # Explicitly pin the existing package settings so every comparison has the same baseline.
    $levels = [ordered]@{}
    foreach ($package in $packages) { $levels[$package] = 2 }
    foreach ($package in @('zeno', 'skrifa', 'read-fonts', 'font-types', 'yazi')) { $levels[$package] = 0 }
    switch ($variant) {
      'workspace' { $levels.Clear() }
      'unoptimized' { foreach ($package in $packages) { $levels[$package] = 0 } }
      'five' { foreach ($package in $packages[5..7]) { $levels[$package] = 0 } }
      'no-cosmic' { $levels['cosmic-text'] = 0 }
      'no-swash' { $levels['swash'] = 0 }
      'no-rustybuzz' { $levels['rustybuzz'] = 0 }
      'no-ttf-parser' { $levels['ttf-parser'] = 0 }
      'no-fontdb' { $levels['fontdb'] = 0 }
      'zeno' { $levels['zeno'] = 2 }
      'raster-deps' { foreach ($package in @('zeno', 'skrifa', 'read-fonts', 'font-types', 'yazi')) { $levels[$package] = 2 } }
      'font-raster-deps' { foreach ($package in @('skrifa', 'read-fonts', 'font-types')) { $levels[$package] = 2 } }
      'skrifa' { $levels['skrifa'] = 2 }
    }
    $cargoArgs = @('bench', '-p', 'lurq', '--bench', 'text_pipeline', '--features', 'markdown,perf_profile', '--profile', 'dev', '--locked', '--no-run', '--message-format=json')
    foreach ($entry in $levels.GetEnumerator()) {
      $cargoArgs += @('--config', "profile.dev.package.$($entry.Key).opt-level=$($entry.Value)")
    }
    Write-Host "Building $variant"
    $buildLog = Join-Path $outputPath "$variant.build.jsonl"
    & cargo @cargoArgs > $buildLog 2> (Join-Path $outputPath "$variant.build.log")
    if ($LASTEXITCODE -ne 0) { throw "Build failed; see $outputPath/$variant.build.log" }
    $artifact = Get-Content -LiteralPath $buildLog | ForEach-Object { $_ | ConvertFrom-Json } |
      Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'text_pipeline' -and $_.executable } |
      Select-Object -Last 1
    if (!$artifact) { throw 'Cargo did not report a text_pipeline executable' }
    if ($artifact.profile.opt_level -ne '0' -or !$artifact.profile.debug_assertions) {
      throw 'Expected an unoptimized dev benchmark with debug assertions; check Cargo environment overrides'
    }
    # Preserve each statically linked binary for repeats without rebuilding or changing profile settings.
    $binary = Join-Path $outputPath "$variant.exe"
    Copy-Item -LiteralPath $artifact.executable -Destination $binary -Force
    $metadata = [ordered]@{
      variant = $variant
      timestamp = (Get-Date).ToString('o')
      commit = (& git rev-parse HEAD)
      rustc = (& rustc -Vv) -join "`n"
      cpu = (Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Name)
      samples = $Samples
      interactions = [bool]$Interactions
      command = @('cargo') + $cargoArgs
      artifact_executable = $artifact.executable
      dev_package_opt_levels = $levels
      readme_sha256 = (Get-FileHash README.md -Algorithm SHA256).Hash
      cargo_lock_sha256 = (Get-FileHash Cargo.lock -Algorithm SHA256).Hash
      source_sha256 = @{}
    }
    foreach ($source in @('Cargo.toml', 'crates/lurq/Cargo.toml', 'crates/lurq/src/app/glyph_engine.rs', 'crates/lurq/src/app/profile_types.rs', 'crates/lurq/benches/text_pipeline.rs', 'crates/lurq/benches/text_pipeline/metrics.rs', 'crates/lurq/benches/text_pipeline/scenario.rs')) {
      $metadata.source_sha256[$source] = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    }
    $metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputPath "$variant.metadata.json")
    if ($Interactions) {
      $env:LURQ_TEXT_METRICS = $null
      $env:LURQ_TEXT_INTERACTIONS = Join-Path $outputPath "$variant.csv"
    } else {
      $env:LURQ_TEXT_INTERACTIONS = $null
      $env:LURQ_TEXT_METRICS = Join-Path $outputPath "$variant.csv"
    }
    $env:LURQ_TEXT_METRICS_SAMPLES = "$Samples"
    Write-Host "Measuring $variant ($Samples fresh apps per case)"
    & $binary 2> (Join-Path $outputPath "$variant.run.log")
    if ($LASTEXITCODE -ne 0) { throw "Metrics failed; see $outputPath/$variant.run.log" }
  }
} finally {
  $env:LURQ_TEXT_METRICS = $previousMetrics
  $env:LURQ_TEXT_METRICS_SAMPLES = $previousSamples
  $env:LURQ_TEXT_INTERACTIONS = $previousInteractions
  Pop-Location
}

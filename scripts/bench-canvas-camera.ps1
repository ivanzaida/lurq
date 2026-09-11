param(
  [string]$Toolchain = 'stable',
  [ValidateRange(0, 62)][int]$Processor = 0,
  [ValidateRange(1, 100)][int]$Runs = 3,
  [string]$Executable
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $workspace
try {
  if (!$Executable) {
    & rustc "+$Toolchain" --version
    $artifacts = & cargo "+$Toolchain" test -p lurq --release --features canvas --lib canvas_camera_prepare_benchmark --no-run --message-format=json |
      ForEach-Object { $_ | ConvertFrom-Json }
    if ($LASTEXITCODE -ne 0) { throw 'Benchmark build failed' }
    $Executable = ($artifacts | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'lurq' -and $_.executable } | Select-Object -Last 1).executable
  }
  $probePath = (Resolve-Path -LiteralPath $Executable).Path
  $logDirectory = Join-Path $workspace 'target/canvas-camera-benchmark'
  New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
  Get-CimInstance Win32_Processor | Select-Object Name
  Write-Output "Benchmark: $probePath; logical processor: $Processor"
  foreach ($run in 1..$Runs) {
    $stdout = Join-Path $logDirectory "run-$run.out"
    $stderr = Join-Path $logDirectory "run-$run.err"
    $probe = Start-Process -FilePath $probePath -ArgumentList @('canvas_camera_prepare_benchmark', '--ignored', '--nocapture') -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $probe.ProcessorAffinity = [IntPtr]([long]1 -shl $Processor)
    $probe.WaitForExit()
    Write-Output "Run $run"
    Get-Content -LiteralPath $stderr
    if ($probe.ExitCode -ne 0) { throw "Benchmark run $run failed" }
  }
} finally {
  Pop-Location
}

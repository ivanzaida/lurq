# Runs graphics_card_probe (build it first with --release) and reads, while it draws, the GPU memory of its
# process by card and its private bytes:
#   pwsh crates/lurq/examples/graphics_card_probe.ps1 -Renderer wgpu -Card low -Hints perf
param([string]$Renderer, [string]$Card, [string]$Hints = "usage", [int]$Seconds = 8)
$exe = Join-Path $PSScriptRoot "../../../target/release/examples/graphics_card_probe.exe"
$out = Join-Path $env:TEMP "gcp_$([guid]::NewGuid().ToString('N')).txt"
$p = Start-Process -FilePath $exe -ArgumentList $Renderer, $Card, $Hints, $Seconds -RedirectStandardOutput $out -RedirectStandardError "$out.err" -PassThru -NoNewWindow
Start-Sleep -Milliseconds ($Seconds * 500)
$id = $p.Id
$mem = @()
try {
  $mem = (Get-Counter "\GPU Process Memory(pid_${id}_*)\Dedicated Usage" -ErrorAction Stop).CounterSamples |
    Where-Object { $_.CookedValue -gt 0 } |
    ForEach-Object { "{0} = {1:N0} MB" -f ($_.InstanceName -replace '^pid_\d+_', ''), ($_.CookedValue / 1MB) }
} catch { $mem = @("counter error: $_") }
$proc = Get-Process -Id $id -ErrorAction SilentlyContinue
$private = if ($proc) { "{0:N0} MB" -f ($proc.PrivateMemorySize64 / 1MB) } else { "gone" }
$ws = if ($proc) { "{0:N0} MB" -f ($proc.WorkingSet64 / 1MB) } else { "gone" }
$p.WaitForExit()
Write-Output "=== $Renderer $Card $Hints  exit=$($p.ExitCode)"
Get-Content $out | Where-Object { $_ -notmatch '^card \d' }
Get-Content "$out.err" | Select-Object -Last 5
Write-Output "GPU dedicated (by luid): $($mem -join '; ')"
Write-Output "private bytes: $private  working set: $ws"
Remove-Item $out, "$out.err" -ErrorAction SilentlyContinue

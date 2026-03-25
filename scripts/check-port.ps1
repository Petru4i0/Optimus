param(
  [int]$Port = 1420
)

$listener = Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction SilentlyContinue
if (-not $listener) {
  Write-Host "No LISTEN process found on port $Port"
  exit 0
}

$pids = $listener | Select-Object -ExpandProperty OwningProcess -Unique
foreach ($pid in $pids) {
  $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
  if ($proc) {
    Write-Host ("Port {0} -> PID {1} ({2})" -f $Port, $proc.Id, $proc.ProcessName)
  } else {
    Write-Host ("Port {0} -> PID {1} (process not found)" -f $Port, $pid)
  }
}

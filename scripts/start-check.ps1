# Starts the built Windows program and checks that it really runs: no Visual C++ runtime
# needed (a fresh Windows 10 lacks it), and after a while it is still running with its window
# on a graphics adapter (the log's "GPU:" line comes once the app is set up in its window).
# CI's green tests never started the program; a crash at start-up was silent to the user.
#
#   pwsh scripts/start-check.ps1 <path to cut-craft.exe> [seconds]
param(
    [Parameter(Mandatory)] [string] $Exe,
    [int] $Seconds = 30
)
$ErrorActionPreference = 'Stop'

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $Exe))
$text = [System.Text.Encoding]::ASCII.GetString($bytes)
if ($text -match '(?i)vcruntime140(_1)?\.dll') {
    Write-Error "$Exe needs $($Matches[0]), which a fresh Windows 10 doesn't have (is crt-static set?)"
}

# its own settings folder, so the check starts as a first run and leaves the runner's alone
$env:APPDATA = Join-Path $env:RUNNER_TEMP 'start-check-appdata'
Remove-Item -Recurse -Force $env:APPDATA -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $env:APPDATA | Out-Null
$folder = Join-Path $env:APPDATA 'cut-craft'
$log = Join-Path $folder 'cut-craft.log'
$crash = Join-Path $folder 'crash.log'

$process = Start-Process -FilePath $Exe -PassThru
Start-Sleep -Seconds $Seconds
$process.Refresh()
if (Test-Path $log) { Write-Host "--- cut-craft.log"; Get-Content $log }
if ($process.HasExited) {
    if (Test-Path $crash) { Write-Host "--- crash.log"; Get-Content $crash }
    Write-Error "Cut Craft stopped within $Seconds s (exit code $($process.ExitCode))"
}
$up = (Test-Path $log) -and (Select-String -Path $log -Pattern 'GPU:' -Quiet)
Stop-Process -Id $process.Id -Force
if (-not $up) {
    Write-Error "Cut Craft is running but never set up its window (no GPU line in its log); a start-up message box may be waiting"
}
Write-Host "Cut Craft started and kept running for $Seconds s"

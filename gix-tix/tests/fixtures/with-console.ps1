param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$TestName,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

# A hidden, separate console makes console inheritance testable even when Cargo
# runs with redirected streams or without an attached console. Keep the working
# directory so the re-executed Rust test can still resolve its fixture scripts.
$stdout = Join-Path $OutputDirectory 'stdout.txt'
$stderr = Join-Path $OutputDirectory 'stderr.txt'
$process = Start-Process -FilePath $Executable `
    -ArgumentList @('--exact', $TestName, '--nocapture', '--test-threads=1') `
    -WorkingDirectory (Get-Location).Path -WindowStyle Hidden `
    -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru
if (-not $process.WaitForExit(30000)) {
    Stop-Process -Id $process.Id -Force
    throw 'The console editor test did not finish within 30 seconds.'
}
# Wait for redirected output to drain as well as for the process to exit.
$process.WaitForExit()
Get-Content -LiteralPath $stdout, $stderr
exit $process.ExitCode

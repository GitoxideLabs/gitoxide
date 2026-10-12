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
# Starting PowerShell and compiling the console probe for both editor launches
# can exceed 30 seconds on a busy Windows runner. Allow startup time as well as
# time for the assertions, while still bounding a hung test.
$exited = $process.WaitForExit(120000)
if (-not $exited) {
    # An editor or shell left alive can keep the redirected streams open.
    $process.Kill($true)
}
# Wait for redirected output to drain as well as for the process to exit.
$process.WaitForExit()
Get-Content -LiteralPath $stdout, $stderr
if (-not $exited) {
    throw 'The console editor test did not finish within 120 seconds.'
}
exit $process.ExitCode

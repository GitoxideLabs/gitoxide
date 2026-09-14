param(
    [Parameter(Mandatory = $true)][uint32]$ExpectedConsoleProcess,
    [Parameter(Mandatory = $true)][string]$Path
)

$ErrorActionPreference = 'Stop'

# CREATE_NO_WINDOW can still provide console devices, but they belong to a
# separate hidden console. Check membership, not just whether CONIN$ opens.
Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public static class ConsoleEditor {
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint GetConsoleProcessList([Out] uint[] processes, uint count);
}
'@
$processes = [uint32[]]::new(16)
while ($true) {
    $count = [ConsoleEditor]::GetConsoleProcessList($processes, $processes.Length)
    if ($count -eq 0) {
        throw 'The editor has no console.'
    }
    if ($count -le $processes.Length) {
        break
    }
    $processes = [uint32[]]::new($count)
}
if ($processes -notcontains $ExpectedConsoleProcess) {
    throw 'The editor must share the calling process console.'
}
[System.IO.File]::WriteAllText($Path, "edited with an inherited console`n", [System.Text.UTF8Encoding]::new($false))

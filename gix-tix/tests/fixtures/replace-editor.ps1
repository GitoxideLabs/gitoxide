param(
    [Parameter(Mandatory = $true)][string]$Old,
    [Parameter(Mandatory = $true)][string]$New,
    [Parameter(Mandatory = $true)][string]$Path
)

$content = [System.IO.File]::ReadAllText($Path)
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($Path, $content.Replace($Old, $New), $utf8WithoutBom)

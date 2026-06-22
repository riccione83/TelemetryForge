param(
    [Parameter(Mandatory = $true)][string]$ProjectDirectory,
    [Parameter(Mandatory = $true)][string]$Output
)

$ErrorActionPreference = "Stop"
$project = (Resolve-Path -LiteralPath $ProjectDirectory).Path
$manifest = Join-Path $project "manifest.yaml"
$wasm = Join-Path $project "widget.wasm"
if (-not (Test-Path -LiteralPath $manifest)) { throw "manifest.yaml is missing" }
if (-not (Test-Path -LiteralPath $wasm)) { throw "widget.wasm is missing" }

$outputPath = [System.IO.Path]::GetFullPath($Output)
$temporary = [System.IO.Path]::ChangeExtension($outputPath, ".zip")
Remove-Item -LiteralPath $temporary, $outputPath -Force -ErrorAction SilentlyContinue
Compress-Archive -LiteralPath $manifest, $wasm -DestinationPath $temporary -CompressionLevel Optimal
Move-Item -LiteralPath $temporary -Destination $outputPath
Write-Output $outputPath

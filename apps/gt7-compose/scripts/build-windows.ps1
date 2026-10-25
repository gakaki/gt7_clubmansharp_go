$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$resources = Join-Path $root '..\..\site\public'
if (-not (Test-Path (Join-Path $resources 'data\cars.json'))) {
    throw "Missing offline resource tree: $resources"
}

Push-Location $root
try {
    & .\gradlew.bat :composeApp:verifyOfflineSources
    & .\gradlew.bat :composeApp:assembleDebug
    & .\gradlew.bat :composeApp:createDistributable
    & .\gradlew.bat :composeApp:packageWindowsPortableZip
} finally {
    Pop-Location
}

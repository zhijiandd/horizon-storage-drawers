param(
    [string]$GamePath='F:\HorizonMods\tooling\horizon-target\fast\horizon-client.exe',
    [string]$ResourcesPath='F:\Games\Horizon\data\versions\nightly-2026-11-05-66f3981\.local\horizon-client\26.3'
)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path $PSScriptRoot -Parent
$testRoot=(Get-Content (Join-Path $projectRoot 'docs\active-regression-root.txt') -Raw).Trim()
if(-not $testRoot.StartsWith((Join-Path $projectRoot 'scratch\'))){throw 'Expected isolated regression directory'}
$env:HORIZON_HOME=Join-Path $testRoot 'home'
$env:HORIZON_LAUNCHER_HOME=Join-Path $testRoot 'launcher'
$env:XDG_DATA_HOME=Join-Path $testRoot 'xdg-data'
$env:XDG_CONFIG_HOME=Join-Path $testRoot 'xdg-config'
$env:HORIZON_LAUNCHER_TRASH_DIR=Join-Path $testRoot 'trash'
$env:HORIZON_LAUNCHER_DESKTOP_DIR=Join-Path $testRoot 'desktop'
$env:HORIZON_LAUNCHER_CLIPBOARD_FILE=Join-Path $testRoot 'clipboard.txt'
$env:HORIZON_LAUNCHER_NO_SELF_UPDATE='1'
$env:HORIZON_NO_AUDIO='1'
$env:TEMP='F:\HorizonMods\tooling\temp'
$env:TMP=$env:TEMP
& 'F:\HorizonMods\tooling\horizon-target\fast\horizon-ctl.exe' --socket (Join-Path $testRoot 'control.json') launch --game $GamePath --game-dir $testRoot --resources $ResourcesPath --world 'Drawers-Regression' --creative --offscreen --render-distance 4 --width 1280 --height 720 --wait | Out-File (Join-Path $testRoot 'launch-result.json') -Encoding utf8
Get-Content (Join-Path $testRoot 'control.log') -TotalCount 8

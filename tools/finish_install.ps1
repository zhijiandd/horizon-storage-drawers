$ErrorActionPreference='Stop'
$projectRoot=Split-Path $PSScriptRoot -Parent
$installRoot=(Resolve-Path -LiteralPath 'F:\Games\Horizon').ProviderPath.TrimEnd('\')
$plan=Get-Content (Join-Path $projectRoot 'docs\install-plan.json') -Raw | ConvertFrom-Json
$keep=(Resolve-Path -LiteralPath $plan.destination).ProviderPath.TrimEnd('\')
$versionRoot=Join-Path $installRoot 'data\versions'
if(-not $keep.StartsWith($versionRoot+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Invalid keep directory'}
if(-not (Test-Path -LiteralPath (Join-Path $keep '.installed'))){throw 'New build is not complete'}
$running=Get-CimInstance Win32_Process | Where-Object {
    $_.Name -match '^horizon-(client|launcher|server)\.exe$' -and $_.ExecutablePath -and
    $_.ExecutablePath.StartsWith($installRoot+'\',[StringComparison]::OrdinalIgnoreCase)
}
if($running){throw 'Close the Horizon game and launcher before cleanup'}
$targets=@(Get-ChildItem -LiteralPath $versionRoot -Directory | Where-Object {$_.FullName -ne $keep} | ForEach-Object {$_.FullName})
$targets+=@('data\native','data\run','game\horizon-native-mods','game\horizon-mods',
    'game\mod-browser','game\.horizon-cache','game\config\ae2') | ForEach-Object {Join-Path $installRoot $_}
$checked=@()
foreach($target in $targets){
    if(-not (Test-Path -LiteralPath $target)){continue}
    $absolute=(Resolve-Path -LiteralPath $target).ProviderPath.TrimEnd('\')
    if(-not $absolute.StartsWith($installRoot+'\',[StringComparison]::OrdinalIgnoreCase) -or $absolute -eq $keep){throw "Unsafe cleanup target: $absolute"}
    $item=Get-Item -LiteralPath $absolute -Force
    if($item.Attributes -band [IO.FileAttributes]::ReparsePoint){throw "Cleanup target is a link: $absolute"}
    $links=@(Get-ChildItem -LiteralPath $absolute -Recurse -Force -Attributes ReparsePoint)
    if($links.Count){throw "Links require separate review inside: $absolute"}
    $checked+=$absolute
}
$accounts=Join-Path $installRoot 'data\accounts.json'
$accountsHash=(Get-FileHash -LiteralPath $accounts -Algorithm SHA256).Hash
$worldRoot=Join-Path $installRoot 'game\saves'
$worlds=@(Get-ChildItem -LiteralPath $worldRoot -Directory | ForEach-Object {$_.Name})
$backup=Join-Path $projectRoot 'docs\installation-settings-before-cleanup'
New-Item -ItemType Directory -Path $backup -Force | Out-Null
foreach($file in @('data\settings.json','data\instances.json','game\horizon-mods.txt','game\mod-browser.json')){
    $source=Join-Path $installRoot $file
    if(Test-Path -LiteralPath $source){Copy-Item -LiteralPath $source -Destination (Join-Path $backup ([IO.Path]::GetFileName($source)))}
}
# Every final deletion path was resolved and checked above. Keep one shell for all removals.
foreach($target in $checked){Remove-Item -LiteralPath $target -Recurse -Force}
$mods=Join-Path $installRoot 'game\horizon-native-mods'
New-Item -ItemType Directory -Path $mods -Force | Out-Null
Copy-Item -LiteralPath $plan.mod -Destination (Join-Path $mods 'storagedrawers.hnmod')
Set-Content -LiteralPath (Join-Path $installRoot 'game\horizon-mods.txt') -Value 'mod.storagedrawers:true' -Encoding utf8
Set-Content -LiteralPath (Join-Path $installRoot 'game\mod-browser.json') -Value '{"entries":[]}' -Encoding utf8
$settingsFile=Join-Path $installRoot 'data\settings.json'
$settings=Get-Content -LiteralPath $settingsFile -Raw | ConvertFrom-Json
$settings | Add-Member -NotePropertyName selected -NotePropertyValue $plan.version -Force
$settings | Add-Member -NotePropertyName install_new_versions -NotePropertyValue $false -Force
$settings | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $settingsFile -Encoding utf8
$instanceFile=Join-Path $installRoot 'data\instances.json'
$instances=Get-Content -LiteralPath $instanceFile -Raw | ConvertFrom-Json
$instances.instances.horizon.version=$plan.version
$instances | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $instanceFile -Encoding utf8
if((Get-FileHash -LiteralPath $accounts -Algorithm SHA256).Hash -ne $accountsHash){throw 'Account data changed unexpectedly'}
$remainingWorlds=@(Get-ChildItem -LiteralPath $worldRoot -Directory | ForEach-Object {$_.Name})
if(Compare-Object $worlds $remainingWorlds){throw 'World directory list changed unexpectedly'}
$remaining=@(Get-ChildItem -LiteralPath $versionRoot -Directory | ForEach-Object {$_.Name})
$remainingMods=@(Get-ChildItem -LiteralPath $mods -File | ForEach-Object {$_.Name})
if($remaining.Count -ne 1 -or $remaining[0] -ne $plan.version){throw 'Unexpected installed versions'}
if($remainingMods.Count -ne 1 -or $remainingMods[0] -ne 'storagedrawers.hnmod'){throw 'Unexpected installed mods'}
$report=[ordered]@{version=$plan.version;mods=$remainingMods;deleted=$checked;accounts_preserved=$true;worlds_preserved=$remainingWorlds}
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $projectRoot 'docs\installation-result.json') -Encoding utf8
$report | ConvertTo-Json -Depth 5

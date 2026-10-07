#Requires -RunAsAdministrator
param([Parameter(Mandatory=$true)][string]$ConfigFile)
$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path (Join-Path $PSScriptRoot '../../target/release/rustdesk-wake.exe')).Path
& $binary check $ConfigFile
if ($LASTEXITCODE -ne 0) { throw 'Invalid wake helper configuration' }
$destination = Join-Path $env:ProgramFiles 'RustDeskWake'
if (Test-Path (Join-Path $destination 'config.json')) {
    throw 'Wake configuration already exists. Preserve or update its device keys explicitly.'
}
New-Item -ItemType Directory -Force -Path $destination | Out-Null
& icacls.exe $destination /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F'
if ($LASTEXITCODE -ne 0) { throw 'Could not restrict wake helper directory permissions' }
Copy-Item $binary (Join-Path $destination 'rustdesk-wake.exe')
Copy-Item $ConfigFile (Join-Path $destination 'config.json')
$action = New-ScheduledTaskAction -Execute (Join-Path $destination 'rustdesk-wake.exe') -Argument ('serve "' + (Join-Path $destination 'config.json') + '"')
$trigger = New-ScheduledTaskTrigger -AtStartup
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero)
Register-ScheduledTask -TaskName RustDeskWake -Action $action -Trigger $trigger -Settings $settings -User SYSTEM -RunLevel Highest -ErrorAction Stop | Out-Null
Start-ScheduledTask -TaskName RustDeskWake

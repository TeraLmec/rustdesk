#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'
Get-Service -Name RustDesk -ErrorAction Stop | Out-Null
Set-Service -Name RustDesk -StartupType Automatic
& sc.exe failure RustDesk reset= 86400 actions= restart/3000/restart/10000/restart/30000
if ($LASTEXITCODE -ne 0) { throw 'Could not configure RustDesk service recovery' }
Start-Service -Name RustDesk
Write-Host 'RustDesk starts at boot and restarts after a service failure.'
Write-Host 'Set a permanent password and enable Require unattended access in RustDesk Security settings.'

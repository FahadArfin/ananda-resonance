$ErrorActionPreference = 'Stop'
$path = Join-Path $env:ANANDA_BACKUP 'peace-startup.json'
if (!(Test-Path -LiteralPath $path)) {exit 0}
$records = @(Get-Content -LiteralPath $path -Raw | ConvertFrom-Json)
foreach ($r in $records) {
    switch ($r.kind) {
        'registry' {
            $existing = Get-ItemPropertyValue -LiteralPath $r.path -Name $r.name -ErrorAction SilentlyContinue
            if ($null -eq $existing) { Set-ItemProperty -LiteralPath $r.path -Name $r.name -Value $r.value }
        }
        'shortcut' { if (!(Test-Path -LiteralPath $r.path)) {Copy-Item -LiteralPath $r.saved -Destination $r.path} }
        'task' { Enable-ScheduledTask -TaskPath $r.path -TaskName $r.name | Out-Null }
    }
}
exit 0

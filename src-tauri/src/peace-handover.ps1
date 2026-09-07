$ErrorActionPreference = 'Stop'
$config = $env:ANANDA_APO_CONFIG
$backup = $env:ANANDA_BACKUP
$peaceExe = [IO.Path]::GetFullPath((Join-Path $config 'Peace.exe'))
$records = [Collections.Generic.List[object]]::new()
$runPaths = @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Run', 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Run')
foreach ($runPath in $runPaths) {
    if (!(Test-Path -LiteralPath $runPath)) { continue }
    $key = Get-Item -LiteralPath $runPath
    foreach ($name in $key.GetValueNames()) {
        $value = $key.GetValue($name)
        if ($value -is [string] -and $value.IndexOf($peaceExe, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
            $records.Add(@{kind='registry'; path=$runPath; name=$name; value=$value})
        }
    }
}
$shell = New-Object -ComObject WScript.Shell
foreach ($folder in @([Environment]::GetFolderPath('Startup'), [Environment]::GetFolderPath('CommonStartup'))) {
    if (!(Test-Path -LiteralPath $folder)) {continue}
    foreach ($file in Get-ChildItem -LiteralPath $folder -Filter '*.lnk') {
        if ($shell.CreateShortcut($file.FullName).TargetPath -ieq $peaceExe) {
            $saved = Join-Path $backup ([Guid]::NewGuid().ToString() + '.lnk')
            Copy-Item -LiteralPath $file.FullName -Destination $saved
            $records.Add(@{kind='shortcut'; path=$file.FullName; saved=$saved})
        }
    }
}
foreach ($task in Get-ScheduledTask -ErrorAction SilentlyContinue) {
    if ($task.State -eq 'Disabled') {continue}
    if (@($task.Actions | Where-Object { $_.Execute -is [string] -and $_.Execute.Trim('"') -ieq $peaceExe }).Count -gt 0) {
        $records.Add(@{kind='task'; path=$task.TaskPath; name=$task.TaskName})
    }
}
# Persist restoration information before disabling anything.
ConvertTo-Json -InputObject @($records.ToArray()) -Depth 5 | Set-Content -LiteralPath (Join-Path $backup 'peace-startup.json') -Encoding UTF8
foreach ($r in $records) {
    switch ($r.kind) {
        'registry' { Remove-ItemProperty -LiteralPath $r.path -Name $r.name }
        'shortcut' { Remove-Item -LiteralPath $r.path }
        'task' { Disable-ScheduledTask -TaskPath $r.path -TaskName $r.name | Out-Null }
    }
}
Get-Process -Name Peace -ErrorAction SilentlyContinue | Where-Object { $_.Path -ieq $peaceExe } | Stop-Process
exit 0

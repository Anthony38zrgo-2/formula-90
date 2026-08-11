. "$PSScriptRoot\_common.ps1"

Assert-Database
$sqlite = Assert-Sqlite

$tables = & $sqlite -readonly -noheader $DbPath @'
SELECT name
FROM sqlite_master
WHERE type='table'
  AND name NOT LIKE 'sqlite_%'
ORDER BY name;
'@

if ($LASTEXITCODE -ne 0) {
    throw "No se pudieron enumerar las tablas."
}

$result = foreach ($table in $tables) {
    if ([string]::IsNullOrWhiteSpace($table)) { continue }
    Assert-SafeIdentifier $table
    $count = (& $sqlite -readonly -noheader $DbPath "SELECT COUNT(*) FROM [$table];").Trim()
    [PSCustomObject]@{
        Table = $table
        Rows  = [int64]$count
    }
}

$result | Format-Table -AutoSize

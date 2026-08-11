. "$PSScriptRoot\_common.ps1"

Write-Section "Tablas"
Invoke-SqliteReadOnly @'
SELECT name
FROM sqlite_master
WHERE type = 'table'
  AND name NOT LIKE 'sqlite_%'
ORDER BY name;
'@

Write-Section "Índices"
Invoke-SqliteReadOnly @'
SELECT tbl_name AS table_name,
       name AS index_name,
       sql
FROM sqlite_master
WHERE type = 'index'
  AND name NOT LIKE 'sqlite_%'
ORDER BY tbl_name, name;
'@

Write-Section "DDL"
Invoke-SqliteReadOnly @'
SELECT type, name, sql
FROM sqlite_master
WHERE type IN ('table','index')
  AND name NOT LIKE 'sqlite_%'
ORDER BY type, name;
'@

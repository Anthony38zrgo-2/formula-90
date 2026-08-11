param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Sql,

    [switch]$Json
)

. "$PSScriptRoot\_common.ps1"

$trimmed = $Sql.TrimStart()
if ($trimmed -notmatch '^(?i)(SELECT|WITH|PRAGMA)\b') {
    throw "Este script es solo lectura. Se permiten SELECT, WITH y PRAGMA."
}

Assert-Database
$sqlite = Assert-Sqlite

if ($Json) {
    & $sqlite -readonly -json $DbPath $Sql
} else {
    & $sqlite -readonly -header -column $DbPath $Sql
}

if ($LASTEXITCODE -ne 0) {
    throw "La consulta SQLite falló."
}

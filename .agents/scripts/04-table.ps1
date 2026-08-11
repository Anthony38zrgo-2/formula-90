param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Table,

    [int]$Limit = 25,

    [switch]$Json
)

. "$PSScriptRoot\_common.ps1"

Assert-SafeIdentifier $Table
Assert-Database
$sqlite = Assert-Sqlite

$exists = (& $sqlite -readonly -noheader $DbPath "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$Table';").Trim()
if ($exists -ne "1") {
    throw "La tabla '$Table' no existe. Ejecuta 02-db-schema.ps1."
}

if ($Limit -lt 1 -or $Limit -gt 1000) {
    throw "Limit debe estar entre 1 y 1000."
}

if ($Json) {
    & $sqlite -readonly -json $DbPath "SELECT * FROM [$Table] LIMIT $Limit;"
} else {
    & $sqlite -readonly -header -column $DbPath "SELECT * FROM [$Table] LIMIT $Limit;"
}

if ($LASTEXITCODE -ne 0) {
    throw "No se pudo consultar '$Table'."
}

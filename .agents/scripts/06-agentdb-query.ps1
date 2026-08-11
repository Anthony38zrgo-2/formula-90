param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("knowledge","problem","instruction","agent")]
    [string]$Type,

    [string]$Domain,

    [Parameter(Mandatory=$true)]
    [string]$Query
)

. "$PSScriptRoot\_common.ps1"

switch ($Type) {
    "knowledge" {
        if (-not $Domain) { throw "-Domain es obligatorio para knowledge." }
        Invoke-AgentDb -Arguments @("knowledge", $Domain, $Query)
    }
    "instruction" {
        if (-not $Domain) { throw "-Domain es obligatorio para instruction." }
        Invoke-AgentDb -Arguments @("instruction", $Domain, $Query)
    }
    "problem" {
        Invoke-AgentDb -Arguments @("problem", $Query)
    }
    "agent" {
        Invoke-AgentDb -Arguments @("agent", $Query)
    }
}

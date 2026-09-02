# CAL-1300 regression-gate metric extractor. Compares physics_cli raw telemetry
# runs and prints subsystem metrics + stability flags for each scenario.
# Usage: cal_telemetry_metrics.ps1 -BaseDir <dir> -CompareDir <dir>
param(
    [Parameter(Mandatory = $true)][string]$BaseDir,
    [Parameter(Mandatory = $true)][string]$CompareDir
)

$modes = @('brake_100', 'skidpad', 'step_steer', 'slalom', 'power_on_exit', 'lift_off', 'curb', 'coast_down')

function Get-Metrics([string]$dir, [string]$mode) {
    $path = Join-Path $dir "$mode.csv"
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    $rows = Import-Csv -LiteralPath $path
    $dt = 0.008333
    # NaN / non-finite scan
    $hdr = $rows[0].PSObject.Properties.Name
    $bad = 0
    foreach ($h in $hdr) {
        foreach ($r in $rows) {
            $v = [string]$r.$h
            if ($v -eq 'NaN' -or $v -eq 'inf' -or $v -eq '-inf' -or $v -eq 'Infinity' -or $v -eq '-Infinity') { $bad++ }
        }
    }
    $out = [ordered]@{ mode = $mode; rows = $rows.Count; non_finite = $bad }
    $speed = $rows | ForEach-Object { [double]$_.Speed_kmh }
    $lng = $rows | ForEach-Object { [double]$_.Long_G }
    $lat = $rows | ForEach-Object { [double]$_.Lat_G }
    $yaw = $rows | ForEach-Object { [double]$_.YawRate_RadS }
    # oscillation: sign changes of yaw rate over the last 4 s (after transients)
    $window = $yaw[($yaw.Count - [int](4.0 / $dt))..($yaw.Count - 1)]
    $signs = 0
    for ($i = 1; $i -lt $window.Count; $i++) {
        if (($window[$i] -ge 0) -ne ($window[$i - 1] -ge 0)) { $signs++ }
    }
    $latAbs = @($lat | ForEach-Object { [math]::Abs($_) })
    $out.peak_lat_g = [double]($latAbs | Measure-Object -Maximum).Maximum
    $out.peak_long_neg_g = [math]::Abs([double]($lng | Measure-Object -Minimum).Minimum)
    $out.peak_yaw_rate = [double]($yaw | ForEach-Object { [math]::Abs($_) } | Measure-Object -Maximum).Maximum
    $out.yaw_sign_changes_4s = $signs
    $out.final_speed_kmh = $speed[$speed.Count - 1]
    if ($mode -eq 'brake_100') {
        $dist = 0.0; $stop_t = -1.0
        for ($i = 1; $i -lt $speed.Count; $i++) {
            $v = $speed[$i] / 3.6
            $dist += $v * $dt
            if ($stop_t -lt 0 -and $speed[$i] -lt 0.5) { $stop_t = $i * $dt }
        }
        $out.stop_distance_m = [math]::Round($dist, 1)
        $out.stop_time_s = [math]::Round($stop_t, 2)
    }
    return $out
}

$rows_any = $false
foreach ($m in $modes) {
    $b = Get-Metrics $BaseDir $m
    $c = Get-Metrics $CompareDir $m
    if (-not $b -or -not $c) { continue }
    $rows_any = $true
    Write-Host "=== $m ==="
    foreach ($k in $b.Keys) {
        if ($k -eq 'mode' -or $k -eq 'rows') { continue }
        $bv = $b[$k]; $cv = $c[$k]
        if ($bv -is [double] -and $cv -is [double]) {
            $delta = $cv - $bv
            $pct = if ($bv -ne 0) { [math]::Round(100.0 * $delta / [math]::Abs($bv), 1) } else { 0.0 }
            Write-Host ("  {0,-22} base={1,10} cmp={2,10} delta={3,10} ({4}%)" -f $k, $bv, $cv, $delta, $pct)
        } else {
            Write-Host ("  {0,-22} base={1} cmp={2}" -f $k, $bv, $cv)
        }
    }
}
if (-not $rows_any) { Write-Host "No matching scenario files found." }

# Diagnostic script to check what sensors are available on Windows

Write-Host "=== Checking CPU Temperature Sensors ===" -ForegroundColor Cyan

# Method 1: MSAcpi_ThermalZoneTemperature
Write-Host "`n1. MSAcpi_ThermalZoneTemperature:" -ForegroundColor Yellow
try {
    $temp = Get-WmiObject MSAcpi_ThermalZoneTemperature -Namespace "root/wmi" -ErrorAction SilentlyContinue
    if ($temp) {
        $temp | ForEach-Object { 
            $celsius = ($_.CurrentTemperature - 2732) / 10.0
            Write-Host "  Temperature: $celsius°C" -ForegroundColor Green
        }
    } else {
        Write-Host "  Not available" -ForegroundColor Red
    }
} catch {
    Write-Host "  Error: $_" -ForegroundColor Red
}

# Method 2: Win32_TemperatureProbe
Write-Host "`n2. Win32_TemperatureProbe:" -ForegroundColor Yellow
try {
    $probe = Get-WmiObject Win32_TemperatureProbe -ErrorAction SilentlyContinue
    if ($probe) {
        $probe | ForEach-Object {
            Write-Host "  CurrentReading: $($_.CurrentReading)" -ForegroundColor Green
        }
    } else {
        Write-Host "  Not available" -ForegroundColor Red
    }
} catch {
    Write-Host "  Error: $_" -ForegroundColor Red
}

Write-Host "`n=== Checking Fan Sensors ===" -ForegroundColor Cyan

# Method 1: Win32_Fan
Write-Host "`n1. Win32_Fan:" -ForegroundColor Yellow
try {
    $fans = Get-WmiObject Win32_Fan -ErrorAction SilentlyContinue
    if ($fans) {
        $fans | ForEach-Object {
            Write-Host "  Name: $($_.Name)" -ForegroundColor Green
            Write-Host "  DesiredSpeed: $($_.DesiredSpeed)" -ForegroundColor Green
            Write-Host "  ActiveCooling: $($_.ActiveCooling)" -ForegroundColor Green
        }
    } else {
        Write-Host "  Not available" -ForegroundColor Red
    }
} catch {
    Write-Host "  Error: $_" -ForegroundColor Red
}

Write-Host "`n=== Checking Power/Performance ===" -ForegroundColor Cyan

# Method 1: Processor Performance Counter
Write-Host "`n1. Processor Performance Counter:" -ForegroundColor Yellow
try {
    $perf = Get-Counter '\Processor Information(_Total)\% Processor Performance' -ErrorAction SilentlyContinue
    if ($perf) {
        $value = $perf.CounterSamples.CookedValue
        Write-Host "  Performance: $value%" -ForegroundColor Green
    } else {
        Write-Host "  Not available" -ForegroundColor Red
    }
} catch {
    Write-Host "  Error: $_" -ForegroundColor Red
}

# Method 2: Win32_Processor LoadPercentage
Write-Host "`n2. Win32_Processor LoadPercentage:" -ForegroundColor Yellow
try {
    $cpu = Get-WmiObject Win32_Processor -ErrorAction SilentlyContinue
    if ($cpu) {
        Write-Host "  LoadPercentage: $($cpu.LoadPercentage)%" -ForegroundColor Green
    } else {
        Write-Host "  Not available" -ForegroundColor Red
    }
} catch {
    Write-Host "  Error: $_" -ForegroundColor Red
}

Write-Host "`n=== Summary ===" -ForegroundColor Cyan
Write-Host "If all sensors show 'Not available', your system doesn't expose these" -ForegroundColor Yellow
Write-Host "sensors through WMI. You'll need third-party tools like:" -ForegroundColor Yellow
Write-Host "  - HWiNFO64 (with shared memory support)" -ForegroundColor White
Write-Host "  - Open Hardware Monitor" -ForegroundColor White
Write-Host "  - LibreHardwareMonitor" -ForegroundColor White
Write-Host "  - AMD Ryzen Master (for AMD CPUs)" -ForegroundColor White

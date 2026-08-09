$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = Split-Path -Parent $PSScriptRoot
$starroomUrl = 'http://127.0.0.1:4173'

function Test-StarroomServer {
    try {
        $response = Invoke-WebRequest -Uri $starroomUrl -UseBasicParsing -TimeoutSec 2
        return $response.StatusCode -eq 200 -and $response.Content -match '<title>Starroom</title>'
    }
    catch {
        return $false
    }
}

try {
    if (-not (Test-Path -LiteralPath (Join-Path $projectRoot 'dist\index.html'))) {
        $npm = (Get-Command npm.cmd -ErrorAction Stop).Source
        Push-Location $projectRoot
        try {
            & $npm run build
            if ($LASTEXITCODE -ne 0) {
                throw "Starroom production build failed with exit code $LASTEXITCODE."
            }
        }
        finally {
            Pop-Location
        }
    }

    if (-not (Test-StarroomServer)) {
        $npm = (Get-Command npm.cmd -ErrorAction Stop).Source
        Start-Process -FilePath $npm `
            -ArgumentList @('run', 'preview', '--', '--host', '127.0.0.1', '--port', '4173') `
            -WorkingDirectory $projectRoot `
            -WindowStyle Hidden | Out-Null

        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        while ([DateTime]::UtcNow -lt $deadline -and -not (Test-StarroomServer)) {
            Start-Sleep -Milliseconds 250
        }
    }

    if (-not (Test-StarroomServer)) {
        throw 'Starroom local server did not become ready on port 4173.'
    }

    Start-Process $starroomUrl
    Write-Host 'Starroom is running.' -ForegroundColor Green
    exit 0
}
catch {
    Write-Error $_
    exit 1
}

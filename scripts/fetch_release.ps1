# Variables
$REPO = "kajmaj87/sb3"
$TOKEN = "REPLACE_WITH_TOKEN"
$ARTIFACT_NAME = "sb3-x86_64-pc-windows-gnu"
$UNPACK_DIR = "C:\temp\sb3"
$RUN_ID_FILE = Join-Path $UNPACK_DIR "last_run_id.txt"

# Make sure destination directory exists
if (!(Test-Path -Path $UNPACK_DIR )) {
    New-Item -ItemType directory -Path $UNPACK_DIR
}

# Headers for requests
$headers = @{
    "Authorization" = "token $TOKEN"
    "Accept" = "application/vnd.github.v3+json"
}

# Get the last successful run
$response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/actions/runs?status=completed&event=push" -Headers $headers -Method Get
$RUN_ID = $response.workflow_runs | Where-Object { $_.conclusion -eq "success" } | Select-Object -First 1 | ForEach-Object { $_.id }

Write-Host "Run ID: $RUN_ID"

# Get the last saved run ID
$LAST_RUN_ID = if (Test-Path -Path $RUN_ID_FILE) {
    Get-Content -Path $RUN_ID_FILE
} else {
    $null
}

# Download and unpack the artifact if needed
if ($RUN_ID -ne $LAST_RUN_ID) {
    # Get artifact URL
    $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID/artifacts" -Headers $headers -Method Get
    $ARTIFACT_URL = $response.artifacts | Where-Object { $_.name -eq "$ARTIFACT_NAME" } | ForEach-Object { $_.archive_download_url }

    Write-Host "Artifact URL: $ARTIFACT_URL"

    # Download artifact
    $ARTIFACT_FILE = Join-Path $UNPACK_DIR "artifact.zip"
    Invoke-WebRequest -Uri $ARTIFACT_URL -Headers $headers -OutFile $ARTIFACT_FILE

    # Unzip the artifact
    Expand-Archive -Path $ARTIFACT_FILE -DestinationPath $UNPACK_DIR -Force

    # Save the run ID
    Set-Content -Path $RUN_ID_FILE -Value $RUN_ID
}

# Run the binary
$executable = Join-Path $UNPACK_DIR "sb3.exe"
Start-Process -FilePath "cmd.exe" -ArgumentList "/k", "$executable" -WorkingDirectory $UNPACK_DIR

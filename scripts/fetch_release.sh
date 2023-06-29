#!/bin/bash

# Set variables
REPO="kajmaj87/sb3"
TOKEN=$GH_TOKEN
ARTIFACT_NAME="sb3-x86_64-unknown-linux-gnu"

# Get the last successful run
RUN_ID=$(curl -s -H "Authorization: token $TOKEN" -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/repos/$REPO/actions/runs?status=completed&event=push" | \
  jq '[.workflow_runs[] | select(.conclusion=="success")][0].id')

echo "Run ID: $RUN_ID"

RAW_RESPONSE=$(curl -s -H "Authorization: token $TOKEN" -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID/artifacts")

echo "Raw response: $RAW_RESPONSE"

# Get artifact URL
ARTIFACT_URL=$(echo $RAW_RESPONSE | jq -r ".artifacts[] | select(.name==\"$ARTIFACT_NAME\") | .archive_download_url")

echo "Artifact URL: $ARTIFACT_URL"

# Download artifact
curl -L -o artifact.zip -H "Authorization: token $TOKEN" "$ARTIFACT_URL"

# Unzip and set permissions
unzip artifact.zip
chmod +x sb3

# Run the binary
./sb3

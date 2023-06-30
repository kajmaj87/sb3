#!/bin/bash

# Set variables
REPO="kajmaj87/sb3"
TOKEN=$GH_TOKEN
ARTIFACT_NAME="sb3-x86_64-unknown-linux-gnu"
UNPACK_DIR=/tmp/sb3
LAST_RUN_ID_FILE=$UNPACK_DIR/last_run_id.txt

# Get the last successful run
RUN_ID=$(curl -s -H "Authorization: token $TOKEN" -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/repos/$REPO/actions/runs?status=completed" | \
  jq '[.workflow_runs[] | select(.conclusion=="success")][0].id')

echo "Run ID: $RUN_ID"

# If last_run_id.txt exists, read its value into LAST_RUN_ID
if [ -f "$LAST_RUN_ID_FILE" ]; then
  LAST_RUN_ID=$(cat $LAST_RUN_ID_FILE)
else
  LAST_RUN_ID=0
fi

# Only download and unpack if the run ID has changed
if [ $RUN_ID != $LAST_RUN_ID ]; then
  RAW_RESPONSE=$(curl -s -H "Authorization: token $TOKEN" -H "Accept: application/vnd.github.v3+json" \
    "https://api.github.com/repos/$REPO/actions/runs/$RUN_ID/artifacts")

  echo "Raw response: $RAW_RESPONSE"

  # Get artifact URL
  ARTIFACT_URL=$(echo $RAW_RESPONSE | jq -r ".artifacts[] | select(.name==\"$ARTIFACT_NAME\") | .archive_download_url")

  echo "Artifact URL: $ARTIFACT_URL"

  # Download artifact
  mkdir -p $UNPACK_DIR
  curl -L -o $UNPACK_DIR/artifact.zip -H "Authorization: token $TOKEN" "$ARTIFACT_URL"

  # Unzip without asking about overrides into $UNPACK_DIR
  unzip -o $UNPACK_DIR/artifact.zip -d $UNPACK_DIR/$LAST_RUN_ID
  chmod +x $UNPACK_DIR/$LAST_RUN_ID/sb3

  # Save the new run ID
  echo $RUN_ID > $LAST_RUN_ID_FILE
fi

# Run the binary
$UNPACK_DIR/$LAST_RUN_ID/sb3

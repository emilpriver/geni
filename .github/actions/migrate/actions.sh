#!/bin/bash

# GitHub user/repo
USER_REPO="emilpriver/geni"

# Fetch the latest tag name
LATEST_TAG=$(curl -s https://api.github.com/repos/$USER_REPO/releases/latest | grep 'tag_name' | cut -d '"' -f 4)

# Determine the OS and set the appropriate download URL
OS="$(uname -s)"
case "$OS" in
    Linux*)     OS_TYPE='linux'; BINARY_SUFFIX='linux-amd64' ;;
    Darwin*)    OS_TYPE='macos'; BINARY_SUFFIX='macos-amd64' ;;
    CYGWIN*|MINGW32*|MSYS*|MINGW*) 
                OS_TYPE='windows'; BINARY_SUFFIX='windows-amd64' ;;
    *)          echo "Unsupported OS: $OS"; exit 1 ;;
esac

DOWNLOAD_URL="https://github.com/$USER_REPO/releases/download/$LATEST_TAG/geni-$BINARY_SUFFIX"

# Download and make the binary executable
curl -L $DOWNLOAD_URL -o geni_binary
if [ "$OS_TYPE" != "windows" ]; then
    chmod +x geni_binary
fi

# Set environment variables
export DATABASE_MIGRATIONS_FOLDER="${inputs.migrations_folder}"
export DATABASE_URL="${inputs.database_url}"
export DATABASE_TOKEN="${inputs.database_token}"
export DATABASE_WAIT_TIMEOUT="${inputs.wait_timeout}"
export DATABASE_NO_DUMP_SCHEMA="false"
export DATABASE_MIGRATIONS_TABLE="${inputs.migrations_table}"

./geni_binary up


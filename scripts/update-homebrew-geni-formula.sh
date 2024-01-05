#!/bin/bash

# Define variables
REPO_URL="https://github.com/emilpriver/homebrew-geni.git"
FORMULA_PATH="Formula/g/geni.rb"
NEW_VERSION="v0.0.8"

# Clone the repository
git clone "$REPO_URL"
cd homebrew-geni

# Update the version in geni.rb
sed -i "s|url \".*\"|url \"https://github.com/emilpriver/geni/archive/refs/tags/v$NEW_VERSION.tar.gz\"|" $FORMULA_PATH

# Get the new tarball and calculate its SHA256 checksum
NEW_TARBALL_URL="https://github.com/emilpriver/geni/archive/refs/tags/v$NEW_VERSION.tar.gz"
CHECKSUM=$(curl -Ls $NEW_TARBALL_URL | sha256sum | awk '{print $1}')

# Update the sha256 in geni.rb
sed -i "s|sha256 \".*\"|sha256 \"$CHECKSUM\"|" $FORMULA_PATH

git add .
git config --global user.email "emil@priver.dev"
git config --global user.name "emilpriver"
git commit -m "CI Action: Update geni to version $NEW_VERSION"
git push origin main

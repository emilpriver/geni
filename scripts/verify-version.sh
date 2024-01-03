#!/bin/bash

# Extract the version from Cargo.toml
cargo_version=$(sed -n '/\[package\]/,/\[.*\]/p' Cargo.toml | awk -F' = ' '$1 ~ /version/ {gsub(/"/, "", $2); print $2; exit}')

# Display the extracted version
echo "Version in Cargo.toml: $cargo_version"

# Display the GitHub Actions tag
echo "GitHub Actions tag: $GITHUB_REF_NAME"

# Compare the versions
if [[ "v$cargo_version" == "$GITHUB_REF_NAME" ]]; then
	echo "Version in Cargo.toml matches the new GitHub Actions tag."
else
	echo "Version in Cargo.toml does not match the new GitHub Actions tag."
	exit 1
fi

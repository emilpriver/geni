#!/bin/bash

# Get the version from Cargo.toml
cargo_version=$(grep -oP '(?<=version = ")[^"]*' Cargo.toml)
echo $cargo_version
echo $GITHUB_REF_NAME

# Compare the versions
if [[ "v$cargo_version" == "$GITHUB_REF_NAME" ]]; then
	echo "Version in Cargo.toml matches the new GitHub Actions tag."
else
	echo "Version in Cargo.toml does not match the new GitHub Actions tag."
	exit 1
fi

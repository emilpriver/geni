#!/bin/bash

# Get the version from Cargo.toml
cargo_version=$(grep -oP '(?<=version = ")[^"]*' Cargo.toml)

# Get the new GitHub Actions tag
github_tag=$GITHUB_REF

# Extract the version from the tag
tag_version=$(echo "$github_tag" | grep -oP '(?<=refs/tags/v)[^"]*')

# Compare the versions
if [[ "v$cargo_version" == "$tag_version" ]]; then
    echo "Version in Cargo.toml matches the new GitHub Actions tag."
else
    echo "Version in Cargo.toml does not match the new GitHub Actions tag."
    exit 1
fi

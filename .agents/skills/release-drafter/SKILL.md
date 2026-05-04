# Release Drafter

Create or update a draft release for the geni project using the `gh` CLI.

## Changelog

Location: `CHANGELOG` (root of repository, no file extension)

### Format

Use these sections under `## Unreleased`:

- `### Breaking Changes` - API changes requiring migration
- `### Added` - New features
- `### Changed` - Changes to existing functionality
- `### Fixed` - Bug fixes
- `### Removed` - Removed features

If no subsections exist, simple bullet points are acceptable.

### Rules

- Before adding entries, read the full `## Unreleased` section to see which subsections already exist
- New entries ALWAYS go under `## Unreleased` section
- Append to existing subsections (e.g., `### Fixed`), do not create duplicates
- NEVER modify already-released version sections
- Each version section is immutable once released

### Attribution

- **Internal changes (from issues)**: `Fixed foo bar ([#123](https://github.com/emilpriver/geni/issues/123))`
- **External contributions**: `Added feature X ([#456](https://github.com/emilpriver/geni/pull/456) by [@username](https://github.com/username))`

## Releasing

**Single package**: The project has one version defined in `Cargo.toml`.

**Version semantics** (no major releases):

- `patch`: Bug fixes and new features
- `minor`: API breaking changes

### Steps

1. **Check for existing draft release**

   ```bash
   gh release list --limit 10 --json tagName,isDraft,name
   ```

   Look for any release where `isDraft` is `true`.

2. **Determine the new version**

   Read the current version from `Cargo.toml`:

   ```bash
   grep '^version = ' Cargo.toml
   ```

   Get the latest tag:

   ```bash
   git describe --tags --abbrev=0
   ```

   Calculate the new version:
   - If `## Unreleased` contains `### Breaking Changes`, bump minor version
   - Otherwise, bump patch version

3. **Update CHANGELOG for the release**

   Replace `## Unreleased` with `## [v<new-version>]` and add the release date:

   ```
   ## [v1.3.2] - 2025-01-15

   * Fixed schema dump for PostgreSQL...
   ```

   Add a new empty `## Unreleased` section at the top:

   ```
   ## Unreleased

   ## [v1.3.2] - 2025-01-15
   ...
   ```

4. **Update Cargo.toml version**

   Update the version in `Cargo.toml` to match the new release version.

5. **Create or update draft release**

   **If no draft exists**, create a new one:

   ```bash
   gh release create v<new-version> --draft --title "v<new-version>" --notes "<release-notes>"
   ```

   **If a draft already exists**, update it:

   ```bash
   gh release edit v<existing-draft-tag> --draft --title "v<new-version>" --notes "<release-notes>"
   ```

   For release notes, use the content from the `## [v<new-version>]` section of the CHANGELOG.

6. **Verify the release**

   ```bash
   gh release view v<new-version> --json tagName,isDraft,name,body,url
   ```

### Quick Commands Reference

```bash
# List releases (find drafts)
gh release list --limit 10 --json tagName,isDraft

# Create new draft release
gh release create v1.3.2 --draft --title "v1.3.2" --notes "Release notes here"

# Update existing draft release
gh release edit v1.3.1 --draft --title "v1.3.2" --notes "Updated notes"

# View release details
gh release view v1.3.2

# Delete a draft (if needed)
gh release delete v1.3.2 --yes
```

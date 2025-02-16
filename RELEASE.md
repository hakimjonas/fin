# Release Process for Finë

 
### Automatic Patch Releases

    When changes are merged into trunk without a manual tag, the CI/CD workflow will automatically bump the patch version.
    The sync-version.sh script will update Cargo.toml and package artifacts accordingly.
    The CHANGELOG.md will be overwritten with a new entry summarizing the changes since the last published release.

## Manual Releases (Major/Minor Updates)

    To manually update the version (e.g., for major or minor changes), push a tag (e.g., v0.2.5).
    The sync-version.sh script will use the provided version without auto bumping.
    Ensure that any necessary documentation is updated accordingly.

Also checkout the [CHANGELOG.md](CHANGELOG.md) for more details on the changes in each release.
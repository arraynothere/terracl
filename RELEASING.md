# Releasing

One-time setup:

1. Run `scripts/setup-apt-key.sh`, then add the generated private key as the repository secret `APT_GPG_PRIVATE_KEY` and delete the file.
2. Push to `main` and wait for CI to pass.

Every release:

1. Bump `version` in `Cargo.toml` and add an entry to `debian/changelog`.
2. Commit and push.
3. `git tag vX.Y.Z && git push --tags`

The Release workflow checks that the tag matches `Cargo.toml`, runs the tests, builds binaries for Linux (amd64, arm64), Android (aarch64) and Windows, builds `.deb` packages for amd64 and arm64, publishes everything to the GitHub release, and publishes a signed apt repository to the `gh-pages` branch.

After the first release, open Settings > Pages and set the source to the `gh-pages` branch, root folder.

Optional channels:

- Termux: submit `packaging/termux/build.sh` to termux-user-repository (fill in the source tarball SHA256).
- Scoop: publish `packaging/scoop/terracl.json` in a `scoop-bucket` repository (fill in the zip SHA256).
- winget: `wingetcreate new <release zip url>` after the release exists.

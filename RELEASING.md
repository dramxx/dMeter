# Releasing & winget publishing

## Cut a release

1. Bump `version` in `Cargo.toml`.
2. Commit, then tag and push:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. The `Release` workflow (`.github/workflows/release.yml`) builds `dmeter.exe`,
   zips it as `dmeter-<version>-windows-x64.zip`, and publishes a GitHub Release
   with the zip and its `.sha256` attached. The workflow fails if the tag
   doesn't match the `Cargo.toml` version, or if tests fail.

## Publish/update on winget

Winget packages live in the [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
repo as manifest YAML files — there's nothing to host yourself beyond the
GitHub Release asset from the step above.

1. Install the authoring tool (one-time):
   ```powershell
   winget install Microsoft.WingetCreate
   ```
2. First submission (creates the manifests interactively and opens a PR
   against winget-pkgs on your behalf — needs a GitHub login):
   ```powershell
   wingetcreate new https://github.com/dramxx/dMeter/releases/download/v0.1.0/dmeter-0.1.0-windows-x64.zip
   ```
   - Package Identifier: `dramxx.dMeter`
   - Installer type: `zip`, nested installer type `portable`, nested installer
     relative path `dmeter.exe`
   - License: MIT
3. For every subsequent release, updating is one command:
   ```powershell
   wingetcreate update dramxx.dMeter --version 0.1.1 --urls https://github.com/dramxx/dMeter/releases/download/v0.1.1/dmeter-0.1.1-windows-x64.zip --submit
   ```

   Add `--out manifests` to also keep a copy of the generated manifests under
   `manifests/` in this repo.

Once the PR is merged (Microsoft runs automated validation, usually
same-day), `winget upgrade dramxx.dMeter` picks up the new version.

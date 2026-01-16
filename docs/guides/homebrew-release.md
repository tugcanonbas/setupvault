# Homebrew Release (homebrew-core)

This guide prepares a `setupvault` formula for Homebrew core and documents the release steps.

## Required inputs
- Version tag (recommended: `v0.1.0`).
- GitHub release tarball URL.
- SHA256 checksum of the tarball.

## 1) Create the release tag
Make sure `Cargo.toml` uses the target version.

```bash
git tag -a v0.1.0 -m "setupvault 0.1.0"
git push origin v0.1.0
```

## 2) Create the GitHub release
Create a GitHub release for `v0.1.0`. Use the auto-generated source tarball (GitHub creates it automatically for tags).

Tarball URL format:
```
https://github.com/tugcanonbas/setupvault/archive/refs/tags/v0.1.0.tar.gz
```

## 3) Get the SHA256
Download the tarball and compute the checksum:

```bash
curl -L -o setupvault-0.1.0.tar.gz \
  https://github.com/tugcanonbas/setupvault/archive/refs/tags/v0.1.0.tar.gz
shasum -a 256 setupvault-0.1.0.tar.gz
```

## 4) Prepare the formula
A ready formula template is included in:
- `packaging/homebrew/setupvault.rb`

Update `sha256` with the value from step 3.

## 5) Create the Homebrew core PR
Homebrew core is managed in `homebrew/homebrew-core` and requires a PR.

```bash
brew update
export HOMEBREW_NO_INSTALL_FROM_API=1
brew tap homebrew/core
cd "$(brew --repository homebrew/core)"

git checkout -b setupvault-0-1-0 origin/HEAD
cp /path/to/setupvault/packaging/homebrew/setupvault.rb Formula/s/setupvault.rb

brew audit --new --formula setupvault
brew install --build-from-source --verbose --debug setupvault
brew test setupvault

git add Formula/s/setupvault.rb

git commit -m "setupvault 0.1.0 (new formula)"
git push https://github.com/tugcanonbas/homebrew-core setupvault-0-1-0
```

Then open a PR against `Homebrew/homebrew-core`.

## 6) After merge
Once the PR is merged, users can install with:

```bash
brew install setupvault
```

## Notes
- Homebrew core requires a stable tagged release.
- Keep commit messages in the required Homebrew format.
- Use `brew audit --new --formula setupvault` before submitting the PR.

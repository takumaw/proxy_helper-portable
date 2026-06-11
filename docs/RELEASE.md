# Release Process

Releases are driven by Git tags and managed via GitHub Actions.

Release notes are maintained in [HISTORY.md](../HISTORY.md).

## Version format

Git tags use a leading `v`:

```text
v0.1.0
```

`HISTORY.md` headings do not use a leading `v`:

```markdown
## 0.1.0
```

## Release assets

A release contains a single compressed archive targeting Linux:

```text
proxy_helper-vX.Y.Z-linux-amd64.tar.gz
```

The tarball has the following prefix directory structure:

```text
libexec/
  └── proxy_helper          # Statically compiled binary (x86_64-unknown-linux-musl)
share/
  └── man/
      └── man8/
          └── proxy_helper.8 # Man page documentation
```

## Pre-release checklist

Before creating a tag:

* `README.md` is up to date.
* `HISTORY.md` contains the release entry under the heading `## X.Y.Z`.
* `man/proxy_helper.8` is updated if CLI options changed.
* CI passes on `main`.
* Local builds and tests pass.

## Creating a release

To trigger a release (e.g., `v0.1.0`), push a signed Git tag representing the version:

```bash
# 1. Update HISTORY.md, README.md, etc., commit changes
git add HISTORY.md README.md Cargo.toml
git commit -m "release: v0.1.0"

# 2. Create a signed tag
git tag -s v0.1.0 -m "Release v0.1.0"

# 3. Push main branch and the tag
git push origin main
git push origin v0.1.0
```

The release workflow is triggered automatically by pushing a tag matching `v*`.

## Workflow behavior

The GitHub Actions release workflow (`.github/workflows/release.yml`) executes the following steps:

1. **Checkout**: Checks out the tag codebase.
2. **Setup dependencies**: Installs `musl-tools` and configures the musl rust target.
3. **Build**: Compiles a static release binary.
4. **Package**: Tarballs the static binary as `proxy_helper-vX.Y.Z-linux-amd64.tar.gz`.
5. **Release Notes**: Uses `awk` to extract the release changelog section matching the tag version from `HISTORY.md`.
6. **Publish**: Creates a GitHub Release, attaches the extracted release notes, and uploads the `.tar.gz` archive.

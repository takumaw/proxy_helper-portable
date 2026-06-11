# CONTRIBUTING

This project is small and focused. Keep changes minimal, clean, and well-tested.

## Requirements

Development targets Linux environments using Rust.

Required tools:

* Rust (Stable compiler toolchain)
* Cargo (Rust package manager)

## Build & install

To build the project in debug mode:

```bash
cargo build
```

To build a release-optimized binary:

```bash
cargo build --release
```

To build for the static target (requires musl-tools on Linux):

```bash
cargo build --release --target x86_64-unknown-linux-musl
```

## Test

To run the Rust unit tests:

```bash
cargo test
```

### Testing guidelines

* **Unit Tests**: Focus on pure logic (e.g., parsing exception domains,
  matching PAC rules inside JavaScript engine). Ensure mock data is used
  where system interactions (like GNOME/KDE config fetching) are involved.
* **Fish & PowerShell testing**: Verify that shell formatter outputs conform
  properly to the target shell syntax without warnings.

## Code style & linting

### rustfmt & clippy

This project enforces Rust style guidelines using standard Rust linting tools.
Make sure your changes are formatted and pass clippy checks:

To format code:

```bash
cargo fmt --check
```

To run clippy linter (warnings are treated as errors in CI):

```bash
cargo clippy --all-targets -- -D warnings
```

### Copyright header

Every Rust source file must start with the standard copyright comment header.
For example:

```rust
// Copyright (c) 2018-2026 Takuma WATANABE
```

Make sure new files include a matching header with the correct year.

## Documentation

Design changes should update [docs/DESIGN.md](docs/DESIGN.md).

Release process changes should update [docs/RELEASE.md](docs/RELEASE.md).

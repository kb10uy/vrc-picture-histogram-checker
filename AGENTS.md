# Repository Guidelines

## Overview

This is a VR overlay application to analyze and notify the saved picture of VRChat.

## Architecture
TBW

## Code Style

- Use Rust edition 2024.
- Follow standard Rust naming and formatting conventions.
- Use LF line endings for all text files. There is no `.gitattributes` to
  correct this after the fact, so every edit has to write LF itself. On Windows
  a tool that opens a file in the platform's text mode converts LF to CRLF
  without reporting it: Python's `pathlib.write_text` and `open(path, "w")`,
  PowerShell's `>` redirection, and `Set-Content` all do. Write bytes instead,
  or edit in place with a tool that preserves what is already there.
- Check `git diff --stat` before committing. A small edit that reports the
  whole file as changed has had its line endings rewritten, and the fix is to
  rewrite the file with LF rather than to commit it.
- Combine imports from the same crate into a single `use` statement within each
  module scope, except when different `cfg` attributes require separate imports.
- Avoid comments by default. Add comments only when explicitly requested by the user.
- Prefer the smallest correct implementation and avoid speculative abstractions.
- Prefer `modname.rs` and `modname/` style over `mod.rs` style.
- Use `rstest` features for parameterized tests, fixtures, and test cases where
  they improve coverage or reduce repetition.

## Documentation
TBW

## Build and Test

Before completing a code change, run at minimum:

```text
cargo fmt --all --check
cargo clippy
cargo test
cargo build
```

# Repository Guidelines

## Overview

This is a VR overlay application to analyze and notify the saved picture of VRChat.

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
- Avoid comments by default. Add comments only when explicitly requested by the
  user. Doc comments on `clap` argument fields are an exception: they are the
  `--help` output rather than an explanation of the code, so write them as doc
  comments instead of `#[arg(help = "...")]`. Do not put a doc comment on the
  `clap` struct itself, because it would override the `about` line that
  otherwise comes from the package description.
- Prefer the smallest correct implementation and avoid speculative abstractions.
- Prefer `modname.rs` and `modname/` style over `mod.rs` style.
- Use `rstest` features for parameterized tests, fixtures, and test cases where
  they improve coverage or reduce repetition.

## Documentation

Since the code carries no explanatory comments, anything a reader would
otherwise need a comment for belongs in one of these places.

- `README.md` is written for whoever runs the program: how to start it, how to
  register it with SteamVR, and what every configuration key means. Keep the
  configuration table in step with `config.rs`, including the defaults.
- `README.md` also records where the OpenVR binding came from: the interface
  versions, the command that regenerates the function table listing from
  `openvr_capi.h`, and the calling convention. Anyone touching `openvr/sys.rs`
  needs this, and it is too long to live in the source.
- Commit messages carry the reasoning. Say what forced the choice rather than
  restating the diff, especially where the obvious approach was rejected, where
  a value was measured rather than guessed, or where a platform detail decided
  the design.
- This file records decisions that outlive a single change: the shape of the
  code and the conventions to follow. Update the Architecture section when a
  module is added or its responsibility moves.

## Build and Test

Before completing a code change, run at minimum:

```text
cargo fmt --all --check
cargo clippy
cargo test
cargo build
```

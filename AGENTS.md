# Repository Guidelines

## Overview

This is a VR overlay application to analyze and notify the saved picture of VRChat.

## Architecture

`main.rs` owns the loop and little else. It wires the pieces together, decides
what reaches the screen, and is the only place that knows the order of a tick:
drain the window's messages, take whatever the watcher has finished, then move
the overlay's alpha along. The two boundaries worth protecting are that `openvr`
knows nothing about pictures or panels, and that everything under `app` knows
nothing about OpenVR.

- `paths` resolves siblings of the executable; `config` and `manifest` both sit
  on it, because the configuration and the manifest live beside the binary.
- `config` loads or creates `config.toml`, and resolves the Pictures known
  folder for the default watch directory.
- `openvr` is the binding. `sys` holds the function tables, `loader` finds and
  opens `openvr_api.dll` and its exports, `error` collects the failures.
  `Context` hands out an `Overlay`, which takes a frame as raw RGBA8 bytes and
  has no idea what is in it.
- `app::watcher` turns filesystem events into paths of pictures that VRChat has
  finished writing. Deciding that a file is complete is its problem, not the
  loop's.
- `app::analysis` decodes a picture into an `Analysis`: histogram, clipping
  ratios, and a thumbnail. `analyze_image` is split out from `analyze` so the
  measuring can be tested without touching the disk.
- `app::panel` draws an `Analysis`, or a message, into a `Frame`.
- `app::notification` is the fade, and only the fade.
- `app::window` is the desktop half: one Win32 window showing the last `Frame`.
- `app::console` reattaches the standard handles for the command line paths.
- `app::manifest` builds the `.vrmanifest` that `--register` hands to SteamVR.

`Frame` is the single currency for anything that ends up on screen: straight
alpha RGBA8, `PANEL_WIDTH` by `PANEL_HEIGHT`. The overlay uploads it unchanged.
The window composites it over the panel's own background colour first, because
the panel is drawn with rounded corners and on the desktop there is nothing
behind it to blend into.

`Notification` is time arithmetic and nothing else. Alpha is a function of
`now - shown_at` rather than a value stepped down once per tick, so an uneven
loop — a slow render, or the modal loop Windows runs while the window is being
dragged — changes how often the fade is sampled but neither its shape nor when
it ends. Keep it that way.

The binary is built for the Windows subsystem, so no module may assume a console
exists. Anything the user needs to see while `run` is going belongs on the
panel, and a failure that stops `run` from starting goes to a message box.
`app::console` exists so that `--register`, `--unregister` and the switches
`clap` answers by itself still report to the shell they were started from.

Win32 is reached through the `windows` crate rather than `windows-sys`, so
handles are typed and anything that can fail returns a `Result`. The one
deliberate exception to using a generated binding is `openvr/sys.rs`, which is
transcribed by hand for the reasons `README.md` gives.

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

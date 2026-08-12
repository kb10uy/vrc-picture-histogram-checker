# Repository Guidelines

## Overview

This is a VR overlay application to analyze and notify the saved picture of VRChat.

## Architecture

The target is Windows and SteamVR only. There is no backend abstraction and no
OpenXR support, so OpenVR is called directly.

```text
watcher   VRChat の写真フォルダを監視し、書き込みが終わったファイルを報告する
analysis  PNG をデコードし、ヒストグラム・白飛び/黒潰れ率・サムネイルを作る
panel     SVG を組み立てて resvg で RGBA8 のパネルにする
openvr    OpenVR の安全なラッパ。unsafe はこの階層から外に出さない
notification  パネルを表示している時間と alpha を経過時間から決める
config    実行ファイルの隣の config.toml
manifest  SteamVR に自動起動を登録するための .vrmanifest
paths     実行ファイルと同じフォルダのパス解決
main      上記を 30Hz のループでつなぐ
```

`openvr` は crates.io の binding を使わず自前の FFI を持つ。`openvr` crate が依存する
`openvr_sys` が cmake と libclang を要求するのに対し、必要な API は 10 個程度しかないため。
`openvr_api.dll` は SteamVR のランタイムから実行時に読み込むので、C ツールチェーンも DLL の
同梱も要らない。

`openvr/sys.rs` の関数テーブルは公式ヘッダ `openvr_capi.h` からの手作業の書き起こしで、
**使わないエントリも含めた宣言順がヘッダと一致していなければならない**。順序がずれると別の
関数を呼ぶ。対象の版数と照合手順は README を参照。

CPU で RGBA8 を作って `SetOverlayRaw` に渡すだけなので、D3D11 や wgpu への依存はない。
パネルは写真 1 枚につき一度だけ生成される静止画で、解析 69ms + 描画 8ms（release, 4K 写真）
とループを止めない範囲に収まっている。

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

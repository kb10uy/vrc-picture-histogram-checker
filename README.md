# VRC Picture Histogram Checker

VRChat が保存した写真を検出し、サムネイル・RGB ヒストグラム・白飛び/黒潰れ率をまとめた
パネルを SteamVR のオーバーレイとして表示します。

対応環境は Windows + SteamVR (OpenVR) のみです。

## 動かす

SteamVR を起動した状態で実行します。

```console
cargo run
```

`openvr_api.dll` は同梱していません。`%LOCALAPPDATA%\openvr\openvrpaths.vrpath` に
記録された SteamVR ランタイムから `bin\win64\openvr_api.dll` を実行時に読み込みます。
見つからない場合は実行ファイルの隣と `PATH` を探索します。

## OpenVR バインディングについて

`src/openvr/sys.rs` の関数テーブルは OpenVR の公式ヘッダ `openvr_capi.h` から手で
書き起こしたものです。対象のインターフェイス版数は次の通りです。

| インターフェイス | 版数 | エントリ数 |
| --- | --- | --- |
| `IVRSystem` | `IVRSystem_026` | 51 |
| `IVROverlay` | `IVROverlay_028` | 82 |

関数テーブルは関数ポインタが並んだ構造体なので、**使わないエントリも含めて宣言の順序が
ヘッダと完全に一致していなければなりません**。順序がずれると別の関数を呼び出してしまいます。
ヘッダを更新するときは、次のコマンドで得られる名前の並びと `sys.rs` のフィールドの並びが
一致していることを確認してください。

```console
awk '/^struct VR_IVROverlay_FnTable/,/^};/' openvr_capi.h \
  | sed -n 's/.*\*\([A-Za-z0-9_]*\))(.*/\1/p'
```

呼び出し規約はヘッダの `OPENVR_FNTABLE_CALLTYPE`、すなわち Win32 では `__stdcall` です。
Rust 側では `extern "system"` を使います。`VR_InitInternal` などの DLL エクスポート関数は
`__cdecl` なので `extern "C"` です。

起動時に `GetOverlayErrorNameFromEnum` を呼んで文字列が返ることを確認しており、関数テーブル
のずれをある程度は早期に検出できるようにしています。

## ライセンス

MIT または Apache-2.0 のデュアルライセンスです。

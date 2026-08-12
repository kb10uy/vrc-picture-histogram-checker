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

## SteamVR に自動起動させる

SteamVR の起動と同時に立ち上がるようにするには、実行ファイルを最終的に置く場所へ配置してから
登録します。

```console
vrc-pichistcheck --register
```

これは実行ファイルの隣に `vrc-pichistcheck.vrmanifest` を書き出し、`IVRApplications` の
`AddApplicationManifest` で SteamVR に登録したうえで `SetApplicationAutoLaunch` を有効に
します。登録後は SteamVR の設定にあるアドオン（スタートアップ）の一覧に現れるので、
オン/オフはそちらからも切り替えられます。

解除するには次のようにします。

```console
vrc-pichistcheck --unregister
```

登録されるのは**そのときの実行ファイルの絶対パス**です。実行ファイルを移動したりビルドし直して
別の場所に置いた場合は、移動先で `--register` をやり直してください。`cargo run` で使っている
`target\release\` のまま登録すると、そのパスに依存した登録になります。

## 設定

設定ファイルは `%APPDATA%\vrc-pichistcheck\config.toml` です。存在しない場合は初回起動時に
既定値で生成されます。

```toml
watch_directory = 'J:\Pictures\VRChat'
panel_width_meters = 0.4
panel_alpha = 0.75
display_seconds = 3.0
fade_seconds = 1.0
```

| 項目 | 意味 |
| --- | --- |
| `watch_directory` | 監視するフォルダ |
| `panel_width_meters` | パネルの横幅（メートル） |
| `panel_alpha` | 表示中の不透明度 |
| `display_seconds` | フェードを始めるまでの表示時間 |
| `fade_seconds` | フェードアウトにかける時間 |

`watch_directory` の既定値は Known Folder API で解決した Pictures フォルダ配下の `VRChat`
です。`%USERPROFILE%\Pictures` を直接組み立てていないのは、Pictures が別ドライブや OneDrive
にリダイレクトされている環境があるためです。

写真が保存されるたびにパネルが差し替わり、表示時間のカウントがやり直されます。連続で撮った
場合は最後の 1 枚が残ります。

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

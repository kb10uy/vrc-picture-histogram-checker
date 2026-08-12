# VRC Picture Histogram Checker

VRChat の写真撮影を検出し、ヒストグラムを解析して VR オーバーレイとして表示します。
OpenVR オーバーレイアプリとして動作し、VRChat のプロセスには干渉しません。またヒストグラムの計算はローカルで行われます。

## 使い方

適当なパスに展開し、SteamVR が起動している状態で vrc-picture-histogram-checker.exe を実行してください。
SteamVR 起動前に実行した場合自動で SteamVR が起動します。

### SteamVR にスタートアップアプリとして登録する

コマンドラインで `vrc-picture-histogram-checker.exe --register` を実行するとスタートアップとして登録されます。
登録解除するには `vrc-picture-histogram-checker.exe --unregister` を実行してください。

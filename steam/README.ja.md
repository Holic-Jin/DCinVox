# Steam アップロード (SteamPipe)

[한국어](README.md) · [English](README.en.md) · [日本語](README.ja.md)

Steamworks でアプリを作ると **App ID** と **Depot ID**（macOS 用・Windows 用）が分かります。
下の vdf ファイルの `APPID`、`DEPOT_MAC`、`DEPOT_WIN` をその数字に置き換えてください。

1. depot フォルダを作る
   - macOS: `tools/make_depot_mac.sh` → `steam/content/mac/`（DCinVox.app + models/ + 文書）
   - Windows: GitHub Actions の成果物（DCinVox-windows）を `steam/content/win/` に展開し、`models/` をコピー
2. アップロード（Steamworks SDK の ContentBuilder）
   ```
   ~/Documents/sdk/tools/ContentBuilder/builder_osx/steamcmd.sh +login <Steam アカウント> +run_app_build "$(pwd)/steam/app_build.vdf" +quit
   ```
   初回ログインでは Steam Guard コードを手入力します。
3. Steamworks のウェブでビルドをブランチ（default）に指定して公開

Steamworks SDK とログイン情報はコミットしないでください。

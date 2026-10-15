# Steam upload (SteamPipe)

[한국어](README.md) · [English](README.en.md) · [日本語](README.ja.md)

After you create the app in Steamworks you get an **App ID** and **Depot IDs** (macOS and Windows).
Replace `APPID`, `DEPOT_MAC`, and `DEPOT_WIN` in the vdf files with those numbers.

1. Build depot folders
   - macOS: `tools/make_depot_mac.sh` → `steam/content/mac/` (DCinVox.app + models/ + docs)
   - Windows: unpack the GitHub Actions artifact (DCinVox-windows) into `steam/content/win/` and copy `models/`
2. Upload with Steamworks SDK ContentBuilder:
   ```
   ~/Documents/sdk/tools/ContentBuilder/builder_osx/steamcmd.sh +login <Steam account> +run_app_build "$(pwd)/steam/app_build.vdf" +quit
   ```
   The first login asks for a Steam Guard code.
3. On the Steamworks site, set the build on the default branch, then ship.

Do not commit the Steamworks SDK or login credentials.

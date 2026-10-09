# Steam 업로드 (SteamPipe)

Steamworks에서 앱을 만들면 **App ID**와 **Depot ID**(macOS용·Windows용)가 나옵니다.
아래 vdf 파일의 `APPID`, `DEPOT_MAC`, `DEPOT_WIN` 을 그 숫자로 바꾸세요.

1. depot 폴더 만들기
   - macOS: `tools/make_depot_mac.sh` → `steam/content/mac/` (DCinVox.app + models/ + 문서)
   - Windows: GitHub Actions 결과물(DCinVox-windows)을 `steam/content/win/` 에 풀고 `models/` 복사
2. 업로드 (Steamworks SDK의 ContentBuilder 사용)
   ```
   ~/Documents/sdk/tools/ContentBuilder/builder_osx/steamcmd.sh +login <Steam 계정> +run_app_build "$(pwd)/steam/app_build.vdf" +quit
   ```
   처음 로그인할 때 Steam Guard 코드를 직접 입력합니다.
3. Steamworks 웹에서 빌드를 브랜치(default)에 지정 → 출시

<div align="center">
  <picture>
    <source
      width="180"
      srcset="./src-tauri/icons/icon.png"
      media="(prefers-color-scheme: dark)"
    />
    <source
      width="180"
      srcset="./src-tauri/icons/icon.png"
      media="(prefers-color-scheme: light), (prefers-color-scheme: no-preference)"
    />
    <img width="180" src="./src-tauri/icons/icon.png" />
  </picture>
</div>

<h1 align="center">아스트랄 파티 자동 패치 프로그램</h1>

<p align="center">
  <b>STAR ENGINE PROJECT</b>가 개발한 대전형 카드 배틀 주사위 보드게임 <b>Astral Party</b>의 
  <a href="https://github.com/maynut02/astralparty-korean-patch">비공식 유저 한국어 패치</a>
  를 자동으로 적용해주는 Windows 전용 프로그램입니다.
  <br>현재 Steam 글로벌 버전과 Bilibili PC버전을 지원합니다.
</p>

<div align="center">
  

</div>

<div align="center">
  
[![][version-shield]][release-link]
[![][ci-shield]][ci-link]
[![][steam-shield]][steam-link]
[![][discord-shield]][discord-link]
<!-- [![][tauri-shield]][tauri-link]
[![][rust-shield]][rust-link] -->

</div>

<p float="left" align="center">
  <!-- downloads -->
  <a href="https://github.com/maynut02/AstralAutoPatcher/releases/download/2.0.0/AstralAutoPatcher.exe">
    <picture>
      <source
        width="30%"
        srcset="./docs/img/program_download_links_1280x420.png"
        media="(prefers-color-scheme: dark)"
      />
      <source
        width="30%"
        srcset="./docs/img/program_download_links_1280x420.png"
        media="(prefers-color-scheme: light), (prefers-color-scheme: no-preference)"
      />
      <img width="30%" src="./docs/img/program_download_links_1280x420.png" />
    </picture>
  </a>
  
  <a href="https://github.com/maynut02/astralparty-korean-patch">
    <picture>
      <source
        width="30%"
        srcset="./docs/img/korean_patch_links_1280x420.png"
        media="(prefers-color-scheme: dark)"
      />
      <source
        width="30%"
        srcset="./docs/img/korean_patch_links_1280x420.png"
        media="(prefers-color-scheme: light), (prefers-color-scheme: no-preference)"
      />
      <img width="30%" src="./docs/img/korean_patch_links_1280x420.png" />
    </picture>
  </a>
</p>

> © 2026 MayNut. All rights reserved.

## 지원 버전

|버전|구분|모드|실행|
|----|----|----|----|
|Steam 글로벌 버전|INT_STEAM|patch, remove|[패치 모드](https://astral.maynutlab.com/autopatch/patch/INT_STEAM) \| [제거 모드](https://astral.maynutlab.com/autopatch/remove/INT_STEAM)|
|BiliBili PC 버전|CN_BILIBILI|patch, remove|[패치 모드](https://astral.maynutlab.com/autopatch/patch/CN_BILIBILI) \| [제거 모드](https://astral.maynutlab.com/autopatch/remove/CN_BILIBILI)|

## 사용 방법

1. Releases에서 최신 [AstralAutoPatcher.exe](https://github.com/maynut02/AstralAutoPatcher/releases/download/2.0.0/AstralAutoPatcher.exe) 다운로드
2. AstralAutoPatcher.exe 실행
3. 설치 모드 완료 후 프로그램 종료
4. [홈페이지](https://astral.maynutlab.com/)에 접속해 원하는 버전의 자동 패치 실행

## 프로그램 접근 경로

프로그램 설치, patch, remove 과정에서 아래 경로를 생성/조회/수정할 수 있습니다.

- 프로그램 설치 경로  
  `C:\Program Files\AstralAutoPatcher`

- 레지스트리 등록 경로  
  `HKEY_CURRENT_USER\Software\Classes\astral`

- 게임 설치 경로  
  `C:\Program Files (x86)\Steam\steamapps\common\Astral Party`  
  `C:\Program Files\bilibili Game\AstralParty`

- 게임 리소스 다운로드 경로  
  `C:\Users\%username%\AppData\LocalLow\feimo`

> [!note]
> 프로그램 자체 삭제 기능은 현재 제공되지 않습니다. 필요한 경우 설치 폴더와 레지스트리를 수동으로 삭제해 주세요.

## 문제 해결

Q. 엣지 브라우저에서 AstralAutoPatcher.exe 다운로드 중에 경고가 발생해요.  
A. ⋯ - 유지 - ∨ - 그래도 계속

## 개발자용

### 요구 사항

- [Tauri](https://tauri.app/start/create-project/)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/#download-section)
- [Rust](https://www.rust-lang.org/ko/tools/install)
- [Node.js](https://nodejs.org/download)
- Npm

### 개발
```bash
# 설치
npm install

# 개발 서버 실행
npm run tauri dev
```

### 빌드

```bash
npm run tauri build
```

### 프로토콜

```bash
astral://<MODE>/<TARGET>

# MODE
patch    # 한글패치 파일을 게임에 적용시키는 작업
remove   # 게임에 적용되어 있는 한글패치 파일을 제거하는 작업

# TARGET
INT_STEAM    # STEAM 글로벌 버전
CN_BILIBILI  # 빌리빌리 PC 버전
```

## 관련 프로젝트
- [astralparty-korean-patch](https://github.com/maynut02/astralparty-korean-patch)
- ~~[AstralParty-KoPatch](https://github.com/maynut02/AstralParty-KoPatch)~~ - 과거 한글패치 배포 저장소




<!-- Link Definitions -->
[discord-shield]: https://img.shields.io/badge/Discord-업데이트_알림_받기-ECF3FF?style=flat-square&logo=discord&logoColor=ffffff&labelColor=000000
[discord-link]: https://discord.gg/khYThH3gPD

[version-shield]: https://img.shields.io/github/v/release/maynut02/AstralAutoPatcher?style=flat-square&color=ECF3FF&labelColor=000000
[release-link]: https://github.com/maynut02/AstralAutoPatcher/releases/latest
[rust-shield]: https://img.shields.io/badge/rust-1.88.0+-ECF3FF?style=flat-square&logo=rust&logoColor=ffffff&labelColor=000000
[rust-link]: https://rust-lang.org/
[tauri-shield]: https://img.shields.io/badge/tauri-2.10.3-ECF3FF?style=flat-square&logo=tauri&logoColor=ffffff&labelColor=000000
[tauri-link]: https://tauri.app/
[ci-shield]: https://img.shields.io/github/actions/workflow/status/maynut02/AstralAutoPatcher/release-windows-exe.yml?style=flat-square&color=ECF3FF&labelColor=000000
[ci-link]: https://github.com/maynut02/AstralAutoPatcher/actions
[steam-shield]: https://img.shields.io/badge/Steam-Astral_Party-ECF3FF?style=flat-square&logo=steam&logoColor=ffffff&labelColor=000000
[steam-link]: https://store.steampowered.com/app/2622000/Astral_Party/

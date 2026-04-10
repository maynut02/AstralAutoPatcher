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
  현재 Steam 글로벌 버전과 Bilibili PC버전을 지원합니다.
</p>

<div align="center">
  
[![][discord-shield]][discord-link]

</div>

<div align="center">
  
[![][version-shield]][release-link]
[![][ci-shield]][ci-link]
[![][tauri-shield]][tauri-link]
[![][rust-shield]][rust-link]
[![][steam-shield]][steam-link]

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

> [!note]
> 최초 1번 프로그램을 실행해 설치 모드를 완료한 이후 자동 패치 및 패치 삭제 기능을 이용할 수 있습니다.

|버전|구분|실행|
|---|-----|-----|
|Steam 글로벌 버전|INT_STEAM|[자동 패치](https://astral.maynutlab.com/autopatch/patch/INT_STEAM) \| [패치 삭제](https://astral.maynutlab.com/autopatch/remove/INT_STEAM)|
|BiliBili PC 버전|CN_BILIBILI|[자동 패치](https://astral.maynutlab.com/autopatch/patch/CN_BILIBILI) \| [패치 삭제](https://astral.maynutlab.com/autopatch/remove/CN_BILIBILI)|

## 설치 정보

> [!WARNING]
> 프로그램 삭제 기능이 없으므로 삭제가 필요할 경우 해당 폴더 및 레지스트리를 수동으로 삭제하시면 됩니다.

```bash
# 설치 경로
C:\Program Files\AstralAutoPatcher

# 레지스트리 등록 경로
HKEY_CURRENT_USER\Software\Classes\astral
```

## 개발자용

```bash
# 설치
npm install

# 개발
npm run tauri dev

# 빌드
npm run tauri build
```

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
[steam-shield]: https://img.shields.io/badge/Astral_Party-ECF3FF?style=flat-square&logo=steam&logoColor=ffffff&labelColor=000000
[steam-link]: https://store.steampowered.com/app/2622000/Astral_Party/

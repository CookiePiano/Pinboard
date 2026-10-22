# Pinboard

**한국어** | [English](README.en.md)

**Windows 11 x64 · 첫 공개 버전: 0.1.0 · MIT**

[공식 다운로드](https://github.com/CookiePiano/Pinboard/releases) ·
[다운로드 검증](docs/download-verification.md) ·
[개인정보 안내](docs/privacy.md) · [코드 서명 정책](docs/code-signing-policy.md)

첫 공개 버전 0.1.0의 실행 파일에는 **Authenticode 서명이 없습니다**.
Windows에서 게시자를 알 수 없다는 경고가 표시될 수 있습니다.
공식 저장소, GitHub 증명(attestation), 체크섬을 확인하고 Windows 보안 기능을 끄지 마세요.
추후 서명된 릴리스는 새 버전으로 제공하며, 변경 불가능한 기존 릴리스를 교체하지 않습니다.
Linux 소스는 개발 목적으로 유지하지만, 이 릴리스의 지원 대상은 Windows 11 x64입니다.

Pinboard는 Tauri, Rust, Svelte, TypeScript, CodeMirror로 만든 Windows 및 Linux용
반투명 바탕화면 메모 앱입니다. Windows 11에서는 반투명 유리 효과를 제공하며,
공유 클라우드 폴더를 통해 여러 PC의 메모를 동기화할 수 있습니다.

이 페이지는 한국어를 기본으로 표시합니다. 상단의 **English** 링크에서 영어 설명을
읽을 수 있습니다. 연결된 상세 기술 문서는 별도 표시가 없으면 영어로 제공됩니다.
앱의 표시 언어는 설정 > 일반에서 별도로 변경할 수 있습니다.

에이전트 작업은 [AGENTS.md](AGENTS.md)부터 확인하세요. `docs/`의 날짜가 붙은 문서는
과거 구현·검증 기록이며, 현재 작업 트리나 설치된 실행 파일을 검증했다는 증거가 아닙니다.
[문서 색인](docs/README.md)에서 현재 동작 명세, 기능별 담당 코드, 진행 중인 점검과
과거 릴리스 검증 기록을 구분할 수 있습니다.

<a id="behavior-and-invariants"></a>

## 주요 기능과 동작 원칙

- 메모마다 여섯 가지 색상과 투명도를 선택할 수 있습니다. 투명도가 70% 이상이면
  Windows 흐림 효과를 끄고 글자에 은은한 외곽선을 적용합니다.
- 메모 도구 모음에서 새 메모, 달력, 설정, 삭제를 사용할 수 있습니다.
  삭제는 두 번 클릭해야 실행되며, 앱 자체의 메모 개수 제한은 없습니다.
- 메모에서 새 메모를 만들면 색상, 투명도, 크기, 배경 이미지와 이미지 투명도를 이어받습니다.
  트레이나 단축키로 만들면 남아 있는 가장 최근 메모를 기준으로 합니다.
  내용, 본문 서식, 고정 모드는 복사하지 않습니다.
- 저장에 실패하면 최신 편집 내용을 재시도할 수 있도록 유지합니다. 숨기기, 창 닫기,
  트레이의 모두 숨기기를 실행해도 저장하지 못한 편집 내용은 계속 표시합니다.
  편집 내용을 저장할 수 없으면 앱 종료와 ZIP 내보내기를 중단하며,
  저장을 방해하는 문제가 해소되면 다시 시도할 수 있습니다.
- 고정 모드는 일반, 항상 위, 바탕화면을 지원합니다. 메모를 클릭하면 포커스가 있는 동안
  일시적으로 위로 올라오고, 포커스를 잃으면 선택한 모드로 돌아갑니다.
- Windows에서 메모를 끌면 작업 영역 가장자리와 다른 메모에 맞춰 붙습니다.
  모서리에 붙여도 크기를 유지하며 최대화되지 않습니다. 다시 끌면 분리됩니다.
- HTTP, HTTPS, `www.` 주소에 밑줄을 표시하며, 클릭하면 기본 브라우저에서 엽니다.
  링크 안의 글자는 드래그로 선택하거나 방향키로 이동하여 편집할 수 있습니다.
- 편집기는 일부 Markdown 문법을 지원하고 첫 줄을 제목으로 표시합니다.
  할 일 체크박스는 클릭으로 전환되지 않으며, 기존 `- [ ]`, `- [x]`는 일반 텍스트로 보존합니다.
- 텍스트를 선택하면 **B / I / − / + / H**로 굵게, 기울임, 글자 크기, 형광펜을 설정할 수 있습니다.
  글자 크기는 기본 1pt 단위로 조절합니다. 일반 설정에서 조절 단위와 형광펜 색상을 바꿀 수 있고,
  여섯 가지 기본 색상과 사용자 지정 색상을 지원합니다.
  서식은 실행 취소·다시 실행, 저장, 동기화에 반영됩니다.
- 설정 > 디자인에서 JPG, PNG, WebP 이미지를 메모 배경으로 지정할 수 있습니다.
  이미지 비율을 유지하면서 중앙을 기준으로 채우며, 별도의 투명도를 설정할 수 있고 메모와 함께 동기화됩니다.
- 설정 > 일반에서 모든 창의 테마를 시스템, 밝게, 어둡게 중에서 선택할 수 있습니다.
  표시 언어는 한국어(`ko`), 영어(`en`), 중국어 간체(`zh`), 일본어(`ja`), 프랑스어(`fr`),
  스페인어(`es`)를 지원합니다. 변경 사항은 즉시 적용되고 저장됩니다.
  언어를 변경해도 사용자가 작성한 메모 내용은 번역하지 않습니다.
- 달력의 날짜별 메모 미리보기를 선택하면 해당 메모만 엽니다.
  달력 창의 고정 모드는 현재 PC에 저장합니다. 메모 카드의 체크박스를 선택하고
  날짜 옆 휴지통 버튼을 두 번 클릭하면 선택한 메모를 삭제합니다.
  이 삭제도 메모 창에서 삭제한 경우와 동일하게 동기화됩니다.
- 저장 위치는 이 PC와 외부 클라우드 중에서 선택합니다. 외부 클라우드는
  데스크톱용 Google Drive가 관리하는 폴더를 연결하며, 로그인은 해당 앱에서 처리합니다.
- 설정 > 동기화에서 메모, 이미지, 서식, 설정, 창 위치를 다운로드 폴더의
  `Pinboard_YYYY-MM-DD_HH-mm-ss-SSS.zip`으로 내보낼 수 있습니다.
  가져오기는 기존 상태를 `import-backups`에 먼저 백업한 뒤 복원합니다.
- AI 협업은 각 PC에서 MCP를 명시적으로 설정한 후 연결을 허용해야 사용할 수 있습니다.
  설정 전에는 연결 스위치가 비활성화됩니다. 앱 시작, 설정 열기, ZIP 가져오기는
  MCP를 자동으로 설정하지 않습니다.
- 설정 > AI와 협업에서 ChatGPT/Codex, Claude, Gemini를 선택하고 **자동으로 연결**을 누르면
  MCP 설정, 연결 허용, 선택한 클라이언트 설정의 `pinboard` 항목 등록을 수행합니다.
  다른 설정과 TOML 주석은 보존하며, 변경한 파일은 `*.pinboard.bak`으로 백업합니다.
  올바르지 않은 설정 파일은 수정하지 않고 클라이언트별로 문제를 알립니다.
  **수동으로 설정**에서는 복사하여 붙여넣는 방법을 제공합니다.
  클라이언트 카드에는 연결됨, 재연결 필요, 연결되지 않음 상태를 표시합니다.
  등록 후 각 클라이언트를 다시 시작하세요. 설정과 연결을 따로 제어하는 기능도 유지합니다.
- MCP로 만든 메모는 마지막으로 작성한 메모(내용이 비어 있지 않고 가장 최근에 편집한 메모)의
  창 크기, 글꼴, 색상, 투명도, 배경을 이어받습니다. AI 앱이 색상을 명시하면 그 색상을 우선합니다.
- 연결을 허용한 동안 AI는 사용자처럼 메모를 조작할 수 있습니다.
  `list_notes`, `read_note`, `create_note`(고정 모드 선택 가능),
  `update_note`(내용 교체·추가, 색상, 투명도, 고정, 표시·숨기기), `delete_note`를 제공합니다.
  자세한 내용은 [AI 협업](docs/ai-collaboration.md#tool-contract)을 확인하세요.
- Pinboard는 트레이에서 실행되며, 일반적으로 한 번에 하나의 인스턴스만 실행합니다.
  로그인 시 자동 실행은 설정 > 일반에서 사용자가 켜야 합니다. 기존 OS 자동 실행 선택은 보존합니다.

<a id="keyboard-shortcuts"></a>

## 키보드 단축키

| 키 | 동작 |
| --- | --- |
| 사용자가 지정한 조합 | 단축키를 활성화한 경우, 어디서든 새 메모 만들기 |
| `Esc` | 열린 패널 닫기 |

새 메모 전역 단축키는 기본적으로 꺼져 있습니다. 설정 > 일반 > 단축키에서 활성화하고
변경을 선택한 다음 Ctrl, Alt, Windows 키 중 하나 이상을 포함한 조합을 누르세요.
다른 앱이 이미 사용하는 조합이면 오류를 알리고 이전 단축키를 유지합니다.
변경 사항은 즉시 적용되고 저장됩니다. 편집기의 기본 복사, 붙여넣기, 실행 취소,
다시 실행 단축키도 사용할 수 있습니다.

<a id="development"></a>

## 개발

저장소 루트(로컬 배포 폴더에서는 `source/`)에서 명령을 실행하세요.
`package-lock.json`, `src-tauri/Cargo.lock`과 호환되는 버전을 사용하고 빌드 전에 설치된 도구를 확인하세요.
Windows에는 Rust MSVC 도구 모음과 WebView2(Windows 11에 포함)가 필요합니다.
`.ts` 파일을 가져오는 독립 실행형 `.mjs` 테스트에는 TypeScript 구문 제거를 지원하는 Node가 필요합니다.
이를 지원하지 않는 환경에서는 `scripts/test-linux-typescript.mjs`가 링크, 서식, 달력,
저장 큐, 단축키 입력 테스트용 Vite 기반 실행기를 제공합니다.

```powershell
npm ci
npm run tauri dev
npm run check
node scripts/test-links.mjs
node scripts/test-formatting.mjs
node scripts/test-calendar-dates.mjs
node scripts/test-i18n.mjs
node scripts/test-save-queue.mjs
node scripts/test-performance.mjs
node scripts/test-shortcut-capture.mjs
node scripts/test-window-drag.mjs
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
```

빌드 및 배포 명령:

```powershell
npm run tauri build
.\scripts\release.ps1 -OutputDirectory C:\build\Pinboard-0.1.0\artifacts -TargetDirectory C:\build\Pinboard-0.1.0\target -SigningPolicy UnsignedInitial
```

`npm run tauri build`는 `src-tauri/target/release/bundle/nsis`에 설치 파일을 만듭니다.
`scripts/release.ps1`에는 별도 빌드 디렉터리와 비어 있는 출력 디렉터리가 필요하며,
상위 폴더의 배포 파일을 교체하지 않습니다. 해시로 고정한 도구를 준비하고 테스트, 검사,
패키징을 마친 뒤 완성된 릴리스를 출력 디렉터리로 복사합니다.
`--remap-path-prefix`로 사용자 프로필, Cargo 홈, 소스 체크아웃 경로를 치환하고,
빌드한 실행 파일에 개발자 프로필 경로가 남으면 실패 처리합니다.
일반 `npm run tauri build`는 이 경로 치환을 적용하지 않습니다.

```text
Pinboard/
  Pinboard.exe                       포터블 앱
  Pinboard_<version>_x64-setup.exe    설치 프로그램
  Pinboard_Uninstall.exe              설치된 앱 제거 마법사
  THIRD-PARTY-NOTICES.txt             배포 시 포함해야 하는 고지
  source/                            소스 저장소
```

테스트할 때 `PINBOARD_DATA_DIR`을 지정하면 앱 데이터를 분리할 수 있습니다.
이 경우 단일 인스턴스 제한도 해제되어 한 PC에서 독립된 인스턴스를 여러 개 실행할 수 있습니다.
테스트 인스턴스마다 서로 다른 데이터 디렉터리를 사용하세요.
시작 시 설정을 열려면 `--settings`를 사용합니다. 언어 설정이 없는 데이터는 한국어를 기본으로 사용합니다.
수정하지 않은 환영 메모는 선택한 언어를 따르며, 편집한 메모는 보존합니다.

릴리스 스크립트는 설치되었거나 실행 중인 앱을 변경하지 않습니다.
설치 파일, 포터블 ZIP, SPDX 소스 의존성 목록, 매니페스트, 체크섬을 생성합니다.
GitHub 릴리스 워크플로는 검토된 태그를 빌드하고, 일회용 실행 환경에서 최종 설치 파일을 검증하며,
빌드 출처 증명을 발급하고 릴리스 초안을 만듭니다. [릴리스 절차](docs/releasing.md)를 확인하세요.

설치, 제거, 브랜딩, 검증은 [installer/README.md](installer/README.md)에 설명되어 있습니다.
`Pinboard_Uninstall.exe`는 현재 사용자의 설치된 앱만 대상으로 하며, 메모와 설정을 보존하고
포터블 실행 파일은 변경하지 않습니다. 필요한 경우 설치된 앱을 닫도록 안내합니다.

<a id="source-map"></a>

## 소스 구조

| 경로 | 역할 |
| --- | --- |
| `src-tauri/src/model.rs` | 저장되는 메모 구조와 하위 호환 동기화 해시 |
| `src-tauri/src/store.rs` | 로컬 메모 저장과 기기별 상태 |
| `src-tauri/src/sync/mod.rs` | 3방향 동기화, 충돌, 삭제 기록 정리, 엔진 테스트 |
| `src-tauri/src/sync/folder.rs` | 폴더 기반 저장소 |
| `src-tauri/src/sync/gdrive.rs`, `oauth.rs` | 과거 직접 Drive 연동; 현재 설정에서는 선택 불가 |
| `src-tauri/src/sync/worker.rs` | 로컬 변경 예약 처리(1.5초), 폴더 감시, 주기적 확인 |
| `src-tauri/src/platform/` | OS별 바탕화면 고정, 창 스타일, 둥근 모서리 |
| `src-tauri/src/windows.rs` | 메모·설정 창과 PC별 창 배치 |
| `src-tauri/src/transfer.rs` | ZIP 내보내기·가져오기, 검증, 롤백, 복구 |
| `src-tauri/src/editors.rs` | 백업·숨기기·종료 시 편집 내용을 저장하고 편집을 일시 중지하는 공통 절차 |
| `src/lib/window-drag.ts` | 메모·달력의 공통 포인터 제어와 드래그 정리 |
| `src-tauri/src/mcp/` | 로컬 stdio 프로토콜, 명시적 설정, 실행 중인 앱과의 연결 |
| `src/lib/note/` | 메모 UI, CodeMirror 연동, 링크, 본문 서식 |
| `src/lib/calendar/`, `src-tauri/src/calendar.rs` | 달력 UI와 네이티브 명령 |
| `src/lib/settings/` | 설정 UI |

<a id="data-and-synchronization"></a>

## 데이터와 동기화

Windows의 기본 데이터 디렉터리는 `%APPDATA%\dev.pinboard.notes`입니다.

| 파일 | 내용 |
| --- | --- |
| `notes/<id>.json` | 메모 내용; 삭제 기록은 30일 동안 유지 |
| `local-state.json` | 현재 PC의 창 위치, 크기, 고정 모드; 동기화하지 않음 |
| `settings.json` | 앱 환경 설정 |
| `sync-state.json` | 저장소별 마지막 동기화 기준 상태 |

동시에 편집하면 최신 버전을 원래 메모 ID로 유지하고 다른 버전을 충돌 사본으로 보존합니다.
편집과 삭제가 충돌하면 편집을 우선합니다. 과거 직접 Google Drive 연동 설정은 로컬 저장소로
전환하며, 메모 파일과 인증 정보는 보존합니다.
자세한 동작은 [클라우드 폴더 설정](docs/google-drive-setup.md),
[데이터 이동](docs/transfer-2026-10-21.md), [AI 협업](docs/ai-collaboration.md)을 확인하세요.

## Linux (Ubuntu 26.04)

빌드 스크립트, Debian 패키징, 바탕화면 동작과 검증 범위는
[docs/linux-ubuntu-26.04.md](docs/linux-ubuntu-26.04.md)에 설명되어 있습니다.

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev libdbus-1-dev pkg-config
```

GNOME Wayland는 앱이 창 위치와 표시 순서를 직접 제어하는 기능을 제한합니다.
창 위치 복원과 바탕화면 고정을 사용하려면 XWayland에서 `GDK_BACKEND=x11 pinboard`로 실행하세요.
Linux는 투명도를 지원하지만 Windows 흐림 효과는 제공하지 않습니다.
이 배포본의 클라우드 폴더 안내와 실제 제공자 확인 기록은 Windows용 데스크톱 Google Drive를 대상으로 합니다.

<a id="security"></a>

## 보안

공개 배포나 외부 연동 활성화 전에 [SECURITY.md](SECURITY.md)를 읽어 주세요.
첫 공개 버전은 0.1.0입니다. 이전 로컬 0.1.1 파일과
[보완 기록](docs/security-remediation-2026-10-22.md)은 내부 개발 이력이며,
이미 공개된 GitHub 릴리스를 뜻하지 않습니다. 해당 보안 수정은 현재 버전에도 포함되어 있습니다.
보안 정책은 평문 저장, 신뢰할 수 있는 동기화 폴더, MCP 권한, 서명, 취약점 신고 요건을 설명합니다.
[2026-10-21 보안 검토](docs/security-review-2026-10-21.md)는 당시 수정과 Windows 검증을 기록합니다.
현재 [GLib 백포트](src-tauri/vendor/README.md)는 GTK3 호환성을 유지하면서 Linux 반복자 문제를 해결하며,
전체 점검 기록에는 현재 검증의 한계가 명시되어 있습니다.
릴리스 스크립트는 빌드 전에 의존성 보안 점검과 회귀 검증을 수행합니다.

<a id="license"></a>

## 라이선스

Pinboard는 [MIT 라이선스](LICENSE)로 배포합니다.

<a id="third-party-notices"></a>

## 타사 소프트웨어 고지

저작권과 라이선스 고지는 앱 정보 화면에 나열하는 대신
[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)에 보존합니다.
설치 프로그램에 포함되며, 포터블 앱은 내장된 사본을 데이터 디렉터리에 기록합니다.
무료로 배포하는 경우에도 이 고지를 유지해야 합니다.
`npm run build`는 설치된 의존성과 잠금 파일을 바탕으로 고지를 다시 생성합니다.
원본 출처와 추가 고지는 [licenses/README.md](licenses/README.md)를 확인하세요.

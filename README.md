# Sponzey Cabinet

Sponzey Cabinet은 개인 사용자가 자신의 PC에 설치해 문서, 파일, 링크, Graph와 Canvas를 직접 소유하고 관리하는 로컬 우선 지식 관리 앱이다. 현재 제품 및 릴리스 검증 범위는 macOS 단일 사용자 데스크톱 앱이다. 멀티 사용자, 서버 호스팅, SaaS와 실시간 협업은 사용자의 명시적 요구 전까지 개발 범위에 포함하지 않는다.

## Current State

현재 구현 기준은 `personal_local_macos_desktop`이다. 데스크톱 UI는 Home, Document, Graph, Canvas, Assets, Backup route를 하나의 workspace shell 안에서 전환한다. 좌측 하단 문서 바로가기는 route별 검색 결과나 현재 화면에 따라 바뀌지 않고, root에서 계산한 최근 문서 목록을 모든 route가 공유한다. 사용자가 Home에서 문서를 열었다가 다른 메뉴로 이동한 뒤 Document 메뉴로 돌아오면 검색 화면으로 점프하지 않고 마지막 작업 문서를 우선 유지해야 한다.

활성 `.tasks/plan.md`는 현재 Obsidian 유사 WYSIWYG 기본 편집과 `원문 편집` modal을 목표로 하는 editor experience plan이다. 이전 phase archive는 완료 이력과 검증 근거로 참고하되, 새 릴리스 판정에서는 같은 source fingerprint에서 필요한 명령을 다시 실행해 확인한다.

## Core Document Experience

- React 기반 WYSIWYG/Live Preview 기본 편집 화면과 CodeMirror 기반 `원문 편집` modal
- 저장 canonical form은 Markdown source이며 WYSIWYG와 plain text editor는 같은 body state를 공유
- Markdown 첫 번째 물리적 줄에서 파생되는 문서 제목
- 문서 화면에서 파일 첨부, 문서별 첨부 목록, 미리보기/열기와 연결 해제
- 현재 문서 대 과거 version 및 과거 version 간 줄 단위 diff
- 복원 전 diff preview와 version conflict 방지
- 과거 snapshot으로 새 version을 생성하는 비파괴 복원
- Wikilink, Markdown link, backlink, 검색과 Graph projection
- 문서, 첨부와 자유 배치 노드를 연결하는 Canvas
- 문서 current/history, Asset과 Canvas를 포함하는 로컬 backup/restore
- route를 이동해도 좌측 사이드바의 문서 목록이 흔들리지 않는 공통 navigation shell

현재 편집기는 기본 화면에서 Markdown heading marker, table alignment row, wikilink/source marker와 raw HTML을 일반 사용자에게 직접 노출하지 않는 WYSIWYG/Live Preview 흐름을 제공한다. 제목/문단, 체크리스트, 표 셀은 기본 화면에서 직접 수정하고, 링크/첨부 참조, code block, blockquote, fallback block은 안전한 chip 또는 읽기 전용 block으로 표시한 뒤 `원문에서 편집` 진입점을 제공한다. `원문 편집` modal은 CodeMirror 기반 Markdown source editor이며, 수정 내용은 별도 적용 버튼 없이 같은 canonical body에 실시간 반영된다.

첨부 원본은 Markdown과 분리된 content-addressed asset store에 저장한다. 문서는 stable asset identity만 참조하며, 한 문서에서 연결을 해제해도 다른 문서나 Canvas의 참조가 남아 있으면 원본을 삭제하지 않는다.

복원은 기존 이력을 덮어쓰지 않는다. 사용자가 preview한 current version이 그대로일 때 대상 snapshot을 내용으로 하는 새 version을 만들고 current를 전환하므로, 복원 직전 상태도 계속 비교하거나 다시 복원할 수 있다. 내부 version ID, 파일 경로와 Git 용어는 일반 사용자 UI에 노출하지 않는다.

## Run

macOS Tauri 개발 앱을 실행한다.

```sh
scripts/run_desktop_app.sh
```

개발 실행 진입점은 데스크톱 앱 하나만 제공한다. 과거 phase gate, smoke, server, mobile, web 실행 wrapper는 현재 개발 중 앱 구동용 스크립트가 아니므로 유지하지 않는다.

현재 남아 있는 스크립트는 다음 역할만 가진다.

- `scripts/run_desktop_app.sh`: macOS Tauri 개발 앱 실행
- `scripts/build_desktop_assets.mjs`, `scripts/desktop_asset_builder.mjs`: 데스크톱 앱 실행 전 React/desktop asset 생성
- `scripts/run_web_app.mjs`: 데스크톱 UI 개발 보조용 web preview entry. 제품 릴리스 범위는 아니다.

표준 검증 명령은 wrapper 없이 직접 실행한다.

```sh
cargo test --workspace
node --experimental-strip-types --test apps/desktop/tests/*.ts
```

최근 archive 기준 검증에서는 `node scripts/build_desktop_assets.mjs`, `npm exec -- tauri build`, packaged UI initial/restart smoke, 51개 evidence contract test, 211개 route UI regression test와 63개 selected native runtime/boundary test가 통과했다. 전체 workspace exhaustive `cargo test`는 해당 archive의 최종 단계에서 실행하지 않았으므로 새 릴리스 직전에는 별도 실행한다.

## Project Documents

- [PROJECT.md](PROJECT.md): 최종 제품 목표와 기능 계약
- [AGENTS.md](AGENTS.md): 아키텍처, TDD, 설정, 로그와 상태머신 규칙
- [ROADMAP.md](ROADMAP.md): 단계별 범위, 산출물과 완료 조건
- [MVP_RELEASE.md](MVP_RELEASE.md): 현재 macOS 로컬 릴리스 범위와 검증 방법
- [RESEARCH.md](RESEARCH.md): 참고 제품과 OSS 조사 기록

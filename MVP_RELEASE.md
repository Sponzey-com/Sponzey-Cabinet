# Sponzey Cabinet Local Desktop Release

최종 갱신일: 2026-07-22

이 문서는 현재 개인용 로컬 데스크톱 제품의 실행 방법, 기능 범위와 데이터 경계를 정의한다. 과거 MVP scaffold가 아니라 현재 macOS Tauri 앱을 기준으로 하며, current fingerprint에서 재실행하지 않은 archive evidence는 현재 완료 증거로 간주하지 않는다.

## Release Scope

- Product scope: `personal_local_macos_desktop`
- Validated platform: macOS
- Deferred platforms: Windows, Linux
- Development preview only: Web
- Excluded until explicit user request: iOS, Android, self-host, SaaS, multi-user, realtime collaboration, organization/RBAC UI, SSO/SCIM, billing and admin console

Windows/Linux용 공통 domain, usecase, port와 adapter 경계는 유지하지만 현재 릴리스 지원으로 주장하지 않는다.

## User-Visible Capabilities

### Documents

- React와 CodeMirror 기반 Markdown source 작성, split view와 preview
- 문서 첫 번째 물리적 줄에서 제목 파생. heading marker를 제거하고 빈 첫 줄은 `제목 없는 문서`로 표시
- 별도 제목 입력 없이 본문 저장과 버전 복원 시 current title metadata 동기화
- 새 문서 생성, 편집, 저장 버튼과 `Cmd+S`
- Markdown preview와 table rendering
- Document 메뉴에서 마지막 작업 문서 재개
- 문서 목록과 카드에서 내부 ID와 문서 파일명 비노출
- 현재 문서 조회와 paginated 이력 조회의 분리
- 현재 대 특정 version 및 특정 version 대 특정 version의 줄 단위 비교
- 복원 preview, expected current version 충돌 방지와 새 version을 생성하는 durable restore
- Wikilink와 Markdown link parsing
- 검색, backlink, unresolved link와 orphan 탐색
- 저장 후 version/readback 및 앱 재시작 확인

### Workspace Shell

- Home, Document, Graph, Canvas, Assets, Backup route가 같은 shell을 공유
- 좌측 하단 문서 바로가기는 root-owned recent document shortcuts를 사용
- route 전환만으로 좌측 하단 문서 바로가기 목록이 검색 결과, 현재 Graph 중심, Canvas 선택 또는 Asset 선택으로 바뀌지 않음
- 상단 검색은 명시적 검색 action으로만 Search route를 열고, `Esc` 또는 문서 선택 뒤에는 이전 작업 문서 context로 돌아감

### Knowledge Graph

- durable link index와 graph projection 기반 node/edge 조회
- local/global graph 탐색과 node 선택
- loading, empty, ready, stale, repairing, failed 상태
- pan, zoom, reset과 선택 context 유지
- 문서 생성, 첫 줄 제목 변경을 포함한 저장, 복원, 삭제 뒤 projection 갱신 및 repair action

### Canvas

- Canvas 생성과 조회
- 문서, 메모, 첨부 target node 추가
- edge, geometry, viewport와 zoom 저장
- 이름 변경과 2단계 보관 확인
- stale revision 충돌 방지
- 손상 상태 감지와 최신 유효 revision 복구
- 앱 재시작 뒤 node, edge, 위치와 viewport 복원

### Attachments

- platform file picker를 통한 import
- staging, content-addressed object store, metadata와 document association 분리
- 문서에서 파일 첨부, 문서별 첨부 목록, 연결과 연결 해제
- 문서별 및 workspace asset metadata 조회
- 지원 범위 안의 bounded native preview
- 문서와 Canvas에서 동일 asset identity 참조
- 다른 참조가 남은 asset의 document association을 해제해도 원본과 다른 참조 보존
- import 중단, 손상, missing object와 unsupported preview 오류 표시

## Document Safety Contract

현재 macOS 개인용 로컬 제품의 문서 release acceptance는 다음 조건을 포함한다.

- 문서 첨부 실패는 완료되지 않은 document association과 고아 metadata를 남기지 않는다.
- 문서 diff는 Markdown 줄 변경과 첨부 association 변경 요약을 구분하고 첨부 bytes를 binary diff하지 않는다.
- 복원은 preview 당시 current version이 유지될 때만 실행한다. version conflict에서는 어떤 write도 수행하지 않으며, 같은 operation 재시도는 version을 중복 생성하지 않는다.
- 복원은 대상 snapshot을 내용으로 하는 새 version을 append하고 복원 대상과 복원 직전 version을 모두 보존한다.
- append 이후 current 전환이나 projection 요청이 실패하면 current를 보존하고 `RecoveryRequired` operation으로 재개하며 성공으로 표시하지 않는다.
- 복원 성공은 본문, 첫 줄 파생 제목, 링크/검색/Graph projection, 첨부 association과 durable readback이 새 version에 일치한 뒤 확정한다.
- 일반 UI와 로그는 내부 document/version/asset ID, snapshot path, Git 용어, 문서 본문, diff hunk, 원본 파일명과 절대 경로를 노출하지 않는다.
- 표준 크기 문서의 history, diff와 restore preview는 정상적인 로컬 store 상태에서 p95 300ms 목표를 따른다.

이 계약은 제품 목표와 이후 release gate의 필수 입력이다. 과거 archive는 위 세부 계약의 모든 항목을 현재 source fingerprint에서 자동으로 증명하는 것으로 간주하지 않는다.

### Data Ownership And Recovery

- 기본 app data directory 자동 결정과 최초 실행 초기화
- 외부 DB, 외부 검색 서버, Git CLI, Node.js runtime, 수동 환경 변수나 설정 파일 편집 없는 설치 제품 경계
- 문서 current/history, Graph/Canvas projection, Asset metadata/object를 포함하는 package backup
- 명시적 restore confirmation, apply, reopen과 durable readback
- projection rebuild와 Canvas recovery
- Product Log, scoped expiring Field Debug Log, production default에서 제외된 Development Log

## Local Data Boundary

bootstrap은 platform app data directory와 외부 환경 값을 시작 시 한 번만 읽고 검증된 immutable config object로 변환한다. 내부 유스케이스에는 생성자 인자, 명시적 context와 dependency injection으로 전달한다. 실행 중 환경 재조회, 전역 config singleton과 동적 설정 변경을 사용하지 않는다.

대표적인 로컬 데이터 범위:

```text
app_data_dir/
  metadata/
  workspaces/
  version-store/
  search-index/
  link-index/
  graph-projections/
  canvases/
  assets/
  operations/
  backups/
```

실제 경로와 파일명은 UI, Product Log와 릴리스 증거에 노출하지 않는다. 문서 body, 첨부 bytes와 절대 경로도 릴리스 artifact에 포함하지 않는다.

## Running The Product

개발 환경에서 실제 Tauri UI를 실행한다.

```sh
scripts/run_desktop_app.sh
```

이 명령은 desktop assets를 빌드하고 개발용 loopback UI server를 시작한 뒤 Tauri 앱을 실행한다. 현재 개발 scaffold에는 Rust, Node.js와 설치된 workspace dependency가 필요하지만, 이는 최종 설치 앱의 사용자 runtime 요구사항이 아니다.

독립 CLI shell과 Web preview는 사용자 실행 진입점으로 제공하지 않는다.

## Package And Verification Commands

현재 코드 테스트는 별도 wrapper 없이 표준 테스트 러너로 직접 실행한다.

```sh
cargo test --workspace
node --experimental-strip-types --test apps/desktop/tests/*.ts
```

패키징, smoke, 성능, 보안 및 종료된 phase별 gate는 실행 스크립트로 유지하지 않는다. 과거 검증 결과는 해당 `.tasks/phaseXXX/` archive에서 문서 증거로만 조회한다.

## Local Task Archive

현재 로컬 `.tasks`에는 활성 `.tasks/plan.md`가 없다. 확인된 archive는 `.tasks/phase001`, `.tasks/phase002`, `.tasks/phase003`, `.tasks/phase004`, `.tasks/phase_mvp`다. 이 archive는 과거 작업 기록이며 새 source fingerprint의 완료 증거가 아니다.

`.tasks/phase004`는 Penpot `20260721` UI fidelity 구현 결과를 보관한다. 해당 archive의 `task040`부터 `task044`는 다음 검증 기록을 포함한다.

- desktop asset build와 Tauri release package build 통과
- packaged UI initial smoke 통과: `phase015_packaged_ui_smoke_initial=passed`, `p95_ms=39`, `error_count=0`, `accessibility_internal_exposure_count=0`
- packaged UI restart smoke 통과: `phase015_packaged_ui_smoke_restart=passed`, `attachment_restart_readback_verified=true`, `canvas_text_restart_readback_verified=true`
- final/current/packaged evidence, source fingerprint, visual/accessibility contract tests 통과: 51 tests
- shared shell과 주요 route render/interaction regression 통과: 211 tests
- selected native runtime/boundary tests 통과: 63 tests

위 결과는 현재 제품 상태를 이해하는 근거로 사용한다. 단, 전체 workspace exhaustive `cargo test`는 해당 최종 검증에서 실행하지 않았으므로 새 release gate 직전에는 별도 실행한다.

현재 완료 증거로 인정하려면 같은 source fingerprint에서 다음을 다시 실행하고 결과를 남겨야 한다.

- `cargo test --workspace`
- `node --experimental-strip-types --test apps/desktop/tests/*.ts`
- 실제 `scripts/run_desktop_app.sh` 실행 뒤 Home, Document, Graph, Canvas, Assets, Backup route smoke
- 문서 저장, 제목 변경, 첨부 연결/해제, Graph/Canvas projection, backup/restore, 앱 재시작 readback 확인
- 검색/조회 p95 300ms 목표 측정
- Product/Field Debug/Development Log와 민감 정보 비노출 확인

## Current Hardening

- 공통 앱 셸, `ko-KR` 사용자 표현, 내부 ID 비노출과 visible action 연결을 통합했다.
- 문서 제목의 단일 원천을 Markdown 첫 줄로 고정하고 create command와 문서 UI의 별도 title 입력을 제거했다.
- 생성, 수정, 복원, durable readback, projection 처리와 재시작 후 Canvas 표시 제목을 Rust와 desktop 통합 테스트로 검증했다.
- macOS Tauri debug/no-sign 앱 번들 빌드를 완료했다.
- 문서 첨부, version diff와 비파괴 restore의 세부 안전 계약을 `PROJECT.md`, `AGENTS.md`와 `ROADMAP.md`에 동기화했다.
- 좌측 하단 문서 바로가기를 route별 데이터가 아니라 root-owned recent document shortcuts로 통일했다. Home, Search, Document, Graph, Canvas, Assets, Backup route는 같은 바로가기 목록을 공유해야 한다.
- Document 메뉴는 검색 route의 별칭이 아니라 마지막 작업 문서로 돌아가는 진입점이어야 한다.
- Penpot `20260721` 기준의 shell, route action, Graph, Canvas, Assets, Backup 화면은 archive 기준 route regression과 packaged smoke에서 검증되었다.

## Logging And Security

### Product Log

사용자 영향, 주요 상태 전이, 안정적인 error code, retryability와 처리 시간 구간만 기록한다. 문서 body, 첨부 bytes, secret, token, 절대 경로와 전체 객체 dump를 기록하지 않는다.

### Field Debug Log

workspace/component 범위와 만료 시간이 있는 명시적 session에서만 활성화한다. 식별자는 마스킹하고 원문 콘텐츠는 기록하지 않는다. 활성화와 종료는 Product Log event로 추적한다.

### Development Log

로컬 개발과 테스트에서만 사용한다. 프로덕션 기본 동작과 package 결과물의 필수 경로에 포함하지 않는다.

## Known Limitations

- 현재 package evidence는 macOS debug/no-sign 앱을 사용한다. 배포용 서명, notarization, 업데이트 채널과 installer UX는 별도 release hardening 범위다.
- Windows/Linux package와 native UI는 인증하지 않았다.
- Markdown table은 preview에서 grid로 렌더링되며 CodeMirror source 자체가 WYSIWYG table widget으로 변환되지는 않는다.
- 현재 편집기는 Obsidian Live Preview가 아니다. source/split/preview 방식이 현재 릴리스 범위이며, inline preview와 WYSIWYG형 block/widget 편집은 후속 editor milestone이다.
- 문서 제목은 별도 필드가 아니다. 제목을 바꾸려면 Markdown 본문의 첫 번째 줄을 수정하고 저장해야 한다.
- native attachment preview는 허용된 형식과 크기로 제한된다. OCR, 대용량 media transcoding과 모든 파일 형식 preview는 제공하지 않는다.
- Web preview는 데스크톱 UI 개발용이며 로컬 설치 앱의 저장소를 대체하지 않는다.
- 서버 호스팅, SaaS, 멀티 사용자, 실시간 공동 편집, 조직 관리, SSO/SCIM과 과금은 의도적으로 비활성 범위다.
- Windows/Linux/Web/iOS/Android 지원을 macOS 결과로 대신 주장하지 않는다.

## Release Rule

기능 완료는 버튼이나 route의 존재, task checkbox 또는 과거 phase marker만으로 판단하지 않는다. 같은 source fingerprint에서 실행된 command result, durable readback, restart/reopen, performance, visual, security와 packaged native evidence를 함께 통과해야 한다.

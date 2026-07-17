# Sponzey Cabinet Local Desktop Release

최종 갱신일: 2026-07-16

이 문서는 Phase 012에서 보관한 릴리스 증거와 Phase 013 UI 통합 및 후속 제목 hardening이 반영된 개인용 로컬 데스크톱 제품의 실행 방법, 기능 범위와 데이터 경계를 정의한다. 과거 MVP scaffold가 아니라 현재 macOS Tauri 앱을 기준으로 하며, current fingerprint에서 재실행하지 않은 archive evidence는 현재 완료 증거로 간주하지 않는다.

## Release Scope

- Product scope: `personal_local_macos_desktop`
- Validated platform: macOS
- Deferred platforms: Windows, Linux
- Development preview only: Web
- Excluded until explicit user request: iOS, Android, self-host, SaaS, multi-user, realtime collaboration, organization/RBAC UI, SSO/SCIM, billing and admin console

Windows/Linux용 공통 domain, usecase, port와 adapter 경계는 유지하지만 Phase 012 완료나 현재 릴리스 지원으로 주장하지 않는다.

## User-Visible Capabilities

### Documents

- React와 CodeMirror 기반 Markdown 작성
- 문서 첫 번째 물리적 줄에서 제목 파생. heading marker를 제거하고 빈 첫 줄은 `제목 없는 문서`로 표시
- 별도 제목 입력 없이 본문 저장과 버전 복원 시 current title metadata 동기화
- 새 문서 생성, 편집, 저장 버튼과 `Cmd+S`
- Markdown preview와 table rendering
- 현재 문서 조회와 paginated 이력 조회의 분리
- 현재 대 특정 version 및 특정 version 대 특정 version의 줄 단위 비교
- 복원 preview, expected current version 충돌 방지와 새 version을 생성하는 durable restore
- Wikilink와 Markdown link parsing
- 검색, backlink, unresolved link와 orphan 탐색
- 저장 후 version/readback 및 앱 재시작 확인

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

이 계약은 제품 목표와 이후 release gate의 필수 입력이다. 아래 Phase 012 evidence는 2026-07-15 이전 기준의 완료 기록이므로, 위 세부 계약의 모든 항목을 현재 source fingerprint에서 자동으로 증명하는 것으로 간주하지 않는다.

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

내부 desktop shell command boundary만 확인한다.

```sh
scripts/run_desktop_shell.sh
```

이 명령은 GUI launcher가 아니라 smoke command다. 화면을 열려면 `scripts/run_desktop_app.sh`를 사용한다.

개발용 Web preview를 실행한다.

```sh
scripts/run_web_app.sh
```

Web preview는 UI 개발과 browser verification을 위한 adapter이며 서버 호스팅 제품이나 authoritative persistence boundary가 아니다.

## Package And Verification Commands

macOS debug/no-sign `.app`을 빌드하고 package content와 native bootstrap을 확인한다.

```sh
scripts/run_desktop_packaged_app_smoke.sh
```

실제 packaged WebView에서 핵심 사용자 workflow와 durable readback을 확인한다.

```sh
scripts/run_desktop_packaged_ui_smoke.sh
```

Phase 012 전체 evidence를 새 source fingerprint에서 다시 생성한다.

```sh
scripts/run_phase012_release_evidence.sh
```

이 마지막 명령은 archive/plan/release contract, Rust workspace, desktop TypeScript, native/render 성능, visual, macOS package, packaged UI와 security 검증을 순서대로 수행한다. Phase 012 archive 완료 후 historical plan path가 `.tasks/phase012/plan.md`로 이동했으므로 새 phase에서 그대로 실행하기 전에 validator input path를 해당 archive에 맞춰야 한다.

## Phase 012 Evidence

Authoritative archive는 `.tasks/phase012/`다.

- Final result: `.tasks/phase012/phase012-release-gate-result.md`
- Archived plan: `.tasks/phase012/plan.md`
- Archived tasks: `.tasks/phase012/task001.md` through `.tasks/phase012/task126.md`
- Requirement matrix: `.tasks/phase012/release/requirement-evidence-matrix-phase012.md`
- Command summary: `.tasks/phase012/release/command-summary-phase012.md`
- Native platform matrix: `.tasks/phase012/release/native-platform-matrix-phase012.md`
- Packaged UI smoke: `.tasks/phase012/release/packaged-ui-smoke-phase012.md`
- Native and rendered query performance: `.tasks/phase012/release/query-performance-phase012.md`, `.tasks/phase012/release/query-render-performance-phase012.md`
- Visual evidence: `.tasks/phase012/release/exploration-visual-phase012.json`
- Security/log policy manifest: `.tasks/phase012/release/security-log-policy-manifest-phase012.json`

최종 gate는 다음을 기록한다.

- 33/33 requirements passed
- 19/19 release commands passed
- macOS passed
- Windows/Linux `deferred_future`
- packaged workflow 91 actions and 33 durable readbacks
- packaged UI 200 samples, p95 14ms and zero errors
- current/history/search/link/Graph/Canvas/Asset metadata native query p95 below 300ms

## Current Post-Phase 012 Hardening

- Phase 013에서 공통 앱 셸, `ko-KR` 사용자 표현, 내부 ID 비노출과 visible action 연결을 통합했다.
- 문서 제목의 단일 원천을 Markdown 첫 줄로 고정하고 create command와 문서 UI의 별도 title 입력을 제거했다.
- 생성, 수정, 복원, durable readback, projection 처리와 재시작 후 Canvas 표시 제목을 Rust와 desktop 통합 테스트로 검증했다.
- macOS Tauri debug/no-sign 앱 번들 빌드를 완료했다.
- 문서 첨부, version diff와 비파괴 restore의 세부 안전 계약을 `PROJECT.md`, `AGENTS.md`와 `ROADMAP.md`에 동기화했다. 이 문서 변경 이후의 current source fingerprint에서 계약 테스트와 packaged UI smoke를 다시 실행해야 최종 release evidence로 인정한다.
- 정적 dist browser smoke는 현재 Home 시작 화면에서도 CodeMirror가 즉시 존재한다고 가정하는 기존 조건 때문에 실패한다. 이 항목은 제목 저장 실패가 아니라 smoke 시작 route 계약의 불일치이며, current source fingerprint의 최종 release evidence를 확정하기 전에 수정 또는 기준 갱신이 필요하다.

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
- 문서 제목은 별도 필드가 아니다. 제목을 바꾸려면 Markdown 본문의 첫 번째 줄을 수정하고 저장해야 한다.
- native attachment preview는 허용된 형식과 크기로 제한된다. OCR, 대용량 media transcoding과 모든 파일 형식 preview는 제공하지 않는다.
- Web preview는 데스크톱 UI 개발용이며 로컬 설치 앱의 저장소를 대체하지 않는다.
- 서버 호스팅, SaaS, 멀티 사용자, 실시간 공동 편집, 조직 관리, SSO/SCIM과 과금은 의도적으로 비활성 범위다.
- Windows/Linux/Web/iOS/Android 지원을 macOS 결과로 대신 주장하지 않는다.

## Release Rule

기능 완료는 버튼이나 route의 존재, task checkbox 또는 과거 phase marker만으로 판단하지 않는다. 같은 source fingerprint에서 실행된 command result, durable readback, restart/reopen, performance, visual, security와 packaged native evidence를 함께 통과해야 한다.

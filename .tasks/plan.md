# MVP Development Plan

문서 목적: Sponzey Cabinet MVP를 실제 개발자가 바로 실행할 수 있는 단계별 개발 계획으로 정의한다.  
우선 기준: `AGENTS.md`를 최우선 개발 원칙으로 삼고, `PROJECT.md`와 `ROADMAP.md`의 제품 목표와 MVP 범위를 따른다.  
대상 MVP: 개인 로컬 Knowledge Base Core.  
대상 사용자 경험: 설치 1회 후 추가 수동 설정 없이 로컬 workspace를 생성하고, 문서를 작성, 검색, 연결, 이력 비교, 복원, 첨부 관리할 수 있다.

이 계획의 기술 언급은 현재 프로젝트의 구현 방향을 설명하기 위한 것이다. 아키텍처 규칙, 테스트 기준, 설정 정책, 로그 정책, 상태머신 정책은 특정 언어, 프레임워크, 플랫폼에 종속되지 않아야 한다. 특정 기술은 adapter, shell, editor, packaging 경계에만 머물러야 하며 domain과 usecase에 누출되면 안 된다.

## 1. Project Goal

Sponzey Cabinet의 최종 목표는 Outline, Notion, Obsidian, AFFiNE류 도구를 대체할 수 있는 문서 중심 Knowledge Base Solution을 만드는 것이다. 최종 제품은 개인 로컬 구축, 개인 호스팅, SaaS를 같은 데이터 모델과 유스케이스 위에서 지원해야 한다.

MVP의 목표는 최종 제품 전체를 얕게 흉내 내는 것이 아니다. MVP는 이후 개인 호스팅, 협업, SaaS, AI, 플러그인, 모바일 확장을 견딜 수 있는 로컬 단일 사용자 core를 먼저 완성한다.

MVP에서 반드시 제공할 사용자 기능은 다음이다.

- 로컬 workspace 생성
- Markdown/MDX 문서 생성, 현재 조회, 수정, 삭제
- 현재 문서 기준 조회와 이력 기준 조회의 명확한 분리
- 특정 version 조회
- 문서 diff 조회
- restore preview 조회
- 특정 version 복원
- Markdown link와 Wikilink 파싱
- 문서 간 링크, 백링크, 미해결 링크, 고아 문서 조회
- 로컬 첨부 파일 등록
- 문서에서 첨부 파일 reference 표시
- 첨부 파일 metadata 조회
- 로컬 전체 텍스트 검색
- Markdown folder import
- Markdown export
- HTML export 최소 기반
- PDF export 확장 경계
- Product Log 최소 이벤트
- Field Debug Log 기반 구조
- Development Log 분리
- 설정 bootstrap 1회 로딩
- 최초 실행 자동 초기화
- local migration 기반
- Web local UI
- Windows/macOS/Linux desktop shell의 최소 실행 형태

MVP에서 명시적으로 제외할 범위는 다음이다.

- 다중 사용자 협업
- 실시간 공동 편집
- iOS/Android 앱 구현
- OAuth/OIDC/SAML/SCIM
- SaaS 멀티테넌트
- 외부 SaaS connector
- AI 답변 생성
- 플러그인 런타임
- CRM 객체 구현
- Canvas/Edgeless UI
- 사용자가 DB, Git CLI, 검색 엔진, Node.js, 별도 서버를 직접 설치해야 하는 로컬 실행 방식
- 사용자에게 commit, branch, repository 같은 Git 개념을 노출하는 UI/API

MVP 완료 후 사용자는 다음 문장으로 제품 가치를 이해할 수 있어야 한다.

> Sponzey Cabinet은 설치 후 바로 사용할 수 있고, 문서 원본과 변경 이력을 자동으로 보존하며, 링크, 검색, 첨부, 복원을 갖춘 로컬 지식관리 앱이다.

### Platform Scope

최종 공식 대상 플랫폼은 Web, iOS, Android, Windows, macOS, Linux다. MVP는 이 전체 플랫폼을 한 번에 구현하지 않는다. MVP는 공통 domain/usecase/port 계약을 먼저 고정하고, Web local UI와 Windows/macOS/Linux desktop shell을 통해 로컬 단일 사용자 흐름을 검증한다.

MVP에서 플랫폼별 범위는 다음이다.

| Platform              | MVP Scope                                 | Required Boundary                                          | MVP Validation                              |
| --------------------- | ----------------------------------------- | ---------------------------------------------------------- | ------------------------------------------- |
| Web                   | local UI shell과 shared client contract 검증 | browser adapter는 domain/usecase를 직접 참조하지 않는다               | fake client 기반 UI test와 local flow smoke    |
| Windows/macOS/Linux   | desktop shell 최소 실행과 local workspace 접근   | platform shell은 DTO mapping과 usecase invocation만 수행한다      | packaged 또는 shell smoke, install once smoke |
| iOS/Android           | 구현 제외, contract 고려만 수행                    | domain/usecase에 mobile SDK 또는 mobile-specific rule을 넣지 않는다 | architecture review에서 platform leakage 없음   |
| Self-host/SaaS server | 구현 제외, 향후 확장을 위한 usecase contract 유지      | 로컬 MVP usecase가 서버 전용 request/response에 종속되지 않는다           | usecase input/output review                 |

MVP에서 iOS/Android 또는 SaaS 전용 기능을 구현해야 할 것처럼 보이면 scope creep으로 간주한다. 단, 해당 플랫폼 확장을 막는 domain/usecase 결정을 발견하면 Phase 0 decision record로 분리한다.

## 2. Current Plan Assessment

기존 계획의 강점은 다음이다.

- MVP 범위가 문서 CRUD, 이력, 링크, 검색, 첨부, import/export, UI, release gate까지 넓게 식별되어 있다.
- 현재 문서 조회와 이력 조회 분리 원칙이 포함되어 있다.
- 로컬 설치 1회 원칙과 Git CLI 비노출 원칙이 포함되어 있다.
- Domain, Usecase, Adapter 경계를 초기에 고정하려는 방향이 있다.
- 성능 목표 p95 300ms가 주요 조회 작업에 적용되어 있다.
- 문서와 첨부를 분리하고 asset store로 관리하는 방향이 명확하다.

기존 계획의 부족한 부분은 다음이다.

- 작업 목록이 세부 항목 중심이라 개발자가 어느 단계에서 무엇을 완료해야 하는지 판단하기 어렵다.
- adapter 구현이 UI 작업 뒤에 배치되어 있어 실제 end-to-end 검증 순서가 뒤바뀐다.
- Product Log와 Development Log는 언급되지만 Field Debug Log가 MVP 계획에 충분히 반영되지 않았다.
- 설정 정책은 존재하지만 bootstrap, composition root, config object, 테스트 방식, runtime 변경 금지가 하나의 실행 게이트로 묶여 있지 않다.
- 상태머신이 문서와 asset 일부에만 적용되어 있고 first-run, migration, restore, import/export, index rebuild 같은 실패/재시도 절차에는 충분히 반영되지 않았다.
- TDD 원칙은 언급되지만 각 단계가 어떤 실패 테스트로 시작해야 하는지 분명하지 않다.
- 리뷰 기준이 작업별 완료 조건과 연결되어 있지 않아 변경 단위에서 확인하기 어렵다.
- 성능 테스트가 후반에 몰려 있어 current/history/search 경로 설계를 초기에 강제하지 못한다.
- Web, desktop, mobile 최종 대상 플랫폼과 MVP 대상 플랫폼의 차이가 명확하지 않아 구현자가 iOS/Android를 MVP에 포함한다고 오해할 수 있다.

이 업데이트에서 해결한 방향은 다음이다.

- 기존 작업 항목을 단계별 phase로 재구성한다.
- 각 phase에 Goal, Scope, Required Changes, Architecture Notes, TDD Requirements, Configuration Rules, Logging Rules, State Management, Validation, Done Criteria, Risks를 둔다.
- `AGENTS.md`의 Layered Architecture, Clean Architecture, Tidy First, TDD, 설정 정책, 로그 정책, 상태머신 정책을 계획의 검증 조건으로 변환한다.
- 기존 작업 ID는 `Work Item Register`에 보존한다.
- 구현 순서는 skeleton, runtime foundation, domain, local adapters, usecases, query/projection, UI, release gate 순서로 정리한다.
- Field Debug Log, config bootstrap, runtime env 금지, 상태머신 전이 테스트, dependency boundary test를 명시한다.
- MVP와 최종 플랫폼 범위를 분리해 iOS/Android, SaaS, 협업 구현이 MVP에 섞이지 않게 한다.
- phase 진입/종료 게이트를 추가해 선후관계를 검증 가능한 방식으로 고정한다.
- 구현 전 결정해야 하는 기술 선택을 decision record로 분리한다.

## 3. Architecture Direction

MVP의 아키텍처는 Layered Architecture와 Clean Architecture를 동시에 만족해야 한다.

계층은 다음 방향으로 둔다.

```text
Client UI / Platform Shell / API Boundary
  -> Adapter / Presenter / Mapper
    -> Usecase
      -> Domain
      -> Port Interface

Infrastructure Implementation
  -> Port Interface
```

의존성 규칙은 다음이다.

- Domain은 어떤 외부 계층에도 의존하지 않는다.
- Domain은 framework, UI, filesystem, DB, network, environment, logger implementation, editor state, platform SDK를 import하지 않는다.
- Usecase는 Domain과 Port Interface에만 의존한다.
- Usecase는 명시적 input object를 받고 명시적 output object를 반환한다.
- Usecase는 UI 모델, HTTP request, Tauri command payload, DB row, 파일 경로 primitive를 직접 받지 않는다.
- Port Interface는 usecase가 필요로 하는 행위 기준으로 정의한다.
- Adapter는 외부 표현을 내부 DTO와 value object로 변환한다.
- Infrastructure는 Port Interface 구현체로만 연결한다.
- Composition Root는 bootstrap에서 config를 받고 dependency graph를 조립한다.
- 플랫폼별 차이는 capability object 또는 adapter로 표현한다.

MVP repository shape는 다음 방향으로 시작한다.

```text
crates-or-core/
  cabinet-domain/
  cabinet-usecases/
  cabinet-ports/
  cabinet-core/
  cabinet-adapters/
  cabinet-platform/

packages-or-client/
  ui/
  editor/
  client-core/

apps/
  web/
  desktop/

.tasks/
  plan.md
```

이 구조는 특정 빌드 도구나 언어 패키지명에 고정되지 않는다. 실제 저장소 구조는 프로젝트의 선택한 도구에 맞게 조정할 수 있지만, 다음 경계는 변경하지 않는다.

- domain package는 외부 I/O를 모른다.
- usecase package는 adapter 구현체를 모른다.
- adapter package는 domain rule을 직접 구현하지 않는다.
- editor package는 editor event를 document operation으로 변환할 뿐 document rule을 소유하지 않는다.
- platform shell은 usecase 호출 경계이며 비즈니스 규칙을 포함하지 않는다.
- local storage, internal version store, search index, asset store는 port 뒤에 숨긴다.

현재 문서 조회와 이력 조회는 다음처럼 분리한다.

```text
GetCurrentDocument
  input: workspace_id, document_id_or_path
  source: current snapshot repository
  forbidden: version history full scan

GetDocumentHistory
  input: workspace_id, document_id, cursor, limit
  source: version entry index
  required: pagination

GetDocumentVersion
  input: workspace_id, document_id, version_id
  source: version snapshot store
  forbidden: loading all versions
```

성능 목표는 다음 query path에 적용한다.

- 현재 문서 조회
- 이력 목록 조회
- 특정 version metadata 조회
- 특정 version snapshot 조회
- 문서 목록 조회
- 링크/백링크 조회
- 미해결 링크 조회
- 고아 문서 조회
- 첨부 metadata 조회
- 권한 필터링이 적용될 수 있는 검색
- 작업 상태 조회
- 캐시된 비동기 결과 조회

정상적인 인덱스 상태에서 사용자-facing 검색과 조회는 p95 300ms 이내를 목표로 설계하고 측정한다. AI 답변 생성, OCR, embedding, 대용량 export, 대용량 첨부 preview 생성은 비동기 작업으로 분리한다.

## 4. Development Principles

모든 MVP 작업은 다음 원칙을 통과해야 한다.

### Layered Architecture

- 계층 간 의존 방향을 바깥에서 안쪽으로 유지한다.
- 내부 계층은 외부 계층의 타입을 import하지 않는다.
- 플랫폼별 코드는 adapter 계층에 둔다.
- UI, command, controller, file picker, local path resolver는 domain/usecase를 우회하지 않는다.

### Clean Architecture

- 도메인 로직과 외부 시스템 접근을 분리한다.
- 유스케이스를 중심으로 기능을 정의한다.
- 외부 시스템은 port/interface 뒤로 숨긴다.
- mapper를 사용해 외부 표현과 내부 모델을 분리한다.
- 테스트 더블이 어렵다면 경계 설계를 먼저 고친다.

### Tidy First

- 기능 변경 전 필요한 이름 변경, 파일 이동, 중복 제거, fixture 정리, dependency boundary 정리는 별도 변경으로 수행한다.
- Tidy First 변경은 동작을 바꾸지 않는다.
- Tidy First 변경 후 기존 테스트를 모두 실행한다.
- 기능 변경과 리팩터링은 같은 커밋에 섞지 않는다.

### TDD

- 실패하는 테스트를 먼저 작성한다.
- 테스트를 통과하는 최소 구현을 작성한다.
- 중복과 구조 문제를 정리한다.
- 정리 후 모든 관련 테스트를 다시 실행한다.
- 설정, 로그, 상태 전이, 오류 처리, 성능 조건도 테스트 대상에 포함한다.

### Install Once Local Principle

- 개인 로컬 앱은 설치 1회 후 기본 workspace 생성까지 완료되어야 한다.
- 로컬 기본 실행은 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 환경 변수, 수동 설정 파일 편집을 요구하지 않는다.
- 필요한 local metadata store, internal version store, asset store, search index, app data directory는 최초 실행 시 자동 초기화한다.
- 고급 설정은 명시적 설정 화면 또는 명시적 import/export 절차로 제공하되 기본 실행의 필수 조건으로 만들지 않는다.

## 5. Implementation Phases

각 phase는 리뷰 가능한 단위로 완료되어야 한다. phase 안의 작업은 작은 변경으로 쪼개되, phase의 Done Criteria를 모두 만족하지 못하면 다음 phase로 넘어가지 않는다.

### Phase Gate Rules

phase 진행 규칙은 다음이다.

- Phase 0은 문서와 품질 게이트를 고정한다. 제품 기능 구현을 포함하지 않는다.
- Phase 1은 실행 가능한 skeleton만 만든다. domain rule, storage rule, search rule을 넣지 않는다.
- Phase 2는 runtime foundation을 고정한다. bootstrap, config, logging, first-run, migration을 통과하기 전 domain 기능을 저장소에 연결하지 않는다.
- Phase 3은 pure domain만 구현한다. adapter, filesystem, parser library, search engine, UI를 연결하지 않는다.
- Phase 4는 local adapter를 구현한다. usecase orchestration rule을 adapter에 넣지 않는다.
- Phase 5는 document usecase를 완성한다. UI 구현을 완료 조건으로 삼지 않는다.
- Phase 6은 parser, index, projection, import/export를 완성한다. 본문 전체 스캔을 조회 기본값으로 삼지 않는다.
- Phase 7은 UI와 platform integration을 완성한다. UI에서 domain rule을 재구현하지 않는다.
- Phase 8은 release gate를 완성한다. 성능 실패나 설치 실패를 문서상 예외로 처리하지 않는다.

각 phase의 진입 조건은 다음이다.

| Phase | Entry Gate                                   | Exit Gate                                                   |
| ----- | -------------------------------------------- | ----------------------------------------------------------- |
| 0     | `PROJECT.md`, `ROADMAP.md`, `AGENTS.md`를 읽는다 | MVP scope, out-of-scope, quality gate가 충돌 없이 정리된다           |
| 1     | Phase 0 exit gate 완료                         | 모든 skeleton package와 smoke test가 존재한다                       |
| 2     | Phase 1 exit gate 완료                         | bootstrap/config/logging/first-run/migration 테스트가 통과한다      |
| 3     | Phase 2 exit gate 완료                         | domain test가 외부 I/O 없이 통과한다                                 |
| 4     | Phase 3 exit gate 완료                         | storage/version/asset adapter contract test가 통과한다           |
| 5     | Phase 4 exit gate 완료                         | document usecase test와 current/history 분리 테스트가 통과한다         |
| 6     | Phase 5 exit gate 완료                         | parser/index/search/import/export contract test가 통과한다       |
| 7     | Phase 6 exit gate 완료                         | UI와 platform adapter smoke test가 통과한다                       |
| 8     | Phase 7 exit gate 완료                         | release smoke, data preservation smoke, p95 benchmark가 통과한다 |

요구사항 추적 기준은 다음이다.

| Requirement             | Primary Phase | Secondary Phase | Required Evidence                                 |
| ----------------------- | ------------- | --------------- | ------------------------------------------------- |
| 설치 1회 로컬 실행             | 2             | 8               | first-run test, clean install smoke               |
| 현재 문서 조회와 이력 조회 분리      | 5             | 8               | no history scan test, current/history UI test     |
| p95 300ms 검색/조회         | 5             | 6, 8            | instrumentation point, benchmark result           |
| 외부 설정 최초 1회 수신          | 2             | 8               | env read once test, bootstrap scan                |
| 3단계 로그 정책               | 2             | 5, 6, 8         | logger port tests, sensitive data exclusion tests |
| 상태머신 기반 내부 절차           | 2             | 3, 5, 6, 8      | transition tests, invalid transition tests        |
| domain과 외부 시스템 분리       | 1             | 3, 8            | dependency boundary check, domain pure tests      |
| Git CLI 비의존과 Git 개념 비노출 | 4             | 5, 8            | Git CLI absence smoke, user-facing model review   |
| 첨부 파일과 문서 본문 분리         | 3             | 4, 7            | asset reference tests, asset store contract tests |
| UI의 domain rule 비복제     | 7             | 8               | fake client UI tests, platform adapter review     |

### Phase 0. MVP 기준 고정과 품질 게이트 정의

- Goal: MVP의 제품 범위, 아키텍처 경계, 테스트 게이트, 성능 게이트, 설정/로그/상태머신 기준을 개발 시작 전에 고정한다.
- Scope: 문서, 결정 기록, 테스트 명명 규칙, 작업 추적 기준, 금지 패턴, release gate 정의.
- Required Changes:
  - MVP scope와 out-of-scope를 이 문서와 project documentation에 일치시킨다.
  - AGENTS 기준과 충돌하는 계획 표현을 제거한다.
  - p95 300ms 측정 대상 query path를 목록화한다.
  - clean machine install smoke test 기준을 정의한다.
  - dependency boundary check 기준을 정의한다.
  - TDD 작업 템플릿을 만든다.
  - 기술 선택이 필요한 항목은 decision record로 분리한다.
- Architecture Notes:
  - 이 phase에서는 제품 기능을 구현하지 않는다.
  - 구조 변경이 필요하면 Tidy First 변경으로 분리한다.
  - 결정 기록은 domain/usecase/adapter 경계를 기준으로 작성한다.
- TDD Requirements:
  - 이 phase에서 테스트 코드를 만들 수 없는 항목은 추후 phase의 첫 테스트 이름과 검증 대상을 명시한다.
  - `current document query must not scan version history` 같은 architecture expectation을 테스트 이름으로 예약한다.
  - `bootstrap reads external environment once` 같은 runtime expectation을 테스트 이름으로 예약한다.
- Configuration Rules:
  - 설정 기본값은 외부 파일 없이 동작해야 한다.
  - 새 설정이 필요하면 source, validation rule, owner component, delivery path를 먼저 정의한다.
  - runtime 중간 설정 삽입 또는 변경이 필요한 설계는 이 phase에서 거부한다.
- Logging Rules:
  - 로그 event name 체계를 정의한다.
  - Product Log, Field Debug Log, Development Log의 사용 위치를 phase별로 연결한다.
  - 로그에 문서 본문, 첨부 내용, secret, token을 넣는 계획을 금지한다.
- State Management:
  - 상태가 3개 이상이거나 실패/재시도/종료가 있는 절차를 상태머신 대상으로 지정한다.
  - MVP 상태머신 대상은 first-run initialization, migration, document lifecycle, asset lifecycle, restore, import, export, index rebuild다.
- Validation:
  - 이 문서가 AGENTS의 필수 원칙을 모두 참조하는지 확인한다.
  - 금지 표현이 남아 있지 않은지 확인한다.
  - MVP out-of-scope가 명확한지 확인한다.
- Done Criteria:
  - MVP 작업자가 이 문서만 보고 첫 구현 순서를 판단할 수 있다.
  - 리뷰어가 각 phase의 완료 여부를 판단할 수 있다.
  - AGENTS 기준과 충돌하는 계획이 없다.
- Risks:
  - MVP 범위가 커져 시작이 늦어질 수 있다.
  - 완화: phase별 Done Criteria를 넘지 않는 작은 변경으로 구현한다.

### Phase 1. Executable Skeleton과 Dependency Boundary 구축

- Goal: domain, usecase, port, adapter, UI, platform shell의 빈 골격을 만들고 dependency direction을 자동 또는 수동으로 검증할 수 있게 한다.
- Scope: workspace scaffold, package/crate/module 경계, composition root skeleton, Web shell, desktop shell, test runner, dependency boundary check.
- Required Changes:
  - domain package를 만든다.
  - usecase package를 만든다.
  - port interface 위치를 정한다.
  - adapter package를 만든다.
  - platform shell package를 만든다.
  - shared UI package를 만든다.
  - editor package를 만든다.
  - client-core package를 만든다.
  - Web local UI shell을 만든다.
  - desktop shell을 만든다.
  - composition root entrypoint를 만든다.
  - empty usecase를 호출하는 smoke path를 만든다.
  - dependency graph check script 또는 review checklist를 추가한다.
- Architecture Notes:
  - domain은 어떤 외부 package에도 의존하지 않는다.
  - usecase는 domain과 port interface에만 의존한다.
  - adapter는 usecase boundary를 호출한다.
  - platform command는 DTO mapping과 usecase 호출만 수행한다.
  - UI는 client-core interface를 통해 usecase 결과를 받는다.
  - desktop shell의 로컬 호출과 Web shell의 API 호출은 같은 client contract를 사용한다.
- TDD Requirements:
  - 빈 domain test가 외부 시스템 없이 통과해야 한다.
  - usecase smoke test는 fake port를 주입해 실행해야 한다.
  - platform command smoke test는 domain rule 없이 DTO mapping만 검증해야 한다.
  - UI render smoke test는 fake client를 사용해야 한다.
- Configuration Rules:
  - skeleton 단계에서 `.env` 또는 수동 설정 파일을 필수로 만들지 않는다.
  - 기본 config object를 코드에서 직접 생성해 테스트한다.
  - test runner는 환경 변수 순서에 의존하지 않는다.
- Logging Rules:
  - Development Log만 skeleton smoke test에서 허용한다.
  - Product Log는 아직 제품 이벤트가 없으므로 placeholder port만 둔다.
  - Field Debug Log는 기본 비활성 port와 config field만 둔다.
- State Management:
  - skeleton에는 상태머신 구현을 넣지 않는다.
  - 상태머신 대상 목록만 interface와 test plan으로 남긴다.
- Validation:
  - 모든 package의 test command가 실행된다.
  - domain package에서 filesystem, network, env, UI, platform SDK import가 없는지 확인한다.
  - UI와 platform shell이 domain package를 직접 참조하지 않고 client/usecase boundary를 통해 접근하는지 확인한다.
- Done Criteria:
  - 빈 앱 shell이 실행된다.
  - 모든 test runner가 최소 1개 이상의 smoke test를 실행한다.
  - dependency boundary가 문서와 코드 구조에서 일치한다.
- Risks:
  - 초기 skeleton이 실제 기능 없이 과도하게 복잡해질 수 있다.
  - 완화: package는 public boundary와 test runner만 만들고 내부 추상화는 기능 phase에서 추가한다.

### Phase 2. Runtime Foundation, Configuration, Logging, 상태머신 기반 구축

- Goal: 외부 환경 값 수신, 최초 실행 초기화, migration, 로그 분리, 상태머신 실행 모델을 제품 기능보다 먼저 안정화한다.
- Scope: bootstrap config, runtime environment handling, first-run initializer, local migration runner, logging ports, field debug activation policy, state machine base pattern.
- Required Changes:
  - `AppConfig` value object를 정의한다.
  - `LocalPathsConfig`를 정의한다.
  - `LoggingConfig`를 정의한다.
  - `StorageConfig`를 정의한다.
  - `SearchConfig`를 정의한다.
  - `BootstrapContext`를 정의한다.
  - 외부 환경 값은 bootstrap 또는 composition root에서 1회만 읽는다.
  - 검증된 config object를 dependency graph에 명시적으로 전달한다.
  - `PlatformPathResolver` port를 정의한다.
  - `FirstRunInitializer` usecase 또는 application service를 만든다.
  - local metadata store directory를 자동 생성한다.
  - local internal version store directory를 자동 생성한다.
  - local asset store directory를 자동 생성한다.
  - local search index directory를 자동 생성한다.
  - `LocalMigrationRunner` port와 no-op initial migration을 만든다.
  - `ProductLogger`, `FieldDebugLogger`, `DevelopmentLogger` port를 분리한다.
  - Field Debug Log 활성화 scope, TTL, masking rule을 config에 포함하되 기본값은 비활성으로 둔다.
  - 상태머신 transition result 표준 형태를 정의한다.
- Architecture Notes:
  - config object는 domain model이 아니다.
  - usecase는 environment, process args, external config file을 직접 읽지 않는다.
  - first-run과 migration은 외부 I/O를 port로 요청한다.
  - 로그 구현체는 port 뒤에 숨긴다.
  - 상태머신 transition function은 pure function으로 유지하고 side effect request를 반환한다.
- TDD Requirements:
  - 외부 환경 값이 bootstrap에서 1회만 읽히는 테스트를 작성한다.
  - bootstrap 바깥에서 환경 조회가 없음을 dependency or static scan으로 검증한다.
  - 기본 config 생성 테스트를 작성한다.
  - invalid path config 실패 테스트를 작성한다.
  - first-run clean directory 초기화 테스트를 작성한다.
  - first-run idempotency 테스트를 작성한다.
  - partial initialization recovery 테스트를 작성한다.
  - migration version 기록 테스트를 작성한다.
  - migration 실패 후 retry 테스트를 작성한다.
  - Product Log payload에 문서 본문과 첨부 내용이 들어가지 않는 테스트를 작성한다.
  - Field Debug Log가 기본 비활성인지 테스트한다.
  - Development Log가 production default에 포함되지 않는 테스트 또는 build check를 작성한다.
- Configuration Rules:
  - runtime 중간에 config를 삽입하거나 수정하는 API를 만들지 않는다.
  - config는 immutable object로 취급한다.
  - 테스트는 환경 변수를 변경하지 않고 config object를 직접 만든다.
  - 고급 설정 변경이 필요하면 MVP에서는 앱 재시작 또는 명시적 재구성 절차로만 처리한다.
- Logging Rules:
  - Product Log: first-run completed/failed, migration completed/failed, usecase failed, local setup unhealthy만 기록한다.
  - Field Debug Log: first-run step, migration state, path decision, cache/index diagnostic을 scope와 TTL이 있을 때만 기록한다.
  - Development Log: local test setup, fake port call, parser intermediate result, benchmark detail만 local/test에서 기록한다.
  - 모든 로그 event는 stable event name과 error code를 가져야 한다.
- State Management:
  - First-run states: `NotStarted`, `ResolvingPaths`, `CreatingStores`, `WritingMetadata`, `Completed`, `Failed`, `Retrying`.
  - First-run events: `Start`, `PathsResolved`, `StoreCreated`, `MetadataWritten`, `Fail`, `Retry`, `Complete`.
  - Migration states: `NotStarted`, `Locked`, `Running`, `Completed`, `Failed`, `Retrying`.
  - Migration events: `AcquireLock`, `RunMigration`, `MigrationSucceeded`, `MigrationFailed`, `Retry`, `ReleaseLock`.
  - 모든 실패 상태는 retry 가능 여부와 user-facing error code를 반환한다.
- Validation:
  - local 기본 실행이 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일을 요구하지 않는지 확인한다.
  - bootstrap 이후 환경 값을 재조회하지 않는지 확인한다.
  - 로그 3단계가 코드상 타입 또는 interface로 분리되어 있는지 확인한다.
  - first-run과 migration 전이가 상태머신 테스트로 검증되는지 확인한다.
- Done Criteria:
  - clean temp profile에서 first-run 초기화가 성공한다.
  - 이미 초기화된 profile에서 first-run 재실행이 idempotent하다.
  - migration runner가 version을 기록한다.
  - logging port 3종이 분리되어 있다.
  - Development Log가 production default path에 포함되지 않는다.
- Risks:
  - 설정을 편하게 쓰기 위해 global singleton을 만들 위험이 있다.
  - 완화: config가 필요한 모든 생성자는 config 또는 context를 명시적으로 받는다.
  - 로그를 디버깅 편의로 과도하게 남길 위험이 있다.
  - 완화: 로그 테스트에서 문서 원문, 첨부 내용, secret 포함 여부를 검증한다.

### Phase 3. Domain Core와 명시적 상태 모델 구현

- Goal: workspace, document, version, link, asset의 핵심 모델을 외부 시스템과 분리된 pure domain으로 만든다.
- Scope: entity, value object, domain error, lifecycle state machine, link model, version model, asset model.
- Required Changes:
  - `Workspace`, `WorkspaceId`, `WorkspaceName`, `WorkspacePath`를 정의한다.
  - `Document`, `DocumentId`, `DocumentPath`, `DocumentTitle`, `DocumentSlug`, `DocumentMetadata`, `DocumentStatus`를 정의한다.
  - `DocumentBody`를 정의하고 본문 크기 제한, line ending normalization, encoding 정책을 둔다.
  - `DocumentLifecycleState`와 `DocumentLifecycleEvent`를 정의한다.
  - `VersionEntry`, `VersionId`, `VersionAuthor`, `VersionSummary`, `DocumentSnapshotRef`를 정의한다.
  - current snapshot과 history version entry의 차이를 domain에 표현한다.
  - restore preview model을 정의한다.
  - `Asset`, `AssetId`, `AssetMetadata`, `AssetReference`를 정의한다.
  - content-addressed asset identity 정책을 정의한다.
  - `AssetLifecycleState`와 `AssetLifecycleEvent`를 정의한다.
  - `DocumentLink`, `LinkTarget`, `Backlink`, resolved/unresolved state를 정의한다.
  - link source range와 block reference 최소 모델을 정의한다.
- Architecture Notes:
  - domain model은 storage schema나 editor state에 맞춰 오염되지 않는다.
  - document body는 editor buffer가 아니다.
  - asset 원본은 document body에 포함되지 않는다.
  - version model은 Git 개념을 사용자 모델로 노출하지 않는다.
  - link는 단순 문자열이 아니라 source, target, status, range를 가진 value object다.
- TDD Requirements:
  - value object validation test를 먼저 작성한다.
  - lifecycle transition test를 먼저 작성한다.
  - invalid transition test를 먼저 작성한다.
  - current snapshot과 history entry 분리 테스트를 먼저 작성한다.
  - asset reference가 document body 원본 저장을 요구하지 않는 테스트를 작성한다.
  - Unicode 본문 보존 테스트를 작성한다.
  - slug normalization 테스트를 작성한다.
- Configuration Rules:
  - domain은 config object를 직접 읽지 않는다.
  - domain policy가 설정값을 필요로 하면 primitive가 아니라 명시적 policy value object를 인자로 받는다.
  - domain test는 외부 파일이나 환경 값 없이 실행한다.
- Logging Rules:
  - domain은 logger를 호출하지 않는다.
  - domain transition result는 usecase가 로그로 변환할 수 있는 stable event 또는 error code를 제공할 수 있다.
  - Product Log 생성은 usecase phase에서 처리한다.
- State Management:
  - Document lifecycle states: `Draft`, `Saved`, `Editing`, `Archived`, `Deleted`, `Restored`.
  - Document lifecycle events: `Create`, `Save`, `StartEdit`, `Archive`, `Delete`, `Restore`.
  - Asset lifecycle states: `Registered`, `Linked`, `Unlinked`, `Archived`, `Restored`, `Missing`.
  - Asset lifecycle events: `Register`, `Link`, `Unlink`, `Archive`, `Restore`, `MarkMissing`.
  - transition function은 같은 입력에 대해 같은 결과를 반환한다.
  - invalid transition은 명확한 domain error code를 반환한다.
- Validation:
  - domain package가 framework, filesystem, DB, network, env, logger, editor, platform SDK를 import하지 않는지 확인한다.
  - 모든 entity/value object가 단위 테스트를 가진다.
  - 상태머신은 성공 전이와 실패 전이를 모두 테스트한다.
- Done Criteria:
  - domain test suite가 외부 시스템 없이 통과한다.
  - document identity와 path가 분리되어 rename이 가능하다.
  - current snapshot과 history entry가 domain에서 구분된다.
  - asset 원본과 document body가 분리된다.
  - link/backlink projection을 만들 수 있는 domain model이 존재한다.
- Risks:
  - storage schema 편의를 위해 domain model이 외부 표현에 종속될 수 있다.
  - 완화: adapter mapper가 외부 schema와 domain model을 변환한다.

### Phase 4. Local Storage, Internal Version Store, Asset Store Adapter 구현

- Goal: 로컬 문서 원본, metadata, internal version history, asset 원본을 port 뒤에 숨긴 adapter로 구현한다.
- Scope: document repository, metadata store, internal version store, asset store, platform path resolver, atomic write, corruption detection.
- Required Changes:
  - `DocumentRepository` port를 정의한다.
  - current snapshot direct read/write 계약을 정의한다.
  - metadata 저장 layout을 정의한다.
  - atomic write 정책을 구현한다.
  - partial write recovery 정책을 구현한다.
  - `VersionStore` port를 정의한다.
  - internal version store adapter를 구현한다.
  - Git CLI 의존을 금지한다.
  - version entry 생성 adapter를 구현한다.
  - version snapshot 조회 adapter를 구현한다.
  - history pagination adapter를 구현한다.
  - diff source adapter를 구현한다.
  - restore source adapter를 구현한다.
  - `AssetStore` port를 정의한다.
  - local asset disk adapter를 구현한다.
  - content-addressed object 저장을 구현한다.
  - asset metadata store와 object file을 분리한다.
  - `LocalSetupHealthChecker`를 구현한다.
- Architecture Notes:
  - adapter는 domain rule을 재구현하지 않는다.
  - internal version store는 사용자에게 history, diff, restore로만 보인다.
  - current document repository는 version history 전체 스캔 없이 latest snapshot을 읽을 수 있어야 한다.
  - storage path 결정은 platform path resolver가 담당한다.
  - usecase는 raw filesystem path 대신 value object나 repository port를 사용한다.
- TDD Requirements:
  - repository contract test를 먼저 작성한다.
  - current snapshot direct read test를 먼저 작성한다.
  - version store가 Git CLI 없이 동작하는 테스트를 작성한다.
  - history pagination test를 작성한다.
  - 특정 version snapshot 조회 테스트를 작성한다.
  - atomic write failure test를 작성한다.
  - metadata corruption detection test를 작성한다.
  - asset duplicate registration test를 작성한다.
  - asset original file missing test를 작성한다.
  - local setup health checker test를 작성한다.
- Configuration Rules:
  - storage root는 bootstrap에서 검증된 `LocalPathsConfig`로 전달한다.
  - adapter 내부에서 환경 변수나 사용자 home path를 직접 재조회하지 않는다.
  - test는 임시 directory를 명시적 config로 전달한다.
- Logging Rules:
  - Product Log: storage corruption, version store failure, unrecoverable setup failure만 기록한다.
  - Field Debug Log: path decision, atomic write retry, version store operation metadata, cache hit/miss를 scoped diagnostic에서만 기록한다.
  - Development Log: adapter test fixture 생성과 fake failure injection을 local/test에서만 기록한다.
  - 로그는 파일명 원문을 기본 기록하지 않고 필요한 경우 hash 또는 마스킹된 값만 기록한다.
- State Management:
  - Atomic write states: `Prepared`, `WritingTemp`, `Syncing`, `Replacing`, `Completed`, `Failed`, `Recovering`.
  - Version write states: `PreparingSnapshot`, `WritingSnapshot`, `WritingEntry`, `UpdatingHead`, `Completed`, `Failed`.
  - Asset registration states: `Hashing`, `WritingObject`, `WritingMetadata`, `Linked`, `Failed`.
  - 상태 전이 실패 시 문서 current snapshot과 version entry의 일관성을 보존한다.
- Validation:
  - 외부 API, DB, filesystem 접근이 adapter/infrastructure 계층에만 있는지 확인한다.
  - usecase와 domain에서 filesystem import가 없는지 확인한다.
  - current lookup이 version history scan을 호출하지 않는지 contract test로 확인한다.
  - asset 원본이 document body 또는 metadata DB에 직접 저장되지 않는지 확인한다.
- Done Criteria:
  - document create/read/update/delete adapter contract가 통과한다.
  - current snapshot direct read가 가능하다.
  - Git CLI 없이 version entry, snapshot, history pagination이 동작한다.
  - asset registration과 metadata 조회가 동작한다.
  - local setup health checker가 정상, 누락, 손상, 복구 가능 상태를 구분한다.
- Risks:
  - internal version store 선택이 current/history 분리를 어렵게 만들 수 있다.
  - 완화: 선택 전 decision record에서 current snapshot direct read, pagination, corruption recovery를 검증한다.
  - 파일시스템 failure가 테스트되지 않을 수 있다.
  - 완화: failure injection adapter와 temp directory contract test를 사용한다.

### Phase 5. Document Usecase와 Current/History Query 분리 구현

- Goal: 사용자가 문서를 만들고, 수정하고, 현재 조회하고, 이력을 조회하고, diff/preview/restore를 수행하는 핵심 usecase를 완성한다.
- Scope: workspace creation, document CRUD, current query, version query, history query, diff, restore preview, restore, rename, delete.
- Required Changes:
  - `CreateWorkspaceInput/Output`을 정의한다.
  - `CreateDocumentInput/Output`을 정의한다.
  - `GetCurrentDocumentInput/Output`을 정의한다.
  - `GetDocumentVersionInput/Output`을 정의한다.
  - `GetDocumentHistoryInput/Output`을 정의한다.
  - `CompareDocumentVersionsInput/Output`을 정의한다.
  - `PreviewDocumentRestoreInput/Output`을 정의한다.
  - `RestoreDocumentVersionInput/Output`을 정의한다.
  - `UpdateDocumentInput/Output`을 정의한다.
  - `RenameDocumentInput/Output`을 정의한다.
  - `DeleteDocumentInput/Output`을 정의한다.
  - stale base snapshot detection을 구현한다.
  - history pagination default limit과 max limit을 정의한다.
  - not found, conflict, invalid state, storage failure error code를 정의한다.
  - document write 후 version entry를 생성한다.
  - document write 후 search/link indexing event를 발행한다.
- Architecture Notes:
  - usecase는 외부 request type을 받지 않는다.
  - usecase는 concrete repository를 직접 생성하지 않는다.
  - `GetCurrentDocument`는 current snapshot repository만 사용한다.
  - `GetDocumentHistory`는 version entry list query만 사용한다.
  - `GetDocumentVersion`은 version snapshot store를 사용한다.
  - diff와 restore는 current snapshot과 target version을 명시적으로 입력받는다.
  - restore 실패 시 current snapshot은 보존되어야 한다.
- TDD Requirements:
  - 각 usecase는 실패하는 usecase test로 시작한다.
  - fake repository, fake version store, fake clock, fake id generator, fake logger를 주입한다.
  - current 조회가 version history scan 없이 동작하는 fake call assertion을 작성한다.
  - history 조회가 pagination 없이 전체 history를 로드하지 않는 테스트를 작성한다.
  - restore 중 persist 실패 시 current 보존 테스트를 작성한다.
  - Product Log에 문서 본문이 포함되지 않는 테스트를 작성한다.
  - stale base snapshot reject 테스트를 작성한다.
- Configuration Rules:
  - usecase는 timeout, path, log mode, feature flag를 전역 조회하지 않는다.
  - 필요한 policy 값은 생성자 인자 또는 context object로 받는다.
  - 테스트는 policy config를 직접 생성해 주입한다.
- Logging Rules:
  - Product Log: workspace created, document created, document updated, document renamed, document deleted, document restored, usecase failed를 기록한다.
  - Field Debug Log: restore state, conflict reason summary, version lookup metadata, fake disabled unless scoped diagnostic is enabled.
  - Development Log: fake port call order, diff intermediate summary, usecase test fixtures만 local/test에서 기록한다.
  - Product Log payload에는 document body, full title, file content, attachment content를 넣지 않는다.
- State Management:
  - Restore states: `Requested`, `LoadingCurrent`, `LoadingTarget`, `Previewed`, `WritingNewVersion`, `UpdatingCurrent`, `ReindexRequested`, `Completed`, `Failed`.
  - Restore events: `Request`, `CurrentLoaded`, `TargetLoaded`, `PreviewCreated`, `VersionWritten`, `CurrentUpdated`, `ReindexQueued`, `Fail`, `Complete`.
  - Delete는 hard delete가 아니라 recoverable state transition으로 시작한다.
  - Rename은 document identity를 바꾸지 않는 transition으로 처리한다.
- Validation:
  - 모든 usecase가 explicit input/output을 가진다.
  - 외부 API, DB, filesystem, network, env 접근이 usecase에 없는지 확인한다.
  - 테스트 더블로 모든 외부 의존성을 대체할 수 있는지 확인한다.
  - 현재 조회와 이력 조회가 별도 code path인지 확인한다.
  - 현재 조회는 p95 300ms 측정을 위한 instrumentation point를 가진다.
- Done Criteria:
  - 문서 생성 후 current 조회, history 조회, search/link indexing event 발행이 가능하다.
  - 문서 수정 후 version entry가 생성된다.
  - 특정 version 조회가 가능하다.
  - diff 조회가 가능하다.
  - restore preview가 가능하다.
  - 특정 version 복원이 가능하다.
  - 삭제는 복구 가능한 상태 전이로 처리된다.
- Risks:
  - usecase가 indexing, storage, logging side effect를 직접 강하게 결합할 수 있다.
  - 완화: side effect는 port와 event request로 분리하고 contract test로 호출 순서를 검증한다.

### Phase 6. Parser, Link Projection, Search Index, Import/Export 구현

- Goal: 문서 내용을 구조화하고, 링크/백링크/검색/가져오기/내보내기를 인덱스와 projection 기반으로 제공한다.
- Scope: Markdown parser, Wikilink parser, link index, graph-lite projection, local search index, search usecase, Markdown import, Markdown/HTML export, PDF extension boundary.
- Required Changes:
  - `MarkdownParser` port를 정의한다.
  - Markdown link 추출을 구현한다.
  - Wikilink 추출을 구현한다.
  - heading 추출을 구현한다.
  - asset reference 문법을 정의한다.
  - parser output을 domain value object로 변환하는 mapper를 만든다.
  - `LinkIndex` 또는 `GraphIndex` port를 정의한다.
  - document changed event에서 link index 갱신을 수행한다.
  - target path/id resolution 정책을 구현한다.
  - backlinks query를 구현한다.
  - unresolved links query를 구현한다.
  - orphan documents query를 구현한다.
  - graph-lite depth 1 projection을 구현한다.
  - `SearchIndex` port를 정의한다.
  - local embedded search index adapter를 구현한다.
  - search index freshness metadata를 기록한다.
  - index rebuild command/usecase를 구현한다.
  - `SearchDocumentsInput/Output`을 정의한다.
  - query normalization과 pagination을 구현한다.
  - result snippet 최소 형태를 제공한다.
  - Markdown folder import usecase를 구현한다.
  - Markdown export usecase를 구현한다.
  - HTML rendering adapter를 구현한다.
  - PDF export는 unsupported 또는 queued extension result로 명시한다.
- Architecture Notes:
  - parser implementation은 adapter다.
  - parser output은 domain value object로 변환한 뒤 usecase에 전달한다.
  - link/backlink/search 조회는 본문 전체 재파싱 또는 전체 파일 스캔을 기본값으로 삼지 않는다.
  - search implementation은 usecase에 노출되지 않는다.
  - import/export는 상태머신 기반 job으로 설계한다.
- TDD Requirements:
  - Markdown link parsing test를 먼저 작성한다.
  - Wikilink parsing test를 먼저 작성한다.
  - asset reference parsing test를 먼저 작성한다.
  - broken link와 unresolved target test를 작성한다.
  - backlinks query test를 작성한다.
  - orphan documents query test를 작성한다.
  - search index upsert/delete/query contract test를 작성한다.
  - index corruption rebuild test를 작성한다.
  - search p95 300ms fixture test를 작성한다.
  - import duplicate path test를 작성한다.
  - import partial failure test를 작성한다.
  - export metadata policy test를 작성한다.
- Configuration Rules:
  - search index location은 bootstrap config로 전달한다.
  - parser와 search 설정은 runtime 중간에 변경하지 않는다.
  - import/export target path는 user action input으로 명시적으로 받는다.
  - import/export는 숨겨진 전역 path를 조회하지 않는다.
- Logging Rules:
  - Product Log: import completed/failed, export completed/failed, index rebuild completed/failed, search failed를 기록한다.
  - Field Debug Log: query hash, candidate count, filtered count, index freshness, parser warning count를 scoped diagnostic에서만 기록한다.
  - Development Log: parser intermediate AST summary, index fixture stats, benchmark raw samples를 local/test에서만 기록한다.
  - 검색 query 원문은 Product Log에 기록하지 않는다.
- State Management:
  - Index rebuild states: `Requested`, `ScanningMetadata`, `BuildingIndex`, `SwappingIndex`, `Completed`, `Failed`.
  - Import states: `Requested`, `ValidatingSource`, `ScanningFiles`, `MappingPaths`, `WritingDocuments`, `WritingAssets`, `Reindexing`, `Completed`, `Failed`, `Cancelled`.
  - Export states: `Requested`, `ResolvingDocuments`, `Rendering`, `WritingOutput`, `Completed`, `Failed`, `Cancelled`.
  - 각 job은 progress, retryability, user-facing error code를 가진다.
- Validation:
  - link/backlink/search 조회가 원본 전체 스캔 없이 projection/index를 사용하는지 확인한다.
  - search와 link projection 조회가 p95 300ms 목표를 측정할 수 있는 benchmark를 가진다.
  - import/export 실패 시 원본 workspace가 손상되지 않는지 확인한다.
  - Development Log가 production default에 포함되지 않는지 확인한다.
- Done Criteria:
  - Markdown link와 Wikilink가 파싱된다.
  - 백링크, 미해결 링크, 고아 문서 조회가 가능하다.
  - graph-lite depth 1 조회가 가능하다.
  - 로컬 검색이 외부 검색 서버 없이 동작한다.
  - search result에서 current document metadata로 연결된다.
  - Markdown folder import가 동작한다.
  - Markdown export가 동작한다.
  - HTML export가 동작한다.
  - PDF export는 명시적 unsupported 또는 queued extension boundary를 가진다.
- Risks:
  - parser가 domain rule을 직접 판단할 수 있다.
  - 완화: parser는 syntax extraction만 수행하고 link resolution은 usecase/domain policy가 수행한다.
  - 검색 성능을 UI loading으로 숨길 위험이 있다.
  - 완화: benchmark와 p95 gate를 release gate에 연결한다.

### Phase 7. Shared UI, Editor, Platform Integration 구현

- Goal: Web local UI와 desktop shell에서 같은 사용자 흐름으로 문서 작성, 검색, 링크, 이력, 복원, 첨부를 사용할 수 있게 한다.
- Scope: shared layout, document tree, CodeMirror editor, current/history split UI, diff/restore UI, search UI, backlink UI, asset UI, platform file picker adapter, desktop smoke.
- Required Changes:
  - shared app layout을 만든다.
  - document tree 또는 document list를 만든다.
  - editor panel을 만든다.
  - metadata side panel을 만든다.
  - status bar를 만든다.
  - command palette placeholder를 만든다.
  - editor load/save adapter를 연결한다.
  - dirty state 표시를 구현한다.
  - save command를 구현한다.
  - Wikilink decoration을 구현한다.
  - unresolved link 표시를 구현한다.
  - asset reference decoration을 구현한다.
  - missing asset 표시를 구현한다.
  - current document view를 만든다.
  - history panel을 만든다.
  - version list pagination을 구현한다.
  - version detail view를 만든다.
  - diff view를 만든다.
  - restore preview view를 만든다.
  - restore confirm action을 만든다.
  - search input과 result list를 만든다.
  - backlinks panel을 만든다.
  - unresolved links panel을 만든다.
  - orphan documents list를 만든다.
  - attach file action을 만든다.
  - platform file picker adapter를 연결한다.
  - asset metadata list를 만든다.
  - Web shell과 desktop shell이 같은 client contract를 사용하도록 한다.
- Architecture Notes:
  - UI는 domain rule을 구현하지 않는다.
  - editor state는 document model이 아니다.
  - CodeMirror extension은 editor event를 document operation으로 변환한다.
  - platform file picker 결과는 value object로 변환한 뒤 usecase에 전달한다.
  - desktop command는 DTO mapping과 usecase 호출만 수행한다.
  - Web shell의 API client와 desktop shell의 local client는 같은 client-core interface를 구현한다.
- TDD Requirements:
  - UI component render test를 작성한다.
  - editor mount test를 작성한다.
  - dirty state test를 작성한다.
  - save command가 client usecase를 호출하는 test를 작성한다.
  - Wikilink decoration test를 작성한다.
  - asset reference decoration test를 작성한다.
  - current/history UI가 서로 다른 client query를 호출하는 test를 작성한다.
  - search result click이 current document open을 호출하는 test를 작성한다.
  - file picker result mapping test를 작성한다.
  - desktop command adapter test를 작성한다.
- Configuration Rules:
  - UI는 환경 변수를 직접 읽지 않는다.
  - UI는 bootstrap에서 전달된 client config 또는 capability object만 사용한다.
  - Field Debug activation UI가 생기더라도 MVP 기본값은 비활성이다.
  - local end-user desktop package는 Node.js runtime을 요구하지 않는다.
- Logging Rules:
  - Product Log: 사용자 영향이 있는 command failure와 restore/delete 완료만 usecase를 통해 기록한다.
  - Field Debug Log: UI action trace가 아니라 state/query diagnostic summary만 scoped diagnostic에서 기록한다.
  - Development Log: component test debug와 editor test trace만 local/test에서 기록한다.
  - UI에서 문서 본문을 로그로 남기지 않는다.
- State Management:
  - UI state는 loading, empty, error, success 같은 표시 상태로 제한한다.
  - document lifecycle, restore, import/export, index rebuild 상태 결정은 usecase/state machine 결과를 표시한다.
  - dirty state는 editor-local state로 관리하되 save/restore conflict rule은 usecase에 둔다.
- Validation:
  - current document view와 history view가 UI에서 명확히 분리되어 있는지 확인한다.
  - UI가 version history 전체를 조회해 current view를 만드는지 확인하고 금지한다.
  - desktop shell이 비즈니스 규칙을 포함하지 않는지 리뷰한다.
  - shared UI가 Web과 desktop에서 같은 client contract를 사용하는지 확인한다.
- Done Criteria:
  - 사용자가 문서를 생성하고 편집하고 저장할 수 있다.
  - 사용자가 current view와 history view를 구분해 사용할 수 있다.
  - 사용자가 diff와 restore preview를 볼 수 있다.
  - 사용자가 특정 version을 복원할 수 있다.
  - 사용자가 검색 결과에서 문서를 열 수 있다.
  - 사용자가 백링크와 미해결 링크를 볼 수 있다.
  - 사용자가 첨부 파일을 연결하고 metadata를 볼 수 있다.
  - Web local UI와 desktop shell 중 최소 하나에서 MVP end-to-end flow가 자동 검증된다.
- Risks:
  - UI에서 빠른 구현을 위해 domain rule을 복제할 수 있다.
  - 완화: UI 테스트는 client contract 호출을 검증하고 domain rule 결과는 fake response로 표시한다.
  - editor state와 document state가 혼동될 수 있다.
  - 완화: editor package public API는 document body string과 operation event만 받도록 제한한다.

### Phase 8. Performance, Reliability, Release Gate 완성

- Goal: MVP를 릴리즈 가능한 기준까지 검증하고, 성능, 설치, 데이터 보존, 복구, 문서화를 완료한다.
- Scope: performance fixtures, p95 benchmarks, clean install smoke, data preservation smoke, corruption recovery smoke, end-to-end flow, documentation.
- Required Changes:
  - small workspace fixture를 만든다.
  - medium workspace fixture를 만든다.
  - large local MVP fixture를 만든다.
  - fixture 문서 수, 평균 본문 크기, 링크 수, 첨부 수를 정의한다.
  - 측정 환경 기록 형식을 정의한다.
  - `GetCurrentDocument` benchmark를 만든다.
  - `GetDocumentHistory` benchmark를 만든다.
  - `GetDocumentVersion` benchmark를 만든다.
  - search benchmark를 만든다.
  - backlinks benchmark를 만든다.
  - unresolved links benchmark를 만든다.
  - orphan documents benchmark를 만든다.
  - asset metadata benchmark를 만든다.
  - clean machine install smoke test를 만든다.
  - local data preservation smoke test를 만든다.
  - MVP end-to-end flow smoke test를 만든다.
  - local setup health checker smoke를 만든다.
  - known limitations 문서를 작성한다.
  - local data location 문서를 작성한다.
  - backup/export 기본 정책 문서를 작성한다.
  - developer test gate 실행 방법을 문서화한다.
- Architecture Notes:
  - performance fixture는 production code path를 우회하지 않는다.
  - benchmark는 index/projection 정상 상태와 degraded 상태를 구분한다.
  - release gate는 구현 기술보다 사용자-facing 동작과 architecture boundary를 검증한다.
- TDD Requirements:
  - benchmark fixture 생성 재현성 테스트를 작성한다.
  - clean install smoke 실패 테스트를 먼저 정의한다.
  - data preservation smoke에서 v1 store fixture를 만들고 migration 후 데이터 보존을 검증한다.
  - E2E는 단위 테스트를 대체하지 않는다.
  - 성능 기준 실패는 테스트 실패 또는 release gate 실패로 처리한다.
- Configuration Rules:
  - clean install smoke는 수동 env, 수동 config file, 외부 DB, 외부 검색 서버, Git CLI, Node.js runtime이 없는 조건을 검증한다.
  - benchmark config는 명시적 config object로 전달한다.
  - 테스트 중 runtime config를 변경하지 않는다.
- Logging Rules:
  - Product Log smoke는 문서 본문과 첨부 내용이 빠져 있는지 검증한다.
  - Field Debug Log smoke는 scope와 TTL 없이는 활성화되지 않는지 검증한다.
  - Development Log smoke는 production default artifact에 포함되지 않는지 검증한다.
- State Management:
  - release smoke는 first-run, migration, restore, import/export, index rebuild 실패 상태를 최소 1개 이상 검증한다.
  - 상태머신 실패는 user-facing error code와 retryability를 반환해야 한다.
- Validation:
  - domain 계층이 외부 framework에 의존하지 않는지 확인한다.
  - usecase가 명시적 input/output을 가지는지 확인한다.
  - 외부 환경 값이 프로그램 시작 이후 암묵적으로 재조회되지 않는지 확인한다.
  - 설정 값이 프로세스 중간에 삽입되거나 변경되지 않는지 확인한다.
  - 외부 API, DB, filesystem, network 접근이 boundary 계층에만 존재하는지 확인한다.
  - 테스트 더블로 외부 의존성을 대체할 수 있는지 확인한다.
  - 로그가 Product Log, Field Debug Log, Development Log 정책에 맞게 분리되어 있는지 확인한다.
  - Development Log가 production 기본 동작에 포함되지 않는지 확인한다.
  - 복잡한 내부 흐름이 flag 조합이 아니라 명시적 상태 전이로 표현되는지 확인한다.
  - 리팩터링과 기능 변경이 분리되어 있는지 확인한다.
- Done Criteria:
  - clean machine install smoke test가 통과한다.
  - local data preservation smoke test가 통과한다.
  - MVP end-to-end flow가 통과한다.
  - current document lookup p95 300ms 목표가 측정되고 기준을 만족한다.
  - history list lookup p95 300ms 목표가 측정되고 기준을 만족한다.
  - specific version lookup p95 300ms 목표가 측정되고 기준을 만족한다.
  - search p95 300ms 목표가 측정되고 기준을 만족한다.
  - link/backlink lookup p95 300ms 목표가 측정되고 기준을 만족한다.
  - asset metadata lookup p95 300ms 목표가 측정되고 기준을 만족한다.
  - MVP 사용과 개발 문서가 최신 상태다.
- Risks:
  - p95 300ms 목표가 하드웨어와 fixture 크기에 따라 흔들릴 수 있다.
  - 완화: fixture 크기와 측정 환경을 기록하고, release gate는 기준 환경에서 실행한다.
  - smoke test가 실제 사용자 설치 경로를 충분히 반영하지 못할 수 있다.
  - 완화: clean temporary user profile과 packaged desktop artifact를 사용한다.

## 6. Work Item Register

이 목록은 기존 세부 작업을 보존한 실행 단위다. 실제 구현은 `Implementation Phases` 순서로 진행한다.

| ID      | Phase | Work Item                           | Primary Output                            | Required Validation                         |
| ------- | ----- | ----------------------------------- | ----------------------------------------- | ------------------------------------------- |
| MVP-000 | 0     | 개발 기준 고정                            | MVP scope, out-of-scope, quality gate     | AGENTS 기준 충돌 없음                             |
| MVP-001 | 1     | Core workspace scaffold             | domain/usecase/ports/adapters skeleton    | dependency boundary check                   |
| MVP-002 | 1     | Frontend workspace scaffold         | shared UI/editor/client packages          | UI smoke with fake client                   |
| MVP-003 | 1     | Desktop shell scaffold              | platform shell command boundary           | command adapter contains no domain rule     |
| MVP-004 | 2     | Bootstrap config object             | immutable config and bootstrap context    | env read once test                          |
| MVP-005 | 2     | First-run initializer               | local store auto initialization           | clean temp profile test                     |
| MVP-006 | 2     | Local migration runner              | migration version and lock                | failure/retry state test                    |
| MVP-007 | 2     | Logging foundation                  | Product/Field Debug/Development log ports | sensitive data exclusion test               |
| MVP-010 | 3     | Workspace domain model              | workspace entity/value objects            | pure domain tests                           |
| MVP-011 | 3     | Document identity and metadata      | document entity/value objects             | path/title/slug validation                  |
| MVP-012 | 3     | Document body model                 | normalized document body                  | line ending/size/unicode tests              |
| MVP-013 | 3     | Document lifecycle state machine    | state/event/transition function           | valid/invalid transition tests              |
| MVP-014 | 3     | Asset domain model                  | asset entity/reference/metadata           | content separation tests                    |
| MVP-015 | 3     | Asset lifecycle state machine       | asset state/event/transition function     | valid/invalid transition tests              |
| MVP-016 | 3     | Version domain model                | version entry/snapshot/preview model      | current/history separation tests            |
| MVP-017 | 3     | Link domain model                   | link/backlink/target models               | resolved/unresolved tests                   |
| MVP-020 | 5     | CreateWorkspace                     | workspace creation usecase                | fake initializer and Product Log test       |
| MVP-021 | 5     | CreateDocument                      | document creation usecase                 | version/search/link event tests             |
| MVP-022 | 5     | GetCurrentDocument                  | current snapshot query usecase            | no version history scan test                |
| MVP-023 | 5     | GetDocumentVersion                  | version snapshot query usecase            | no full history load test                   |
| MVP-024 | 5     | GetDocumentHistory                  | paginated history usecase                 | limit/cursor/p95 test                       |
| MVP-025 | 5     | CompareDocumentVersions             | line diff usecase                         | current-vs-version/version-vs-version tests |
| MVP-026 | 5     | PreviewDocumentRestore              | restore preview usecase                   | diff and availability tests                 |
| MVP-027 | 5     | RestoreDocumentVersion              | restore state machine/usecase             | failure preserves current test              |
| MVP-028 | 5     | UpdateDocument                      | document update usecase                   | stale snapshot/version entry tests          |
| MVP-029 | 5     | RenameDocument                      | title/path rename usecase                 | identity stable/backlink tests              |
| MVP-030 | 5     | DeleteDocument                      | recoverable delete usecase                | search exclusion/history policy tests       |
| MVP-040 | 6     | Markdown parser adapter             | parser port and adapter                   | markdown/wikilink/asset reference tests     |
| MVP-041 | 6     | Link index                          | backlink/unresolved/orphan queries        | projection p95 test                         |
| MVP-042 | 6     | Graph-lite projection               | depth 1 graph query                       | deleted/unresolved node tests               |
| MVP-050 | 4     | Local asset store                   | content-addressed asset adapter           | duplicate/missing/large file tests          |
| MVP-051 | 5     | AttachFileToDocument                | attach usecase                            | failure does not pollute document           |
| MVP-052 | 5     | ListDocumentAssets                  | asset metadata query usecase              | metadata p95 test                           |
| MVP-060 | 6     | Search index port                   | search index contract                     | upsert/delete/query tests                   |
| MVP-061 | 6     | Local search index adapter          | embedded/local index                      | no external search server test              |
| MVP-062 | 6     | SearchDocuments                     | search usecase                            | pagination/p95 tests                        |
| MVP-070 | 6     | Markdown folder import              | import state machine/usecase              | duplicate/partial failure tests             |
| MVP-071 | 6     | Markdown export                     | export usecase                            | metadata/asset reference tests              |
| MVP-072 | 6     | HTML/PDF export foundation          | HTML adapter and PDF boundary             | HTML render/PDF unsupported tests           |
| MVP-080 | 7     | Shared shell UI                     | layout and navigation shell               | render/responsive tests                     |
| MVP-081 | 7     | Code editor integration             | editor load/save/dirty state              | editor mount/save tests                     |
| MVP-082 | 7     | Wikilink editor extension           | wikilink decoration and event             | decoration/click tests                      |
| MVP-083 | 7     | Attachment editor extension         | asset reference decoration                | missing asset tests                         |
| MVP-084 | 7     | Current/history UI split            | current view and history panel            | separate query tests                        |
| MVP-085 | 7     | Search UI                           | search input/result list                  | result open tests                           |
| MVP-086 | 7     | Link/backlink UI                    | backlinks/unresolved/orphan panels        | navigation tests                            |
| MVP-087 | 7     | Asset UI                            | attach/list/missing asset UI              | file picker mapping tests                   |
| MVP-090 | 4     | Local file document repository      | current snapshot repository               | atomic write/current direct read tests      |
| MVP-091 | 4     | Internal version store adapter      | embedded version store                    | Git CLI absence/history tests               |
| MVP-092 | 4     | Platform path resolver              | platform data path adapter                | OS fixture/path validation tests            |
| MVP-093 | 4     | Local setup health checker          | setup diagnostics                         | healthy/missing/corrupt tests               |
| MVP-100 | 8     | Performance fixture design          | small/medium/large fixtures               | reproducibility tests                       |
| MVP-101 | 8     | Current document lookup performance | current query benchmark                   | p95 300ms target                            |
| MVP-102 | 8     | Version history lookup performance  | history/version benchmarks                | p95 300ms target                            |
| MVP-103 | 8     | Search performance                  | search benchmarks                         | p95 300ms target                            |
| MVP-104 | 8     | Link/backlink performance           | projection benchmarks                     | p95 300ms target                            |
| MVP-105 | 8     | Asset metadata performance          | asset metadata benchmarks                 | p95 300ms target                            |
| MVP-110 | 8     | Clean machine install smoke         | install once smoke                        | no external runtime/config                  |
| MVP-111 | 8     | Local data preservation smoke       | migration and reinstall smoke             | data/version/assets preserved               |
| MVP-112 | 8     | MVP end-to-end flow                 | full user flow smoke                      | create/edit/link/search/restore             |
| MVP-113 | 8     | Documentation update                | user/developer MVP docs                   | current release gates documented            |

## 7. Decision Records

다음 항목은 구현 중 즉흥적으로 결정하지 않는다. 각 항목은 Phase 0 또는 해당 phase 진입 전에 decision record로 남긴다. decision record는 문제, 선택지, 결정, 거부한 선택지, architecture impact, test impact, rollback plan을 포함해야 한다.

decision record 형식은 다음이다.

```text
Decision:
Context:
Options:
Selected Option:
Rejected Options:
Architecture Impact:
Configuration Impact:
Logging Impact:
State Machine Impact:
Test Strategy:
Rollback or Migration Plan:
Review Owner:
```

필수 decision record는 다음이다.

| Decision                                   | Deadline     | Required Criteria                                                                                                  | Rejection Criteria                                       |
| ------------------------------------------ | ------------ | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| Local metadata store                       | Phase 4 시작 전 | 외부 DB 서버 없이 설치 1회로 초기화된다. migration runner와 contract test가 가능하다. current snapshot metadata 조회가 300ms 목표를 방해하지 않는다. | 수동 DB 설치, runtime daemon 필수, 숨겨진 global connection       |
| Internal version store                     | Phase 4 시작 전 | Git CLI 없이 동작한다. current snapshot과 history 조회가 분리된다. corruption detection과 recovery test가 가능하다.                    | 사용자-facing Git 개념 노출, history 전체 스캔 기반 current 조회        |
| Current snapshot layout                    | Phase 4 시작 전 | latest snapshot direct read가 가능하다. atomic write와 partial recovery가 가능하다.                                           | version history를 매번 순회해야 최신 문서를 알 수 있는 구조                |
| Local search index                         | Phase 6 시작 전 | 외부 검색 서버 없이 동작한다. rebuild가 가능하다. p95 300ms benchmark가 가능하다.                                                        | 원본 파일 전체 스캔이 기본 조회 경로인 구조                                |
| Markdown/MDX parser                        | Phase 6 시작 전 | Markdown link, Wikilink, heading, asset reference 추출이 가능하다. parser output을 domain value object로 변환할 수 있다.          | parser 내부에 document lifecycle 또는 storage rule을 넣어야 하는 구조 |
| Asset reference syntax                     | Phase 3 종료 전 | 문서 본문에 원본 파일을 넣지 않고 asset id/reference만 표현한다. export/import roundtrip이 가능하다.                                       | 파일 경로 원문을 영구 식별자로 사용하는 구조                                |
| Import conflict policy                     | Phase 6 시작 전 | duplicate path, unsupported file, partial failure, broken link를 명시적으로 처리한다.                                        | 실패 시 workspace를 오염시키는 구조                                 |
| Export output policy                       | Phase 6 시작 전 | Markdown export와 HTML export가 데이터 이동성을 보장한다. PDF는 unsupported 또는 async extension boundary로 명시한다.                   | export가 내부 storage schema에 종속되는 구조                       |
| Product/Field/Development log event naming | Phase 2 시작 전 | stable event name, error code, masking 기준이 있다.                                                                     | free text only log, 민감 정보 포함 가능성                         |
| Performance fixture shape                  | Phase 8 시작 전 | 문서 수, 평균 본문 크기, 링크 수, 첨부 수, 측정 환경이 고정된다.                                                                           | 측정 환경과 데이터 크기를 기록하지 않는 benchmark                         |

decision record 리뷰 기준은 다음이다.

- AGENTS 원칙과 충돌하면 결정하지 않는다.
- 설정 정책 완화를 요구하는 선택지는 거부한다.
- Product Log에 원문 데이터를 요구하는 선택지는 거부한다.
- 상태가 3개 이상인데 상태머신이 없는 선택지는 보류한다.
- TDD를 어렵게 만드는 선택지는 경계 설계를 수정한 뒤 재검토한다.
- 로컬 기본 실행에 외부 DB, 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일을 요구하는 선택지는 거부한다.

## 8. TDD Strategy

모든 작업은 다음 사이클을 따른다.

1. 실패하는 테스트를 먼저 작성한다.
2. 테스트를 통과하는 최소 구현을 작성한다.
3. 중복과 구조 문제를 정리한다.
4. 정리 후 관련 테스트를 다시 실행한다.
5. 외부 의존성은 테스트 더블, port, interface로 대체 가능하게 유지한다.
6. 설정, 로그, 상태 전이, 오류 처리, 성능 조건을 테스트 대상에 포함한다.

테스트 분류는 다음이다.

- Domain test: 외부 시스템 없이 entity, value object, domain service, state machine을 검증한다.
- Usecase test: fake repository, fake version store, fake clock, fake id generator, fake logger를 주입한다.
- Port contract test: repository, version store, search index, asset store, parser, logger 구현체가 계약을 지키는지 검증한다.
- Adapter integration test: temp directory 또는 local embedded store를 사용해 외부 boundary 구현을 검증한다.
- UI component test: fake client를 사용해 rendering과 user event mapping을 검증한다.
- Platform adapter test: command/controller가 DTO mapping과 usecase invocation만 수행하는지 검증한다.
- Performance test: fixture 크기와 측정 환경을 기록하고 p95 300ms 목표를 측정한다.
- Smoke test: clean install, data preservation, full MVP flow를 검증한다.

필수 테스트 기준은 다음이다.

- 도메인 계층이 외부 framework에 의존하지 않는지 확인한다.
- 유스케이스가 명시적 input/output을 가지는지 확인한다.
- 외부 환경 값이 프로그램 시작 이후 암묵적으로 재조회되지 않는지 확인한다.
- 설정 값이 프로세스 중간에 삽입되거나 변경되지 않는지 확인한다.
- 외부 API, DB, filesystem, network 접근이 boundary 계층에만 존재하는지 확인한다.
- 테스트 더블로 외부 의존성을 대체할 수 있는지 확인한다.
- 로그가 3단계 정책에 맞게 분리되어 있는지 확인한다.
- Development Log가 production 기본 동작에 포함되지 않는지 확인한다.
- 복잡한 내부 흐름이 flag 조합이 아니라 명시적 상태 전이로 표현되는지 확인한다.
- 리팩터링과 기능 변경이 분리되어 있는지 확인한다.

테스트 이름은 동작을 설명해야 한다.

허용되는 테스트 이름:

```text
get_current_document_reads_current_snapshot_without_scanning_history
bootstrap_reads_external_environment_only_once
restore_failure_preserves_current_snapshot
product_log_excludes_document_body
field_debug_log_requires_scope_and_expiry
document_lifecycle_rejects_invalid_transition
```

거부되는 테스트 이름:

```text
test_document
test_config
works
handles_error
```

작업 단위 템플릿은 다음이다.

```text
Work Item:
Phase:
Goal:
Layer:
Boundary:
Failing Test First:
Minimal Implementation:
Tidy First Needed:
Configuration Impact:
Logging Classification:
State Machine Impact:
Performance Impact:
Validation Commands or Checks:
Done Evidence:
```

작업 단위 작성 규칙은 다음이다.

- `Layer`에는 domain, usecase, adapter, infrastructure, UI, platform shell 중 하나를 적는다.
- `Boundary`에는 호출 방향과 금지 의존성을 적는다.
- `Failing Test First`에는 최소 1개의 실패 테스트 이름을 적는다.
- `Tidy First Needed`가 `yes`이면 기능 변경 전에 별도 변경으로 처리한다.
- `Configuration Impact`가 있으면 bootstrap 수신, validation, 명시적 전달 방식을 적는다.
- `Logging Classification`에는 Product Log, Field Debug Log, Development Log, none 중 하나를 적는다.
- `State Machine Impact`가 있으면 state, event, failure state, terminal state를 적는다.
- `Performance Impact`가 있으면 p95 300ms 적용 여부와 fixture를 적는다.
- `Done Evidence`에는 테스트 결과, boundary check, benchmark, smoke 중 필요한 증거를 적는다.

## 9. Configuration and Runtime Environment Policy

설정 정책은 AGENTS 기준을 그대로 적용한다.

허용되는 방식은 다음이다.

- 프로그램 시작 시 외부 환경 값을 1회만 읽는다.
- 읽은 값은 즉시 검증한다.
- 검증된 값은 immutable config object로 변환한다.
- config object는 composition root에서 dependency graph에 주입한다.
- usecase에는 필요한 policy object, 생성자 인자, 함수 인자, context object로 전달한다.
- 테스트에서는 환경 변수를 변경하지 않고 config object를 직접 생성한다.
- 로컬 기본 실행은 config file 없이 동작한다.
- 고급 설정 변경은 명시적 설정 화면 또는 import/export 절차로만 제공한다.

거부되는 방식은 다음이다.

- domain 또는 usecase에서 environment variable을 직접 읽는 코드
- runtime 중간에 환경 값을 삽입하거나 변경하는 API
- 설정 값을 global singleton 또는 static mutable object로 보관하는 코드
- 함수 내부에서 숨겨진 config registry를 조회하는 코드
- 테스트가 외부 `.env` 파일 또는 실행 순서에 의존하는 구조
- 로컬 기본 실행이 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일 편집을 요구하는 구조

설정 추가 시 반드시 기록할 항목은 다음이다.

- 설정 이름
- 설정 목적
- 값의 출처
- 기본값
- validation rule
- 민감 정보 여부
- 전달 방식
- 사용 계층
- 테스트 방식
- runtime 변경 가능 여부

MVP 기본값은 다음 원칙을 따른다.

- local app data path는 platform path resolver가 결정한다.
- metadata store는 최초 실행 시 자동 생성한다.
- internal version store는 최초 실행 시 자동 생성한다.
- asset store는 최초 실행 시 자동 생성한다.
- search index는 최초 실행 시 자동 생성한다.
- logging mode는 Product Log 최소 + Development Log local/test only + Field Debug disabled로 시작한다.
- search/index rebuild는 명시적 command 또는 자동 recovery flow로 처리한다.

## 10. Logging Strategy

로그는 Product Log, Field Debug Log, Development Log 세 종류만 사용한다.

### Product Log

- Purpose: 사용자 영향, 핵심 상태 변화, 장애 원인 추적에 필요한 최소 운영 이벤트만 안정적인 event name과 error code로 기록한다.
- Allowed:
  - correlation id
  - usecase name
  - masked workspace id
  - document id
  - stable event name
  - stable error code
  - retryable 여부
  - duration bucket
  - 핵심 상태 전이 이름
- Forbidden:
  - 문서 본문
  - 첨부 파일 내용
  - token, secret, API key, session id
  - 사용자 개인정보 원문
  - 전체 request/response dump
  - 테스트 fixture 상세
- Use:
  - first-run completed/failed
  - migration completed/failed
  - document created/updated/deleted/restored
  - import/export completed/failed
  - index rebuild completed/failed
  - unrecoverable storage failure
- Example:

```text
INFO document.restore.completed correlation_id=... workspace_id=masked:... document_id=... duration_ms=42
WARN migration.failed correlation_id=... error_code=MIGRATION_LOCK_TIMEOUT retryable=true
```

- Review:
  - 운영 판단에 필요하지 않은 로그는 제거한다.
  - 민감 정보가 포함되면 거부한다.
  - stable event name 또는 error code가 없으면 수정한다.

### Field Debug Log

- Purpose: 운영 또는 고객 환경에서 재현이 어려운 문제를 제한적으로 진단한다.
- Activation:
  - 기본값은 비활성이다.
  - scope를 지정해야 한다.
  - expiry를 지정해야 한다.
  - 활성화와 비활성화는 Product Log에 남긴다.
- Allowed:
  - masked id
  - query hash
  - state machine name/current state/event
  - index freshness metadata
  - cache hit/miss
  - retry count
  - parser warning count
  - permission decision summary
- Forbidden:
  - 문서 본문 원문
  - 검색 query 원문
  - 첨부 파일 내용
  - token, secret, credential
  - 전체 request/response body
  - 전체 사용자 대상 무기한 활성화
- Use:
  - search가 300ms 목표를 넘는 원인 확인
  - index freshness 확인
  - restore 실패 전이 확인
  - migration 재시도 상태 확인
  - import/export partial failure 확인
- Example:

```text
DEBUG field.search.query correlation_id=... scope=workspace:masked query_hash=... candidate_count=120 filtered_count=13 duration_ms=87
DEBUG field.state.transition correlation_id=... machine=Restore from=WritingNewVersion event=Fail to=Failed retryable=true
```

- Review:
  - scope와 expiry가 없으면 거부한다.
  - Product Log로 충분한 내용을 중복 기록하면 제거한다.
  - 마스킹 기준이 불명확하면 거부한다.

### Development Log

- Purpose: 로컬 개발, 테스트, 검증 과정에서 구현 상태와 테스트 실패 원인을 확인한다.
- Allowed:
  - fake port call order
  - parser intermediate summary
  - mapper result summary
  - benchmark raw samples
  - local fixture creation summary
  - state transition detail in tests
- Forbidden:
  - production default artifact 포함
  - 운영 장애 분석용 장기 로그 대체
  - 실제 고객 데이터 원문
  - secret, token, credential
- Use:
  - local development
  - unit test
  - integration test
  - smoke test
  - benchmark development
- Example:

```text
DEV fake.version_store.appended document_id=test-doc version_id=v3
DEV parser.links.detected markdown=2 wikilink=4 unresolved=1
```

- Review:
  - Development Log가 production default path에 포함되면 거부한다.
  - Development Log가 테스트 assertion을 대체하면 거부한다.
  - 임시 로그가 제거되지 않으면 거부한다.

## 11. State Machine Strategy

상태가 3개 이상이거나 실패, 재시도, 취소, 종료 상태가 있는 내부 절차는 상태머신으로 관리한다. boolean flag 조합으로 복잡한 흐름을 관리하지 않는다.

상태머신 정의에는 다음 항목이 반드시 포함되어야 한다.

- machine name
- state
- event
- guard condition
- transition
- terminal state
- failure state
- retryability
- side effect request
- product log mapping
- field debug mapping
- error code
- transition tests

표준 transition result는 다음 구조를 따른다.

```text
TransitionResult:
  next_state
  side_effect_requests
  product_log_event
  field_debug_event
  error_code
  retryable
```

MVP 상태머신 대상은 다음이다.

| Machine                | Required States                                                                                                                     | Required Events                                                                                                     | Validation                          |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FirstRunInitialization | NotStarted, ResolvingPaths, CreatingStores, WritingMetadata, Completed, Failed, Retrying                                            | Start, PathsResolved, StoreCreated, MetadataWritten, Fail, Retry, Complete                                          | partial failure recovery            |
| LocalMigration         | NotStarted, Locked, Running, Completed, Failed, Retrying                                                                            | AcquireLock, RunMigration, MigrationSucceeded, MigrationFailed, Retry, ReleaseLock                                  | idempotent retry                    |
| DocumentLifecycle      | Draft, Saved, Editing, Archived, Deleted, Restored                                                                                  | Create, Save, StartEdit, Archive, Delete, Restore                                                                   | invalid transition rejection        |
| AssetLifecycle         | Registered, Linked, Unlinked, Archived, Restored, Missing                                                                           | Register, Link, Unlink, Archive, Restore, MarkMissing                                                               | missing asset handling              |
| RestoreDocument        | Requested, LoadingCurrent, LoadingTarget, Previewed, WritingNewVersion, UpdatingCurrent, ReindexRequested, Completed, Failed        | Request, CurrentLoaded, TargetLoaded, PreviewCreated, VersionWritten, CurrentUpdated, ReindexQueued, Fail, Complete | current preservation on failure     |
| IndexRebuild           | Requested, ScanningMetadata, BuildingIndex, SwappingIndex, Completed, Failed                                                        | Request, ScanCompleted, BuildCompleted, SwapCompleted, Fail                                                         | old index remains usable on failure |
| ImportJob              | Requested, ValidatingSource, ScanningFiles, MappingPaths, WritingDocuments, WritingAssets, Reindexing, Completed, Failed, Cancelled | Request, SourceValid, FilesScanned, PathsMapped, DocumentsWritten, AssetsWritten, Reindexed, Fail, Cancel           | partial import safety               |
| ExportJob              | Requested, ResolvingDocuments, Rendering, WritingOutput, Completed, Failed, Cancelled                                               | Request, DocumentsResolved, Rendered, OutputWritten, Fail, Cancel                                                   | no partial output corruption        |

상태머신 리뷰 기준은 다음이다.

- 상태와 이벤트가 enum 또는 동등하게 명시적인 형태인지 확인한다.
- 전이 함수가 pure function인지 확인한다.
- 외부 I/O가 전이 결정 안에 섞이지 않는지 확인한다.
- 실패 상태와 종료 상태가 명확한지 확인한다.
- invalid transition 테스트가 있는지 확인한다.
- 상태 변경이 Product Log 또는 Field Debug Log 정책과 연결되어 있는지 확인한다.

## 12. Dependency and Boundary Rules

다음 dependency direction을 지켜야 한다.

```text
Domain:
  may depend on: none
  must not depend on: usecase, adapter, infrastructure, framework, UI, editor, environment

Usecase:
  may depend on: domain, port interfaces
  must not depend on: database, network, filesystem, framework context, UI model, platform SDK

Adapter:
  may depend on: usecase contract, DTO, mapper, port implementation dependencies
  must not contain: domain rules, lifecycle decisions, permission decisions

Infrastructure:
  may depend on: port interfaces, external libraries
  must not contain: usecase decisions

Client UI:
  may depend on: client contract, view model, editor package
  must not contain: domain rules, storage rules, version rules

Platform Shell:
  may depend on: adapter, composition root, platform SDK
  must not contain: document business rules
```

boundary별 허용 작업은 다음이다.

- Domain: validation, identity, lifecycle transition, pure domain policy.
- Usecase: input validation orchestration, port call sequence, transaction boundary decision, error mapping, log event request.
- Adapter: DTO mapping, parser implementation, storage implementation, platform API integration, search implementation.
- Infrastructure: filesystem, local embedded store, internal version engine, search index, asset object store, clock, id generator, logger implementation.
- UI: rendering, user event capture, editor interaction, command invocation, loading/error display.

boundary 위반 예시는 다음이다.

- domain이 filesystem path를 직접 읽는다.
- usecase가 environment variable을 직접 읽는다.
- UI가 document lifecycle transition을 직접 판단한다.
- desktop command가 version restore rule을 직접 구현한다.
- search adapter가 permission 또는 document lifecycle rule을 임의로 결정한다.
- Product Log 구현체를 domain에 주입한다.
- CodeMirror state를 document entity로 저장한다.

## 13. Risk and Mitigation

| Risk                                     | Impact                       | Mitigation                                    | Validation                    |
| ---------------------------------------- | ---------------------------- | --------------------------------------------- | ----------------------------- |
| Domain이 framework 또는 storage schema에 종속됨 | 서버/모바일/협업 확장 불가              | mapper와 port를 강제한다                            | dependency boundary check     |
| current 조회가 history store scan에 의존함      | 300ms 목표 실패                  | current snapshot repository를 분리한다             | no history scan test          |
| search가 원본 전체 스캔으로 구현됨                   | 검색 성능 실패                     | embedded index와 freshness metadata를 사용한다      | search p95 benchmark          |
| local setup이 수동 설정을 요구함                  | 설치 1회 원칙 실패                  | first-run initializer를 구현한다                   | clean install smoke           |
| config가 global singleton으로 숨겨짐           | 테스트 어려움, runtime 변경 위험       | immutable config를 DI로 전달한다                    | bootstrap/env scan test       |
| 로그에 문서 원문이 남음                            | 보안/신뢰성 문제                    | log payload test와 masking rule 적용             | sensitive data exclusion test |
| Field Debug가 무기한 활성화됨                    | 운영 데이터 노출 위험                 | scope와 expiry 필수화                             | field debug activation test   |
| restore 실패 시 current 문서가 손상됨             | 데이터 손실                       | restore state machine과 atomic write           | failure injection test        |
| import partial failure가 workspace를 오염시킴  | 데이터 일관성 손상                   | import job state와 rollback/skip policy        | partial import test           |
| adapter가 domain rule을 재구현함               | 플랫폼별 동작 불일치                  | adapter는 mapper와 port 구현만 담당                  | code review checklist         |
| UI가 current/history를 혼동함                 | 사용자 경험 오류, 성능 실패             | UI query contract 분리                          | current/history UI test       |
| version store 선택이 Git CLI에 의존함           | 설치 1회 원칙 실패                  | embedded version engine만 허용                   | Git CLI absence smoke         |
| 성능 테스트가 늦게 발견됨                           | 구조 변경 비용 증가                  | phase 5부터 instrumentation point 추가            | incremental benchmark         |
| 리팩터링과 기능 변경이 섞임                          | 리뷰 어려움, 회귀 위험                | Tidy First 변경 분리                              | change review                 |
| decision record 없이 기술 선택이 진행됨            | store/search/parser 교체 비용 증가 | 필수 decision record deadline을 phase gate로 둔다   | decision record review        |
| phase gate를 건너뛰고 UI부터 구현됨                | domain/usecase 경계 약화         | phase entry/exit gate를 변경 리뷰 기준에 포함한다         | phase gate checklist          |
| MVP에 iOS/Android/SaaS 구현이 섞임             | MVP 일정과 구조 불안정               | MVP platform scope table을 기준으로 구현 제외 항목을 거부한다 | scope review                  |
| Field Debug Log가 Product Log와 섞임         | 운영 로그 과다, 민감 정보 노출 위험        | logger port와 event name을 분리한다                 | logger classification test    |
| import/export가 내부 storage schema에 종속됨    | 데이터 이동성 저하                   | domain DTO와 export mapper를 분리한다               | roundtrip export/import test  |

## 14. Review Checklist

모든 변경은 다음 체크리스트를 통과해야 한다.

- 변경이 Layered Architecture를 지키는가.
- 변경이 Clean Architecture 경계를 지키는가.
- domain 계층이 외부 framework, DB, filesystem, network, environment, UI, editor에 의존하지 않는가.
- usecase가 명시적 input/output을 가지는가.
- usecase가 framework request/response, platform command payload, DB row를 직접 받지 않는가.
- 외부 API, DB, filesystem, network, environment 접근이 boundary 계층에만 있는가.
- interface와 implementation이 분리되어 있는가.
- 테스트 더블로 외부 의존성을 대체할 수 있는가.
- 실패하는 테스트가 먼저 작성되었거나 변경과 함께 추적 가능한가.
- 설정 값은 bootstrap에서 최초 1회만 수신되는가.
- bootstrap 이후 환경 값 재조회가 없는가.
- runtime 중간 설정 삽입 또는 변경 API가 없는가.
- config가 global singleton으로 숨겨지지 않는가.
- Product Log, Field Debug Log, Development Log가 분리되어 있는가.
- Product Log에 문서 본문, 첨부 내용, secret이 없는가.
- Field Debug Log에 scope와 expiry가 있는가.
- Development Log가 production 기본 동작에 포함되지 않는가.
- 복잡한 내부 흐름이 상태머신으로 표현되는가.
- 상태머신에 상태, 이벤트, 전이 조건, 실패 상태, 종료 상태가 있는가.
- invalid transition 테스트가 있는가.
- 현재 문서 조회와 이력 조회가 별도 usecase와 query path로 분리되어 있는가.
- 현재 문서 조회가 version history 전체 스캔에 의존하지 않는가.
- 이력 조회가 pagination을 사용하는가.
- 검색, 링크, 백링크, 첨부 metadata 조회가 index/projection/cache/pagination 중 하나를 사용하는가.
- p95 300ms 목표와 측정 방법이 정의되어 있는가.
- Tidy First 변경과 기능 변경이 분리되어 있는가.
- UI 또는 platform shell에 domain rule이 복제되지 않았는가.
- CodeMirror state가 domain model로 사용되지 않았는가.
- Git 개념이 사용자-facing UI/API에 노출되지 않는가.
- Git CLI가 로컬 기본 실행 조건이 아닌가.
- clean machine install smoke가 깨지지 않는가.
- 해당 변경이 현재 phase의 entry gate와 exit gate를 만족하는가.
- 필수 decision record deadline 전에 기술 선택이 코드로 고정되지 않았는가.
- MVP scope 밖의 플랫폼 구현이나 SaaS 전용 기능이 섞이지 않았는가.

## 15. Definition of Done

MVP 전체 Definition of Done은 다음이다.

- 앱 설치 1회 후 추가 수동 설정 없이 기본 workspace를 만들 수 있다.
- 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 환경 변수, 수동 설정 파일 편집 없이 로컬 MVP가 동작한다.
- 단일 사용자가 Markdown/MDX 문서를 생성, 현재 조회, 수정, 삭제할 수 있다.
- 현재 문서 조회와 이력 조회가 UI/API/usecase/query path에서 분리되어 있다.
- 현재 문서 조회는 latest snapshot을 사용하고 version history 전체 스캔에 의존하지 않는다.
- 이력 목록 조회는 pagination을 사용한다.
- 특정 version 조회가 가능하다.
- diff 조회가 가능하다.
- restore preview가 가능하다.
- 특정 version 복원이 가능하다.
- 문서 수정은 version entry를 생성한다.
- Markdown link와 Wikilink가 파싱된다.
- 백링크, 미해결 링크, 고아 문서 조회가 가능하다.
- 첨부 파일은 문서 본문과 분리되어 asset store에 저장된다.
- 문서에서 첨부 파일 reference를 표시할 수 있다.
- 첨부 metadata 조회가 가능하다.
- 로컬 전체 텍스트 검색이 가능하다.
- Markdown folder import가 가능하다.
- Markdown export가 가능하다.
- HTML export가 가능하다.
- PDF export는 명확한 unsupported 또는 extension boundary를 가진다.
- Product Log에는 문서 원문, 첨부 내용, secret이 포함되지 않는다.
- Field Debug Log는 기본 비활성이고 scope와 expiry 없이는 활성화되지 않는다.
- Development Log는 production 기본 경로에 포함되지 않는다.
- first-run, migration, document lifecycle, asset lifecycle, restore, import/export, index rebuild가 상태머신 또는 명시적 상태 전이로 관리된다.
- 정상 인덱스 상태에서 현재 문서 조회, 이력 목록 조회, 특정 version 조회, 링크/백링크 조회, 첨부 metadata 조회, 기본 검색의 p95 300ms 목표가 측정되고 기준을 만족한다.
- clean machine install smoke test가 통과한다.
- local data preservation smoke test가 통과한다.
- MVP end-to-end flow가 통과한다.
- 사용자 문서와 개발자 test gate 문서가 최신 상태다.

## 16. Prohibited Implementation Patterns

다음 구현은 MVP에서 금지한다.

- domain 계층에서 DB, HTTP, filesystem, network, environment variable을 직접 접근하는 코드
- usecase에서 framework request/response 객체를 직접 사용하는 코드
- usecase에서 concrete repository를 직접 생성하는 코드
- UI 또는 platform shell에서 document lifecycle, restore, permission, version rule을 직접 구현하는 코드
- 전역 mutable state
- 설정, logger, repository, clock, id generator singleton
- 함수 내부에서 숨겨진 runtime config를 조회하는 코드
- runtime 중간에 환경 설정 값을 삽입하거나 변경하는 코드
- 테스트가 외부 환경 파일이나 환경 변수 순서에 의존하는 구조
- 로컬 기본 실행에 외부 DB, 검색 서버, Git CLI, Node.js 설치를 요구하는 구조
- 사용자가 설정 파일을 직접 편집해야만 앱을 시작할 수 있는 구조
- Product Log에 문서 원문, 첨부 내용, token, secret을 남기는 코드
- Field Debug Log를 scope/expiry 없이 활성화하는 코드
- Development Log를 production default artifact에 포함하는 코드
- 복잡한 절차를 boolean flag 조합으로 관리하는 코드
- 테스트 없는 상태 전이 로직
- 실패 경로 없이 성공 경로만 구현하는 코드
- current document 조회에서 version history 전체를 스캔하는 코드
- history 조회가 current document 조회 path를 느리게 만드는 구조
- 검색이 원본 파일 전체 스캔을 기본값으로 삼는 구조
- 검색/조회 성능 문제를 UI loading spinner로만 숨기는 구현
- CodeMirror state를 domain document model로 저장하는 코드
- Git commit, branch, repository 같은 개념을 사용자-facing 문서 이력 UI에 노출하는 구현
- 기능 변경과 리팩터링을 하나의 변경으로 섞는 커밋

## 17. Next Actions

다음 순서로 즉시 작업을 시작한다.

1. Phase 0 기준으로 이 문서와 AGENTS 기준이 충돌하지 않는지 리뷰한다.
2. Phase Gate Rules의 entry/exit gate를 작업 추적 기준으로 등록한다.
3. 필수 decision record 목록을 만들고 deadline을 Phase 0 산출물로 고정한다.
4. Local metadata store, internal version store, current snapshot layout decision record를 먼저 작성한다.
5. Work Item 템플릿을 사용해 `MVP-001`, `MVP-002`, `MVP-003` 작업 카드를 만든다.
6. Phase 1의 skeleton 변경을 Tidy First 성격의 작은 변경으로 시작한다.
7. domain/usecase/adapter/UI/platform shell의 빈 package boundary를 만든다.
8. dependency boundary check를 추가한다.
9. test runner smoke test를 추가한다.
10. Phase 2의 `AppConfig` 실패 테스트를 먼저 작성한다.
11. bootstrap에서 외부 환경 값을 1회만 읽는 최소 구현을 작성한다.
12. first-run initializer 실패 테스트를 작성한다.
13. Product/Field Debug/Development logger port 분리 테스트를 작성한다.
14. Phase 3으로 넘어가기 전에 Phase 1과 Phase 2의 Done Criteria를 모두 확인한다.

구현 중 새 리스크가 발견되면 다음 규칙을 따른다.

- architecture boundary를 바꾸는 리스크는 즉시 decision record로 분리한다.
- Tidy First가 필요한 리스크는 기능 변경 전에 별도 변경으로 처리한다.
- 설정 정책을 완화해야 해결되는 리스크는 거부하고 다른 설계를 찾는다.
- 로그에 민감 정보를 넣어야 해결되는 리스크는 거부하고 id, hash, count, status, error code로 대체한다.
- 상태 flag가 늘어나는 리스크는 상태머신으로 전환한다.
- p95 300ms 목표를 넘는 동기 조회는 pagination, index, projection, cache, 비동기 job 중 하나로 설계를 바꾼다.
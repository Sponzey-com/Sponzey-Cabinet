# Sponzey Cabinet 단계별 목표와 개발 계획

작성일: 2026-06-22  
최종 갱신일: 2026-07-15
문서 성격: `PROJECT.md`의 최종 제품 목표를 단계적으로 구현하기 위한 개발 계획  
기준 문서: `PROJECT.md`, `AGENTS.md`

## 계획 수립 원칙

현재 활성 로드맵은 개인 PC에 설치하는 단일 사용자 로컬 지식관리 제품만 다룬다. 아래 6단계 구조 중 self-host, 멀티 사용자, SaaS, 엔터프라이즈에 해당하는 단계는 장기 확장 참고안이며 사용자의 명시적 요구 전까지 활성 개발 순서, task, release gate 또는 기본 UI 범위로 사용하지 않는다.

Phase 001부터 Phase 012까지의 누적 개발은 초기 MVP 범위를 넘어 macOS 개인용 데스크톱 제품의 durable document, Graph, Canvas, Asset, backup/recovery 기준선을 완성했다. Phase 013은 공통 UI, 한국어 사용자 표현, 내부 ID 비노출, 실제 action 연결을 통합했고 후속 hardening은 문서 첫 줄을 제목의 단일 원천으로 확정했다. 다음 단계가 생성되더라도 사용자가 범위를 변경하기 전에는 이 로컬 제품의 사용성, 안정성, 데이터 소유권과 배포 품질만 확장해야 한다.

모든 단계는 다음 기준을 따른다.

- Layered Architecture를 유지한다.
- Clean Architecture에 따라 도메인, 유스케이스, 어댑터, 인프라를 분리한다.
- TDD를 기본 개발 방식으로 적용한다.
- Tidy First 원칙에 따라 정리 작업과 기능 변경을 분리한다.
- 외부 설정은 프로그램 시작 시 1회만 읽고 내부에는 명시적 인자나 의존성 주입으로 전달한다.
- Product Log, Field Debug Log, Development Log를 구분한다.
- 복잡한 내부 절차는 상태머신으로 관리한다.
- Git은 Markdown/MDX 원본과 변경 이력 관리를 위한 내부 엔진으로만 사용한다.
- 사용자는 Git, commit, branch, repository를 몰라도 제품을 사용할 수 있어야 한다.
- Git provider 연동과 코드 저장소형 리뷰/병합 흐름은 제품 목표에 포함하지 않는다.
- 현재 구현 및 인증 플랫폼은 macOS다. Windows/Linux는 공통 아키텍처를 유지하는 차후 데스크톱 인증 대상이고, Web/iOS/Android는 차후 제품 대상이다.
- 플랫폼별 구현은 공통 도메인/유스케이스를 재사용하고, 플랫폼 차이는 어댑터 계층에서 처리한다.
- 개인 구축 로컬 설정은 설치 1회로 완료되어야 한다.
- 로컬 앱은 별도 DB, 검색 서버, Git CLI, Node.js, 환경 변수 편집, 외부 설정 파일 수정을 기본 사용 조건으로 요구하지 않는다.

## 단계 요약

| 단계 | 이름                             | 핵심 목표                                                  | 주요 산출물                                                                                                          |
| -- | ------------------------------ | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| 1  | 개인 로컬 Knowledge Base 제품 | 단일 사용자가 Markdown 문서를 작성하고 관계와 파일을 관리하며 Graph와 Canvas로 지식을 탐색한다. | 공통 core, macOS Tauri 앱, CodeMirror 작성, 첫 줄 기반 문서 제목, durable current/history, 검색/링크 projection, Graph, Canvas, Asset, backup/recovery. Phase 013 구현 기준선 완료, current fingerprint 재검증 진행 |
| 2  | 개인 호스팅과 팀 협업 기반 | **중지된 차후 목표.** 사용자가 명시적으로 활성화한 뒤에만 계획한다. | 서버 API, RBAC, 댓글, 리뷰와 감사는 현재 구현 범위가 아니다. |
| 3  | 협업 Graph/Canvas 확장 | **중지된 차후 목표.** 현재 개인용 Graph/Canvas와 실시간 협업을 혼동하지 않는다. | 실시간 공동 편집, 협업 Canvas, 모바일 클라이언트는 현재 구현 범위가 아니다. |
| 4  | AI와 외부 연동 플랫폼 | 로컬 개인 제품 범위의 AI/연동은 별도 후속 계획으로만 구체화한다. 서버형 integration platform은 중지한다. | provider boundary와 확장성은 유지하되 원격 플랫폼 기능을 현재 완료로 주장하지 않는다. |
| 5  | 플러그인과 업무 객체 플랫폼 | **중지된 차후 목표.** 사용자의 명시적 범위 변경이 필요하다. | plugin runtime, CRM과 custom object는 현재 구현 범위가 아니다. |
| 6  | SaaS와 엔터프라이즈 운영 | **중지된 차후 목표.** 사용자의 명시적 범위 변경이 필요하다. | 멀티테넌트, 과금, SSO/SCIM, 관리자 콘솔과 엔터프라이즈 운영은 현재 구현 범위가 아니다. |

## 현재 완료 상태

- Phase 012 archive: `.tasks/phase012/`
- 최종 gate: `.tasks/phase012/phase012-release-gate-result.md`
- 검증 범위: macOS 개인용 로컬 데스크톱 앱
- 요구사항 증거: 33개 requirement가 동일한 current source fingerprint에 연결됨
- packaged workflow: Home, Document, Graph, Canvas, Assets, Backup/Restore, lifecycle와 recovery를 actual `.app`에서 검증함
- 성능: current/history/search/link/Graph/Canvas/Asset metadata의 release-mode p95 300ms 기준을 충족함
- 유예 범위: Windows/Linux native certification, Web/iOS/Android 제품, self-host, SaaS, 멀티 사용자, 실시간 협업, 조직/RBAC UI

이 완료 상태는 task 체크박스만으로 판단하지 않는다. Phase 012의 command summary, requirement evidence matrix, native platform matrix, packaged UI smoke, query performance, visual 및 security artifact를 함께 사용한다.

## 공통 개발 게이트

각 단계는 완료 전에 다음 게이트를 통과해야 한다.

### Architecture Gate

- 도메인 계층이 외부 프레임워크, DB, 파일시스템, 네트워크, 환경 변수에 의존하지 않아야 한다.
- 유스케이스는 명확한 입력과 출력을 가져야 한다.
- 외부 I/O는 포트와 어댑터 뒤에 있어야 한다.
- 인프라 구현체는 유스케이스와 도메인에 직접 노출되지 않아야 한다.
- 새 기능은 도메인 규칙, 유스케이스, 어댑터, 인프라 책임을 구분해야 한다.

### TDD Gate

- 실패하는 테스트가 먼저 작성되어야 한다.
- 도메인 규칙은 외부 시스템 없이 테스트되어야 한다.
- 유스케이스는 fake repository, fake clock, fake id generator, fake logger로 테스트되어야 한다.
- 설정 검증, 로그 이벤트, 상태 전이, 오류 처리가 테스트되어야 한다.
- 외부 어댑터는 계약 테스트 또는 통합 테스트로 검증되어야 한다.

### Tidy First Gate

- 정리 작업과 기능 변경이 분리되어야 한다.
- 리팩터링은 동작 변경 없이 수행되어야 한다.
- 구조 변경 후 기존 테스트가 통과해야 한다.
- 기능 구현 커밋에는 불필요한 대규모 포맷 변경이 없어야 한다.

### Configuration Gate

- 외부 환경 값은 bootstrap에서만 읽어야 한다.
- 설정 값은 검증된 config object로 변환되어야 한다.
- 내부 흐름에서는 명시적 인자, 생성자 인자, 컨텍스트 객체, 의존성 주입으로 전달되어야 한다.
- 함수 내부에서 환경 변수나 외부 설정 파일을 다시 읽으면 안 된다.

### Logging Gate

- 모든 로그는 Product Log, Field Debug Log, Development Log 중 하나로 분류되어야 한다.
- Product Log에는 운영에 필요한 최소 정보만 기록되어야 한다.
- Field Debug Log는 활성화 범위와 만료 조건을 가져야 한다.
- Development Log는 프로덕션 기본 빌드에 포함되면 안 된다.
- 민감 정보, 문서 원문, 첨부 내용, 토큰, AI prompt 원문은 로그에 남기면 안 된다.

### State Machine Gate

- 상태가 3개 이상이거나 실패/재시도/종료가 있는 흐름은 상태머신으로 표현해야 한다.
- 상태, 이벤트, 전이 조건, 실패 상태, 종료 상태가 명시되어야 한다.
- 상태 전이는 독립적으로 테스트되어야 한다.
- 상태 변경은 로그 정책과 연결되어야 한다.

### Platform Gate

- 장기 대상인 Web, iOS, Android, Windows, macOS, Linux는 공통 도메인/유스케이스 위에 구현되어야 한다.
- 현재 gate는 macOS만 native `passed`를 요구하고 Windows/Linux는 `deferred_future`로 기록한다. Web/iOS/Android 제품 gate는 활성화하지 않는다.
- 플랫폼별 파일시스템, 알림, 인증, 보안 저장소, 네트워크 상태, 오프라인 캐시는 어댑터로 분리되어야 한다.
- 플랫폼별 UI가 도메인 규칙을 직접 구현하면 안 된다.
- 플랫폼별 기능 차이가 생기면 capability matrix에 명시해야 한다.
- 모든 플랫폼은 동일한 workspace, document, asset, permission, graph 모델을 사용해야 한다.
- 핵심 유스케이스는 플랫폼별 테스트가 아니라 공통 유스케이스 테스트로 검증되어야 한다.
- 플랫폼별 E2E 테스트는 해당 플랫폼 어댑터, UI 흐름, 저장소/알림/인증 통합을 검증해야 한다.

### Performance Gate

- 모든 사용자-facing 검색과 조회는 정상적인 인덱스 상태에서 p95 300ms 이내 응답을 목표로 해야 한다.
- 문서 현재 조회와 문서 이력 조회는 별도 query path로 분리되어야 한다.
- 현재 문서 조회는 latest snapshot과 metadata를 사용해야 하며, version history 전체를 스캔하면 안 된다.
- 이력 조회는 pagination과 특정 version query를 제공해야 하며, 현재 문서 조회 경로를 느리게 만들면 안 된다.
- 링크/백링크, 폴더/컬렉션, 첨부 metadata, 권한 필터링 검색은 성능 테스트 대상에 포함되어야 한다.
- AI 답변 생성, OCR, embedding, 대용량 export는 비동기로 처리하되, 작업 상태 조회와 캐시된 결과 조회는 300ms 목표를 따라야 한다.
- 성능 기준을 만족하지 못하는 기능은 index, projection, cache, pagination, async worker 중 하나로 구조를 조정해야 한다.

## 1단계: 개인 로컬 Knowledge Base 제품

### 단계 목표

단일 사용자가 로컬 환경에서 Sponzey Cabinet의 핵심 가치를 일상적으로 사용하게 한다. 사용자는 Markdown 문서를 만들고, 문서 간 링크와 Graph를 탐색하고, Canvas를 구성하고, 첨부 파일을 별도 asset으로 관리하며, 검색하고, 변경 이력을 비교/복원하고, 전체 로컬 데이터를 백업/복원할 수 있어야 한다.

이 단계는 이후 모든 단계의 기반이다. 문서 모델, 링크 모델, 첨부 참조 모델, 내부 버전 관리 모델, 검색 인덱스 모델을 처음부터 깨끗하게 만든다.

### 사용자 가치

- 사용자는 Obsidian처럼 로컬 문서를 소유한다.
- 사용자는 Git을 몰라도 모든 문서 변경 이력을 자동으로 가진다.
- 사용자는 문서 간 관계를 링크와 백링크로 확인한다.
- 사용자는 첨부 파일을 문서와 분리된 자산으로 관리한다.
- 사용자는 나중에 개인 호스팅이나 SaaS로 확장 가능한 데이터 구조를 처음부터 사용한다.

### 제품 범위

포함한다.

- 공통 domain/usecase core
- 플랫폼 중립 API/서비스 경계
- React 기반 데스크톱 UI와 개발용 Web preview
- macOS Tauri 데스크톱 앱. Windows/Linux native 인증은 유예한다.
- 로컬 워크스페이스 생성
- Markdown/MDX 문서 생성, 읽기, 수정, 삭제
- Markdown 첫 번째 물리적 줄에서 파생되는 문서 제목. 별도 제목 입력과 독립 title mutation은 기본 사용자 흐름에서 제외한다.
- 제목에서 파생되는 slug와 별도 관리되는 owner, tags, status, createdAt, updatedAt 메타데이터
- Markdown link와 Wikilink 파싱
- 문서 간 링크 생성
- 백링크 조회
- 미해결 링크 조회
- 고아 문서 조회
- 문서 history 조회
- 문서 diff 조회
- 현재 문서 기준 조회
- 이력 기준 조회
- 특정 버전 조회
- 복원 preview 조회
- 특정 버전 복원
- 로컬 첨부 파일 등록
- 문서에서 첨부 파일 참조
- 첨부 파일 metadata 조회
- 실제 link/graph projection 기반 local/global Graph
- durable Canvas 생성, node/edge/geometry/viewport 수정, 보관과 recovery
- content-addressed Asset 저장, document association과 bounded preview
- 문서, Canvas와 Asset을 포함하는 package backup/restore
- 로컬 전체 텍스트 검색
- Markdown folder import
- Markdown/HTML/PDF export의 최소 기반
- Product Log 최소 이벤트
- Development Log 분리
- 설정 bootstrap 1회 로딩
- 플랫폼별 파일 경로, 앱 데이터 경로, 로컬 보안 저장소 adapter의 최소 추상화
- 설치 1회 후 기본 workspace 생성
- 최초 실행 자동 초기화
- 로컬 metadata/version/asset/search store 자동 준비
- 외부 런타임 없는 기본 실행

포함하지 않는다.

- 다중 사용자 협업
- 실시간 공동 편집
- iOS/Android 앱
- OAuth/OIDC/SAML/SCIM
- SaaS 멀티테넌트
- 외부 SaaS connector
- AI 답변 생성
- 플러그인 런타임
- CRM 객체
- 사용자가 DB, Git CLI, 검색 엔진, Node.js, 별도 서버를 직접 설치해야 하는 로컬 실행 방식

### 아키텍처 산출물

도메인 계층:

- `Workspace`
- `Document`
- `DocumentId`
- `DocumentPath`
- `DocumentMetadata`
- `DocumentBody`
- `DocumentLink`
- `Backlink`
- `Asset`
- `AssetId`
- `AssetReference`
- `VersionEntry`
- `SearchDocument`

유스케이스 계층:

- `CreateWorkspace`
- `CreateDocument`
- `UpdateDocument`
- `DeriveDocumentTitleFromBody`
- `UpdateDocument`와 `RestoreDocumentVersion`의 title metadata 동기화
- `DeleteDocument`
- `GetDocument`
- `GetCurrentDocument`
- `GetDocumentVersion`
- `ListDocuments`
- `ParseDocumentLinks`
- `GetBacklinks`
- `GetUnresolvedLinks`
- `GetOrphanDocuments`
- `AttachFileToDocument`
- `ListDocumentAssets`
- `SearchDocuments`
- `GetDocumentHistory`
- `CompareDocumentVersions`
- `PreviewDocumentRestore`
- `RestoreDocumentVersion`
- `ImportMarkdownFolder`
- `ExportDocument`

포트:

- `DocumentRepository`
- `VersionStore`
- `AssetStore`
- `SearchIndex`
- `MarkdownParser`
- `Clock`
- `IdGenerator`
- `ProductLogger`
- `DevelopmentLogger`

인프라 어댑터:

- 로컬 파일 기반 document store
- 내부 Git 기반 version store
- 로컬 디스크 asset store
- 로컬 search index
- Markdown parser adapter
- export adapter
- Web local UI adapter
- Windows desktop shell adapter contract. Native 인증은 차후 수행한다.
- macOS desktop shell adapter
- Linux desktop shell adapter contract. Native 인증은 차후 수행한다.
- platform path resolver
- platform secure storage abstraction
- first-run initializer
- local setup health checker
- local migration runner

### 상태머신

문서 생명주기:

```text
Draft -> Saved -> Archived
Saved -> Editing -> Saved
Saved -> Deleted
Archived -> Restored
Deleted -> Restored
```

첨부 파일 생명주기:

```text
Registered -> Linked -> Unlinked
Linked -> Archived
Archived -> Restored
```

버전 복원 흐름:

```text
RestoreRequested -> VersionLoaded -> RestoreApplied -> RestoreCompleted
RestoreRequested -> VersionMissing -> RestoreFailed
RestoreApplied -> PersistFailed -> RestoreFailed
```

### 테스트 전략

도메인 테스트:

- 문서 ID와 slug 규칙을 검증한다.
- Markdown link와 Wikilink를 파싱한다.
- 중복 링크를 정규화한다.
- 미해결 링크와 고아 문서 규칙을 검증한다.
- 첨부 참조가 문서 본문과 분리되어 유지되는지 검증한다.

유스케이스 테스트:

- 문서 생성 후 검색 대상에 포함되는지 검증한다.
- 문서 수정 후 version entry가 생성되는지 검증한다.
- 현재 문서 조회가 최신 snapshot을 반환하는지 검증한다.
- 이력 기준 조회가 특정 version entry를 반환하는지 검증한다.
- 현재 문서 조회가 version history 전체 스캔에 의존하지 않는지 검증한다.
- 문서 삭제 후 링크 인덱스가 갱신되는지 검증한다.
- 복원 실패 시 원본 문서가 훼손되지 않는지 검증한다.
- 첨부 등록 실패 시 문서 참조가 생성되지 않는지 검증한다.

인프라 테스트:

- 로컬 파일 저장소가 문서 원문과 metadata를 보존하는지 검증한다.
- 내부 Git 기반 version store가 사용자의 Git 지식 없이 history를 제공하는지 검증한다.
- search index가 제목, 본문, 태그를 검색하는지 검증한다.
- macOS 앱 데이터 경로와 파일 경로 adapter를 검증한다. Windows/Linux native path 검증은 해당 플랫폼 인증을 활성화할 때 수행한다.
- Web 로컬 UI가 공통 유스케이스를 직접 호출하지 않고 adapter를 통해 호출하는지 검증한다.
- 깨끗한 OS 사용자 프로필에서 앱 최초 실행만으로 기본 workspace 생성까지 완료되는지 검증한다.
- Git CLI, 외부 DB, 외부 검색 서버, Node.js가 없어도 로컬 MVP가 실행되는지 검증한다.
- 앱 업그레이드 후 기존 local workspace와 내부 store가 보존되는지 검증한다.
- local setup failure가 안전한 error code와 복구 가능한 사용자 메시지를 반환하는지 검증한다.
- 문서 현재 조회, 이력 목록 조회, 특정 버전 조회, 링크/백링크 조회, 첨부 metadata 조회, 기본 검색이 p95 300ms 목표를 만족하는지 측정한다.
- 인덱스가 정상 상태일 때 검색이 본문 파일 전체 스캔에 의존하지 않는지 검증한다.

로그 테스트:

- 문서 생성, 수정, 삭제, 복원은 Product Log 이벤트를 남긴다.
- 문서 본문 원문은 Product Log에 남지 않는다.
- Development Log는 테스트/로컬 모드에서만 활성화된다.

### 완료 조건

- 신규 생성, 현재 문서 저장과 버전 복원이 모두 첫 줄 제목 규칙을 사용한다.
- 제목을 바꾼 뒤 durable readback과 projection 처리가 완료되면 Home, Navigator, Search, Graph, Canvas와 Asset 연결 문서 표시에 이전 제목이나 raw ID가 남지 않는다.
- 별도 문서 제목 입력 control과 create command의 독립 title 필드가 존재하지 않는다.
- 단일 사용자가 로컬에서 문서를 생성, 수정, 삭제, 검색할 수 있다.
- macOS 데스크톱 앱과 개발용 Web preview가 같은 core/client contract를 사용한다. Windows/Linux 인증은 `deferred_future`로 유지한다.
- 앱 설치 1회 후 추가 수동 설정 없이 기본 workspace를 만들고 사용할 수 있다.
- 외부 DB, 검색 서버, Git CLI, Node.js 없이 로컬 MVP가 동작한다.
- 최초 실행 초기화와 업그레이드 migration이 테스트된다.
- 현재 문서 조회와 이력 조회가 UI/API에서 분리된다.
- 현재 문서 조회, 이력 목록 조회, 특정 버전 조회, 링크/백링크 조회, 첨부 metadata 조회, 기본 검색의 p95 300ms 목표가 측정된다.
- 문서 간 링크와 백링크가 동작한다.
- 내부 Git 기반 변경 이력이 UI/API에서는 일반 history로 보인다.
- 특정 버전 복원이 동작한다.
- 첨부 파일이 문서 본문과 분리되어 관리된다.
- 설정은 시작 시 1회만 읽힌다.
- 핵심 도메인/유스케이스 테스트가 존재한다.
- Product Log에 민감 정보가 남지 않는다.
- Graph가 실제 durable projection의 node와 edge를 표시한다.
- Canvas node, edge, geometry와 viewport가 앱 재시작 후 유지된다.
- Asset metadata/object와 document association이 앱 재시작 후 유지된다.
- backup/restore가 문서, Canvas와 Asset을 함께 보존한다.

## 2단계: 개인 호스팅과 팀 협업 기반

### 단계 목표

1단계의 로컬 개인 지식베이스를 개인 호스팅 가능한 서버 제품으로 확장한다. 이 단계부터 다중 사용자 문서 협업 기능을 제공한다. 실시간 공동 편집은 아직 전체 범위로 확장하지 않고, 우선 사용자, 그룹, 권한, 댓글, 리뷰 요청, 승인, 감사 로그, 서버 API를 구축한다.

이 단계의 핵심은 "개인 로컬 앱"을 "작은 팀이 자체 서버에서 신뢰하고 쓸 수 있는 지식베이스"로 바꾸는 것이다.

### 선행 의존성

- 1단계의 문서 모델이 안정화되어 있어야 한다.
- 내부 version store가 다중 요청 환경에서도 안전하게 동작해야 한다.
- 링크/백링크 인덱스가 문서 변경 이벤트로 갱신될 수 있어야 한다.
- 첨부 파일이 문서와 분리된 asset store로 관리되어야 한다.

### 사용자 가치

- 소규모 팀은 자체 서버에 Sponzey Cabinet을 설치해 문서를 공동 관리한다.
- 사용자는 문서를 공유하고 댓글을 남기고 리뷰를 요청한다.
- 관리자는 사용자, 그룹, 역할을 관리한다.
- 조직은 문서 접근과 변경 이력을 감사할 수 있다.

### 제품 범위

포함한다.

- 서버 API
- Web self-host UI
- Windows/macOS/Linux 데스크톱 클라이언트의 원격 workspace 접속
- iOS/Android 클라이언트를 위한 read API 계약
- 단일 tenant self-host 모드
- 사용자 계정
- 그룹
- 기본 RBAC
- workspace role: owner, admin, editor, reviewer, viewer
- 문서 권한: read, write, review, publish, manage
- 폴더/컬렉션 단위 권한
- 문서 댓글
- 인라인 댓글의 최소 모델
- 문서 리뷰 요청
- 문서 승인/반려
- 문서 publish workflow
- 문서 잠금
- 변경 알림의 내부 이벤트 모델
- audit log
- Product Log 확장
- Field Debug Log 활성화/만료 모델
- asset store 추상화
- local disk, S3-compatible storage adapter 기반
- backup/export job
- server health check

포함하지 않는다.

- 완전한 실시간 공동 편집
- iOS/Android 네이티브 앱의 완성
- Canvas/Edgeless
- AI 검색/답변
- 플러그인 런타임
- SaaS 멀티테넌트
- SAML/SCIM
- 외부 CRM 연동

### 아키텍처 산출물

도메인 계층:

- `User`
- `Group`
- `Role`
- `Permission`
- `WorkspacePolicy`
- `DocumentPolicy`
- `Comment`
- `ReviewRequest`
- `AuditEvent`
- `PublishWorkflow`

유스케이스 계층:

- `CreateUser`
- `CreateGroup`
- `AssignRole`
- `CheckPermission`
- `ShareDocument`
- `AddComment`
- `ResolveComment`
- `RequestDocumentReview`
- `ApproveDocument`
- `RejectDocument`
- `PublishDocument`
- `LockDocument`
- `UnlockDocument`
- `ListAuditEvents`
- `CreateBackup`
- `RestoreBackup`

포트:

- `UserRepository`
- `GroupRepository`
- `PermissionPolicyRepository`
- `AuditLogStore`
- `NotificationPort`
- `ObjectStorage`
- `BackupStore`
- `SessionStore`

어댑터:

- HTTP API adapter
- Web client adapter
- desktop remote workspace adapter
- mobile API contract adapter
- local auth adapter
- object storage adapter
- audit log persistence adapter
- notification stub adapter

### 상태머신

문서 publish workflow:

```text
Editing -> ReviewRequested
ReviewRequested -> ChangesRequested
ChangesRequested -> Editing
ReviewRequested -> Approved
Approved -> Published
ReviewRequested -> Rejected
Published -> Editing
```

문서 잠금:

```text
Unlocked -> LockRequested -> Locked
Locked -> UnlockRequested -> Unlocked
Locked -> LockExpired -> Unlocked
```

백업 작업:

```text
Queued -> Running -> Completed
Queued -> Running -> Failed
Failed -> Retrying -> Running
Failed -> Abandoned
```

### 테스트 전략

도메인 테스트:

- role과 permission matrix를 검증한다.
- reviewer 권한 없는 사용자의 승인을 거부한다.
- published 문서 수정 시 editing 상태로 전환되는지 검증한다.
- 잠금 만료 규칙을 검증한다.

유스케이스 테스트:

- 권한 없는 문서 조회/수정/승인을 거부한다.
- 승인 후 publish가 가능해지는지 검증한다.
- 댓글 생성과 해결 상태 전이를 검증한다.
- backup 실패 시 감사 로그와 Product Log가 남는지 검증한다.

인프라 테스트:

- object storage adapter가 파일 metadata와 content-addressed ID를 보존한다.
- 서버 API가 유스케이스 DTO로 request를 변환한다.
- audit log store가 검색 가능한 형태로 이벤트를 저장한다.
- Web self-host UI가 서버 API를 통해서만 유스케이스에 접근하는지 검증한다.
- 데스크톱 클라이언트가 로컬 workspace와 원격 workspace를 명확히 구분하는지 검증한다.
- iOS/Android용 read API 계약이 문서, 첨부, 권한 응답을 안정적으로 제공하는지 검증한다.

로그 테스트:

- 권한 거부는 민감 정보 없이 Product Log에 남는다.
- Field Debug Log 활성화는 관리자 승인, 범위, 만료 시간을 가진다.
- 댓글 본문과 문서 본문 원문은 운영 로그에 남지 않는다.

### 완료 조건

- 단일 서버에서 다중 사용자가 문서를 관리할 수 있다.
- Web self-host UI가 기본 관리/협업 기능을 제공한다.
- Windows/macOS/Linux 데스크톱 클라이언트가 원격 workspace에 접속할 수 있다.
- RBAC가 문서, 폴더/컬렉션, 첨부 파일에 적용된다.
- 권한 필터링이 적용된 문서 조회와 검색이 p95 300ms 목표를 기준으로 측정된다.
- 리뷰/승인/publish workflow가 동작한다.
- audit log와 Product Log가 구분되어 동작한다.
- 첨부 파일 저장소를 local disk와 S3-compatible backend로 교체할 수 있다.
- 설정과 runtime environment 처리가 `AGENTS.md` 기준을 만족한다.

## 3단계: 지식 그래프와 협업 UX 확장

### 단계 목표

2단계의 팀 협업 기반 위에 Sponzey Cabinet의 차별화 요소인 지식 그래프, 로컬/전역 그래프, 관계 기반 탐색, Canvas/Edgeless 기본 기능, 실시간 공동 편집을 추가한다.

이 단계는 문서 저장소를 "팀 위키"에서 "관계형 지식 공간"으로 확장한다. 그래프와 협업 기능은 이후 AI retrieval과 CRM relation의 기반이 된다.

### 선행 의존성

- 다중 사용자와 RBAC가 안정적으로 동작해야 한다.
- 문서 링크/백링크 인덱스가 이벤트 기반으로 갱신되어야 한다.
- 문서 변경 이력과 잠금/승인 workflow가 존재해야 한다.
- Product Log와 Field Debug Log가 상태 전이를 기록할 수 있어야 한다.

### 사용자 가치

- 사용자는 문서 간 관계를 그래프로 이해한다.
- 팀은 문서를 동시에 편집하고 변경 충돌을 줄인다.
- 사용자는 문서, 첨부, 외부 링크를 Canvas에 배치해 지식 맵을 만든다.
- 관리자는 그래프와 협업 이벤트를 감사할 수 있다.

### 제품 범위

포함한다.

- 전역 지식 그래프
- 문서 기준 로컬 그래프
- depth 기반 주변 문서 탐색
- 태그/상태/소유자/권한 기반 그래프 필터
- 미해결 링크와 고아 문서 시각화
- 허브 문서 탐지
- 관련 문서 추천의 rule-based 기본 버전
- 실시간 공동 편집 기반
- presence 표시
- 충돌 감지와 병합 UI
- 섹션 단위 잠금 또는 편집 충돌 완화
- Canvas/Edgeless 기본 모델
- 문서 카드, 첨부 카드, 외부 링크 카드
- 카드 간 directed edge
- Canvas를 문서에 임베드
- 문서 헤딩 구조를 Canvas 노드로 변환하는 기본 기능
- iOS/Android 기본 클라이언트
- 모바일 문서 조회, 검색, 댓글, 승인/반려
- 모바일 첨부 파일 preview
- 모바일 push 알림 adapter의 기본 경계
- 플랫폼별 그래프 표시 capability 구분

포함하지 않는다.

- AI 기반 그래프 요약
- AI 기반 누락 링크 추천
- CRM 객체 그래프
- 플러그인 기반 custom object
- SaaS 멀티테넌트
- 모바일 오프라인 편집
- 모바일 Canvas 전체 편집

### 아키텍처 산출물

도메인 계층:

- `KnowledgeGraph`
- `GraphNode`
- `GraphEdge`
- `GraphFilter`
- `GraphProjection`
- `Canvas`
- `CanvasNode`
- `CanvasEdge`
- `Presence`
- `EditSession`
- `Conflict`

유스케이스 계층:

- `BuildKnowledgeGraph`
- `GetLocalGraph`
- `GetGraphNeighbors`
- `FilterGraph`
- `DetectOrphanDocuments`
- `DetectHubDocuments`
- `CreateCanvas`
- `AddCanvasNode`
- `ConnectCanvasNodes`
- `EmbedCanvasInDocument`
- `ConvertDocumentOutlineToCanvas`
- `StartEditSession`
- `ApplyCollaborativeEdit`
- `ResolveEditConflict`
- `UpdatePresence`

포트:

- `GraphIndex`
- `CollaborationSessionStore`
- `RealtimeTransport`
- `ConflictResolver`
- `CanvasRepository`
- `MobileNotificationPort`
- `PlatformCapabilityProvider`

### 상태머신

협업 편집 세션:

```text
Idle -> SessionStarted
SessionStarted -> Editing
Editing -> Syncing
Syncing -> Synced
Syncing -> ConflictDetected
ConflictDetected -> Resolving
Resolving -> Synced
Resolving -> Failed
SessionStarted -> SessionEnded
```

Canvas 변경:

```text
Draft -> Saved
Saved -> Embedded
Embedded -> Updated
Updated -> Saved
Saved -> Archived
```

그래프 인덱스:

```text
Clean -> ReindexRequested
ReindexRequested -> Reindexing
Reindexing -> Clean
Reindexing -> Degraded
Degraded -> ReindexRequested
```

### 테스트 전략

도메인 테스트:

- 문서 링크가 graph edge로 변환되는지 검증한다.
- 권한 없는 문서는 그래프 결과에서 제외되는지 검증한다.
- Canvas edge가 존재하지 않는 node를 참조하지 못하게 한다.
- 충돌 감지 규칙을 테스트한다.

유스케이스 테스트:

- 로컬 그래프 depth 결과가 올바른지 검증한다.
- 필터 조건에 따라 graph projection이 변하는지 검증한다.
- 동시에 발생한 편집 이벤트가 충돌 상태로 전이되는지 검증한다.
- 충돌 해결 후 문서 version entry가 생성되는지 검증한다.

인프라 테스트:

- realtime transport adapter가 끊김/재연결을 처리한다.
- graph index adapter가 대량 문서를 incremental update한다.
- Canvas repository가 노드와 edge를 일관되게 저장한다.
- iOS/Android 클라이언트가 권한 없는 문서와 그래프 노드를 표시하지 않는지 검증한다.
- 모바일 알림 adapter가 민감 정보를 payload에 포함하지 않는지 검증한다.
- 플랫폼별 graph/canvas capability matrix가 UI에 반영되는지 검증한다.

로그 테스트:

- 그래프 재색인 실패는 Product Log에 남는다.
- 협업 충돌 상세 내용은 Field Debug Log로 제한한다.
- 문서 본문 원문은 협업 로그에 남지 않는다.

### 완료 조건

- 사용자가 전역 그래프와 로컬 그래프를 탐색할 수 있다.
- 권한이 없는 문서는 그래프와 검색 결과에 노출되지 않는다.
- 그래프/로컬 그래프/백링크 projection 조회가 p95 300ms 목표를 기준으로 측정된다.
- 실시간 공동 편집의 기본 흐름이 동작한다.
- 충돌 감지와 해결 흐름이 테스트된다.
- Canvas에 문서/첨부/외부 링크를 배치하고 문서에 임베드할 수 있다.
- iOS/Android에서 문서 조회, 검색, 댓글, 승인/반려가 가능하다.
- Web, 데스크톱, 모바일 클라이언트가 동일 권한 모델을 따른다.
- 그래프 인덱스와 협업 세션 상태머신이 테스트된다.

## 4단계: AI와 외부 연동 플랫폼

### 단계 목표

3단계까지 구축한 문서, 권한, 그래프, 검색 기반 위에 AI와 외부 연동 플랫폼을 추가한다. 이 단계의 핵심은 "권한을 지키는 AI 검색/요약"과 "외부 시스템이 안전하게 Sponzey Cabinet에 연결되는 기반"이다.

AI는 문서 작성 보조가 아니라 지식 운영 기능으로 제공한다. 모든 AI 결과는 출처, 권한, 최신성, 인용 범위를 가져야 한다.

### 선행 의존성

- RBAC와 permission decision이 안정적으로 동작해야 한다.
- 문서/첨부/그래프 인덱스가 준비되어야 한다.
- Product Log와 Field Debug Log 기준이 적용되어야 한다.
- 설정 값은 provider별 config object로 명시적으로 주입되어야 한다.

### 사용자 가치

- 사용자는 권한 범위 안에서 문서에 질문하고 답변을 받는다.
- 사용자는 AI 답변의 출처 문서를 확인한다.
- 팀은 Slack, Teams, Jira 등 업무 도구에서 지식베이스와 연결한다.
- AI agent는 MCP/API를 통해 안전하게 문서와 첨부를 검색한다.

### 제품 범위

포함한다.

- 전체 텍스트 검색 고도화
- semantic search
- vector index abstraction
- exact keyword와 semantic search 결과 병합
- permission-aware retrieval
- AI answer generation
- citation 생성
- 문서 요약
- 섹션 요약
- 변경 요약
- 오래된 문서 감지의 기본 규칙
- 중복 문서 감지의 기본 규칙
- 누락 링크 추천의 기본 규칙
- MCP server
- REST API 정리
- webhook
- event stream
- OAuth 2.0 app authorization 기본
- API token
- service account
- Slack 알림/검색 기본 연동
- Microsoft Teams 알림/검색 기본 연동
- Jira deep link와 이슈 참조 기본 연동
- Zapier/Make/n8n용 webhook 기반 연동
- Web, iOS, Android, Windows, macOS, Linux에서 AI 질의와 출처 확인
- 플랫폼별 AI provider secret 저장 방식 분리
- 모바일과 데스크톱의 네트워크 단절/재시도 처리

포함하지 않는다.

- 플러그인 marketplace
- CRM custom object
- SaaS 과금
- SAML/SCIM
- 고급 data residency

### 아키텍처 산출물

도메인 계층:

- `SearchQuery`
- `SearchResult`
- `RetrievalScope`
- `Citation`
- `Answer`
- `Connector`
- `WebhookSubscription`
- `ExternalAppAuthorization`
- `ApiCredential`

유스케이스 계층:

- `SearchKnowledgeBase`
- `BuildRetrievalContext`
- `GenerateAnswer`
- `SummarizeDocument`
- `SummarizeChanges`
- `DetectStaleDocuments`
- `DetectDuplicateDocuments`
- `RecommendMissingLinks`
- `CreateApiToken`
- `AuthorizeExternalApp`
- `RegisterWebhook`
- `PublishEvent`
- `HandleConnectorEvent`
- `McpSearch`
- `McpReadDocument`

포트:

- `VectorIndex`
- `EmbeddingProvider`
- `LLMProvider`
- `McpTransport`
- `WebhookDispatcher`
- `EventStream`
- `ConnectorGateway`
- `SecretStore`
- `PlatformNetworkMonitor`
- `PlatformSecureSecretStore`

### 상태머신

AI answer generation:

```text
Requested -> PermissionChecked
PermissionChecked -> RetrievalStarted
RetrievalStarted -> ContextBuilt
ContextBuilt -> AnswerGenerating
AnswerGenerating -> AnswerReady
AnswerGenerating -> AnswerFailed
RetrievalStarted -> NoAccessibleContext
```

Webhook delivery:

```text
Queued -> Delivering
Delivering -> Delivered
Delivering -> RetryScheduled
RetryScheduled -> Delivering
RetryScheduled -> DeadLettered
```

External app authorization:

```text
Requested -> UserApproved
Requested -> UserDenied
UserApproved -> Active
Active -> Revoked
Active -> Expired
```

### 테스트 전략

도메인 테스트:

- retrieval scope가 권한 범위를 벗어나지 않는지 검증한다.
- citation이 접근 가능한 문서만 참조하는지 검증한다.
- webhook retry 정책을 검증한다.
- API credential scope 규칙을 검증한다.

유스케이스 테스트:

- 권한 없는 문서는 AI context에 포함되지 않는지 검증한다.
- AI provider 실패 시 안정적인 error code가 반환되는지 검증한다.
- 답변 생성 시 citation이 필수로 생성되는지 검증한다.
- webhook 실패가 dead-letter 상태로 전이되는지 검증한다.
- 외부 앱 권한 취소 후 API 접근이 거부되는지 검증한다.

인프라 테스트:

- vector index adapter가 incremental update를 처리한다.
- LLM provider adapter가 timeout과 retry를 처리한다.
- MCP endpoint가 search/read scope를 지킨다.
- connector adapter가 외부 payload를 내부 DTO로 변환한다.
- iOS/Android/desktop secure secret store가 token을 평문 로그나 파일에 남기지 않는지 검증한다.
- 네트워크 단절 시 AI 질의와 connector 호출이 안정적인 실패 상태로 전이되는지 검증한다.

로그 테스트:

- AI prompt 원문은 Product Log에 남지 않는다.
- Field Debug Log는 query hash, retrieval count, citation count만 기록한다.
- webhook 실패는 event id와 error code 중심으로 기록한다.

### 완료 조건

- 사용자가 문서 검색 질의에 대한 AI 답변을 받을 수 있다.
- AI 답변은 출처와 인용 범위를 포함한다.
- 권한 없는 문서는 검색, AI, MCP 결과에 포함되지 않는다.
- AI 답변 생성은 비동기/streaming 대상이지만, retrieval result 조회와 citation metadata 조회는 p95 300ms 목표를 기준으로 측정된다.
- MCP server가 기본 search/read 기능을 제공한다.
- webhook과 event stream이 동작한다.
- Slack/Teams/Jira 기본 연동이 동작한다.
- 모든 공식 대상 플랫폼에서 AI 답변의 출처 문서를 확인할 수 있다.
- connector activity가 감사 가능하다.

## 5단계: 플러그인과 업무 객체 플랫폼

### 단계 목표

4단계의 API, event, permission, AI 기반 위에 플러그인 플랫폼을 구축한다. 이 단계부터 Sponzey Cabinet은 문서 위키를 넘어 CRM, 고객지원, 프로젝트, 결제, 개발 문서 같은 업무 기능을 문서와 그래프에 연결하는 확장형 지식 운영 플랫폼이 된다.

핵심은 core domain을 오염시키지 않고 업무 기능을 확장하는 것이다. 플러그인은 custom object, custom field, workflow hook, UI extension, AI tool extension으로 동작해야 한다.

### 선행 의존성

- API, webhook, event stream이 안정적으로 동작해야 한다.
- RBAC와 permission scope가 외부 앱에도 적용되어야 한다.
- 그래프 모델이 문서 외 객체를 node로 수용할 수 있어야 한다.
- AI retrieval이 custom object를 권한 기반으로 처리할 수 있어야 한다.

### 사용자 가치

- 사용자는 문서에서 고객, 딜, 티켓, 프로젝트, 계약 같은 업무 객체를 직접 연결한다.
- 조직은 CRM과 지식베이스를 분리하지 않고 같은 지식 그래프에서 관리한다.
- 개발자는 core 수정 없이 플러그인으로 기능을 확장한다.
- SaaS와 self-host 모두 같은 플러그인 모델을 사용할 수 있다.

### 제품 범위

포함한다.

- 플러그인 manifest
- 플러그인 install/update/disable/remove lifecycle
- plugin permission scope
- UI extension point
- document block extension
- sidebar/action/menu extension
- custom object type
- custom field/schema
- workflow hook
- search index extension
- AI tool/function extension
- background job
- plugin migration
- plugin event subscription
- plugin audit event
- plugin configuration object
- CRM 기본 플러그인
- 고객 계정
- 담당자
- 리드
- 딜
- 계약
- 미팅 노트
- 고객 문서 허브
- 고객지원 기본 플러그인
- FAQ
- ticket deflection 기반
- 프로젝트/이슈 기본 플러그인
- task
- milestone
- roadmap
- release note
- 결제 플러그인 기본 abstraction
- Web 플러그인 관리 UI
- 데스크톱 플러그인 렌더링 정책
- 모바일 플러그인 렌더링 제한 정책
- 플랫폼 capability 기반 extension point 노출

포함하지 않는다.

- 공개 marketplace 전체 기능
- 서드파티 결제 정산 자동화 전체
- 엔터프라이즈 SAML/SCIM 전체
- 멀티리전 SaaS 운영

### 아키텍처 산출물

도메인 계층:

- `Plugin`
- `PluginManifest`
- `PluginScope`
- `ExtensionPoint`
- `CustomObjectType`
- `CustomObject`
- `CustomField`
- `WorkflowHook`
- `PluginEvent`
- `CustomerAccount`
- `Contact`
- `Lead`
- `Deal`
- `Contract`
- `MeetingNote`
- `Ticket`
- `ProjectTask`

유스케이스 계층:

- `InstallPlugin`
- `DisablePlugin`
- `UpdatePlugin`
- `RemovePlugin`
- `ValidatePluginManifest`
- `RegisterExtensionPoint`
- `CreateCustomObject`
- `UpdateCustomObject`
- `LinkObjectToDocument`
- `IndexCustomObject`
- `RunWorkflowHook`
- `RunPluginBackgroundJob`
- `CreateCustomerAccount`
- `CreateDeal`
- `LinkMeetingNoteToCustomer`
- `CreateTicketKnowledgeLink`

포트:

- `PluginRegistry`
- `PluginPackageStore`
- `PluginRuntime`
- `PluginConfigStore`
- `PluginPermissionChecker`
- `CustomObjectRepository`
- `WorkflowHookRunner`
- `BackgroundJobRunner`
- `PluginUiRenderer`
- `PlatformExtensionPolicy`

### 상태머신

플러그인 설치:

```text
Uploaded -> ManifestValidated
ManifestValidated -> PermissionReviewed
PermissionReviewed -> Installed
Installed -> Enabled
Enabled -> Disabled
Disabled -> Enabled
Installed -> UpdateRequested
UpdateRequested -> Updated
Installed -> RemoveRequested
RemoveRequested -> Removed
ManifestValidated -> InstallFailed
```

CRM deal:

```text
Lead -> Qualified
Qualified -> Proposal
Proposal -> Negotiation
Negotiation -> Won
Negotiation -> Lost
Won -> Archived
Lost -> Archived
```

고객지원 지식 연결:

```text
TicketReceived -> KnowledgeSuggested
KnowledgeSuggested -> AnswerSent
AnswerSent -> Resolved
AnswerSent -> Reopened
KnowledgeSuggested -> Escalated
```

### 테스트 전략

도메인 테스트:

- plugin manifest validation 규칙을 검증한다.
- plugin scope가 허용된 extension point만 사용할 수 있는지 검증한다.
- custom object relation이 권한 규칙을 위반하지 않는지 검증한다.
- CRM deal 상태 전이를 검증한다.

유스케이스 테스트:

- plugin install 실패 시 부분 설치가 남지 않는지 검증한다.
- plugin disable 후 extension point가 비활성화되는지 검증한다.
- custom object가 graph node로 색인되는지 검증한다.
- 고객 문서 허브가 고객 계정과 관련 문서를 모아 보여주는지 검증한다.
- workflow hook 실패가 유스케이스를 오염시키지 않는지 검증한다.

인프라 테스트:

- plugin package store가 패키지 무결성을 검증한다.
- background job runner가 실패와 재시도를 처리한다.
- plugin runtime이 core domain에 직접 접근하지 못하게 한다.
- Web, desktop, mobile별 허용 extension point가 policy대로 제한되는지 검증한다.
- 모바일에서 허용되지 않은 plugin UI extension이 로드되지 않는지 검증한다.

로그 테스트:

- plugin install/update/remove는 Product Log에 남는다.
- plugin 내부 Development Log는 프로덕션 기본 경로로 노출되지 않는다.
- 고객/계약/티켓 원문 내용은 Product Log에 남지 않는다.

### 완료 조건

- 플러그인을 설치, 비활성화, 업데이트, 제거할 수 있다.
- custom object와 custom field를 정의할 수 있다.
- 문서와 업무 객체를 양방향으로 연결할 수 있다.
- CRM 기본 플러그인이 고객, 담당자, 딜, 미팅 노트를 제공한다.
- 고객지원과 프로젝트 기본 플러그인이 최소 기능을 제공한다.
- 플러그인 권한과 감사 로그가 동작한다.
- Web, desktop, mobile에서 플러그인 기능 노출 범위가 명확히 분리된다.
- core domain이 특정 업무 플러그인에 의존하지 않는다.

## 6단계: SaaS와 엔터프라이즈 운영

### 단계 목표

5단계까지의 self-host 제품을 멀티테넌트 SaaS와 엔터프라이즈 운영 제품으로 확장한다. 이 단계는 제품 운영, 보안, 과금, 조직 관리, 엔터프라이즈 인증, 보존 정책, 운영 가시성, 마켓플레이스 배포를 완성한다.

이 단계에서 Sponzey Cabinet은 개인 구축, 개인 호스팅, SaaS를 모두 지원하는 제품이 된다.

### 선행 의존성

- core domain이 tenant와 workspace 경계를 수용할 수 있어야 한다.
- RBAC와 audit log가 안정적으로 동작해야 한다.
- 플러그인 permission scope가 검증되어야 한다.
- Product Log와 Field Debug Log가 운영 환경에 맞게 분리되어야 한다.
- 설정과 secret 처리가 중앙화되지 않고 명시적 주입 구조를 유지해야 한다.

### 사용자 가치

- 조직은 SaaS로 즉시 도입할 수 있다.
- 엔터프라이즈 고객은 SSO, SCIM, 감사, 보존 정책을 사용할 수 있다.
- 관리자는 조직, 워크스페이스, 사용자, 플러그인, AI 사용량을 통제한다.
- 운영팀은 장애, 비용, 사용량, 보안 이벤트를 관측한다.

### 제품 범위

포함한다.

- 멀티테넌트 모델
- 조직
- 워크스페이스
- workspace isolation
- tenant-aware RBAC
- 관리자 콘솔
- 사용자 초대
- guest access
- SAML SSO
- OIDC 고도화
- SCIM provisioning
- MFA 정책
- API token governance
- service account governance
- IP allowlist
- audit log retention
- data retention policy
- data deletion policy
- data export policy
- AI usage governance
- connector catalog
- plugin catalog
- plugin marketplace 기본
- 구독/과금
- invoice
- usage metering
- plan limit
- SLA 관측 지표
- 백업/복원 운영 절차
- 장애 대응용 Product Log dashboard
- Field Debug Log 운영 승인/만료 UI
- 보안 리포트
- Web 공식 릴리즈 채널
- iOS 공식 릴리즈 채널
- Android 공식 릴리즈 채널
- Windows 공식 릴리즈 채널
- macOS 공식 릴리즈 채널
- Linux 공식 릴리즈 채널
- 플랫폼별 crash/error report 수집 정책
- 플랫폼별 업데이트 정책
- 플랫폼별 E2E smoke test

포함하지 않는다.

- 모든 국가/지역별 data residency 조합
- 모든 외부 결제 provider 전체 기능
- 모든 엔터프라이즈 connector 전체 구현
- 무제한 marketplace 정책 자동화

### 아키텍처 산출물

도메인 계층:

- `Organization`
- `Tenant`
- `Workspace`
- `Subscription`
- `Plan`
- `UsageMeter`
- `Invoice`
- `EnterprisePolicy`
- `RetentionPolicy`
- `DataExportRequest`
- `SecurityAuditReport`

유스케이스 계층:

- `CreateOrganization`
- `CreateWorkspace`
- `InviteUser`
- `ConfigureSso`
- `ProvisionUserFromScim`
- `CreateSubscription`
- `RecordUsage`
- `GenerateInvoice`
- `EnforcePlanLimit`
- `ConfigureRetentionPolicy`
- `ExportTenantData`
- `DeleteTenantData`
- `GenerateSecurityReport`
- `ApproveFieldDebugSession`
- `ExpireFieldDebugSession`
- `PublishPluginToCatalog`

포트:

- `TenantRepository`
- `BillingGateway`
- `UsageStore`
- `SsoProvider`
- `ScimProvider`
- `PolicyStore`
- `AuditReportStore`
- `DataExportStore`
- `MonitoringSink`
- `MarketplaceRegistry`
- `ReleaseChannel`
- `PlatformTelemetrySink`
- `CrashReportSink`

### 상태머신

구독:

```text
Trial -> Active
Trial -> Expired
Active -> PastDue
PastDue -> Active
PastDue -> Suspended
Suspended -> Reactivated
Suspended -> Cancelled
Active -> Cancelled
```

데이터 내보내기:

```text
Requested -> Authorized
Authorized -> Preparing
Preparing -> Ready
Preparing -> Failed
Ready -> Downloaded
Ready -> Expired
```

Field Debug session:

```text
Requested -> Approved
Requested -> Denied
Approved -> Active
Active -> Expired
Active -> Revoked
Expired -> Archived
```

플러그인 catalog:

```text
Submitted -> Validating
Validating -> Approved
Validating -> Rejected
Approved -> Published
Published -> Deprecated
Deprecated -> Removed
```

### 테스트 전략

도메인 테스트:

- tenant isolation 규칙을 검증한다.
- plan limit 계산을 검증한다.
- subscription 상태 전이를 검증한다.
- retention policy와 deletion policy 충돌을 검증한다.
- Field Debug session 만료 규칙을 검증한다.

유스케이스 테스트:

- 다른 tenant의 문서에 접근할 수 없는지 검증한다.
- SCIM으로 생성/비활성화된 사용자의 권한이 갱신되는지 검증한다.
- SSO 설정 실패 시 기존 로그인 경로가 안전하게 유지되는지 검증한다.
- 사용량 초과 시 plan limit이 적용되는지 검증한다.
- data export가 권한과 감사 로그를 남기는지 검증한다.

인프라 테스트:

- billing gateway adapter의 성공/실패/재시도 계약을 검증한다.
- SSO/SCIM adapter의 외부 응답 변환을 검증한다.
- monitoring sink가 민감 정보 없이 event를 기록하는지 검증한다.
- marketplace registry가 plugin manifest와 scope를 검증하는지 검증한다.
- 각 공식 플랫폼의 smoke test가 로그인, 문서 조회, 검색, 댓글, AI 질의의 최소 흐름을 검증한다.
- 플랫폼별 crash/error report가 문서 원문과 민감 정보를 포함하지 않는지 검증한다.

로그 테스트:

- 구독 변경과 보안 이벤트는 Product Log에 남는다.
- Field Debug session 활성화/만료/취소는 감사 가능해야 한다.
- tenant id와 user id는 마스킹 정책을 따른다.
- 결제 정보 원문은 로그에 남지 않는다.

### 완료 조건

- SaaS 멀티테넌트 환경에서 조직과 워크스페이스가 격리된다.
- 구독, 과금, 사용량 제한이 동작한다.
- SAML/OIDC/SCIM 기반 엔터프라이즈 인증과 프로비저닝이 동작한다.
- 관리자 콘솔에서 사용자, 워크스페이스, 플러그인, AI 사용량, 보안 정책을 관리할 수 있다.
- 데이터 export/delete/retention 정책이 동작한다.
- Field Debug Log 활성화는 승인, 범위, 만료 조건을 가진다.
- 운영 모니터링과 감사 리포트가 민감 정보 없이 제공된다.
- Web, iOS, Android, Windows, macOS, Linux 공식 배포물이 생성된다.
- 공식 대상 플랫폼별 최소 smoke test가 릴리즈 게이트에 포함된다.

## 단계 간 연결 구조

### 1단계에서 2단계로

1단계는 단일 사용자 로컬 문서 모델을 만든다. 2단계는 이 모델을 서버와 다중 사용자 환경으로 확장한다. 따라서 1단계에서 문서 ID, 링크, 첨부 참조, version entry가 안정적이지 않으면 2단계의 권한, 감사, 리뷰 workflow가 흔들린다.

필수 연결 산출물:

- 안정적인 문서 ID 체계
- 문서 metadata schema
- 내부 version store abstraction
- link/backlink index
- asset reference model
- Product Log event name 규칙
- 공통 core와 Web/Windows/macOS/Linux shell 사이의 adapter boundary
- 플랫폼별 파일 경로와 로컬 저장소 추상화

### 2단계에서 3단계로

2단계는 사용자, 권한, 리뷰, 감사 로그를 만든다. 3단계의 그래프와 실시간 협업은 권한을 기준으로 결과를 필터링해야 하므로 2단계 RBAC가 선행되어야 한다. 또한 iOS/Android 기본 클라이언트는 2단계의 서버 API와 read API 계약 위에 올라와야 한다.

필수 연결 산출물:

- permission decision API
- document workflow state
- audit log store
- edit lock model
- event-driven document change notification
- attachment permission model
- Web self-host UI와 서버 API 계약
- 데스크톱 원격 workspace 접속 모델
- iOS/Android read API 계약
- 플랫폼별 인증/session adapter 경계

### 3단계에서 4단계로

3단계는 문서와 관계를 graph/index로 만들고 Web, desktop, mobile의 기본 사용 흐름을 연결한다. 4단계의 AI retrieval은 이 graph/index와 권한 모델을 사용해 context를 구성한다. 그래프가 없으면 AI는 단순 검색 보조에 머물고, 권한이 없으면 운영 환경에 넣을 수 없다.

필수 연결 산출물:

- graph index
- search index
- permission-aware graph projection
- document freshness metadata
- collaboration event history
- Product/Field Debug log 분리
- iOS/Android 기본 클라이언트
- 모바일/데스크톱 네트워크 상태와 재시도 adapter
- 플랫폼 capability matrix

### 4단계에서 5단계로

4단계는 API, event, webhook, MCP, OAuth app authorization을 만들고 모든 공식 대상 플랫폼에서 AI 질의와 출처 확인을 제공한다. 5단계의 플러그인은 이 확장 지점을 사용한다. API와 event가 안정적이지 않으면 플러그인은 core를 오염시키게 된다.

필수 연결 산출물:

- API scope model
- external app authorization
- webhook/event stream
- connector activity audit
- search extension point
- AI tool/function boundary
- 플랫폼별 secret store 경계
- 플랫폼별 AI 질의 UI 계약
- Web/desktop/mobile 공통 citation 표시 모델

### 5단계에서 6단계로

5단계는 플러그인과 업무 객체를 만들고 플랫폼별 플러그인 노출 정책을 정의한다. 6단계는 이를 SaaS와 엔터프라이즈 환경에서 안전하게 운영한다. 따라서 플러그인 permission, custom object isolation, plugin audit, 플랫폼별 extension policy가 선행되어야 marketplace와 tenant isolation을 제공할 수 있다.

필수 연결 산출물:

- plugin manifest
- plugin permission scope
- custom object schema
- plugin lifecycle state machine
- plugin audit event
- workflow hook isolation
- Web/desktop/mobile extension policy
- plugin UI rendering capability matrix
- 플랫폼별 플러그인 차단/허용 테스트

## 우선순위 기준

기능 우선순위는 다음 순서로 판단한다.

1. 데이터 소유권과 문서 원본 안정성
2. 내부 version store와 복원 가능성
3. 링크/백링크/검색 가능성
4. 첨부 파일의 독립 생명주기
5. 권한과 감사 가능성
6. 협업 workflow
7. 그래프 기반 탐색
8. AI와 외부 연동
9. 플러그인 확장
10. 공식 대상 플랫폼의 기능 일관성
11. SaaS 운영과 엔터프라이즈 제어

이 순서를 위반해 후순위 기능을 먼저 구현하지 않는다. 예를 들어 AI 답변 기능은 permission-aware retrieval이 준비되기 전에는 제품 기능으로 제공하지 않는다. 플러그인 marketplace는 plugin permission scope와 lifecycle state machine이 준비되기 전에는 제공하지 않는다. SaaS 과금은 tenant isolation과 usage metering이 준비되기 전에는 제공하지 않는다.

## 단계별 비목표

### 1단계 비목표

- 다중 사용자 기능을 넣지 않는다.
- 외부 SaaS 연동을 넣지 않는다.
- AI 기능을 넣지 않는다.
- 플러그인 플랫폼을 넣지 않는다.
- iOS/Android 앱을 넣지 않는다.
- 로컬 사용자가 별도 서버, DB, 검색 엔진, Git CLI, Node.js를 직접 설치하게 만들지 않는다.
- UI를 최종 제품 수준으로 만들지 않는다.

### 2단계 비목표

- SaaS 멀티테넌트를 넣지 않는다.
- 고급 실시간 공동 편집을 완성하려 하지 않는다.
- 그래프 시각화를 최종 수준으로 만들지 않는다.
- CRM이나 결제 플러그인을 넣지 않는다.
- iOS/Android 앱을 완성하려 하지 않는다.

### 3단계 비목표

- AI 기반 그래프 요약을 넣지 않는다.
- CRM 객체 그래프를 넣지 않는다.
- 모든 Canvas 기능을 완성하려 하지 않는다.
- 모바일 오프라인 편집을 넣지 않는다.
- 모바일 Canvas 전체 편집을 넣지 않는다.
- SaaS 운영 기능을 넣지 않는다.

### 4단계 비목표

- 플러그인 marketplace를 만들지 않는다.
- AI가 권한 모델을 우회하는 기능을 만들지 않는다.
- prompt 원문을 운영 로그에 남기지 않는다.
- 모든 외부 연동을 한 번에 만들지 않는다.
- 플랫폼별 AI 기능을 별도 구현으로 분기하지 않는다.

### 5단계 비목표

- SaaS 과금과 marketplace 운영을 완성하려 하지 않는다.
- core domain에 CRM 전용 필드를 직접 추가하지 않는다.
- 플러그인 실패가 core usecase를 불안정하게 만들게 하지 않는다.
- 모바일에서 모든 플러그인 UI를 Web과 동일하게 제공하려 하지 않는다.

### 6단계 비목표

- 모든 엔터프라이즈 기능을 무제한 확장하지 않는다.
- 모든 국가별 compliance를 한 번에 다루지 않는다.
- 모든 외부 connector를 직접 구현하지 않는다.
- 운영 로그를 디버그 로그 대체물로 사용하지 않는다.
- 플랫폼별 릴리즈 채널 차이를 도메인 기능 차이로 만들지 않는다.

## 단계별 핵심 리스크와 통제

### 1단계 리스크

리스크:

- 문서 모델이 파일 저장 방식에 종속될 수 있다.
- 내부 Git 사용이 사용자 경험에 노출될 수 있다.
- Markdown parser가 도메인 규칙을 오염시킬 수 있다.
- Windows/macOS/Linux 파일 경로 차이가 core 로직에 새어 들어올 수 있다.
- 로컬 설치가 외부 런타임이나 수동 설정에 의존할 수 있다.

통제:

- `DocumentRepository`, `VersionStore`, `MarkdownParser`를 포트로 분리한다.
- 사용자는 history, diff, restore만 보게 한다.
- parser 결과는 내부 value object로 변환한다.
- 플랫폼별 파일 경로와 앱 데이터 경로는 adapter에서 정규화한다.
- first-run initializer와 local setup health checker를 별도 adapter로 두고 테스트한다.
- 로컬 MVP 릴리즈 게이트에 clean machine install smoke test를 포함한다.

### 2단계 리스크

리스크:

- RBAC가 문서, 첨부, 그래프, AI 단계에서 재사용되지 못할 수 있다.
- 협업 workflow가 ad-hoc flag 조합으로 커질 수 있다.
- Web, desktop, mobile API 계약이 서로 달라질 수 있다.

통제:

- permission decision을 독립 유스케이스와 도메인 정책으로 만든다.
- publish workflow와 lock workflow를 상태머신으로 관리한다.
- 클라이언트별 API를 만들지 않고 공통 유스케이스 DTO와 capability 기반 응답을 사용한다.

### 3단계 리스크

리스크:

- 그래프 인덱스가 권한을 반영하지 못할 수 있다.
- 실시간 편집이 문서 이력과 충돌할 수 있다.
- 모바일 그래프/Canvas 기능이 Web/Desktop과 다른 도메인 규칙을 만들 수 있다.

통제:

- graph projection은 항상 permission-aware로 생성한다.
- 협업 편집 결과는 version store에 일관된 변경 단위로 기록한다.
- 플랫폼별 UI 제한은 capability matrix로 표현하고 도메인 규칙은 공유한다.

### 4단계 리스크

리스크:

- AI가 접근 권한 없는 문서를 context로 사용할 수 있다.
- 운영 로그에 AI prompt나 문서 원문이 남을 수 있다.
- connector가 core domain을 오염시킬 수 있다.
- 플랫폼별 secret 저장 방식이 토큰 노출을 만들 수 있다.

통제:

- retrieval scope는 permission decision을 반드시 통과하게 한다.
- AI 로그는 query hash, count, citation metadata만 남긴다.
- connector payload는 adapter에서 내부 DTO로 변환한다.
- 플랫폼별 secret store는 `PlatformSecureSecretStore` 포트 뒤에 둔다.

### 5단계 리스크

리스크:

- 플러그인이 core domain에 직접 의존할 수 있다.
- custom object가 권한/검색/그래프 규칙을 우회할 수 있다.
- Web 전용 플러그인 UI가 모바일/데스크톱에서 깨진 경험을 만들 수 있다.

통제:

- plugin runtime은 extension point와 port로만 core와 통신한다.
- custom object는 permission scope, search extension, graph projection을 통과해야 한다.
- 플랫폼별 extension point 지원 범위와 fallback UI를 명시한다.

### 6단계 리스크

리스크:

- tenant isolation 실패가 보안 사고로 이어질 수 있다.
- Field Debug Log가 과도하게 활성화될 수 있다.
- 과금/사용량 로직이 도메인과 인프라에 섞일 수 있다.
- 플랫폼별 crash/error report가 민감 정보를 수집할 수 있다.
- 플랫폼별 릴리즈 품질 차이가 제품 신뢰도를 낮출 수 있다.

통제:

- tenant boundary를 모든 repository/query/usecase 테스트에 포함한다.
- Field Debug session은 승인, 범위, 만료 상태머신을 사용한다.
- billing gateway는 포트로 분리하고 subscription/usage는 도메인 규칙으로 검증한다.
- crash/error report는 Product Log 민감 정보 정책을 따른다.
- Web, iOS, Android, Windows, macOS, Linux smoke test를 릴리즈 게이트에 포함한다.

## 최종 연결 목표

6단계가 완료되면 Sponzey Cabinet은 다음 흐름을 자연스럽게 제공해야 한다.

1. 개인 사용자가 로컬 Markdown 지식베이스로 시작한다.
2. Windows/macOS/Linux 데스크톱과 Web 로컬 UI가 같은 core를 사용한다.
3. 같은 데이터 모델을 유지한 채 개인 서버로 옮겨 팀 협업을 시작한다.
4. iOS/Android 클라이언트가 서버 workspace에 접속해 조회, 검색, 댓글, 승인 workflow를 수행한다.
5. 팀 문서는 그래프와 Canvas로 관계형 지식 공간이 된다.
6. 권한을 지키는 AI와 외부 연동이 모든 공식 대상 플랫폼에서 지식 활용을 확장한다.
7. 플러그인이 CRM, 고객지원, 프로젝트, 결제 등 업무 객체를 문서와 연결한다.
8. SaaS 운영 계층이 조직, 보안, 과금, 감사, 엔터프라이즈 제어와 공식 플랫폼 릴리즈 채널을 제공한다.

이 순서를 유지해야 제품의 핵심 철학이 흔들리지 않는다. Sponzey Cabinet은 문서를 먼저 안정적으로 소유하고, 그 위에 협업, 관계, AI, 업무 확장, SaaS 운영을 누적하는 방식으로 개발되어야 한다.

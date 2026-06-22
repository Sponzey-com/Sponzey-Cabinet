# Sponzey Cabinet 프로젝트 최종 목표

작성일: 2026-06-22  
프로젝트명: Sponzey Cabinet  
문서 성격: 제품의 최종 목표와 사용자에게 제시할 기능 범위 정의. 실행 계획, 일정, 단계별 로드맵은 포함하지 않는다.

## 제품 정의

Sponzey Cabinet은 Outline, Notion, Obsidian, AFFiNE, Confluence, Document360, Guru, Redmine DMSF류 기능을 참고해 설계하는 차세대 Knowledge Base Solution이다. 핵심 목표는 "문서를 보관하는 위키"가 아니라, 문서, 관계, 파일, 업무 데이터, AI, 외부 시스템을 하나의 지식 운영 계층으로 연결하는 것이다.

Sponzey Cabinet은 다음 세 가지 사용 형태를 동시에 지원해야 한다.

- 개인 구축: 로컬 또는 단일 사용자 환경에서 문서, 링크, 그래프, 자동 변경 이력, 첨부 파일 관리, AI 연동을 사용할 수 있다.
- 개인 호스팅 구축: 개인 또는 소규모 조직이 자체 서버에 올려 사용할 수 있으며, 이 단계부터 다중 사용자 문서 협업 기능을 지원한다.
- SaaS 서비스: 멀티테넌트, 조직/워크스페이스, 과금, 관리자 콘솔, 보안 감사, 확장 플러그인 마켓플레이스까지 제공한다.

Sponzey Cabinet은 다음 플랫폼을 공식 대상 플랫폼으로 지원해야 한다.

- Web
- iOS
- Android
- Windows
- macOS
- Linux

플랫폼 지원은 같은 제품을 여러 번 따로 만드는 방식이 아니라, 공통 도메인/유스케이스/동기화/권한/검색/AI 계층 위에 플랫폼별 클라이언트 어댑터를 얹는 방식이어야 한다. 문서 모델, 권한 모델, 첨부 파일 모델, 그래프 모델, 플러그인 모델, AI retrieval 모델은 플랫폼에 종속되지 않아야 한다.

플랫폼별 역할은 다음과 같다.

- Web: SaaS와 개인 호스팅의 기본 사용 환경이며, 관리자 콘솔, 협업, 설정, 플러그인 관리, 외부 연동 관리의 기준 클라이언트다.
- Windows/macOS/Linux: 개인 구축의 핵심 클라이언트이며, 로컬 문서 저장소, 로컬 검색, 내부 버전 관리, 첨부 파일 관리, 오프라인 우선 사용을 지원한다. 동시에 개인 호스팅과 SaaS workspace에도 접속할 수 있어야 한다.
- iOS/Android: 이동 중 문서 조회, 편집, 검색, 댓글, 알림, 첨부 확인, AI 질의, 승인 workflow를 지원하는 모바일 클라이언트다. 서버 역할을 수행하지 않고, 개인 호스팅 또는 SaaS workspace에 접속하는 클라이언트로 동작한다.

최종 제품은 사용자가 다음 문장으로 이해할 수 있어야 한다.

> Sponzey Cabinet은 내가 직접 소유하고, 모든 변경 이력이 자동으로 보존되며, AI와 외부 업무 시스템에 쉽게 연결할 수 있는 문서 중심 지식 운영 플랫폼이다.

## 대체 대상과 차별화 방향

### Outline 대체

Outline은 빠른 문서 작성, Markdown 지원, 실시간 협업, 댓글, 강력한 검색/AI 질의응답, Slack 통합, 공개 공유, 권한/그룹, 자체호스팅 옵션을 강점으로 한다.

Sponzey Cabinet은 Outline의 장점을 기본값으로 삼되, 다음 한계를 넘어선다.

- 문서 본문을 내부 버전 관리 엔진의 일급 데이터로 관리하되, 사용자에게는 일반 문서 경험으로 제공한다.
- 문서 간 링크와 백링크를 그래프 데이터로 해석한다.
- 첨부 파일을 문서 내부 부속물이 아니라 별도 관리되는 디지털 자산으로 취급한다.
- CRM, 결제, 고객지원, 프로젝트 관리 같은 업무 기능을 문서에 플러그인 형태로 부착한다.
- 개인 로컬 사용, 개인 호스팅, SaaS를 동일한 제품 모델 안에서 제공한다.
- AI 도구가 읽고, 쓰고, 질의하고, 동기화할 수 있는 API/MCP 계층을 기본 제공한다.

### Notion 대체

Notion은 위키, 문서, 프로젝트, 데이터베이스, 템플릿, 페이지 검증, Synced Blocks, 외부 연결, Enterprise Search, SAML SSO, SCIM, 고급 권한을 결합한다.

Sponzey Cabinet은 Notion의 사용성을 목표로 하되, 다음을 차별화한다.

- 데이터 소유권: 문서 원본은 표준 포맷과 내부 버전 관리 저장소로 보존되며, 사용자는 Git을 몰라도 데이터를 소유하고 내보낼 수 있다.
- 자체호스팅 우선권: 개인과 조직이 벤더 종속 없이 운영할 수 있다.
- 관계형 지식 탐색: 문서 링크, 블록 링크, 첨부 파일, CRM 레코드, 외부 객체를 그래프로 탐색한다.
- 플러그인 중심 확장: Notion database가 모든 문제를 해결하는 방식이 아니라, 도메인 기능을 독립 플러그인으로 설치한다.
- AI 친화성: AI connector, MCP server, webhook, event stream, permission-aware retrieval을 제품 핵심 기능으로 둔다.

### Obsidian 대체

Obsidian은 Markdown 파일, Wikilink/Markdown link, 헤딩/블록 링크, 백링크, 전역/로컬 그래프, Canvas, 플러그인 생태계가 강하다.

Sponzey Cabinet은 Obsidian의 개인 지식관리 장점을 흡수하되, 다음을 더한다.

- 개인 지식관리에서 팀 협업과 SaaS까지 자연스럽게 확장된다.
- 그래프는 시각화에 머물지 않고 권한, 검색, 추천, AI context, CRM 관계 분석에 사용된다.
- Canvas류 자유 배치와 문서 본문, 데이터베이스 뷰를 동시에 제공한다.
- 버전 관리와 협업 충돌 해결을 제품 수준에서 다루며, Git 개념은 사용자 경험에 노출하지 않는다.

### AFFiNE 모드 지원

AFFiNE은 docs, whiteboards, databases, AI를 하나의 local-first workspace로 결합하고, Page Docs와 Edgeless Whiteboard를 Cloud와 Self-hosted 모두에서 제공하는 방향을 취한다.

Sponzey Cabinet의 AFFiNE 모드는 다음을 의미한다.

- 문서 모드: 일반 문서, 위키, 기술문서, 업무 기록을 구조적으로 작성한다.
- Edgeless 모드: 무한 캔버스에서 문서, 카드, 이미지, 첨부, CRM 객체, 외부 링크를 자유롭게 배치한다.
- 전환 가능성: 문서 페이지를 캔버스 노드로 놓거나, 캔버스의 카드 묶음을 문서 구조로 변환한다.
- 협업 가능성: 개인 호스팅 이상에서는 문서 모드와 Edgeless 모드 모두 실시간 공동 편집, 댓글, 멘션, 변경 이력을 지원한다.
- 데이터 일관성: 문서와 캔버스는 별도 앱이 아니라 동일한 지식 그래프 위의 다른 뷰로 동작한다.

## 핵심 제품 원칙

### 1. 문서는 파일이면서 데이터다

문서는 사람이 읽는 Markdown/MDX/구조화 문서인 동시에, 시스템이 질의하고 연결할 수 있는 데이터여야 한다.

- 문서 원본은 Markdown/MDX 같은 텍스트 기반 포맷을 우선하며, 내부적으로 Git을 Markdown 원본 관리와 변경 이력 저장에 사용할 수 있다. 단, 일반 사용자는 commit, branch, repository 같은 Git 개념을 알 필요가 없어야 한다.
- 문서의 제목, slug, owner, 상태, 태그, 권한, 리뷰일, 관련 CRM 객체 같은 메타데이터를 보존한다.
- 문서 내부의 헤딩, 블록, 체크리스트, 표, 코드, 임베드, 콜아웃, 첨부 참조를 구조화한다.
- 문서와 데이터베이스 레코드 사이의 경계를 낮춘다. 사용자는 문서를 쓰면서 CRM, 계약, 제품, 고객, 티켓, 결제 같은 업무 데이터를 연결할 수 있어야 한다.

### 2. 사용자는 자기 데이터를 소유한다

Sponzey Cabinet은 SaaS만 전제하지 않는다.

- 개인 사용자는 로컬 또는 개인 저장소에 문서를 둘 수 있다.
- 개인 호스팅 사용자는 자체 인프라, 자체 도메인, 자체 인증, 자체 백업 정책을 사용할 수 있다.
- SaaS 사용자는 관리형 운영, 확장성, 백업, 과금, 조직 관리, 엔터프라이즈 보안을 사용할 수 있다.
- 모든 배포 형태에서 데이터 이동성과 내보내기를 보장한다.
- 개인 구축의 로컬 설정은 설치 1회로 완료되어야 한다.
- 개인 구축 사용자는 별도 런타임, DB, 검색 서버, Git CLI, Node.js, 외부 설정 파일을 직접 설치하거나 구성하지 않아야 한다.
- 로컬 앱은 필요한 기본 저장소, 내부 버전 관리, 검색 인덱스, 첨부 저장소, 앱 데이터 경로를 최초 실행 시 자동으로 준비해야 한다.
- 고급 사용자를 위한 위치 변경, 백업 대상 변경, AI provider 연결은 명시적 설정 화면에서 제공하되, 기본 사용 흐름의 필수 단계가 되어서는 안 된다.
- Web, iOS, Android, Windows, macOS, Linux에서 동일한 핵심 데이터 모델과 권한 규칙을 유지한다.
- 플랫폼별 저장소, 알림, 파일 선택기, 오프라인 캐시, 인증 흐름은 어댑터로 분리한다.
- 플랫폼 차이 때문에 도메인 규칙이나 유스케이스가 분기되지 않게 한다.

### 3. AI와 외부 솔루션 연동은 부가 기능이 아니라 기본 기능이다

Sponzey Cabinet은 AI agent, LLM, 자동화 도구, 업무 SaaS가 쉽게 읽고 쓸 수 있어야 한다.

- REST/GraphQL API를 제공한다.
- Webhook과 event stream으로 문서 생성, 수정, 삭제, 승인, 댓글, 파일 업로드, 권한 변경 이벤트를 외부로 보낸다.
- MCP Server를 제공해 Claude, Cursor, OpenAI 기반 에이전트, 사내 AI 도구가 권한을 지키며 문서와 첨부를 검색하고 인용할 수 있게 한다.
- OAuth 2.0 기반 앱 설치 흐름을 제공해 외부 서비스가 사용자 승인 범위 안에서 접근한다.
- API token, personal access token, service account, workspace app 같은 여러 인증 모델을 지원한다.
- AI 답변은 반드시 출처 링크, 인용 범위, 접근 권한, 최신성 상태를 표시한다.

### 4. 문서 지식은 그래프로 이해되어야 한다

문서 간 링크는 단순 HTML 링크가 아니라 지식 그래프의 edge다.

- Wikilink와 Markdown link를 모두 지원한다.
- 문서, 헤딩, 블록, 첨부 파일, 캔버스 카드, CRM 객체, 외부 URL을 링크 대상으로 삼는다.
- 백링크, 미해결 링크, 고아 문서, 허브 문서, 관련 문서 추천을 제공한다.
- Obsidian형 전역 그래프와 로컬 그래프를 제공한다.
- AFFiNE/Canvas형 자유 배치 그래프를 제공한다.
- 그래프 필터는 태그, 소유자, 권한, 상태, 문서 유형, 고객/계정/프로젝트/제품 관계를 기준으로 동작한다.
- 그래프는 AI 검색과 추천의 context source로 사용된다.

## 기술 스택과 아키텍처 목표

Sponzey Cabinet은 Rust, React, CodeMirror, Tauri를 중심 기술로 사용한다.

- Rust: 도메인 모델, 유스케이스, 내부 버전 관리, 문서 파서, 링크/그래프 처리, 첨부 파일 처리, 검색 인덱싱, 권한 정책, 상태머신, 서버 API, 협업 backend의 기준 언어다.
- React: Web, desktop, mobile WebView UI의 공통 UI 계층이다.
- CodeMirror: Markdown/MDX 문서 편집기의 기준 editor engine이다. CodeMirror는 편집 UI로 사용하고, 문서 모델과 도메인 규칙의 소유자가 되어서는 안 된다.
- Tauri: Windows, macOS, Linux, iOS, Android 네이티브 앱의 shell과 platform adapter 계층이다. Tauri는 core domain이 아니라 파일시스템, 보안 저장소, 알림, OS 통합, local process lifecycle을 연결하는 경계 계층이어야 한다.

핵심 아키텍처 원칙은 다음과 같다.

- Web과 앱은 같은 React/CodeMirror UI 패키지를 공유한다.
- Web은 Rust server API를 통해 workspace에 접근한다.
- Tauri 앱은 로컬 workspace에서는 Rust core를 in-process로 호출하고, 서버 workspace에서는 Rust server API와 realtime collaboration endpoint를 사용한다.
- iOS/Android 앱은 서버 workspace 접속 클라이언트로 시작하며, 서버 역할을 수행하지 않는다.
- 문서 모델, 권한 모델, 첨부 모델, 그래프 모델, 협업 모델, AI retrieval 모델은 플랫폼에 종속되지 않는다.
- 플랫폼별 파일 선택, 로컬 경로, secure storage, push notification, deep link, 네트워크 상태는 adapter로 격리한다.

### 앱 구조

앱은 Web app과 Tauri app이 최대한 많은 UI와 editor 코드를 공유하는 구조여야 한다.

```text
apps/
  web/
    React app
    CodeMirror editor
    browser API adapter
    SaaS/self-host server connector

  desktop/
    Tauri shell
    React app
    CodeMirror editor
    local workspace adapter
    remote workspace adapter
    platform filesystem adapter
    platform secure storage adapter
    platform notification adapter

  mobile/
    Tauri mobile shell
    React app
    CodeMirror editor
    remote workspace adapter
    platform secure storage adapter
    platform push notification adapter
```

공유 패키지는 다음 경계를 가져야 한다.

```text
packages/
  ui/
    shared React components
    layout
    command palette
    document tree
    comments
    review workflow UI

  editor/
    CodeMirror configuration
    Markdown/MDX extensions
    wikilink extension
    attachment reference extension
    slash command extension
    collaboration binding
    editor theme

  client-core/
    client-side application state
    workspace capability model
    API client interfaces
    realtime client interfaces
    offline queue interface
```

Rust workspace는 다음 경계를 가져야 한다.

```text
crates/
  cabinet-domain/
    entities
    value objects
    domain services
    state machines

  cabinet-usecases/
    application usecases
    ports
    input/output DTOs

  cabinet-core/
    Markdown parser orchestration
    link graph logic
    document operation model
    permission policy
    version operation model

  cabinet-adapters/
    filesystem
    internal version store
    asset store
    search index
    graph index
    clock
    id generator
    logger

  cabinet-server/
    HTTP API
    realtime gateway
    background workers
    server composition root

  cabinet-tauri/
    Tauri commands
    platform adapters
    desktop/mobile composition root
```

앱 구조의 개발 규칙은 다음과 같다.

- React component는 도메인 규칙을 직접 구현하지 않는다.
- CodeMirror extension은 editor event를 문서 operation으로 변환하는 역할만 한다.
- 문서 저장, 버전 기록, 권한 검증, 링크 인덱싱은 Rust usecase를 통해 수행한다.
- Web app은 browser storage를 신뢰 가능한 원본 저장소로 사용하지 않는다.
- Tauri app의 로컬 workspace만 로컬 문서 저장소를 원본으로 사용할 수 있다.
- Tauri command는 얇은 adapter로 유지하고, 비즈니스 규칙을 포함하지 않는다.
- 모바일 앱은 서버 workspace 클라이언트로 시작하고, 모바일 오프라인 편집은 별도 capability로 분리한다.

### 서버 구조

서버는 개인 호스팅과 SaaS를 모두 지원해야 하며, 문서 협업과 검색/AI 부하를 고려해 수평확장 가능한 구조여야 한다.

서버는 다음 컴포넌트로 구성한다.

```text
edge/
  load balancer
  TLS termination
  request routing
  websocket routing

api layer/
  stateless HTTP API servers
  auth/session middleware
  workspace/tenant context resolver
  rate limit adapter

realtime layer/
  collaboration gateway
  document room router
  presence service
  edit operation broadcaster
  collaboration session coordinator

application layer/
  document usecases
  asset usecases
  permission usecases
  review/publish workflow usecases
  graph usecases
  search usecases
  AI retrieval usecases
  plugin usecases

worker layer/
  indexing workers
  graph projection workers
  asset processing workers
  OCR/text extraction workers
  AI embedding workers
  webhook delivery workers
  backup/export workers

storage layer/
  relational metadata store
  internal version store
  object storage
  search index
  vector index
  graph projection store
  event log
  cache/session store
```

각 서버 컴포넌트의 책임은 다음과 같다.

- API server: 인증, 권한 context 구성, request DTO 변환, 유스케이스 호출, response DTO 반환을 담당한다. API server는 stateless로 유지해 replica를 수평확장할 수 있어야 한다.
- Collaboration gateway: WebSocket 또는 동등한 realtime connection을 수용하고, 문서별 collaboration room으로 사용자를 연결한다.
- Document room router: workspace id와 document id를 기준으로 collaboration session의 소유 노드를 결정한다.
- Collaboration session coordinator: 문서별 edit operation, presence, cursor, transient state를 관리한다.
- Document service: 문서 command를 검증하고 내부 version store, metadata store, graph/search event를 갱신한다.
- Asset service: 첨부 파일 metadata, object storage, file revision, scan/OCR job을 관리한다.
- Search service: keyword index, semantic index, permission-aware result filtering을 담당한다.
- Graph service: 문서 링크, 백링크, 첨부, canvas, 업무 객체 관계를 projection으로 관리한다.
- AI service: retrieval scope, citation, answer generation, provider adapter를 담당한다.
- Worker: indexing, graph projection, embedding, webhook, export처럼 요청 경로에서 분리 가능한 작업을 비동기로 처리한다.

### 문서 협업 아키텍처

문서 협업은 단순 저장 API가 아니라 realtime operation pipeline으로 관리해야 한다.

```text
Client Editor
  -> local editor transaction
  -> document operation
  -> realtime gateway
  -> document room
  -> permission check
  -> operation ordering
  -> broadcast
  -> durable event append
  -> snapshot/version update
  -> async index/graph update
```

협업 기능은 다음 규칙을 따른다.

- CodeMirror transaction은 서버에 직접 저장하지 않고, 도메인에서 이해 가능한 document operation으로 변환한다.
- 문서 operation은 사용자, workspace, document, base revision, operation id, timestamp를 포함해야 한다.
- operation은 권한 검증을 통과해야 한다.
- 같은 문서의 operation ordering은 document room에서 결정한다.
- operation은 durable event log에 append되어야 한다.
- 일정 기준마다 snapshot을 생성해 재접속과 장애 복구 비용을 낮춘다.
- 내부 version store는 사용자에게 보이는 history/diff/restore 모델을 제공한다.
- 검색/그래프/AI 인덱스는 협업 operation을 직접 처리하지 않고, 확정된 document changed event를 비동기로 소비한다.
- presence, cursor, selection은 transient state로 관리하고 durable version history에 직접 포함하지 않는다.
- 댓글, 리뷰, 승인 workflow는 문서 본문 operation과 분리된 도메인 이벤트로 관리한다.

협업 상태머신은 다음 개념을 가져야 한다.

```text
Disconnected -> Connecting
Connecting -> Connected
Connected -> JoiningDocument
JoiningDocument -> Editing
Editing -> Syncing
Syncing -> Synced
Syncing -> ConflictDetected
ConflictDetected -> Resolving
Resolving -> Synced
Connected -> Reconnecting
Reconnecting -> Connected
Reconnecting -> Offline
Offline -> ReplayingLocalChanges
ReplayingLocalChanges -> Synced
```

### 수평확장 아키텍처

수평확장은 stateless API와 stateful collaboration room을 분리하는 방식으로 달성한다.

- API server는 stateless로 유지한다.
- API server는 tenant/workspace/document context를 request마다 명시적으로 구성한다.
- API server session은 외부 session/cache store에 둔다.
- Collaboration gateway는 여러 replica로 실행한다.
- 같은 문서의 realtime edit stream은 하나의 active document room owner가 처리한다.
- document room owner는 lease, consistent hashing, shard routing 중 하나의 명시적 방식으로 결정한다.
- node 장애 시 document room은 event log와 snapshot으로 다른 node에서 복구되어야 한다.
- presence와 cursor는 cache/session store 또는 realtime pub/sub에 저장한다.
- 영속 operation은 event log에 기록한다.
- document changed event는 message bus를 통해 search, graph, AI indexing worker로 전달한다.
- worker는 queue partition을 기준으로 수평확장한다.
- search index, vector index, graph projection은 tenant/workspace 단위 partition 또는 shard 전략을 가져야 한다.
- object storage는 서버 local disk에 종속되지 않아야 한다.
- 첨부 파일 upload/download는 가능한 경우 pre-signed URL 또는 streaming adapter로 API server 부하를 줄인다.
- AI embedding과 OCR은 request path에서 직접 수행하지 않고 background worker에서 처리한다.

확장 단위는 다음과 같다.

| 부하 유형            | 확장 단위                      | 상태 관리                               |
| ---------------- | -------------------------- | ----------------------------------- |
| 일반 API 요청        | API server replica         | stateless, external session/cache   |
| 실시간 협업           | document room shard        | event log + snapshot + lease        |
| presence/cursor  | realtime replica + pub/sub | transient cache                     |
| 검색 색인            | indexing worker partition  | queue + search index                |
| 그래프 projection   | graph worker partition     | queue + graph projection store      |
| 첨부 처리            | asset worker partition     | object storage + metadata store     |
| AI embedding/OCR | AI worker partition        | queue + vector index/object storage |
| webhook 전송       | delivery worker partition  | queue + dead-letter store           |

### 서버 저장소 구조

서버 저장소는 원본 데이터, projection, cache를 분리해야 한다.

- Relational metadata store: tenant, workspace, user, group, permission, document metadata, asset metadata, comment, review, workflow, plugin metadata를 저장한다.
- Internal version store: Markdown/MDX 원본과 변경 이력을 관리한다. 사용자는 Git 개념을 보지 않고 history/diff/restore만 사용한다.
- Event log: collaboration operation, document changed event, asset event, workflow event, plugin event를 append-only로 기록한다.
- Object storage: 첨부 파일 원본, preview, OCR 결과, export package, backup archive를 저장한다.
- Search index: 제목, 본문, 태그, 첨부 텍스트, 업무 객체 검색을 담당한다.
- Vector index: semantic search와 AI retrieval을 위한 embedding을 저장한다.
- Graph projection store: 문서 링크, 백링크, 첨부, canvas, CRM 객체 관계를 빠르게 조회하기 위한 projection을 저장한다.
- Cache/session store: session, rate limit, presence, cursor, short-lived capability, Field Debug activation state를 저장한다.
- Dead-letter store: webhook, connector, indexing, AI job 실패 이벤트를 보관한다.

저장소 규칙은 다음과 같다.

- 원본 문서와 projection을 혼동하지 않는다.
- projection은 재생성 가능해야 한다.
- cache는 원본이 아니어야 한다.
- 첨부 파일 원본은 metadata DB에 직접 넣지 않는다.
- 검색/그래프/vector index 실패가 문서 저장 성공을 되돌리면 안 된다.
- event log와 version store는 장애 복구와 감사의 기준이 되어야 한다.

### 배포 구조

개인 구축, 개인 호스팅, SaaS는 같은 core를 사용하되 배포 단위만 달라야 한다.

개인 구축:

```text
Tauri desktop app
  React + CodeMirror UI
  Rust core in-process
  local metadata store
  local internal version store
  local asset store
  local search index
```

개인 구축의 로컬 배포는 다음 기준을 만족해야 한다.

- 사용자는 앱 설치 후 바로 workspace를 만들 수 있어야 한다.
- 앱 설치 후 별도 CLI 설정, DB 초기화, 검색 엔진 설치, Git 설치, 환경 변수 편집, 설정 파일 수정을 요구하지 않는다.
- 앱은 최초 실행 시 기본 app data directory를 자동 선택하고, 필요한 local metadata store, internal version store, asset store, search index를 초기화한다.
- 로컬 workspace 생성은 마법사 없이도 기본값으로 완료되어야 한다.
- 백업 위치, workspace 위치, local AI provider, 고급 로그 설정은 선택 사항이어야 한다.
- 설치 프로그램은 플랫폼별 필수 런타임을 최대한 포함하거나 OS 기본 기능을 사용해야 한다.
- 로컬 앱의 재설치와 업그레이드는 기존 workspace를 보존해야 한다.
- 로컬 설정 실패는 사용자에게 복구 가능한 메시지와 안전한 재시도 경로를 제공해야 한다.

개인 호스팅:

```text
Web / Tauri desktop / mobile clients
  -> self-host server
    -> API server
    -> realtime collaboration gateway
    -> metadata store
    -> internal version store
    -> object storage
    -> search/graph workers
```

SaaS:

```text
Web / Tauri desktop / mobile clients
  -> edge/load balancer
    -> API server replicas
    -> collaboration gateway replicas
    -> worker pools
    -> managed metadata store
    -> managed object storage
    -> managed search/vector/graph stores
    -> monitoring/audit/billing systems
```

### 성능과 안정성 목표

- 문서 조회는 현재 문서 조회와 이력 조회를 명확히 분리해야 한다.
- 현재 문서 조회는 최신 snapshot과 metadata를 기준으로 응답해야 한다.
- 이력 조회는 version history, diff, restore preview, 특정 시점 snapshot을 기준으로 응답해야 한다.
- 현재 문서 조회 경로는 이력 저장소 전체를 스캔하지 않아야 한다.
- 이력 조회 경로는 현재 문서 조회 성능을 저하시키지 않도록 별도 query path와 pagination을 가져야 한다.
- 모든 사용자-facing 검색과 조회는 정상적인 인덱스 상태에서 p95 300ms 이내 응답을 목표로 해야 한다.
- 300ms 기준은 문서 현재 조회, 문서 이력 목록 조회, 특정 버전 metadata 조회, 폴더/컬렉션 목록 조회, 링크/백링크 조회, 첨부 metadata 조회, 권한 필터링이 적용된 검색에 적용한다.
- AI 답변 생성, OCR, embedding, 대용량 export, 대용량 첨부 preview 생성처럼 본질적으로 비동기인 작업은 300ms 기준의 직접 대상이 아니다. 단, 작업 상태 조회와 캐시된 결과 조회는 300ms 이내를 목표로 해야 한다.
- 문서 읽기 경로는 metadata, current snapshot, permission decision, search/graph projection을 활용해 빠르게 응답해야 한다.
- 문서 쓰기 경로는 operation validation, permission check, durable append, broadcast를 우선하고, 검색/그래프/AI indexing은 비동기로 처리해야 한다.
- 협업 편집은 문서별 room 단위로 순서를 보장해야 한다.
- 대형 workspace는 document id 또는 workspace id 기준으로 collaboration room과 worker queue를 분산해야 한다.
- 대용량 첨부 처리는 API server memory에 파일 전체를 적재하지 않아야 한다.
- AI embedding, OCR, preview 생성은 background worker로 격리해야 한다.
- 장애 복구는 event log, snapshot, version store를 기준으로 수행해야 한다.
- 서버 node가 사라져도 문서 원본, version history, 확정된 operation은 손실되면 안 된다.
- Product Log는 협업 session, document room ownership, worker failure, indexing lag, queue backlog를 추적할 수 있어야 한다.
- Field Debug Log는 특정 workspace/document/session 범위로만 협업 문제를 진단할 수 있어야 한다.
- Product Log와 운영 metric은 검색/조회 latency, p95, p99, index freshness, cache hit rate, query timeout을 추적할 수 있어야 한다.

## 사용자에게 제시할 핵심 기능

### 0. 공식 대상 플랫폼

- Web 클라이언트
- iOS 모바일 클라이언트
- Android 모바일 클라이언트
- Windows 데스크톱 클라이언트
- macOS 데스크톱 클라이언트
- Linux 데스크톱 클라이언트
- 플랫폼 간 동일 workspace 접근
- 플랫폼 간 문서/첨부/그래프/권한 모델 일관성
- 데스크톱 로컬 workspace와 서버 workspace 전환
- 모바일 문서 조회/편집/검색/댓글/승인 workflow
- Web 기반 관리자 콘솔과 협업 관리
- 플랫폼별 네이티브 기능은 어댑터 계층에서만 처리

### 1. 문서 작성과 위키

- 빠른 Markdown 기반 편집
- WYSIWYG와 Markdown source 모드 전환
- Slash command 기반 블록 삽입
- 코드 블록, 표, 콜아웃, 체크리스트, 수식, Mermaid/PlantUML/diagrams.net류 다이어그램
- 문서 템플릿, 스니펫, 반복 가능한 섹션
- 페이지 트리, 컬렉션, 폴더, 태그, 즐겨찾기
- 문서 owner, reviewer, verified 상태, stale 상태
- 문서 만료일과 재검토 주기
- 현재 문서 기준 조회
- 이력 기준 조회
- 특정 버전 조회
- 문서 diff 조회
- 복원 preview 조회
- 문서 공개 링크와 비공개 공유 링크
- 문서별 댓글, 인라인 댓글, 멘션, 해결됨 상태
- 문서 변경 알림, 구독, watcher
- 문서 가져오기: Markdown, HTML, PDF/Word 변환, Notion/Confluence/Outline export
- 문서 내보내기: Markdown, HTML, PDF, ZIP, 원본 문서 패키지, static site source

### 2. 협업 기능

개인 호스팅과 SaaS에서는 협업 기능이 기본 지원되어야 한다.

- 실시간 공동 편집
- presence 표시
- 인라인 댓글과 thread
- 리뷰 요청과 승인
- 변경 제안, draft, publish workflow
- 문서 잠금 또는 섹션 잠금
- 충돌 감지와 병합 UI
- 팀/그룹/역할 기반 공유
- 조직/워크스페이스/프로젝트 단위 권한
- 활동 피드와 감사 로그
- Slack, Microsoft Teams, 이메일 알림

### 3. 투명한 버전 관리

Sponzey Cabinet의 문서 버전 관리는 사용자에게는 변경 이력, 비교, 복원, 승인 흐름으로 보이고, 내부적으로는 Markdown 원본 관리를 위해 Git을 사용할 수 있다. Git은 문서 저장과 이력 관리를 위한 내부 엔진이며, 코드 개발식 리뷰/병합 흐름은 제품 목표에 포함하지 않는다.

- 문서 생성, 수정, 이동, 삭제는 내부 변경 단위로 기록된다.
- 작성자, 수정자, 변경 요약, timestamp, 상태, 릴리즈 정보를 보존한다.
- 문서 history, diff, 작성자 추적, revert를 문서 UI에서 제공한다.
- 현재 문서 조회와 이력 조회는 UI/API에서 명확히 구분한다.
- 현재 문서 조회는 latest snapshot을 기준으로 빠르게 응답하고, 이력 조회는 version entry와 snapshot/diff를 기준으로 별도 처리한다.
- 문서 초안, 승인, 배포는 사용자에게 draft, review, published 같은 상태로 보인다.
- 리뷰와 승인은 문서 워크플로로 제공하며, 코드 저장소형 리뷰 절차로 제공하지 않는다.
- 개인 로컬 저장소와 원격 백업 저장소를 연결할 수 있다.
- 문서와 첨부 파일은 분리 관리하되, 문서에서 첨부의 content-addressed ID 또는 asset ID를 참조한다.
- 대용량 첨부는 내부 버전 관리 저장소에 직접 넣지 않고 별도 asset store에 두며, 필요 시 외부 object storage와 연결한다.

### 4. 첨부 파일 관리

첨부 파일은 Redmine DMSF(Document Management System Features)처럼 문서 부속물이 아니라 별도 생명주기를 가진 관리 대상이어야 한다.

- 폴더/디렉터리 구조
- 파일 버전과 revision history
- 파일 잠금
- 다중 업로드와 다중 다운로드
- ZIP 다운로드
- 파일별 owner, reviewer, 승인 상태
- 승인 workflow
- 접근 감사
- 문서, 이슈, CRM 객체, 캔버스 카드에 첨부 연결
- WebDAV 또는 호환 파일 접근 계층
- 전체 텍스트 검색과 OCR 연동
- 파일 custom fields
- symbolic link 또는 shortcut
- 휴지통과 복원
- 파일 보존 정책과 만료 정책
- S3, MinIO, local disk, WebDAV, NAS, cloud object storage backend 선택

### 5. 그래프와 AFFiNE/Canvas 기능

- 전역 지식 그래프
- 문서 기준 로컬 그래프
- 특정 depth 기준 주변 문서 탐색
- 링크 방향 표시
- 태그/첨부/고아 문서 표시 여부
- 문서 유형, 조직, 담당자, 고객, 프로젝트 기준 색상 그룹
- Edgeless canvas에서 문서, 첨부, 외부 링크, CRM 객체, 노트를 카드로 배치
- 카드 간 directed edge, 라벨, 색상, 그룹
- 캔버스 자체를 문서에 임베드
- 캔버스의 일부를 문서 섹션으로 변환
- 문서의 헤딩 구조를 캔버스 맵으로 변환
- 관계도를 AI가 요약하고 누락된 링크를 추천

### 6. 문서 기반 CRM

Sponzey Cabinet의 CRM 기능은 별도 CRM 앱을 단순히 붙이는 수준이 아니라 문서와 CRM 레코드가 같은 지식 그래프 위에서 동작해야 한다.

- 고객 계정, 담당자, 리드, 딜, 계약, 프로젝트, 티켓, 미팅 노트 객체
- 문서에서 CRM 객체를 직접 mention/link
- 고객별 문서 허브 자동 구성
- 미팅 노트에서 action item, follow-up, 관련 문서 자동 추출
- 계약/제안서/요구사항 문서와 CRM 딜 연결
- 고객지원 FAQ, 제품 문서, 내부 해결 노트와 고객 티켓 연결
- CRM 객체별 첨부 파일, 승인 상태, activity log
- CRM 필드 custom schema
- CRM view: table, board, timeline, calendar, graph
- 외부 CRM 연동: Salesforce, HubSpot, Pipedrive 등
- CRM 플러그인은 core document model을 오염시키지 않고 확장 필드와 relation으로 동작한다.

### 7. 플러그인 플랫폼

Sponzey Cabinet은 다양한 기능을 플러그인으로 제공해야 한다.

플러그인 예시는 다음과 같다.

- 결제: Stripe, Paddle, Toss Payments, Lemon Squeezy, invoice, subscription, usage billing
- CRM: 고객, 리드, 딜, 계약, 미팅 노트, 고객 문서 허브
- 고객지원: FAQ, ticket deflection, help center, AI chatbot
- 프로젝트/이슈: task, issue, milestone, roadmap, release note
- 문서 승인: SOP, 품질문서, 규정문서, 전자결재
- 개발 문서: issue, release, API docs, changelog, technical decision record
- 다이어그램: Mermaid, PlantUML, diagrams.net
- 검색: OpenSearch, Meilisearch, Typesense, vector DB connector
- AI: OpenAI, Anthropic, Gemini, local LLM, embedding provider, RAG pipeline
- 인증: OIDC, SAML, LDAP, SCIM, OAuth app connector
- 저장소: S3, MinIO, WebDAV, local filesystem

플러그인 시스템은 다음 기능을 제공해야 한다.

- UI extension point
- document block extension
- sidebar/action/menu extension
- custom object type
- custom field/schema
- workflow hook
- search index extension
- AI tool/function extension
- permission scope
- event subscription
- background job
- migration
- marketplace metadata
- install/update/disable/remove lifecycle
- SaaS와 self-host 모두에서 동작하는 배포 방식

### 8. AI 기능

AI는 문서 작성 보조가 아니라 지식 운영 기능으로 제공되어야 한다.

- 문서 검색 질의에 대한 답변 생성
- 답변의 출처 문서와 인용 범위 표시
- 사용자의 권한을 반영한 permission-aware retrieval
- 문서 요약, 섹션 요약, 변경 요약
- 오래된 문서 감지
- 중복 문서 감지
- 누락된 링크 추천
- 관련 문서 추천
- 문서 owner/reviewer 추천
- 문서 품질 점수
- glossary 자동 생성
- 고객/프로젝트별 브리핑 생성
- 미팅 노트 정리와 action item 추출
- 문서 변경 내역 기반 release note 생성
- 첨부 파일 OCR/텍스트 추출 후 검색/요약
- AI agent가 사용할 수 있는 tool API와 MCP server
- 외부 AI 시스템에서 사용할 안전한 search/read/write scope
- 고객 데이터가 외부 모델 학습에 사용되지 않도록 하는 정책과 설정

### 9. 검색과 발견성

- 전체 텍스트 검색
- 제목, 태그, owner, 상태, 첨부, CRM 객체 검색
- vector search와 semantic search
- exact keyword와 semantic 결과 병합
- 검색/조회 p95 300ms 목표
- 검색 결과 pagination
- 권한 필터링 이후 latency 관리
- index freshness 표시
- 문서 freshness, 검증 상태, 권한, 사용 빈도를 반영한 랭킹
- 검색 결과에서 답변 생성
- 검색 결과 explainability
- 최근 문서, 인기 문서, 자주 실패한 검색어
- 사용자가 검색했지만 찾지 못한 지식 gap 분석
- 외부 앱 데이터까지 통합 검색하는 connector architecture

### 10. 권한, RBAC, SSO

Sponzey Cabinet은 개인 도구에서 엔터프라이즈 서비스까지 확장되어야 하므로 권한 모델이 제품 핵심이어야 한다.

- 사용자, 그룹, 팀, 조직, 워크스페이스
- role 기반 권한: owner, admin, editor, reviewer, viewer, guest, service account
- 문서 단위 권한
- 폴더/컬렉션 단위 권한
- 첨부 파일 단위 권한
- CRM 객체 단위 권한
- 캔버스 단위 권한
- inherited permission과 explicit override
- public link, private link, guest access
- OAuth 2.0 app authorization
- OIDC login
- SAML SSO
- SCIM user/group provisioning
- LDAP/Active Directory 연동
- MFA
- API token, PAT, service account key
- audit log
- IP allowlist
- data residency와 보존 정책
- permission-aware AI answer
- DLP/redaction hook

### 11. 통합과 마이그레이션

Sponzey Cabinet은 "닫힌 위키"가 아니라 다른 시스템과 연결되는 지식 허브여야 한다.

필수 import/export 대상:

- Markdown folder
- 원본 문서 패키지 백업
- Notion export
- Outline export/API
- Confluence export/API
- Obsidian vault
- AFFiNE workspace export 가능 포맷
- BookStack/Wiki.js/XWiki류 위키 export
- PDF, Word, HTML

필수 연동 대상:

- Slack
- Microsoft Teams
- Google Drive
- OneDrive/SharePoint
- Jira
- Linear
- Figma
- Loom
- Salesforce
- HubSpot
- Zendesk
- Intercom
- Zapier/Make/n8n
- OpenAI/Anthropic/Gemini/local LLM

통합은 다음 품질을 만족해야 한다.

- 읽기 전용 connector와 읽기/쓰기 connector 구분
- 권한 위임 범위 명확화
- 동기화 상태 표시
- 충돌 표시
- 외부 객체 deep link
- 외부 변경 이벤트 수신
- 실패 재시도와 dead-letter 로그
- 감사 가능한 connector activity

## 배포 모델별 최종 기능 범위

### 개인 구축

- 단일 사용자
- Windows/macOS/Linux 데스크톱 클라이언트
- Web 기반 로컬 UI 또는 로컬 서버 UI
- 로컬 문서 저장소
- 자동 변경 이력과 복원
- 문서 링크/백링크/그래프
- 첨부 파일 로컬 관리
- 로컬 검색
- AI provider 직접 연결
- 플러그인 일부 지원
- export/import 완전 지원

### 개인 호스팅 구축

- 다중 사용자
- Web 클라이언트
- Windows/macOS/Linux 데스크톱 클라이언트
- iOS/Android 모바일 클라이언트
- 실시간 공동 편집
- 자체 도메인
- 관리자용 외부 버전 저장소/백업 연결
- 자체 첨부 파일 저장소
- RBAC
- OIDC/OAuth login
- Webhook/API/MCP
- 플러그인 설치
- 백업/복원
- 감사 로그

### SaaS 서비스

- 멀티테넌트
- Web 클라이언트
- Windows/macOS/Linux 데스크톱 클라이언트
- iOS/Android 모바일 클라이언트
- 조직/워크스페이스 관리
- 사용량/과금/구독
- 플러그인 마켓플레이스
- SAML SSO/SCIM
- 고급 RBAC와 관리자 콘솔
- 엔터프라이즈 감사/보안 리포트
- 데이터 보존/삭제 정책
- connector catalog
- AI usage governance
- SLA, 백업, 모니터링, 장애 대응

## 사용자에게 제시할 최종 가치

### 개인 사용자

- Obsidian처럼 내 문서를 내가 소유한다.
- 모든 문서 변경이 자동으로 기록되고 필요할 때 복원할 수 있다.
- 문서 간 관계를 그래프로 탐색한다.
- AI를 내 문서에 연결해 검색, 요약, 정리를 자동화한다.
- 나중에 개인 서버나 SaaS 조직으로 자연스럽게 확장할 수 있다.

### 소규모 팀

- Outline처럼 빠르게 문서를 쓰고 협업한다.
- Notion처럼 문서와 업무 데이터를 함께 관리한다.
- 내부적으로 신뢰 가능한 버전 관리가 동작해 변경 이력과 복원을 믿을 수 있다.
- 자체호스팅으로 비용과 데이터 통제를 확보한다.
- Slack/Teams/Jira/CRM과 연결한다.

### 기업/조직

- Confluence처럼 팀 단위 지식 허브를 만든다.
- Guru처럼 검증된 지식을 AI 답변으로 제공한다.
- Document360처럼 내부/외부 지식베이스와 고객지원 흐름을 지원한다.
- Redmine DMSF처럼 첨부 파일을 승인/감사/버전 관리한다.
- RBAC, SSO, SCIM, 감사 로그, DLP, 데이터 보존 정책을 지원한다.
- SaaS 또는 자체호스팅 중 조직 정책에 맞는 운영 방식을 선택한다.

## 참고 솔루션에서 반영한 기능

| 참고 솔루션       | 확인한 강점                                                                               | Sponzey Cabinet에 반영할 목표                     |
| ------------ | ------------------------------------------------------------------------------------ | ------------------------------------------- |
| Notion       | 위키, 페이지 링크, 검증, Synced Blocks, Enterprise Search, 외부 연결, SAML/SCIM                   | 쉬운 문서 UX, 최신성 관리, AI 검색, 권한/SSO, 데이터베이스형 문서 |
| Outline      | Markdown, 실시간 협업, AI Q&A, Slack, 공개 공유, 자체호스팅, open API                              | 빠른 팀 위키, self-host, API 우선, 협업 기본값          |
| Obsidian     | Markdown vault, Wikilink/Markdown link, 헤딩/블록 링크, 그래프, Canvas, 플러그인                  | 개인 지식관리, 관계 그래프, 자유 캔버스, 로컬 우선              |
| AFFiNE       | docs + whiteboard + database + AI, local-first, Cloud/Self-hosted, Edgeless          | 문서 모드와 Edgeless 모드 통합, 캔버스 협업, 배포 모델 일관성    |
| Confluence   | 실시간 편집, 댓글, whiteboards, databases, Jira 연동, page versioning, RBAC/SSO               | 엔터프라이즈 협업, 프로젝트/개발 조직 지식 허브                 |
| Guru         | permission-aware AI answers, verification, duplicate detection, MCP/API, Slack/Teams | AI 답변, 검증 워크플로, 업무 흐름 내 지식 전달               |
| Document360  | 내부/외부 KB, workflow, SSO/SCIM, API, media storage, AI search/chatbot, MCP             | 고객지원 지식베이스, API/MCP, 분석, ticket deflection  |
| Redmine DMSF | 디렉터리, 파일 버전, locking, approval workflow, audit, WebDAV, REST API                     | 첨부 파일 독립 관리, 승인/감사/파일 생명주기                  |
| BookStack    | 단순한 Books/Chapters/Pages, paragraph link, Markdown, OIDC/SAML2/LDAP, MFA             | 사용하기 쉬운 self-host 위키와 강한 인증 연동              |
| Wiki.js      | 모듈형 인증/댓글/검색/저장소, asset 관리, 개발자 확장성                                                  | 모듈식 아키텍처와 저장소/검색/인증 교체성                     |
| XWiki        | page versioning, attachments, rights management, REST API, extensions                | 엔터프라이즈 위키의 권한/확장/원격 API                     |
| Gollum       | Git-backed wiki, human-editable files, 다양한 markup, version/revert, local web UI      | Markdown 원본 관리를 위한 내부 Git 사용과 사용자 UI 분리 원칙  |

## 참고 링크

- Notion Wikis: https://www.notion.com/product/wikis
- Notion Enterprise Search: https://www.notion.com/product/enterprise-search
- Notion Developers: https://developers.notion.com/
- Outline: https://www.getoutline.com/
- Obsidian Links: https://help.obsidian.md/links
- Obsidian Graph View: https://help.obsidian.md/plugins/graph
- Obsidian Canvas: https://help.obsidian.md/plugins/canvas
- Obsidian Community Plugins: https://help.obsidian.md/Extending+Obsidian/Community+plugins
- AFFiNE: https://affine.pro/
- AFFiNE Whiteboard/Edgeless: https://affine.pro/whiteboard
- AFFiNE Teamhub: https://affine.pro/teamhub
- Confluence Features: https://www.atlassian.com/software/confluence/features
- Guru Features: https://www.getguru.com/features
- Guru Security: https://www.getguru.com/security
- Guru Integrations: https://www.getguru.com/integrations
- Document360 Features: https://document360.com/features/
- Redmine DMSF Plugin: https://www.redmine.org/plugins/redmine_dmsf
- BookStack: https://www.bookstackapp.com/
- Wiki.js Docs: https://docs.requarks.io/
- XWiki Features: https://www.xwiki.org/xwiki/bin/view/Documentation/UserGuide/Features/

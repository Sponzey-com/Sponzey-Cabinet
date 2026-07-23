# Sponzey Cabinet 프로젝트 최종 목표

작성일: 2026-06-22  
최종 갱신일: 2026-07-22
프로젝트명: Sponzey Cabinet  
문서 성격: 제품의 최종 목표와 사용자에게 제시할 기능 범위 정의. 실행 계획, 일정, 단계별 로드맵은 포함하지 않는다.

## 제품 정의

Sponzey Cabinet은 Outline, Notion, Obsidian, AFFiNE, Confluence, Document360, Guru, Redmine DMSF류 기능을 참고해 설계하는 차세대 개인용 Knowledge Base Solution이다. 현재의 최종 목표는 멀티 사용자 서버 제품이나 SaaS가 아니라, 개인 사용자가 자신의 개인 PC에 설치해 사용하는 설치형 지식 관리 솔루션이다. 핵심 목표는 "문서를 보관하는 위키"가 아니라, 개인의 문서, 관계, 파일, 업무 데이터, AI, 외부 시스템을 하나의 로컬 우선 지식 운영 계층으로 연결하는 것이다.

현재 제품 목표는 다음과 같이 고정한다.

- 현재 최종 목표: 개인 사용자의 개인 PC에 설치되는 단일 사용자 지식 관리 앱이다.
- 현재 개발 범위: 로컬 문서 저장소, 로컬 검색, 내부 버전 관리, 첨부 파일 관리, 문서 관계 그래프, AI 연동, 외부 도구 연동, 백업/복원, import/export, 모던한 개인용 UI/UX를 제공한다.
- 현재 제외 범위: 멀티 사용자, 서버 호스팅, 개인 호스팅 구축, SaaS, 멀티테넌트, 조직/워크스페이스 운영, 과금, 관리자 콘솔, 실시간 공동 편집, SSO/SCIM, 엔터프라이즈 감사 기능은 사용자의 명시적 요구가 있기 전까지 개발하지 않는다.
- 설계 기준: 서버 호스팅과 SaaS로 확장할 수 있는 Clean Architecture, 포트/어댑터 경계, 권한/동기화/이벤트 모델을 고려하되, 이를 현재 기능 범위로 구현하지 않는다.
- 사용자 경험 기준: 서버형 제품의 복잡한 관리 화면을 전면에 드러내지 않고, 개인 사용자가 설치 후 즉시 문서를 만들고, 찾고, 연결하고, 복원하고, AI로 질의할 수 있는 간결하고 모던한 로컬 앱 경험을 제공한다.

멀티 사용자, 서버 호스팅, SaaS 형태는 차후 목표다. 이 차후 목표는 현재 아키텍처가 막지 않아야 할 확장 방향일 뿐이며, 사용자의 명시적 요구가 있기 전까지 개발 계획, task, release gate, 기본 UI에 포함하지 않는다.

Sponzey Cabinet의 플랫폼 목표는 현재 검증 범위와 차후 목표를 분리한다.

- 현재 구현 및 릴리스 검증 플랫폼: macOS 데스크톱 설치형 앱
- 데스크톱 확장 대상: Windows, Linux. 공통 core와 adapter 경계는 유지하지만 사용자의 명시적 요구 전까지 package smoke와 release certification을 수행하지 않는다.
- 차후 공식 대상 플랫폼: Web, iOS, Android

플랫폼 지원은 같은 제품을 여러 번 따로 만드는 방식이 아니라, 공통 도메인/유스케이스/동기화/권한/검색/AI 계층 위에 플랫폼별 클라이언트 어댑터를 얹는 방식이어야 한다. 현재는 데스크톱 설치형 앱을 완성하고, Web/iOS/Android는 같은 core와 UI model을 재사용할 수 있는 차후 확장 대상으로 둔다. 문서 모델, 권한 모델, 첨부 파일 모델, 그래프 모델, 플러그인 모델, AI retrieval 모델은 플랫폼에 종속되지 않아야 한다.

플랫폼별 역할은 현재 목표와 차후 목표를 분리해 정의한다.

- macOS: 현재 최종 목표를 구현하고 검증하는 기준 클라이언트다. 로컬 문서 저장소, 로컬 검색, 내부 버전 관리, 첨부 파일 관리, Graph, Canvas, 오프라인 우선 사용, 로컬 백업/복원을 설치형 앱 안에서 제공한다.
- Windows/Linux: 장기 데스크톱 대상이다. 플랫폼 전용 규칙을 core에 넣지 않고 Tauri/platform adapter로 격리하되, 현재 완료 상태로 주장하지 않는다.
- Web: 현재 단계에서는 데스크톱 앱 내부 UI와 공유 가능한 React/CodeMirror UI 계층 또는 로컬 preview/development UI로 취급한다. 서버 호스팅/SaaS용 Web 클라이언트와 관리자 콘솔은 차후 목표다.
- iOS/Android: 현재 개발 범위가 아니라 차후 목표다. 현재 아키텍처는 모바일 클라이언트가 나중에 같은 문서/검색/AI API와 UI 모델을 재사용할 수 있도록 설계하되, 모바일 서버 접속, push notification, 승인 workflow는 사용자의 명시적 요구 전까지 구현하지 않는다.

## 문서 기능의 필수 제품 계약

기존 제품 목표와 기능 범위는 모두 유지한다. 그 위에 다음 네 가지를 개인용 로컬 제품의 필수 문서 기능으로 고정한다. 아래 항목은 선택 기능이나 차후 서버 기능이 아니며, macOS 개인용 로컬 앱의 문서 작성 흐름에서 사용자가 직접 확인할 수 있어야 한다.

### 1. 문서 작성과 첨부

- 사용자는 문서를 작성하거나 읽는 화면에서 파일을 선택해 해당 문서에 첨부할 수 있어야 한다.
- 파일 선택은 macOS platform picker 같은 UI adapter에서 수행하고, 유스케이스에는 검증된 파일 입력만 전달한다.
- 파일 원본은 문서 Markdown이나 내부 version store에 삽입하지 않고 content-addressed asset store에 저장한다. 문서는 안정적인 asset identity와 document association만 참조한다.
- 문서 화면은 해당 문서에 연결된 첨부 목록, 파일 종류, 크기, 상태, 연결 시점, 미리보기 가능 여부를 보여주고, 열기, 미리보기, 연결 해제를 제공한다.
- 첨부 연결 실패는 문서 본문이나 current version을 변경하지 않아야 하며, 완료되지 않은 document association과 고아 metadata를 남기지 않아야 한다.
- 문서에서 연결을 해제해도 다른 문서나 Canvas가 같은 asset을 참조하면 파일 원본을 삭제하지 않는다. 실제 원본 삭제는 참조 수 확인과 명시적 삭제 또는 보존 정책을 통과한 별도 생명주기로 처리한다.
- 첨부 참조가 버전 snapshot의 일부라면 복원 시 해당 시점의 association을 재현하되, asset 원본 자체를 중복 저장하거나 과거 원본을 자동 삭제하지 않는다.

### 2. 문서별 변경 비교

- 사용자는 문서 이력에서 현재 문서와 선택한 버전, 또는 서로 다른 두 버전을 비교할 수 있어야 한다.
- 비교 결과는 Markdown 원문을 기준으로 추가, 삭제, 동일한 줄을 결정적으로 표시한다. 줄 삽입 하나가 이후 모든 줄의 변경으로 오인되지 않도록 검증된 sequence diff 알고리즘을 사용하며, 첫 번째 물리적 줄의 변경은 문서 제목 변경으로도 이해할 수 있게 표시한다.
- 첨부 변경은 파일 bytes를 이진 비교하지 않고, 첨부 association의 추가, 제거, 교체와 metadata 변경 요약으로 표시한다.
- 내용이 같은 버전, 빈 문서, 매우 긴 줄, 줄바꿈 형식 차이, 한글과 Unicode가 포함된 문서에서도 결과 순서가 안정적이어야 한다.
- 일반 UI에는 내부 version ID, document ID, snapshot path, Git commit, branch, repository 같은 구현 정보를 노출하지 않는다. 사용자는 변경 시각, 변경 요약과 읽을 수 있는 버전 순서로 대상을 선택한다.

### 3. 문서 복원과 버전 변경

- 사용자는 복원 전에 선택한 버전과 현재 문서의 diff 및 첨부 association 변경 요약을 확인해야 한다.
- 복원은 과거 이력을 수정하거나 삭제하지 않는다. 선택한 과거 snapshot을 내용으로 하는 새 버전을 생성하고 그 새 버전을 current로 전환한다.
- 복원 직전 current version도 이력에 남아야 하므로, 사용자는 다시 비교하거나 별도 복원으로 복원 동작을 되돌릴 수 있어야 한다.
- 복원 command는 preview에서 확인한 expected current version과 idempotent operation identity를 받아야 한다. preview 이후 문서가 변경되었다면 쓰기 전에 `version conflict`를 반환하고 current document와 history를 변경하지 않는다. 같은 operation 재시도는 restore version을 중복 생성하지 않는다.
- 복원 성공은 본문, 첫 줄에서 파생된 제목, 링크/백링크, 검색/Graph projection과 첨부 association이 같은 새 version을 가리키고 durable readback이 완료된 뒤에만 확정한다.
- append, current 전환 또는 projection enqueue가 실패하면 부분 성공을 사용자에게 성공으로 표시하지 않는다. current document는 복원 전 상태를 유지한다. append가 이미 완료된 뒤 후속 단계가 실패했다면 operation journal에 `RecoveryRequired`를 기록하고 같은 operation identity로 재개하거나 실패 attempt를 비-current 상태로 종결한다.

### 4. 문서 이력의 조회와 사용자 경험

- 현재 문서 조회, 이력 목록 조회, 특정 버전 조회, diff 조회, restore preview와 restore command를 별도 유스케이스와 계약으로 분리한다.
- 표준 크기 문서의 이력, diff와 restore preview는 정상적인 로컬 인덱스와 cache 상태에서 p95 300ms 이내를 목표로 한다.
- 대용량 diff가 300ms 안에 완료될 수 없으면 전체 UI를 멈추지 말고 비동기 작업으로 전환한다. 작업 접수와 상태 조회는 p95 300ms 목표를 유지한다.
- 문서 저장, 첨부, 비교와 복원은 모두 로컬에서 동작하고 외부 서버, Git provider, Git CLI, 수동 설정 파일 또는 계정 로그인을 요구하지 않는다.
- Product Log에는 안정적인 이벤트 이름, 마스킹된 identity, 변경 줄 수, 첨부 변경 수, 처리 시간과 error code만 기록한다. 문서 본문, diff hunk 원문, 파일 bytes, 원본 파일명과 절대 경로는 기록하지 않는다.

## 현재 구현 기준선

현재 제품 기준선은 `personal_local_macos_desktop`이다. 이 기준선은 프로젝트 전체 장기 목표의 완료가 아니라, 개인 사용자가 실제로 확인할 수 있는 로컬 데스크톱 제품 범위의 구현 기준선을 뜻한다. 이전 phase archive와 task checkbox는 참고 자료일 뿐이며, 새 릴리스 판정에서는 현재 source fingerprint에서 재실행한 테스트와 실제 앱 동작을 완료 증거로 사용한다. 현재 로컬 `.tasks`에는 활성 `.tasks/plan.md`가 없고 phase archive만 남아 있으므로, 새 개발 목표가 생기면 먼저 활성 plan을 새로 작성해야 한다.

- React 기반 WYSIWYG/Live Preview 문서 작성 화면에서 새 문서 작성, 현재 문서 저장, `Cmd+S`, 제목/문단/체크리스트/표 셀 편집, 안전한 링크/첨부 참조 표시, CodeMirror 기반 `원문 편집` modal, 문서 첨부, 이력 조회, 버전 비교와 preview 기반 복원을 제공한다. 저장 canonical form은 계속 Markdown source다.
- 문서 제목은 별도 입력값이나 독립 metadata가 아니라 Markdown 본문의 첫 번째 물리적 줄에서 파생한다. 첫 줄의 Markdown heading marker를 제거해 표시하고, 유효한 문자가 없으면 `제목 없는 문서`를 사용하며, 사용자 UI와 신규 문서 생성 command에 별도 제목 입력을 두지 않는다.
- 본문 저장과 버전 복원은 같은 제목 파생 규칙으로 current metadata를 갱신한다. 저장 성공은 durable readback과 projection 동기화 뒤에 확정하고, Home, 문서 목록, Graph, Canvas와 연결 문서 표시는 같은 current title을 사용한다.
- Home, Document, Graph, Canvas, Assets, Backup은 같은 workspace shell을 공유한다. 좌측 하단 문서 바로가기는 route별 임시 결과나 현재 화면이 아니라 root에서 계산한 최근 문서 목록을 사용해야 하며, 메뉴 이동만으로 사이드바 문서 목록이 바뀌면 안 된다.
- Document 메뉴는 독립 검색 화면이 아니라 사용자가 마지막으로 작업하던 문서로 돌아가는 기본 진입점이어야 한다. 마지막 문서가 없을 때만 문서 선택 또는 빈 상태를 표시한다.
- 일반 사용자 UI에는 문서 파일명, 내부 document ID, version ID, asset ID, snapshot path, Git 용어를 기본 노출하지 않는다. 필요한 경우에도 사용자가 이해할 수 있는 제목, 변경 시각, 파일 종류, 크기, 상태, 복구 동작으로 표현한다.
- 현재 문서 조회와 이력 조회를 별도 query path로 유지하고, 저장 성공은 native durable write 뒤의 version/readback과 앱 재시작 결과로 확인한다.
- Wikilink와 Markdown link를 durable link index와 Graph projection에 반영하고, Graph 화면은 fixture나 최근 문서 추정 관계가 아니라 실제 projection의 node와 edge만 표시한다.
- Canvas 생성, 조회, node/edge/geometry/viewport 수정, 이름 변경, 보관과 손상 복구를 durable revision store에 저장한다.
- 첨부 파일은 문서에서 연결할 수 있고 platform picker, staging, content-addressed object store, metadata와 document association으로 분리 관리하며, 허용된 형식과 크기 범위에서 native preview를 제공한다. 연결 해제는 다른 참조가 남은 asset 원본을 삭제하지 않는다.
- backup/export/restore는 문서 current/history뿐 아니라 Canvas와 Asset metadata/object를 포함하고, 복원 뒤 durable readback과 reopen으로 결과를 확인한다.
- Graph, Canvas, Asset, projection 손상과 중단 상태는 빈 화면이나 성공 상태로 숨기지 않고 안정적인 error code와 recovery action으로 전달한다.
- 현재 문서, 이력, 검색, 링크, Graph, Canvas viewport, Asset metadata 조회는 표준 fixture의 release-mode 성능 검증에서 p95 300ms 목표를 충족한다.
- Product Log, Field Debug Log, Development Log를 분리하고 문서 본문, 첨부 bytes, 절대 경로와 비밀값을 릴리스 증거 및 운영 로그에서 제외한다.
- macOS packaged UI smoke는 Home, Document, Graph, Canvas, Assets, Backup/Restore와 recovery 흐름을 실제 `.app`에서 검증해야 한다. 최근 Penpot `20260721` UI fidelity archive인 `.tasks/phase004`에는 packaged UI initial/restart smoke, route UI regression, evidence contract와 selected native boundary test 통과 기록이 있다. 새 release gate에서는 같은 source fingerprint에서 필요한 검증 명령을 다시 실행해 완료를 확정한다.

현재 로컬 `.tasks`에는 이전 phase archive만 있고 활성 `.tasks/plan.md`는 없다. phase archive가 존재하더라도 새 source fingerprint의 테스트 결과와 사용자가 확인 가능한 앱 동작을 대체하지 않는다. Windows/Linux는 `deferred_future`이며 Web/iOS/Android, 서버 호스팅, SaaS, 멀티 사용자 기능은 현재 완료 범위가 아니다.

최종 제품은 사용자가 다음 문장으로 이해할 수 있어야 한다.

> Sponzey Cabinet은 내 개인 PC에 설치해 내 문서를 직접 소유하고, 모든 변경 이력이 자동으로 보존되며, AI와 외부 업무 시스템에 쉽게 연결할 수 있는 모던한 개인용 지식 관리 앱이다.

## 대체 대상과 차별화 방향

### Outline 대체

Outline은 빠른 문서 작성, Markdown 지원, 실시간 협업, 댓글, 강력한 검색/AI 질의응답, Slack 통합, 공개 공유, 권한/그룹, 자체호스팅 옵션을 강점으로 한다.

Sponzey Cabinet은 Outline의 장점을 기본값으로 삼되, 다음 한계를 넘어선다.

- 문서 본문을 내부 버전 관리 엔진의 일급 데이터로 관리하되, 사용자에게는 일반 문서 경험으로 제공한다.
- 문서 간 링크와 백링크를 그래프 데이터로 해석한다.
- 첨부 파일을 문서 내부 부속물이 아니라 별도 관리되는 디지털 자산으로 취급한다.
- CRM, 결제, 고객지원, 프로젝트 관리 같은 업무 기능을 문서에 플러그인 형태로 부착한다.
- 현재는 개인 로컬 사용을 완성하고, 개인 호스팅과 SaaS는 차후 확장 가능한 제품 모델로만 고려한다.
- AI 도구가 읽고, 쓰고, 질의하고, 동기화할 수 있는 API/MCP 계층을 기본 제공한다.

### Notion 대체

Notion은 위키, 문서, 프로젝트, 데이터베이스, 템플릿, 페이지 검증, Synced Blocks, 외부 연결, Enterprise Search, SAML SSO, SCIM, 고급 권한을 결합한다.

Sponzey Cabinet은 Notion의 사용성을 목표로 하되, 다음을 차별화한다.

- 데이터 소유권: 문서 원본은 표준 포맷과 내부 버전 관리 저장소로 보존되며, 사용자는 Git을 몰라도 데이터를 소유하고 내보낼 수 있다.
- 로컬 우선권: 개인 사용자가 벤더 종속 없이 자신의 PC에서 데이터를 소유하고 운영할 수 있다. 자체호스팅은 차후 목표로 분리한다.
- 관계형 지식 탐색: 문서 링크, 블록 링크, 첨부 파일, CRM 레코드, 외부 객체를 그래프로 탐색한다.
- 플러그인 중심 확장: Notion database가 모든 문제를 해결하는 방식이 아니라, 도메인 기능을 독립 플러그인으로 설치한다.
- AI 친화성: AI connector, MCP server, webhook, event stream, permission-aware retrieval을 제품 핵심 기능으로 둔다.

### Obsidian 대체

Obsidian은 Markdown 파일, Wikilink/Markdown link, 헤딩/블록 링크, 백링크, 전역/로컬 그래프, Canvas, 플러그인 생태계가 강하다.

Sponzey Cabinet은 Obsidian의 개인 지식관리 장점을 흡수하되, 다음을 더한다.

- 개인 지식관리 경험을 먼저 완성하고, 팀 협업과 SaaS는 같은 core model 위에서 차후 확장할 수 있게 한다.
- 그래프는 시각화에 머물지 않고 권한, 검색, 추천, AI context, CRM 관계 분석에 사용된다.
- Canvas류 자유 배치와 문서 본문, 데이터베이스 뷰를 동시에 제공한다.
- 버전 관리와 로컬 변경 복구를 제품 수준에서 다루며, Git 개념은 사용자 경험에 노출하지 않는다. 멀티 사용자 협업 충돌 해결은 차후 목표로 분리한다.

### AFFiNE 모드 지원

AFFiNE은 docs, whiteboards, databases, AI를 하나의 local-first workspace로 결합하고, Page Docs와 Edgeless Whiteboard를 Cloud와 Self-hosted 모두에서 제공하는 방향을 취한다.

Sponzey Cabinet의 AFFiNE 모드는 다음을 의미한다.

- 문서 모드: 일반 문서, 위키, 기술문서, 업무 기록을 구조적으로 작성한다.
- Edgeless 모드: 무한 캔버스에서 문서, 카드, 이미지, 첨부, CRM 객체, 외부 링크를 자유롭게 배치한다.
- 전환 가능성: 문서 페이지를 캔버스 노드로 놓거나, 캔버스의 카드 묶음을 문서 구조로 변환한다.
- 협업 확장 가능성: 현재는 개인 사용자의 문서 모드와 Edgeless 모드를 우선 제공한다. 개인 호스팅 이상에서의 실시간 공동 편집, 댓글, 멘션은 차후 목표로만 유지한다.
- 데이터 일관성: 문서와 캔버스는 별도 앱이 아니라 동일한 지식 그래프 위의 다른 뷰로 동작한다.

## 핵심 제품 원칙

### 1. 문서는 파일이면서 데이터다

문서는 사람이 읽는 Markdown/MDX/구조화 문서인 동시에, 시스템이 질의하고 연결할 수 있는 데이터여야 한다.

- 문서 원본은 Markdown/MDX 같은 텍스트 기반 포맷을 우선하며, 내부적으로 Git을 Markdown 원본 관리와 변경 이력 저장에 사용할 수 있다. 단, 일반 사용자는 commit, branch, repository 같은 Git 개념을 알 필요가 없어야 한다.
- 문서 제목은 Markdown 본문의 첫 번째 물리적 줄에서 결정하고 저장·복원 때마다 동일하게 파생한다. 제목을 별도 사용자 입력이나 본문과 독립된 수정 가능 metadata로 두지 않는다.
- slug, owner, 상태, 태그, 권한, 리뷰일, 관련 CRM 객체 같은 나머지 메타데이터는 제목 파생 규칙과 분리해 보존한다.
- 문서 내부의 헤딩, 블록, 체크리스트, 표, 코드, 임베드, 콜아웃, 첨부 참조를 구조화한다.
- 문서와 데이터베이스 레코드 사이의 경계를 낮춘다. 사용자는 문서를 쓰면서 CRM, 계약, 제품, 고객, 티켓, 결제 같은 업무 데이터를 연결할 수 있어야 한다.

### 2. 사용자는 자기 데이터를 소유한다

Sponzey Cabinet은 SaaS를 현재 목표로 전제하지 않는다.

- 개인 사용자는 로컬 또는 개인 저장소에 문서를 둘 수 있다.
- 현재 최종 목표는 개인 사용자가 개인 PC에 설치한 앱에서 데이터를 만들고, 저장하고, 검색하고, 백업하고, 복원하는 것이다.
- 개인 호스팅 사용자의 자체 인프라, 자체 도메인, 자체 인증, 자체 백업 정책은 차후 목표다.
- SaaS 사용자의 관리형 운영, 확장성, 백업, 과금, 조직 관리, 엔터프라이즈 보안은 차후 목표다.
- 현재 개발은 차후 서버/SaaS 전환을 막지 않는 내부 경계를 유지하되, 서버/SaaS 기능 자체를 만들지 않는다.
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

## 현재 개인용 UI/UX 목표

현재 최종 목표의 UI/UX는 개인 사용자가 매일 사용하는 설치형 생산성 앱이어야 한다. 서버형 관리자 도구, 엔터프라이즈 콘솔, SaaS 설정 화면처럼 보이면 안 된다.

현재 UI/UX는 다음 기준을 만족해야 한다.

- 첫 실행 경험: 설치 후 앱을 열면 별도 서버 주소, DB 정보, tenant, organization, administrator 설정 없이 바로 개인 workspace를 만들 수 있어야 한다.
- 홈 화면: 최근 문서, 즐겨찾기, 태그, 미완료 작업, 최근 변경, 빠른 검색, AI 질의 진입점을 한 화면에서 제공한다.
- 문서 작성 화면: 기본 화면은 WYSIWYG/Live Preview 편집을 제공하고, Markdown source가 필요한 사용자는 `원문 편집` modal에서 CodeMirror 기반 plain text editor를 사용한다. 두 editor는 같은 canonical Markdown body를 공유해야 한다.
- 제목 편집 경험: 사용자는 별도 제목 입력란을 사용하지 않고 문서 첫 줄을 편집한다. 저장 뒤 파생된 제목은 편집 화면, Home, 문서 탐색, 검색, Graph, Canvas, 첨부 연결 문서에서 일관되게 표시되어야 한다.
- 좌측 사이드바 경험: route 전환은 좌측 하단 문서 목록의 의미를 바꾸지 않는다. 이 영역은 최근 문서 바로가기이며, Search route의 결과 목록, Graph의 중심 문서, Canvas의 선택 노드, Assets의 선택 첨부에 따라 바뀌면 안 된다.
- Document 메뉴 경험: Home에서 문서를 선택해 편집한 뒤 다른 메뉴로 갔다가 Document를 누르면 마지막 작업 문서를 다시 보여준다. 사용자가 명시적으로 검색을 선택하거나 상단 검색을 실행할 때만 검색 결과 화면으로 이동한다.
- 검색 경험: 상단 검색은 장식 버튼이 아니라 문서 검색 route를 여는 명시적 입력이어야 한다. 검색어 입력, 결과 목록, 결과에서 문서 열기, `Esc` 또는 뒤로 가기 시 이전 작업 문서 복귀가 일관되게 동작해야 한다.
- Preview 경험: 표, 체크리스트, 코드 블록, 콜아웃, 다이어그램, 첨부 참조, wikilink가 읽기 좋은 형태로 보인다.
- 탐색 경험: 좌측에는 문서 트리/컬렉션/태그, 중앙에는 문서 편집기, 우측에는 백링크/첨부/AI citation/문서 metadata를 배치할 수 있어야 한다.
- 그래프 경험: 사용자는 Obsidian처럼 문서 관계를 볼 수 있어야 하며, AFFiNE처럼 자유 배치 canvas에서 문서와 첨부를 카드로 다룰 수 있어야 한다.
- 검색 경험: 단축키 기반 빠른 검색, 전체 텍스트 검색, 태그/상태/첨부 필터, 최근 검색, 검색 결과에서 바로 AI 질의를 제공한다.
- 이력 경험: 사용자는 Git을 몰라도 문서 history, diff, restore preview, 복원 버튼만으로 변경 이력을 이해할 수 있어야 한다.
- 첨부 경험: 첨부는 문서 하단 부속 파일이 아니라 별도 asset panel에서 버전, 위치, 참조 문서, 미리보기, OCR/search 상태를 확인할 수 있어야 한다.
- AI 경험: AI 답변은 원문을 대신하는 블랙박스가 아니라 citation, 문서 링크, 접근 가능 여부, 최신성 표시를 함께 보여야 한다.
- 설정 경험: 기본 설정은 자동으로 끝나야 하며, 고급 설정은 `백업`, `저장 위치`, `AI provider`, `import/export`, `개발/Field Debug`처럼 사용자가 이해할 수 있는 범주로 숨겨야 한다.
- 오류 경험: 로컬 store 초기화 실패, 검색 index 손상, 백업 실패, AI provider 실패는 복구 가능한 메시지와 재시도 버튼을 제공해야 한다.
- 차후 확장 표시: 서버 접속, 팀 초대, 조직 관리자, 과금, SSO 같은 항목은 현재 기본 UI에 노출하지 않는다. 해당 기능은 사용자가 명시적으로 차후 목표를 요청한 뒤에만 설계/구현한다.
- 시각 스타일: 개인용 생산성 앱답게 조용하고 정돈된 화면 밀도, 빠른 keyboard navigation, 명확한 icon button, 좁은 화면에서도 깨지지 않는 responsive layout을 제공한다.
- 성능 체감: 현재 문서 조회, 링크/백링크, 첨부 metadata, 검색 결과는 정상 인덱스 상태에서 p95 300ms 이내를 목표로 하고, 로딩 UI는 성능 문제를 숨기는 용도로 사용하지 않는다.

## 기술 스택과 아키텍처 목표

Sponzey Cabinet은 Rust, React, CodeMirror, Tauri를 중심 기술로 사용한다.

- Rust: 도메인 모델, 유스케이스, 내부 버전 관리, 문서 파서, 링크/그래프 처리, 첨부 파일 처리, 로컬 검색 인덱싱, 권한 정책, 상태머신, 로컬 앱 runtime의 기준 언어다. 서버 API와 협업 backend는 차후 목표를 위한 확장 지점으로만 설계한다.
- React: 현재 데스크톱 앱 UI의 기준 계층이며, 차후 Web과 mobile WebView UI까지 공유할 수 있는 공통 UI 계층이다.
- CodeMirror: Markdown/MDX 문서 편집기의 기준 editor engine이다. CodeMirror는 편집 UI로 사용하고, 문서 모델과 도메인 규칙의 소유자가 되어서는 안 된다.
- Tauri: 현재 macOS 데스크톱 앱의 shell과 platform adapter 계층이며 Windows/Linux 확장의 공통 기반이다. Windows/Linux native 인증과 iOS/Android Tauri mobile은 차후 목표다. Tauri는 core domain이 아니라 파일시스템, 보안 저장소, 알림, OS 통합, local process lifecycle을 연결하는 경계 계층이어야 한다.

핵심 아키텍처 원칙은 다음과 같다.

- Web과 앱은 같은 React/CodeMirror UI 패키지를 공유할 수 있게 설계한다.
- 현재 기본 제품은 Tauri 데스크톱 앱이며, 로컬 workspace에서는 Rust core를 in-process로 호출한다.
- Web은 현재 단계에서 로컬 UI 재사용, development preview, future server client를 위한 UI 계층으로 취급한다. Rust server API를 통한 원격 workspace 접근은 차후 목표다.
- iOS/Android 앱은 차후 목표다. 현재는 같은 domain/usecase/client-core 계약을 재사용할 수 있도록 capability model만 고려한다.
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
    future SaaS/self-host server connector

  desktop/
    Tauri shell
    React app
    CodeMirror editor
    local workspace adapter
    future remote workspace adapter
    platform filesystem adapter
    platform secure storage adapter
    platform notification adapter

  mobile/
    Tauri mobile shell
    React app
    CodeMirror editor
    future remote workspace adapter
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
    future HTTP API
    future realtime gateway
    future background workers
    future server composition root

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
- 현재 제품에서 Tauri app의 로컬 workspace만 로컬 문서 저장소를 원본으로 사용할 수 있다.
- Tauri command는 얇은 adapter로 유지하고, 비즈니스 규칙을 포함하지 않는다.
- 모바일 앱과 서버 workspace 클라이언트는 차후 목표이며, 모바일 오프라인 편집은 future capability로 분리한다.

### 차후 서버 구조

서버 구조는 현재 개발 범위가 아니다. 멀티 사용자, 개인 호스팅, SaaS, 실시간 공동 편집, 수평확장 서버는 사용자의 명시적 요구가 있기 전까지 구현하지 않는다.

다만 현재 로컬 앱의 도메인/유스케이스/포트/어댑터 경계는 차후 개인 호스팅과 SaaS를 막지 않도록 설계한다. 아래 서버 구조는 future architecture reference이며, 현재 task, release gate, 기본 UI의 필수 범위가 아니다.

차후 서버는 개인 호스팅과 SaaS를 모두 지원해야 하며, 문서 협업과 검색/AI 부하를 고려해 수평확장 가능한 구조여야 한다.

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

### 차후 문서 협업 아키텍처

문서 협업은 현재 개발 범위가 아니다. 현재 제품은 개인 사용자의 로컬 편집, 자동 저장, 변경 이력, 복원, 충돌 없는 단일 사용자 경험을 우선한다.

차후 멀티 사용자 협업을 개발할 때 문서 협업은 단순 저장 API가 아니라 realtime operation pipeline으로 관리해야 한다.

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

차후 협업 기능은 다음 규칙을 따른다.

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

차후 협업 상태머신은 다음 개념을 가져야 한다.

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

### 차후 수평확장 아키텍처

수평확장은 현재 개발 범위가 아니다. 현재 제품은 개인 PC 설치형 로컬 앱이므로 외부 API replica, collaboration shard, worker pool, external session/cache store를 요구하지 않는다.

차후 서버 호스팅 또는 SaaS 요구가 명시되면 수평확장은 stateless API와 stateful collaboration room을 분리하는 방식으로 달성한다.

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

### 차후 서버 저장소 구조

서버 저장소는 현재 개발 범위가 아니다. 현재 제품은 로컬 metadata store, local internal version store, local asset store, local search index를 앱이 자동 초기화하고 관리한다.

차후 서버 저장소는 원본 데이터, projection, cache를 분리해야 한다.

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

현재 배포 구조의 최종 목표는 개인 PC 설치형 앱이다. 개인 호스팅과 SaaS는 같은 core를 재사용할 수 있는 차후 목표로 유지하지만, 사용자의 명시적 요구가 있기 전까지 구현하지 않는다.

현재 목표: 개인 PC 설치형 앱

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

차후 목표: 개인 호스팅

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

차후 목표: SaaS

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

차후 배포 모델은 다음 기준을 따른다.

- 개인 호스팅과 SaaS는 현재 로컬 앱의 domain/usecase/port contract를 재사용해야 한다.
- 서버 배포를 위해 현재 로컬 앱에 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 환경 변수 설정을 요구하지 않는다.
- 서버/SaaS 전용 UI, 관리자 콘솔, 조직 관리, 과금, SSO, 멀티테넌트 설정은 현재 기본 UI에 노출하지 않는다.
- 현재 UI는 개인 사용자의 문서 작성, 탐색, 그래프, 첨부, 검색, AI 질의, 백업/복원을 중심으로 구성한다.

### 성능과 안정성 목표

- 문서 조회는 현재 문서 조회와 이력 조회를 명확히 분리해야 한다.
- 현재 문서 조회는 최신 snapshot과 metadata를 기준으로 응답해야 한다.
- 이력 조회는 version history, diff, restore preview, 특정 시점 snapshot을 기준으로 응답해야 한다.
- diff 조회는 현재 문서 대 특정 버전과 특정 버전 대 특정 버전을 구분하고, 파일 원본의 binary diff가 아니라 Markdown 줄 변경과 첨부 association 변경 요약을 반환해야 한다.
- restore preview는 대상 version과 preview 당시 current version을 식별해야 하며, restore command는 그 current version을 optimistic concurrency guard로 사용해야 한다.
- 문서 복원은 기존 version을 덮어쓰지 않고 새 version을 append한 뒤 current를 전환해야 한다. 복원 실패나 version conflict에서는 복원 전 current를 보존해야 한다.
- 현재 문서 조회 경로는 이력 저장소 전체를 스캔하지 않아야 한다.
- 이력 조회 경로는 현재 문서 조회 성능을 저하시키지 않도록 별도 query path와 pagination을 가져야 한다.
- 모든 사용자-facing 검색과 조회는 정상적인 인덱스 상태에서 p95 300ms 이내 응답을 목표로 해야 한다.
- 300ms 기준은 문서 현재 조회, 문서 이력 목록 조회, 특정 버전 metadata 조회, 표준 크기 문서의 diff/restore preview, 폴더/컬렉션 목록 조회, 링크/백링크 조회, 첨부 metadata 조회, 권한 필터링이 적용된 검색에 적용한다.
- AI 답변 생성, OCR, embedding, 대용량 export, 대용량 첨부 preview 생성처럼 본질적으로 비동기인 작업은 300ms 기준의 직접 대상이 아니다. 단, 작업 상태 조회와 캐시된 결과 조회는 300ms 이내를 목표로 해야 한다.
- 문서 읽기 경로는 metadata, current snapshot, permission decision, search/graph projection을 활용해 빠르게 응답해야 한다.
- 문서 쓰기 경로는 operation validation, permission check, durable append, broadcast를 우선하고, 검색/그래프/AI indexing은 비동기로 처리해야 한다.
- 현재 로컬 편집은 단일 사용자 operation ordering을 보장해야 한다.
- 협업 편집의 문서별 room ordering과 대형 workspace의 collaboration room/worker queue 분산은 차후 서버 목표다.
- 대용량 첨부 처리는 API server memory에 파일 전체를 적재하지 않아야 한다.
- AI embedding, OCR, preview 생성은 background worker로 격리해야 한다.
- 장애 복구는 event log, snapshot, version store를 기준으로 수행해야 한다.
- 차후 서버 node가 사라져도 문서 원본, version history, 확정된 operation은 손실되면 안 된다.
- Product Log는 현재 로컬 앱에서 workspace 초기화, 문서 저장/복원, 검색/AI 실패, 백업/복원 실패를 추적하고, 차후 서버에서는 협업 session, document room ownership, worker failure, indexing lag, queue backlog를 추적할 수 있어야 한다.
- Field Debug Log는 현재 로컬 workspace/document 범위로 문제를 진단하고, 차후 서버에서는 특정 workspace/document/session 범위로 협업 문제를 진단할 수 있어야 한다.
- Product Log와 운영 metric은 검색/조회 latency, p95, p99, index freshness, cache hit rate, query timeout을 추적할 수 있어야 한다.

## 사용자에게 제시할 핵심 기능

### 0. 공식 대상 플랫폼

- 현재 구현 및 인증: macOS 데스크톱 클라이언트
- 차후 native 인증: Windows 데스크톱 클라이언트
- 차후 native 인증: Linux 데스크톱 클라이언트
- 현재 목표: 데스크톱 설치형 로컬 workspace
- 현재 목표: 데스크톱 앱 내부에서 공유 가능한 React/CodeMirror UI
- 차후 목표: Web 클라이언트
- 차후 목표: iOS 모바일 클라이언트
- 차후 목표: Android 모바일 클라이언트
- 차후 목표: 플랫폼 간 동일 workspace 접근
- 플랫폼 간 문서/첨부/그래프/권한 모델 일관성
- 현재 목표: 로컬 workspace 생성, 열기, 백업, 복원, import/export
- 차후 목표: 데스크톱 로컬 workspace와 서버 workspace 전환
- 차후 목표: 모바일 문서 조회/편집/검색/댓글/승인 workflow
- 차후 목표: Web 기반 관리자 콘솔과 협업 관리
- 플랫폼별 네이티브 기능은 어댑터 계층에서만 처리

### 1. 문서 작성과 위키

- 빠른 Markdown 기반 편집
- 문서 첫 번째 줄을 제목으로 사용하는 단일 제목 규칙. Markdown heading marker는 표시 제목에서 제거하고 빈 첫 줄은 `제목 없는 문서`로 처리한다.
- 별도 제목 입력 없이 본문 저장과 복원 결과를 기준으로 모든 화면의 문서 제목을 갱신한다.
- 현재 구현: React 기반 WYSIWYG/Live Preview 기본 편집 화면과 CodeMirror 기반 `원문 편집` modal을 제공한다.
- WYSIWYG 구현 기준: 제목/문단, 체크리스트, 표 셀은 기본 화면에서 직접 수정한다. Wikilink, Markdown link, 첨부 참조는 안전한 chip으로 표시하고, code block, blockquote/callout, unsupported Markdown block은 원문 marker를 일반 화면에 노출하지 않은 채 `원문에서 편집` fallback을 제공한다.
- 저장 기준: WYSIWYG는 별도 문서 모델이 아니며 canonical 저장 형식은 Markdown source다. WYSIWYG patch와 plain text source edit는 같은 body state에 수렴하고, stale patch는 body를 변경하지 않고 원문 편집 fallback으로 이어진다.
- 구현 경계: WYSIWYG/Live Preview 기능은 editor presentation adapter, React UI boundary와 CodeMirror source modal 경계에서 처리하며 domain/usecase에는 UI 렌더링 규칙을 넣지 않는다.
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
- preview에서 확인한 current version을 기준으로 한 충돌 방지
- 과거 snapshot으로 새 version을 생성하는 비파괴 복원
- 문서 화면에서 파일 첨부, 첨부 목록 조회, 미리보기/열기와 연결 해제
- 문서 diff에서 첨부 association 추가/제거/교체 요약
- 현재 목표: 로컬 workspace 안의 문서 즐겨찾기, 최근 문서, 태그 기반 탐색
- 차후 목표: 문서 공개 링크와 비공개 공유 링크
- 차후 목표: 문서별 댓글, 인라인 댓글, 멘션, 해결됨 상태
- 차후 목표: 문서 변경 알림, 구독, watcher
- 문서 가져오기: Markdown, HTML, PDF/Word 변환, Notion/Confluence/Outline export
- 문서 내보내기: Markdown, HTML, PDF, ZIP, 원본 문서 패키지, static site source

### 2. 차후 협업 기능

협업 기능은 현재 개발 범위가 아니다. 현재 제품은 개인 사용자의 로컬 편집, 자동 저장, 변경 이력, 복원, 백업/복원, AI 질의, 그래프 탐색을 우선한다.

개인 호스팅과 SaaS가 사용자의 명시적 요구로 개발될 때 협업 기능은 다음을 목표로 한다.

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
- 문서 history, 현재 대 과거 version diff, 과거 version 간 diff, 작성자 추적과 restore를 문서 UI에서 제공한다.
- 현재 문서 조회와 이력 조회는 UI/API에서 명확히 구분한다.
- 현재 문서 조회는 latest snapshot을 기준으로 빠르게 응답하고, 이력 조회는 version entry와 snapshot/diff를 기준으로 별도 처리한다.
- diff는 Markdown 줄 단위 변경과 첨부 association 변경을 구분한다. 첨부 파일 bytes의 binary diff는 기본 문서 diff 범위에 포함하지 않는다.
- 복원 전에는 현재 문서와 대상 version의 차이를 preview하고, preview 당시 current version이 바뀌면 복원을 거부한다.
- 복원은 선택한 snapshot을 내용으로 하는 새 version entry를 append하고 그 version을 current로 전환한다. 기존 version entry와 복원 직전 current는 삭제하거나 덮어쓰지 않는다.
- 복원 성공은 본문, 첫 줄 파생 제목, 링크/검색/Graph projection, 첨부 association과 durable readback이 새 version에 일치한 뒤 확정한다.
- 사용자 UI에는 내부 version ID, snapshot path나 Git 용어를 표시하지 않고 변경 시각, 변경 요약, 비교와 복원 action만 제공한다.
- 문서 초안, 승인, 배포는 사용자에게 draft, review, published 같은 상태로 보인다.
- 리뷰와 승인은 문서 워크플로로 제공하며, 코드 저장소형 리뷰 절차로 제공하지 않는다.
- 개인 로컬 저장소와 원격 백업 저장소를 연결할 수 있다.
- 문서와 첨부 파일은 분리 관리하되, 문서에서 첨부의 content-addressed ID 또는 asset ID를 참조한다.
- 대용량 첨부는 내부 버전 관리 저장소에 직접 넣지 않고 별도 asset store에 두며, 필요 시 외부 object storage와 연결한다.

### 4. 첨부 파일 관리

첨부 파일은 Redmine DMSF(Document Management System Features)처럼 문서 부속물이 아니라 별도 생명주기를 가진 관리 대상이어야 한다.

현재 개인용 로컬 제품은 다음 동작을 필수로 제공한다.

- 문서 작성/조회 화면에서 platform file picker로 파일을 선택하고 현재 문서에 연결
- 파일 검증, staging, content-addressed 원본 저장, metadata 기록, document association 생성을 순서가 명시된 상태머신으로 처리
- 문서별 첨부 목록과 workspace asset 목록을 같은 asset identity로 조회
- 지원 형식의 bounded preview, 원본 열기, 문서 연결과 연결 해제
- 연결 실패 시 고아 association과 부분 저장을 남기지 않는 보상 또는 재개 가능한 실패 상태
- 연결 해제 시 다른 문서나 Canvas의 참조를 보존하고, 참조가 남은 원본을 삭제하지 않는 참조 무결성
- 문서 이력 비교와 복원에서 첨부 association snapshot의 추가/제거/교체를 추적

다음 고급 파일 관리 항목은 기존 장기 목표로 유지하되, 멀티 사용자 승인과 서버 저장소가 필요한 항목은 사용자의 명시적 범위 변경 전까지 현재 release gate에 포함하지 않는다.

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
- 현재 로컬 앱에서 동작하는 배포 방식
- 차후 self-host/SaaS에서도 재사용 가능한 extension metadata와 migration boundary

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

Sponzey Cabinet은 현재 개인용 로컬 앱이지만, 차후 서버/SaaS 확장을 막지 않기 위해 권한 모델의 개념 경계를 보존해야 한다.

현재 목표:

- 로컬 owner profile
- 로컬 workspace 잠금 또는 앱 잠금
- 문서/폴더/첨부/캔버스의 내부 permission metadata 보존
- AI retrieval이 로컬 workspace의 접근 정책을 우회하지 않도록 하는 permission-aware query boundary
- local API/MCP/tool scope의 읽기/쓰기 범위 구분
- 로컬 감사 이벤트와 변경 이력
- 민감 정보 redaction hook의 내부 경계

차후 목표:

- 사용자, 그룹, 팀, 조직, 워크스페이스
- role 기반 권한: owner, admin, editor, reviewer, viewer, guest, service account
- 문서/폴더/컬렉션/첨부/CRM 객체/캔버스 단위 권한
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
- enterprise DLP/redaction hook

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

## 배포 모델별 기능 범위

### 현재 최종 목표: 개인 PC 설치형

- 단일 사용자
- macOS 데스크톱 클라이언트
- Windows/Linux desktop adapter 호환성 유지. native package와 release certification은 차후 목표다.
- React/CodeMirror 기반 모던 데스크톱 UI
- Web 기반 local preview UI 또는 앱 내부 UI 재사용
- 로컬 문서 저장소
- 자동 변경 이력과 복원
- 현재 문서 조회와 이력 조회 분리
- 현재 대 과거 및 과거 대 과거 문서 diff
- preview, optimistic concurrency guard와 새 version append를 사용하는 비파괴 문서 복원
- 문서 링크/백링크/그래프
- AFFiNE/Canvas형 개인용 관계도와 자유 배치
- 문서 화면에서 파일 첨부와 연결 해제, 문서별 첨부 목록, 로컬 asset 원본 관리
- 로컬 검색
- 검색/조회 p95 300ms 목표
- AI provider 직접 연결
- AI 답변의 출처/인용/최신성 표시
- local API/MCP/tool scope
- 플러그인 일부 지원
- export/import 완전 지원
- 설치 1회 후 기본 workspace 생성
- 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 환경 변수, 설정 파일 직접 편집 불필요
- 로컬 백업/복원
- 개인 사용자가 서버, tenant, organization, billing, admin console 개념을 몰라도 사용할 수 있는 UI/UX

현재 제외:

- 멀티 사용자
- 실시간 공동 편집
- 서버 배포
- 조직/워크스페이스 관리자 콘솔
- SSO/SCIM
- 과금/구독
- 멀티테넌트

### 차후 목표: 개인 호스팅 구축

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

### 차후 목표: SaaS 서비스

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
- 현재 문서와 과거 버전의 차이를 읽기 쉬운 diff로 확인하고, 기존 이력을 잃지 않은 채 원하는 버전으로 되돌릴 수 있다.
- 문서에서 파일을 바로 첨부하되 원본은 별도 자산으로 안전하게 관리하고 여러 문서와 Canvas에서 재사용할 수 있다.
- 문서 간 관계를 그래프로 탐색한다.
- AI를 내 문서에 연결해 검색, 요약, 정리를 자동화한다.
- 설치 한 번으로 개인 PC에서 바로 사용할 수 있다.
- Notion처럼 보기 좋고 편한 UI/UX를 가지되, 서버 계정이나 조직 관리 없이 로컬에서 동작한다.
- 나중에 명시적으로 원할 때 개인 서버나 SaaS 조직으로 확장할 수 있는 데이터 모델과 export 경로를 가진다.

### 차후 목표: 소규모 팀

- Outline처럼 빠르게 문서를 쓰고 협업한다.
- Notion처럼 문서와 업무 데이터를 함께 관리한다.
- 내부적으로 신뢰 가능한 버전 관리가 동작해 변경 이력과 복원을 믿을 수 있다.
- 자체호스팅으로 비용과 데이터 통제를 확보한다.
- Slack/Teams/Jira/CRM과 연결한다.

### 차후 목표: 기업/조직

- Confluence처럼 팀 단위 지식 허브를 만든다.
- Guru처럼 검증된 지식을 AI 답변으로 제공한다.
- Document360처럼 내부/외부 지식베이스와 고객지원 흐름을 지원한다.
- Redmine DMSF처럼 첨부 파일을 승인/감사/버전 관리한다.
- RBAC, SSO, SCIM, 감사 로그, DLP, 데이터 보존 정책을 지원한다.
- SaaS 또는 자체호스팅 중 조직 정책에 맞는 운영 방식을 선택한다.

## 참고 솔루션에서 반영한 기능

| 참고 솔루션       | 확인한 강점                                                                               | Sponzey Cabinet에 반영할 목표                                          |
| ------------ | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| Notion       | 위키, 페이지 링크, 검증, Synced Blocks, Enterprise Search, 외부 연결, SAML/SCIM                   | 쉬운 문서 UX, 최신성 관리, AI 검색, 권한/SSO, 데이터베이스형 문서                      |
| Outline      | Markdown, 실시간 협업, AI Q&A, Slack, 공개 공유, 자체호스팅, open API                              | 현재는 빠른 개인 문서 작성 UX와 API 친화성만 반영하고, 팀 위키/self-host/협업은 차후 목표로 둔다  |
| Obsidian     | Markdown vault, Wikilink/Markdown link, 헤딩/블록 링크, 그래프, Canvas, 플러그인                  | 개인 지식관리, 관계 그래프, 자유 캔버스, 로컬 우선                                   |
| AFFiNE       | docs + whiteboard + database + AI, local-first, Cloud/Self-hosted, Edgeless          | 현재는 문서 모드와 Edgeless 모드 통합 및 로컬 우선 UX를 반영하고, 캔버스 협업은 차후 목표로 둔다    |
| Confluence   | 실시간 편집, 댓글, whiteboards, databases, Jira 연동, page versioning, RBAC/SSO               | 현재는 page versioning과 지식 구조화만 반영하고, 엔터프라이즈 협업/RBAC/SSO는 차후 목표로 둔다 |
| Guru         | permission-aware AI answers, verification, duplicate detection, MCP/API, Slack/Teams | AI 답변, 검증 워크플로, 업무 흐름 내 지식 전달                                    |
| Document360  | 내부/외부 KB, workflow, SSO/SCIM, API, media storage, AI search/chatbot, MCP             | 고객지원 지식베이스, API/MCP, 분석, ticket deflection                       |
| Redmine DMSF | 디렉터리, 파일 버전, locking, approval workflow, audit, WebDAV, REST API                     | 첨부 파일 독립 관리, 승인/감사/파일 생명주기                                       |
| BookStack    | 단순한 Books/Chapters/Pages, paragraph link, Markdown, OIDC/SAML2/LDAP, MFA             | 현재는 단순한 문서 구조와 쉬운 탐색을 반영하고, self-host 위키/강한 인증 연동은 차후 목표로 둔다     |
| Wiki.js      | 모듈형 인증/댓글/검색/저장소, asset 관리, 개발자 확장성                                                  | 모듈식 아키텍처와 저장소/검색/인증 교체성                                          |
| XWiki        | page versioning, attachments, rights management, REST API, extensions                | 엔터프라이즈 위키의 권한/확장/원격 API                                          |
| Gollum       | Git-backed wiki, human-editable files, 다양한 markup, version/revert, local web UI      | Markdown 원본 관리를 위한 내부 Git 사용과 사용자 UI 분리 원칙                       |

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

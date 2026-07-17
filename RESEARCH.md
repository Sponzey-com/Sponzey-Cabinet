# 지식관리 솔루션 리서치

조사일: 2026-06-22  
제품 반영 검토일: 2026-07-16
범위: 사내 위키, 문서 협업, AI 검색/답변, 고객지원 지식베이스, 엔지니어링 Q&A, 자체호스팅 옵션  
주의: 가격은 공식 페이지에 노출된 공개 가격 기준이며, 지역, 연간/월간 결제, 할인, 엔터프라이즈 계약에 따라 달라질 수 있다.

## Sponzey Cabinet 반영 결정

이 문서는 시장과 OSS 후보를 비교한 조사 기록이며, 제품 요구사항의 최종 기준은 `PROJECT.md`다. 기존 조사 결과와 프로젝트 목표는 유지하고, 참고 제품의 attachment, page history, diff와 revert 경험을 현재 macOS 개인용 로컬 제품에 다음과 같이 반영한다.

- 사용자는 문서 화면에서 파일을 첨부한다. 파일 원본은 Markdown과 분리된 asset store에 두고 문서는 stable association으로 참조한다.
- 사용자는 현재 문서와 과거 version 또는 두 과거 version을 문서별로 비교한다. Markdown 줄 변경과 첨부 association 변경을 구분하고 파일 bytes의 binary diff는 기본 범위에서 제외한다.
- 사용자는 diff preview를 확인한 뒤 과거 version으로 복원한다. 복원은 과거 이력을 덮어쓰지 않고 대상 snapshot을 내용으로 하는 새 version을 생성한다.
- preview 이후 current version이 변경되면 쓰기 전에 복원을 거부하고, 첨부 실패나 복원 중간 실패는 current document와 기존 참조를 훼손하지 않으며 idempotent operation으로 재개한다.
- 내부 document/version/asset ID, 파일 경로와 Git 용어는 일반 사용자 UI에서 숨긴다.
- 서버 호스팅, SaaS와 멀티 사용자 기능은 비교 대상으로만 유지하며 사용자의 명시적 요구 전까지 현재 개발 범위로 전환하지 않는다.

## 요약 추천

지식관리 솔루션은 "문서를 잘 쌓는 도구"와 "쌓인 지식을 업무 중 바로 꺼내 쓰는 도구"로 나뉜다. 현재 시장은 AI 검색, 권한 상속, 최신성 검증, Slack/Teams/Jira/GitHub 같은 업무 도구 연동이 핵심 경쟁 포인트다.

| 상황                          | 우선 검토 솔루션                               | 이유                                                            |
| --------------------------- | --------------------------------------- | ------------------------------------------------------------- |
| 스타트업/소규모 팀의 사내 위키와 프로젝트 문서  | Notion, Slab, Outline                   | 도입 장벽이 낮고 문서 작성/검색/공유 경험이 좋다.                                 |
| Jira/Atlassian 중심의 제품/개발 조직 | Confluence                              | Jira와 함께 쓰는 스펙, 회고, 의사결정 문서 관리에 강하다.                          |
| Microsoft 365를 이미 쓰는 조직     | SharePoint + Microsoft 365 Copilot      | 기존 파일 권한, Teams, OneDrive, Microsoft 365 보안/거버넌스와 잘 맞는다.      |
| 고객지원/헬프센터/외부 공개 문서          | Document360, Helpjuice                  | 공개/비공개 KB, 다국어, 분석, 티켓 디플렉션, AI 챗봇에 초점이 있다.                   |
| 세일즈/CS/운영팀의 검증된 답변 자동화      | Guru                                    | 지식 검증, 사용 흐름 내 AI 답변, Slack/Teams/Salesforce/Zendesk 연동에 강하다. |
| 개발자 Q&A와 기술 지식 축적           | Stack Internal                          | Stack Overflow 방식의 Q&A, 투표/검증, 기술 조직용 지식 허브에 특화되어 있다.         |
| 비용 통제/자체호스팅/오픈소스            | BookStack, Outline self-hosted, Wiki.js | 인프라 운영 역량이 있으면 라이선스 비용과 데이터 통제 측면에서 유리하다.                     |
| 국내 협업툴과 한국어 지원, 공공/금융 맥락    | Dooray!                                 | 프로젝트, 메신저, 메일, 위키를 묶은 올인원 협업툴이며 국내 인증/계약 환경에 맞다.              |

## 평가 기준

- 검색 품질: 키워드 검색뿐 아니라 의미 기반 검색, AI 답변, 출처 표시, 권한 반영 여부
- 최신성 관리: 문서 소유자, 검증 배지, 만료/리뷰 워크플로, 변경 이력
- 구조화: 팀스페이스, 태그, 컬렉션, 페이지 계층, 템플릿, 메타데이터
- 작성 경험: WYSIWYG/Markdown, 공동 편집, 댓글, 임베드, 외부 공유
- 연동: Slack, Microsoft Teams, Jira, GitHub, Google Drive, Salesforce, Zendesk, API/MCP
- 보안/관리: SSO, SCIM, 감사 로그, DLP/SIEM, 권한 상속, 데이터 보존 정책
- 비용: 사용자당 과금인지, 워크스페이스 과금인지, 엔터프라이즈 견적인지
- 운영 부담: SaaS인지 자체호스팅인지, 백업/업그레이드/보안 패치 책임

## 주요 후보 비교

| 솔루션                                                                                            | 유형                          | 강점                                                                                                          | 주의점                                                 | 가격/도입 신호                                                                            |
| ---------------------------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [Notion](https://www.notion.com/product/wikis)                                                 | 올인원 문서/위키/프로젝트              | 쉬운 작성 경험, 데이터베이스, 템플릿, 페이지 검증, AI/Enterprise Search, Slack/GitHub/Jira/Figma 등 연결                           | 문서 구조와 권한 규칙을 초기에 설계하지 않으면 위키가 빠르게 흩어질 수 있음         | 공식 가격: Free, Plus $10/멤버/월, Business $20/멤버/월, Enterprise 별도 견적                     |
| [Confluence](https://www.atlassian.com/software/confluence)                                    | 엔터프라이즈 위키/문서 협업             | Jira와의 결합, 템플릿, 팀 워크스페이스, Rovo AI 검색/요약/에이전트, 제품/개발 조직에 적합                                                  | Atlassian 생태계 밖에서는 무겁게 느껴질 수 있고 관리자가 정보 구조를 잡아야 함   | Free/Standard/Premium/Enterprise 구조. 정확한 가격은 Atlassian 가격 페이지 확인 필요                 |
| [Microsoft SharePoint](https://www.microsoft.com/en-us/microsoft-365/sharepoint/collaboration) | Microsoft 365 기반 지식/콘텐츠 플랫폼 | Microsoft 365 권한, Teams/OneDrive/Loop/Copilot 연계, 문서 보안/거버넌스, 대규모 조직 적합                                     | 위키형 작성 UX는 전용 위키보다 딱딱할 수 있음. 정보 아키텍처 설계가 중요         | Microsoft 365 Business Basic은 미국 공식 페이지 기준 $6/사용자/월부터, Copilot 포함 플랜은 더 높음          |
| [Guru](https://www.getguru.com/pricing)                                                        | AI 지식관리/답변 자동화              | 검증 워크플로, permission-aware AI 답변, 100+ 통합, Slack/Teams/Salesforce/Zendesk/Confluence/SharePoint 연동, 감사/보안 기능 | 가격이 맞춤 견적 중심이라 사전 비용 예측이 어렵다                        | Enterprise 패키지형 상담 기반                                                               |
| [Document360](https://document360.com/pricing/)                                                | 고객/내부 지식베이스, 기술문서           | 내부/외부 KB, API 문서, 다국어 자동 번역, SEO, 피드백, 분석, AI Search/Chatbot/MCP, SSO/SCIM                                  | 견적 기반이라 플랜별 실제 비용 확인 필요. 제품 문서/헬프센터 중심이라 범용 협업툴은 아님 | Professional/Business/Enterprise 모두 견적 기반                                           |
| [Helpjuice](https://helpjuice.com/pricing)                                                     | 고객지원/내부 KB                  | 커스텀 브랜딩, 실시간 협업/워크플로, 다국어/AI 번역, AI Writer/Search/Chatbot, 분석, 100+ 통합                                      | 사용자당 저가 도구보다 시작 비용이 높다                              | Knowledge Base $249/월, AI-Knowledge Base $449/월, Unlimited AI-Knowledge Base $799/월 |
| [Slab](https://slab.com/)                                                                      | 가벼운 팀 위키                    | 깔끔한 작성 경험, Topics 구조, Unified Search, Slack/GitHub/Asana/Okta 등 통합, 검증 기능                                   | 복잡한 프로세스/고객지원 KB/강한 엔터프라이즈 거버넌스에는 한계                | Free up to 10 users, Startup $6.67/사용자/월, Business $12.50/사용자/월, Enterprise 별도      |
| [Stack Internal](https://stackoverflow.co/internal/)                                           | 기술 조직 Q&A/지식 엔진             | Q&A, 투표/평판/전문가 검증, Content Health, AI Enhanced Search, MCP/API, 기술 지식의 신뢰도 관리                               | 일반 사내 정책/운영 문서 위키로는 범용성이 낮을 수 있음                    | Free up to 50 users, Basic $6.50/seat/month 공개 가격, Business/Enterprise 제공           |
| [Outline](https://www.getoutline.com/)                                                         | 팀 위키, SaaS/자체호스팅            | 빠른 Markdown 기반 편집, 실시간 협업, AI Q&A, Slack 통합, 권한/그룹, 오픈소스, 한국어 포함 다국어                                        | 엔터프라이즈급 워크플로/고객지원 기능은 제한적                           | Cloud는 팀 규모별 $10/$79/$249 월 단위 티어, self-host 가능                                     |
| [BookStack](https://www.bookstackapp.com/)                                                     | 무료 오픈소스 자체호스팅 위키            | MIT 라이선스, Books/Chapters/Pages 구조, WYSIWYG/Markdown, 검색, 권한, OIDC/SAML2/LDAP, MFA                           | SaaS가 아니므로 서버 운영, 백업, 보안 패치 책임 필요                   | 소프트웨어는 무료. 인프라/운영 비용 별도                                                             |
| [Wiki.js](https://docs.requarks.io/)                                                           | 오픈소스 위키                     | 설치가 빠르고, 페이지/태그/권한/로케일/인증/검색/저장소 모듈을 확장 가능                                                                  | 자체 운영 부담. 비개발자 문서 작성 경험은 SaaS형 도구보다 검증 필요           | 오픈소스 자체호스팅 중심                                                                       |
| [Dooray! Wiki](https://dooray.com/main/service/wiki/)                                          | 국내 올인원 협업툴 내 위키             | 위키, 프로젝트, 메신저, 메일, 화상회의를 한 제품군에서 사용. Markdown, 외부 공유, 버전 관리, 국내 보안 인증 맥락                                    | 글로벌 SaaS 생태계 연동 폭은 별도 확인 필요. 위키 단독 전문 제품은 아님        | Free/Basic/Business/Enterprise 구조. AI Lite는 공식 페이지 기준 ₩15,000/라이선스/월부터              |

## 카테고리별 판단

### 1. 범용 사내 지식관리

Notion, Confluence, Slab, Outline이 가장 직접적인 후보군이다.

- Notion은 문서, 데이터베이스, 프로젝트 관리가 함께 필요할 때 적합하다. Business 플랜에서 SAML SSO, granular database permissions, 페이지 검증, Enterprise Search 베타가 제공되는 점이 장점이다.
- Confluence는 제품/개발 조직에서 PRD, RFC, 회고, 릴리즈 노트, Jira 이슈와 연결되는 문서 흐름에 강하다. Atlassian Rovo가 AI 검색, 요약, 에이전트 기능을 붙이고 있다.
- Slab은 "가벼운 사내 위키"에 초점을 둔다. 복잡한 프로젝트 관리보다 문서 작성, Topics 구조, Unified Search를 원하는 팀에 맞다.
- Outline은 SaaS와 자체호스팅 선택지가 모두 있어 스타트업/개발팀에 실용적이다. 공식 페이지 기준 한국어 포함 다국어 지원과 오픈소스 공개를 강조한다.

### 2. Microsoft 365 기반 조직

이미 Microsoft 365, Teams, OneDrive, Entra ID, Purview를 쓰는 조직은 SharePoint를 먼저 검토하는 것이 현실적이다. SharePoint는 Copilot과 agents의 지식 기반으로 포지셔닝되고 있으며, Microsoft 365 권한/보존/보안 정책을 그대로 활용하기 쉽다.

단, SharePoint는 "위키 작성 경험"만 놓고 보면 Notion/Slab/Outline보다 무겁게 느껴질 수 있다. 파일 서버, 인트라넷, 부서 포털, 대규모 권한 관리를 함께 해결해야 하는 경우에 더 적합하다.

### 3. 고객지원/외부 공개 지식베이스

Document360과 Helpjuice가 적합하다.

- Document360은 API 문서, SEO, 다국어, 임베디드 헬프센터, 티켓 디플렉터, Pro Analytics, AI Search/Answer, MCP Server 같은 기능이 명확하다.
- Helpjuice는 커스텀 브랜딩, 워크플로, AI 번역, AI Writer/Search/Chatbot, 100+ 통합을 강조한다. 사용자당 과금이 아니라 월 정액 패키지 성격이 강하다.

사내 지식관리만 목적이라면 범용 위키가 더 단순할 수 있지만, 고객 셀프서비스나 공개 문서가 핵심이면 이 카테고리를 우선 검토해야 한다.

### 4. AI 답변/업무 흐름 내 지식 전달

Guru와 Stack Internal은 "문서를 보관"하기보다 "검증된 답을 업무 중 전달"하는 데 강하다.

- Guru는 sales, support, operations 팀이 Slack/Teams/Salesforce/Zendesk 안에서 바로 답을 받아야 할 때 좋다. 검증 워크플로와 사용 신호 기반 관리가 핵심이다.
- Stack Internal은 개발자 Q&A, 기술 의사결정, 내부 구현 지식, AI 에이전트용 기술 컨텍스트 관리에 특화되어 있다.

### 5. 자체호스팅/오픈소스

BookStack, Outline self-hosted, Wiki.js가 후보군이다.

- BookStack은 무료 MIT 라이선스, 단순한 Books/Chapters/Pages 구조, OIDC/SAML2/LDAP, MFA 등으로 사내 문서 저장소를 직접 운영하기 좋다.
- Outline self-hosted는 현대적인 문서 UX와 오픈소스 기반을 동시에 원하는 팀에 맞다.
- Wiki.js는 모듈식 확장, 인증/검색/저장소 선택지를 중시할 때 검토할 만하다.

자체호스팅은 라이선스 비용을 줄일 수 있지만, 백업, 장애 대응, 계정/권한, 보안 업데이트, 검색 품질 관리 책임이 내부로 온다.

## 도입 시 권장 절차

1. 2주 파일럿 범위를 정한다.
   - 예: 온보딩 문서 20개, 제품 의사결정 문서 10개, CS FAQ 30개, 개발 Q&A 20개
2. 문서 소유권 모델을 먼저 정한다.
   - 모든 문서에 owner, last reviewed date, review cadence를 둔다.
   - 소유자가 없는 문서는 마이그레이션하지 않는다.
3. 정보 구조를 3단계 이하로 제한한다.
   - 예: Company / Product / Engineering / Operations / Customer Support
   - 태그와 검색을 보조 수단으로 쓰고 폴더를 과도하게 깊게 만들지 않는다.
4. AI 기능은 "권한 반영"과 "출처 표시"를 반드시 확인한다.
   - 사용자가 접근 권한 없는 문서가 AI 답변에 섞이면 도입 리스크가 크다.
   - 답변마다 근거 문서 링크가 나오는지 확인한다.
5. 마이그레이션 전 폐기 기준을 둔다.
   - 12개월 이상 미열람, 소유자 없음, 중복, 오래된 스크린샷 중심 문서는 그대로 옮기지 않는다.
6. 파일럿 성공 기준을 숫자로 잡는다.
   - 검색 성공률, 중복 질문 감소, 신규 입사자 온보딩 시간, CS 티켓 디플렉션, 문서 리뷰 준수율 등

## 최종 후보 선정안

1. 사내 범용 지식관리 1순위: Notion 또는 Confluence
   - Notion: 스타트업/비개발 부서까지 넓게 쓰기 좋음
   - Confluence: Jira 기반 제품/개발 조직이면 더 자연스러움
2. Microsoft 365 조직 1순위: SharePoint + Copilot
   - 파일/권한/보안/Teams가 이미 Microsoft에 있으면 중복 도입을 줄일 수 있음
3. 고객지원/외부 문서 1순위: Document360
   - 제품 문서, API 문서, 헬프센터, AI 검색, 티켓 디플렉션까지 한 번에 보기 좋음
4. 자체호스팅 1순위: Outline 또는 BookStack
   - 현대적인 UX는 Outline, 단순하고 안정적인 계층형 위키는 BookStack
5. 국내 올인원 협업툴 1순위: Dooray!
   - 한국어 지원, 국내 계약/보안 인증 맥락, 프로젝트/메일/메신저/위키 통합이 중요할 때 검토

## 참고 링크

- Notion Wikis: https://www.notion.com/product/wikis
- Notion Pricing: https://www.notion.com/pricing
- Atlassian Confluence: https://www.atlassian.com/software/confluence
- Confluence Pricing: https://www.atlassian.com/software/confluence/pricing
- Microsoft SharePoint: https://www.microsoft.com/en-us/microsoft-365/sharepoint/collaboration
- Microsoft 365 Copilot: https://www.microsoft.com/en-us/microsoft-365-copilot
- Guru Pricing: https://www.getguru.com/pricing
- Document360 Pricing: https://document360.com/pricing/
- Helpjuice Pricing: https://helpjuice.com/pricing
- Slab: https://slab.com/
- Slab Pricing: https://slab.com/pricing/
- Stack Internal: https://stackoverflow.co/internal/
- Stack Internal Pricing: https://stackoverflow.co/internal/pricing/
- Outline: https://www.getoutline.com/
- Outline Pricing: https://www.getoutline.com/pricing
- BookStack: https://www.bookstackapp.com/
- Wiki.js Docs: https://docs.requarks.io/
- Dooray! Wiki: https://dooray.com/main/service/wiki/
- Dooray! Pricing: https://dooray.com/main/pricing/

# Project Development Principles

이 문서는 Sponzey Cabinet 개발에 참여하는 코드 생성 에이전트, 개발자, 리뷰어가 반드시 따라야 하는 운영 지침서다. 모든 변경은 Layered Architecture, Clean Architecture, Tidy First, TDD를 기준으로 판단한다.

## 1. Project Development Principles

- Layered Architecture를 기본 구조로 사용하라.
- Clean Architecture 원칙에 따라 도메인과 유스케이스를 외부 시스템으로부터 분리하라.
- Tidy First 원칙에 따라 정리 작업과 기능 변경을 분리하라.
- TDD를 기본 개발 방식으로 사용하라.
- 테스트 가능한 구조를 먼저 설계하라.
- 핵심 로직을 프레임워크, DB, 파일시스템, 네트워크, 환경 변수, UI와 직접 결합하지 마라.
- 기능은 유스케이스 중심으로 정의하라.
- 외부 입출력은 경계 계층에서만 처리하라.
- 인터페이스와 구현체를 분리하라.
- 암묵적 전역 상태, 숨겨진 I/O, 런타임 중간 설정 변경을 금지하라.
- 변경 이유가 다른 작업을 하나의 변경으로 섞지 마라.
- 공식 대상 플랫폼은 Web, iOS, Android, Windows, macOS, Linux로 취급하라.
- 플랫폼별 차이는 어댑터 계층에서 처리하고 도메인/유스케이스에 누출하지 마라.

## 2. Architecture Rules

- 도메인 계층은 비즈니스 규칙, 엔티티, 값 객체, 도메인 서비스, 도메인 이벤트만 포함하라.
- 도메인 계층은 외부 프레임워크, ORM, HTTP, CLI, UI, 파일시스템, 네트워크, 환경 변수, DB 드라이버에 의존하지 마라.
- 유스케이스 계층은 애플리케이션 동작을 명확한 입력과 출력으로 표현하라.
- 유스케이스는 도메인 규칙을 조합하고, 외부 시스템 접근은 포트 인터페이스로만 요청하라.
- 어댑터 계층은 HTTP handler, CLI command, worker, controller, presenter, serializer, mapper를 포함하라.
- 인프라 계층은 DB, 파일시스템, object storage, 외부 API, queue, clock, UUID generator, logger 구현체를 포함하라.
- 인프라 구현은 반드시 인터페이스 뒤에 숨겨라.
- 외부 API, DB, 파일시스템, 네트워크, 환경 변수 접근은 경계 계층에서만 수행하라.
- 도메인 로직과 외부 시스템 통신을 같은 함수에 섞지 마라.
- 유스케이스는 입력 DTO 또는 command object를 받고, 출력 DTO 또는 result object를 반환하라.
- 유스케이스는 UI 모델, HTTP request, DB row, framework context를 직접 받지 마라.
- 도메인 객체는 직렬화 포맷이나 저장소 스키마에 맞춰 오염시키지 마라.
- mapper를 사용해 외부 표현과 내부 모델을 분리하라.
- Web, iOS, Android, Windows, macOS, Linux 클라이언트는 같은 유스케이스 계약을 사용하라.
- 플랫폼별 파일시스템, 알림, 인증, 보안 저장소, 네트워크 상태, 오프라인 캐시는 인프라 어댑터로 분리하라.
- 플랫폼별 UI에서 도메인 규칙을 다시 구현하지 마라.
- 플랫폼 capability 차이는 명시적 capability object 또는 policy object로 표현하라.
- 문서 현재 조회와 문서 이력 조회는 별도 유스케이스와 query path로 분리하라.
- 모든 사용자-facing 검색과 조회는 정상적인 인덱스 상태에서 p95 300ms 이내 응답을 목표로 설계하라.

의사코드:

```text
Allowed:
Controller -> UseCase -> Domain
UseCase -> RepositoryPort
RepositoryAdapter -> RepositoryPort

Forbidden:
Domain -> Database
Domain -> Environment
UseCase -> HTTP Request
UseCase -> ConcreteRepository
```

## 3. Dependency Direction

- 의존 방향은 항상 바깥 계층에서 안쪽 계층으로 향하게 하라.
- Domain은 어떤 계층에도 의존하지 마라.
- UseCase는 Domain과 Port 인터페이스에만 의존하라.
- Adapter는 UseCase에 의존하라.
- Infrastructure는 Port 인터페이스를 구현하라.
- 내부 계층이 외부 계층의 타입을 import하지 못하게 하라.
- 인터페이스는 소비하는 계층 가까이에 정의하라.
- 구현체는 조립 단계에서 주입하라.
- 순환 의존성을 만들지 마라.
- 의존성 역전으로 외부 구현체를 교체 가능하게 하라.

계층 규칙:

```text
Domain:
  may depend on: none
  must not depend on: usecase, adapter, infrastructure, framework

UseCase:
  may depend on: domain, ports
  must not depend on: database, network, filesystem, framework context

Adapter:
  may depend on: usecase, DTO, presenter, mapper
  must not contain: domain rules

Infrastructure:
  may depend on: port interfaces, external libraries
  must not contain: usecase decisions
```

플랫폼 규칙:

```text
Allowed:
WebController -> UseCase
IosAdapter -> UseCase
AndroidAdapter -> UseCase
DesktopAdapter -> UseCase
PlatformStorageAdapter -> StoragePort

Forbidden:
Domain -> Web Framework
Domain -> Mobile SDK
UseCase -> Platform File Picker
UseCase -> Push Notification SDK
```

## 4. Configuration Policy

- 외부 파일에 설정되는 내용은 최소화하라.
- 설정 파일, 환경 파일, 외부 구성 파일에 의존하는 설계를 기본값으로 삼지 마라.
- 개인 구축 로컬 앱은 설치 1회 후 추가 수동 설정 없이 실행되어야 한다.
- 로컬 기본 실행은 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 환경 변수, 수동 설정 파일 편집을 요구하지 마라.
- 로컬 기본 store, 내부 version store, asset store, search index, app data directory는 최초 실행 시 자동 초기화하라.
- 고급 설정은 명시적 설정 화면 또는 명시적 import/export 절차로 제공하되 기본 실행의 필수 조건으로 만들지 마라.
- 외부 환경 상수는 프로그램 시작 시 최초 1회만 수신하라.
- 실행 프로세스 중간에 환경 설정 값을 삽입하거나 변경하는 방식을 거부하라.
- 최초 수신 이후에는 외부 환경 상수를 전역 상수나 프로그램 상수처럼 사용하지 마라.
- 이후 내부 흐름에서는 명시적 인자, 생성자 인자, 함수 인자, 컨텍스트 객체, 의존성 주입 형태로 전달하라.
- 설정 조회는 bootstrap 또는 composition root에서 끝내라.
- 설정 값은 검증된 config object로 변환한 뒤 내부에 전달하라.
- 암묵적 전역 접근, 런타임 중간 재설정, 동적 환경 변경, 숨겨진 설정 조회를 금지하라.
- 테스트에서 환경 변수를 직접 바꾸는 대신 명시적 config object를 생성하라.
- clean machine install smoke test로 로컬 설치 후 기본 workspace 생성까지 검증하라.

허용되는 방식:

```text
main:
  rawEnv = readEnvironmentOnce()
  config = validateConfig(rawEnv)
  app = buildApplication(config)
  app.run()

usecase:
  constructor(repository, clock, policyConfig)
```

거부해야 하는 방식:

```text
domain:
  timeout = ENV["TIMEOUT"]

usecase:
  if getRuntimeConfig("FEATURE_ENABLED"):
    ...

test:
  setEnv("MODE", "test")
  runUseCase()
```

## 5. Runtime Environment Handling

- runtime environment는 프로그램 시작 시 한 번만 읽어라.
- 읽은 값은 즉시 검증하고 내부 설정 객체로 변환하라.
- 설정 객체는 불변으로 취급하라.
- 설정이 필요한 컴포넌트에는 생성자 인자 또는 명시적 컨텍스트로 전달하라.
- 중간 실행 단계에서 환경 변수를 다시 읽지 마라.
- 중간 실행 단계에서 환경 변수를 수정하지 마라.
- feature flag, timeout, endpoint, storage path, AI provider, logging mode는 전역에서 직접 조회하지 마라.
- 요청 처리 중 환경 설정을 변경하는 API를 만들지 마라.
- 테스트는 설정 객체를 직접 생성해 실행하라.
- 운영 중 설정 변경이 필요하면 프로세스 재시작 또는 명시적 재구성 절차로 처리하라.

리뷰 기준:

- 환경 변수를 읽는 코드가 bootstrap 바깥에 있으면 거부하라.
- 설정 값을 static/global singleton으로 보관하면 거부하라.
- 함수 내부에서 숨겨진 설정 조회가 발생하면 거부하라.
- 테스트가 환경 변수 순서나 외부 파일 존재 여부에 의존하면 거부하라.
- 로컬 기본 실행이 별도 DB, 검색 서버, Git CLI, Node.js, 수동 환경 변수 설정에 의존하면 거부하라.
- 최초 실행 자동 초기화와 migration이 테스트되지 않으면 거부하라.

## 6. Logging Policy

로그는 Product Log, Field Debug Log, Development Log로만 구분한다. 모든 로그는 목적, 범위, 민감 정보 처리 기준을 가져야 한다.

### Product Log

목적:

- 프로덕트 운영용 최소 로그를 남겨라.
- 사용자 영향, 핵심 상태 변화, 장애 원인 추적에 필요한 최소 정보만 기록하라.

허용되는 정보:

- 요청 또는 작업 correlation id
- 사용자 식별이 필요한 경우 내부 user id 또는 tenant id의 마스킹된 값
- 유스케이스 이름
- 핵심 상태 변화
- 실패 유형과 안정적인 error code
- 외부 시스템 호출 성공/실패 여부
- 처리 시간 구간

금지되는 정보:

- 비밀번호, 토큰, API key, session id
- 원문 문서 내용
- 첨부 파일 내용
- 개인정보 원문
- AI prompt 원문과 민감 응답 원문
- 테스트용 상세 상태
- 내부 객체 전체 dump

사용 위치:

- 유스케이스 시작/종료의 핵심 결과
- 상태머신의 중요한 상태 전이
- 외부 시스템 실패
- 사용자 영향이 있는 장애
- 보안상 중요한 이벤트

예시:

```text
INFO document.publish.completed correlation_id=... workspace_id=masked:... document_id=... duration_ms=42
WARN attachment.upload.failed correlation_id=... reason=storage_timeout retryable=true
ERROR usecase.failed correlation_id=... usecase=PublishDocument error_code=DOCUMENT_NOT_APPROVED
```

리뷰 기준:

- 로그가 운영 판단에 필요하지 않으면 제거하라.
- 민감 정보가 포함되면 거부하라.
- 메시지가 자유 텍스트뿐이고 안정적인 event name 또는 error code가 없으면 수정하라.
- 동일 이벤트가 중복 기록되면 하나로 줄여라.

### Field Debug Log

목적:

- 운영 또는 고객 환경에서 문제 재현과 상태 확인을 위해 제한적으로 사용하라.
- 특정 tenant, workspace, request, feature, component 범위로 제한하라.

허용되는 정보:

- 마스킹된 식별자
- 상태머신 현재 상태와 이벤트 이름
- 설정 객체의 비민감 요약
- 외부 호출 metadata
- 재시도 횟수
- 캐시 hit/miss
- permission decision의 요약 결과

금지되는 정보:

- 민감 정보 원문
- 전체 request/response body
- 문서 본문 전체
- 첨부 파일 내용
- 장기간 보존되는 상세 추적 로그
- 전체 사용자 대상 무제한 활성화

사용 위치:

- 재현이 어려운 운영 문제
- 고객 환경에서만 발생하는 장애
- 외부 시스템 연동 상태 확인
- 권한/검색/동기화/AI retrieval 문제의 원인 좁히기

활성화 조건:

- 명시적 관리자 승인으로 활성화하라.
- 범위와 만료 시간을 지정하라.
- 기본값은 비활성화하라.
- 활성화 기록을 Product Log에 남겨라.

보존 기간:

- 기본 보존 기간은 짧게 유지하라.
- 조사 목적이 끝나면 폐기하라.
- 보존 정책은 운영 정책 문서와 일치시켜라.

마스킹 기준:

- 사용자 입력, 문서 제목, 파일명, 외부 URL은 필요 시 해시 또는 부분 마스킹하라.
- 토큰, key, secret은 기록하지 마라.
- AI 관련 로그는 prompt 원문 대신 길이, provider, model, retrieval count, citation count만 기록하라.

예시:

```text
DEBUG field.search.retrieval correlation_id=... scope=workspace:masked query_hash=... candidate_count=12 filtered_count=8
DEBUG field.state.transition correlation_id=... machine=DocumentPublish from=Reviewing event=Approve to=Approved
```

리뷰 기준:

- 활성화 범위와 만료 조건이 없으면 거부하라.
- Product Log로 충분한 내용을 Field Debug Log에 중복 기록하지 마라.
- 민감 정보 마스킹이 불명확하면 거부하라.

### Development Log

목적:

- 로컬 개발, 테스트, 검증 과정에서만 사용하라.
- 개발자가 구현 상태와 테스트 실패 원인을 확인하게 하라.

허용되는 정보:

- 로컬 테스트 입력 요약
- mock, fake, stub 호출 내역
- parser, mapper, serializer 중간 결과
- 상태머신 전이 상세
- 개발 중 성능 측정값

금지되는 정보:

- 프로덕션 빌드 기본 포함
- 배포 결과물 기본 활성화
- 실제 고객 데이터 원문
- secret, token, credential
- 운영 장애 분석용 장기 로그 대체

사용 위치:

- 로컬 개발
- 단위 테스트
- 통합 테스트
- smoke test
- 임시 검증 코드

예시:

```text
DEV parser.block.detected type=table row_count=4
DEV fake.repository.saved document_id=test-doc-1
```

리뷰 기준:

- Development Log가 프로덕션 기본 경로에 포함되면 거부하라.
- 개발 로그가 기능 동작에 필요하면 설계를 수정하라.
- 임시 로그가 테스트 검증을 대체하면 거부하라.

## 7. State Machine Policy

- 복잡한 내부 흐름은 상태머신으로 관리하라.
- 암묵적 boolean flag 조합으로 흐름을 관리하지 마라.
- 상태, 이벤트, 전이 조건, 실패 상태, 종료 상태를 명시하라.
- 상태 전이는 독립적으로 테스트 가능해야 한다.
- 상태 변경은 로그 정책과 연결하라.
- 상태머신은 도메인 규칙 또는 유스케이스 규칙 안에서 관리하라.
- 상태머신은 UI, 외부 어댑터, 인프라에 종속되지 않아야 한다.
- 상태 전이 실패는 명확한 error code를 반환하라.
- 상태 전이 함수는 입력 상태와 이벤트가 같으면 동일한 결과를 반환해야 한다.
- 외부 I/O는 상태 전이 결정 이후 유스케이스가 수행하라.

상태머신에 포함할 항목:

- State
- Event
- Guard condition
- Transition
- Failure state
- Terminal state
- Side effect request
- Log event mapping

의사코드:

```text
transition(state, event, context) -> TransitionResult

TransitionResult:
  next_state
  side_effect_requests
  product_log_event
  error_code
```

금지 예시:

```text
if isApproved and not isLocked and hasPublishedOnce and retryCount > 3:
  ...
```

허용 예시:

```text
from Reviewing on Approve when reviewerAllowed -> Approved
from Approved on PublishRequested -> Publishing
from Publishing on PublishSucceeded -> Published
from Publishing on PublishFailed -> PublishFailed
```

## 8. TDD Policy

- 실패하는 테스트를 먼저 작성하라.
- 테스트를 통과하는 최소 구현을 작성하라.
- 중복과 구조 문제를 정리하라.
- 정리 후 모든 테스트를 다시 실행하라.
- 외부 의존성은 테스트 더블, 포트, 인터페이스로 대체 가능해야 한다.
- 설정, 로그, 상태 전이, 오류 처리도 테스트 대상에 포함하라.
- 테스트가 어려운 코드는 설계를 수정하라.
- 도메인 테스트는 외부 시스템 없이 실행되어야 한다.
- 유스케이스 테스트는 fake repository, fake clock, fake id generator, fake logger를 주입해 실행하라.
- 인프라 테스트는 경계 구현을 검증하고 도메인 규칙을 중복 검증하지 마라.
- E2E 테스트는 핵심 사용자 흐름과 배포 조합을 검증하되, 단위 테스트를 대체하지 마라.
- 플랫폼별 테스트는 공통 유스케이스를 중복 검증하지 말고 platform adapter, UI flow, storage/auth/notification/network integration을 검증하라.
- Web, iOS, Android, Windows, macOS, Linux의 핵심 smoke test는 로그인, 문서 조회, 문서 검색, 댓글, AI 질의 같은 최소 흐름을 검증하라.
- 문서 현재 조회, 문서 이력 조회, 검색, 링크/백링크 조회, 첨부 metadata 조회는 성능 테스트 대상에 포함하라.
- 성능 테스트는 p95 300ms 목표를 측정하고, 측정 조건과 데이터 크기를 명시하라.
- 현재 문서 조회가 version history 전체 스캔에 의존하지 않는지 테스트하라.

TDD 사이클:

1. 실패하는 테스트를 작성하라.
2. 테스트를 통과하는 최소 구현을 작성하라.
3. 중복과 구조 문제를 정리하라.
4. 외부 의존성을 테스트 더블, 포트, 인터페이스로 대체 가능하게 유지하라.
5. 설정, 로그, 상태 전이, 오류 처리를 테스트에 포함하라.

필수 테스트 대상:

- 도메인 규칙
- 유스케이스 입력/출력
- 권한 결정
- 설정 검증
- 상태머신 전이
- 오류 처리
- 로그 이벤트 생성 여부
- 외부 포트 호출 계약
- serializer/mapper 경계

## 9. Tidy First Policy

- 기능 변경 전에 작은 정리 작업이 필요하면 먼저 수행하라.
- 정리 작업과 기능 변경을 같은 커밋에 섞지 마라.
- 리팩터링과 기능 변경은 별도 커밋으로 분리하라.
- 정리 작업은 동작을 변경하지 마라.
- 정리 작업 후 기존 테스트가 모두 통과해야 한다.
- 기능 변경은 정리 작업 이후 별도 변경으로 수행하라.
- 큰 리팩터링을 기능 구현의 전제 조건으로 만들지 마라.
- 이름 변경, 파일 이동, dead code 제거, 중복 제거는 기능 변경과 구분하라.
- Tidy First 변경은 작고 리뷰 가능해야 한다.

허용되는 Tidy First 예:

- 오해를 부르는 이름을 명확히 변경
- 중복 mapper 제거
- 함수 추출
- 테스트 fixture 정리
- 계층 경계에 맞게 파일 이동
- 사용하지 않는 코드 삭제

금지되는 Tidy First 예:

- 정리 작업 중 새 기능 추가
- 리팩터링 중 외부 동작 변경
- 테스트 없이 구조 대규모 변경
- 기능 변경을 숨긴 포맷 변경

## 10. Code Review Checklist

- 변경이 Layered Architecture를 지키는지 확인하라.
- 도메인 계층이 외부 프레임워크에 의존하지 않는지 확인하라.
- 의존 방향이 바깥에서 안쪽으로만 향하는지 확인하라.
- 유스케이스 입력과 출력이 명확한지 확인하라.
- 외부 API, DB, 파일시스템, 네트워크, 환경 변수 접근이 경계 계층에만 있는지 확인하라.
- 인터페이스와 구현체가 분리되어 있는지 확인하라.
- 테스트가 먼저 작성되었거나 변경과 함께 충분히 추가되었는지 확인하라.
- 설정 값이 시작 시 1회만 수신되는지 확인하라.
- 런타임 중간 설정 변경이나 숨겨진 설정 조회가 없는지 확인하라.
- Product Log, Field Debug Log, Development Log가 구분되어 있는지 확인하라.
- 로그에 민감 정보가 없는지 확인하라.
- 복잡한 흐름이 상태머신 또는 명시적 상태 전이로 표현되었는지 확인하라.
- 정리 작업과 기능 변경이 분리되었는지 확인하라.
- 전역 상태, 싱글톤, 숨겨진 I/O가 테스트를 어렵게 만들지 않는지 확인하라.
- 오류 코드와 실패 경로가 테스트되었는지 확인하라.
- 새 public API 또는 plugin extension point에 계약 테스트가 있는지 확인하라.
- 플랫폼별 구현이 공통 도메인/유스케이스를 우회하지 않는지 확인하라.
- 플랫폼 capability 차이가 명시적으로 문서화되고 테스트되는지 확인하라.
- 플랫폼별 로그, crash report, error report에 민감 정보가 포함되지 않는지 확인하라.
- 현재 문서 조회와 이력 조회가 분리되어 있는지 확인하라.
- 검색/조회 경로가 p95 300ms 목표를 만족하도록 index, projection, cache, pagination을 사용하는지 확인하라.
- 조회 경로가 본문 전체 스캔, version history 전체 스캔, 권한 후처리 전체 스캔에 의존하지 않는지 확인하라.

## 11. Prohibited Patterns

- 도메인 계층에서 DB, HTTP, 파일시스템, 환경 변수를 직접 접근하지 마라.
- 유스케이스에서 framework request/response 객체를 직접 사용하지 마라.
- 인프라 구현체를 도메인 또는 유스케이스에 직접 주입하지 마라.
- 전역 mutable state를 사용하지 마라.
- singleton으로 설정, logger, repository, clock을 숨기지 마라.
- 함수 내부에서 환경 변수를 조회하지 마라.
- 실행 중간에 설정 값을 바꾸지 마라.
- 테스트가 외부 환경 파일이나 환경 변수 순서에 의존하게 하지 마라.
- 로컬 기본 실행에 별도 서버, 외부 DB, 외부 검색 엔진, Git CLI, Node.js 설치를 요구하지 마라.
- 사용자가 설정 파일을 직접 편집해야만 앱을 시작할 수 있게 만들지 마라.
- 운영 로그에 민감 정보나 원문 문서를 남기지 마라.
- Development Log를 프로덕션 기본 빌드에 포함하지 마라.
- 복잡한 절차를 boolean flag 조합으로 관리하지 마라.
- 외부 API 응답 형태를 도메인 모델로 직접 사용하지 마라.
- 기능 변경과 리팩터링을 하나의 변경으로 섞지 마라.
- 테스트 없는 상태 전이 로직을 추가하지 마라.
- 실패 경로 없이 성공 경로만 구현하지 마라.
- 플랫폼별 UI 또는 SDK 코드에 도메인 규칙을 복제하지 마라.
- 플랫폼별 파일 경로, 보안 저장소, 알림, 네트워크 상태를 유스케이스에서 직접 접근하지 마라.
- 특정 플랫폼에서만 동작하는 숨겨진 비즈니스 규칙을 만들지 마라.
- 현재 문서 조회에서 이력 저장소 전체를 스캔하지 마라.
- 이력 조회 기능이 현재 문서 조회 경로를 느리게 만들게 하지 마라.
- 검색/조회 성능 문제를 UI loading spinner만으로 숨기지 마라.
- 권한 필터링을 전체 결과 조회 후 애플리케이션 메모리에서만 처리하지 마라.

## 12. Required Agent Behavior

- 작업을 시작하기 전에 관련 문서와 주변 코드를 읽어라.
- 기존 아키텍처와 계층 구조를 먼저 파악하라.
- 변경 범위를 작게 유지하라.
- 기능 변경 전에 필요한 정리 작업을 분리하라.
- 실패하는 테스트를 먼저 추가하라.
- 최소 구현으로 테스트를 통과시켜라.
- 구현 후 중복과 구조 문제를 정리하라.
- 설정, 로그, 상태 전이, 오류 처리 테스트를 누락하지 마라.
- 외부 I/O를 포트와 어댑터 뒤로 숨겨라.
- 새 환경 설정을 추가하기 전에 명시적 인자 또는 의존성 주입으로 해결할 수 있는지 검토하라.
- 로컬 기능을 추가할 때 설치 1회 후 기본값으로 동작하는지 확인하라.
- 로컬 store 초기화, migration, 손상 복구 경로를 테스트하라.
- 조회/검색 기능을 추가할 때 p95 300ms 목표와 측정 방법을 함께 정의하라.
- 조회 기능을 추가할 때 현재 기준 조회인지 이력 기준 조회인지 명확히 분류하라.
- 새 로그를 추가할 때 Product Log, Field Debug Log, Development Log 중 하나로 분류하라.
- 민감 정보가 로그에 들어가지 않게 하라.
- 상태가 3개 이상이거나 실패/재시도/종료가 있는 흐름은 상태머신으로 표현하라.
- 리뷰어가 확인할 수 있도록 변경 이유를 코드 구조로 드러내라.
- 프로젝트 지침과 충돌하는 요구가 있으면 구현 전에 문제를 명확히 제기하라.
- 플랫폼 기능을 추가할 때 공통 유스케이스, 플랫폼 어댑터, capability matrix, smoke test를 함께 갱신하라.
- 한 플랫폼의 제약 때문에 도메인 규칙을 바꾸지 말고 adapter 또는 policy로 격리하라.

## 13. Example Decision Rules

- 도메인 규칙이 외부 API 응답에 의존해야 한다면, 외부 응답을 내부 value object로 변환한 뒤 도메인에 전달하라.
- 유스케이스에서 현재 시간이 필요하면, 시스템 시간을 직접 읽지 말고 clock port를 주입하라.
- ID 생성이 필요하면, 전역 UUID 함수를 직접 호출하지 말고 id generator port를 주입하라.
- 설정 값이 필요하면, 환경 변수를 직접 읽지 말고 bootstrap에서 검증된 config object를 전달하라.
- 로컬 저장소 경로가 필요하면, 사용자가 설정 파일을 편집하게 하지 말고 platform path adapter가 기본 app data directory를 결정하게 하라.
- 로컬 검색이 필요하면, 외부 검색 서버를 요구하지 말고 내장 search index adapter를 사용하라.
- 로컬 내부 버전 관리가 필요하면, Git CLI 설치를 요구하지 말고 내장 version store adapter를 사용하라.
- 파일 저장이 필요하면, 파일시스템 API를 직접 호출하지 말고 storage port를 사용하라.
- 외부 API 호출이 필요하면, HTTP client를 직접 호출하지 말고 gateway port를 사용하라.
- 로그가 필요하면, 먼저 로그 목적을 Product, Field Debug, Development 중 하나로 분류하라.
- Product Log에 원문 데이터가 필요해 보이면, 원문 대신 id, hash, count, status, error code로 대체하라.
- Field Debug Log가 필요하면, 활성화 범위와 만료 시간을 먼저 정의하라.
- Development Log가 필요하면, 프로덕션 기본 빌드에 포함되지 않도록 분리하라.
- 상태가 여러 flag로 표현되기 시작하면, 즉시 명시적 state enum과 transition function으로 바꿔라.
- 실패 복구 또는 재시도가 있으면, 실패 상태와 재시도 이벤트를 테스트하라.
- 리팩터링 중 기능 변경이 발견되면, 리팩터링을 멈추고 별도 변경으로 분리하라.
- 테스트가 작성하기 어렵다면, 구현을 강행하지 말고 의존성 방향과 경계 설계를 먼저 수정하라.
- 새 플러그인 기능이 필요하면, core domain을 오염시키지 말고 extension point와 port를 정의하라.
- 플랫폼별 저장소 접근이 필요하면, 유스케이스에서 직접 SDK를 호출하지 말고 storage port와 platform adapter를 정의하라.
- 모바일 push 알림이 필요하면, 도메인 이벤트를 notification request로 변환하고 플랫폼 push adapter에서 전송하라.
- 데스크톱 파일 선택이 필요하면, UI adapter에서 파일 선택 결과를 value object로 변환한 뒤 유스케이스에 전달하라.
- 플랫폼별 기능 지원 범위가 다르면, 조건문을 도메인에 넣지 말고 capability policy를 주입하라.
- 현재 문서가 필요하면 `GetCurrentDocument` 성격의 유스케이스를 사용하고, 이력 문서가 필요하면 version 전용 유스케이스를 사용하라.
- 검색이 필요하면 원본 문서 전체 스캔을 기본값으로 삼지 말고 search index 또는 projection을 사용하라.
- 300ms 목표를 넘는 조회가 예상되면 동기 API로 만들지 말고 비동기 job, pagination, streaming, cache 중 하나로 설계를 변경하라.
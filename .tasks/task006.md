# Task 006. First-run State Machine과 Initialization Plan

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-005 First-run initializer`를 시작하는 것이다.
- [x] 이 태스크는 최초 실행 초기화 흐름을 암묵적 플래그가 아니라 명시적 상태머신과 side effect plan으로 표현한다.
- [x] 이 태스크 완료 후 프로젝트는 first-run 상태, 이벤트, 전이 결과, 초기화 디렉터리 계획을 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 runtime config object와 bootstrap input을 가진다.
- [x] 이전 태스크 Task 005에서 `AppConfig`와 environment snapshot 기반 config 생성 계약을 완료했다.
- [x] 이번 태스크는 local 설치 1회 원칙을 위한 first-run foundation을 시작했다.
- [x] 현재 확인된 제약 사항은 실제 filesystem directory creation port와 adapter가 아직 없다는 점이다. 이번 태스크는 순수 core 모델과 테스트로 제한했다.

## 3. Scope

### Included

- [x] `cabinet-core`에 first-run 상태머신 타입을 추가한다.
- [x] `AppConfig`에서 초기화 대상 디렉터리 계획을 생성한다.
- [x] 상태 전이 정상/실패/재시도 테스트를 추가한다.

### Excluded

- [x] 실제 filesystem directory creation은 이번 태스크에서 수행하지 않았다.
- [x] `FirstRunInitializer` orchestration은 후속 태스크로 넘긴다.
- [x] Product/Field Debug logger 구현체는 후속 logging foundation 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: first-run state machine을 만들었다.
- [x] 입력: `FirstRunState`, `FirstRunEvent`.
- [x] 출력: `FirstRunTransition`.
- [x] 성공 조건: `NotStarted -> ResolvingPaths -> CreatingStores -> WritingMetadata -> Completed` 전이가 테스트로 검증된다.
- [x] 실패 조건: invalid transition이 조용히 성공한다.

### Functional Unit 2

- [x] 구현한 기능: initialization directory plan을 만들었다.
- [x] 입력: `AppConfig`.
- [x] 출력: `FirstRunPlan`과 directory creation requests.
- [x] 성공 조건: metadata, version store, asset store, search index, workspace root가 모두 plan에 포함된다.
- [x] 실패 조건: core가 filesystem을 직접 생성한다.

### Functional Unit 3

- [x] 구현한 기능: 실패와 재시도 전이를 명시했다.
- [x] 입력: 실패 event와 retry event.
- [x] 출력: `Failed` 상태, retry 가능 여부, user-facing error code.
- [x] 성공 조건: 실패 상태는 error code를 포함하고 retry event가 `Retrying`으로 전이된다.
- [x] 실패 조건: 실패 원인이 boolean flag나 문자열 상태로만 표현된다.

## 5. Architecture Notes

- [x] 변경되는 계층은 core runtime foundation이다.
- [x] domain crate는 변경하지 않았다.
- [x] usecase crate는 변경하지 않았다.
- [x] first-run transition function은 pure function으로 유지했다.
- [x] filesystem, environment, network 접근은 추가하지 않았다.
- [x] side effect는 직접 실행하지 않고 plan/request value로만 반환한다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] `AppConfig`를 명시적 인자로 받아 initialization plan을 만든다.
- [x] 환경 값을 runtime 중간에 재조회하지 않는다.
- [x] 전역 config registry나 mutable singleton을 만들지 않는다.
- [x] 테스트는 environment를 변경하지 않고 `ExternalEnvironmentSnapshot` 또는 `AppConfig`를 직접 만든다.

## 7. Logging Requirements

### Product Log

- [x] 이번 태스크는 logger implementation을 추가하지 않았다.
- [x] 상태 전이는 future Product Log event 이름 후보를 안정적인 enum/error code로 표현한다.
- [x] 문서 본문, 첨부 내용, secret, raw path dump를 Product Log payload로 만들지 않았다.

### Field Debug Log

- [x] 이번 태스크는 Field Debug Log 구현을 추가하지 않았다.
- [x] future Field Debug Log는 first-run state와 비민감 path role만 기록할 수 있도록 plan에 role을 포함했다.
- [x] 실제 path value의 과도한 노출은 후속 logger masking 테스트에서 제한한다.

### Development Log

- [x] 이번 태스크는 runtime Development Log를 추가하지 않았다.
- [x] 검증 결과는 Completion Report에만 기록했다.
- [x] 프로덕션 기본 동작에 포함되는 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] `FirstRunState`는 `NotStarted`, `ResolvingPaths`, `CreatingStores`, `WritingMetadata`, `Completed`, `Failed`, `Retrying`을 포함한다.
- [x] `FirstRunEvent`는 `Start`, `PathsResolved`, `StoreCreated`, `MetadataWritten`, `Fail`, `Retry`, `Complete`을 포함한다.
- [x] `FirstRunTransition`은 이전 상태, 이벤트, 다음 상태, retry 가능 여부, optional error code를 포함한다.
- [x] invalid transition은 `FirstRunError::InvalidTransition`으로 실패한다.
- [x] 상태 전이는 unit test로 검증한다.
- [x] 상태머신은 UI, 외부 어댑터, 인프라에 의존하지 않는다.

## 9. TDD Plan

- [x] 실패하는 first-run state transition test를 먼저 작성했다.
- [x] 실패하는 first-run directory plan test를 먼저 작성했다.
- [x] 실패하는 invalid transition test를 먼저 작성했다.
- [x] 실패와 재시도 전이 테스트를 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 구현 후 중복과 구조 문제를 정리했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 설정, 로그, 상태 전이를 검증 대상에 포함했다.

## 10. Implementation Checklist

- [x] first-run tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] 최소 first-run module을 작성했다.
- [x] `cabinet-core`에서 module을 공개했다.
- [x] 계층 간 의존성을 확인했다.
- [x] filesystem/env/network 접근이 없는지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 상태 전이가 enum 기반인지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] first-run use model이 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-core::first_run` 모듈을 추가했다.
  - `FirstRunPlan`과 `FirstRunDirectoryRole`을 추가했다.
  - `FirstRunState`, `FirstRunEvent`, `FirstRunTransition`, `FirstRunErrorCode`, `FirstRunError`를 추가했다.
  - first-run core가 I/O를 직접 수행하지 않는지 확인하는 `scripts/check_first_run_boundaries.sh`를 추가했다.
  - first-run plan, 정상 전이, invalid transition, 실패와 retry 테스트를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/first_run.rs`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-core/tests/first_run_tests.rs`
  - `scripts/check_first_run_boundaries.sh`
  - `.tasks/task005.md`
  - `.tasks/task006.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-core first_run`: 최초 실행은 `cabinet_core::first_run` 없음으로 실패했고, 구현 후 4개 first-run 테스트가 통과했다.
  - `sh scripts/check_first_run_boundaries.sh`: `first-run boundaries ok`로 통과했다.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 최초 실행은 포맷 차이로 실패했고, `cargo fmt --all` 적용 후 통과했다.
- [x] 검증한 항목:
  - first-run 상태 전이는 enum 기반 pure function이다.
  - first-run plan은 metadata, version store, asset store, search index, workspace root를 모두 포함한다.
  - invalid transition은 명시적 오류로 실패한다.
  - 실패 상태는 retry 가능 여부와 user-facing error code를 포함한다.
  - first-run core에는 filesystem/env/network 직접 접근이 없다.
- [x] 남은 위험 요소:
  - 실제 directory creation port와 clean temp profile smoke는 아직 없다.
  - first-run idempotency는 orchestration 레벨에서 아직 검증되지 않았다.
  - first-run completed/failed Product Log event는 아직 구현되지 않았다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-005 First-run initializer`를 이어서 수행한다.
  - `FirstRunInitializer`는 `FirstRunPlan`을 받아 directory creation port를 호출하고, fake port로 idempotency를 테스트한다.
  - 실제 filesystem adapter는 port contract가 고정된 뒤 별도 adapter 태스크에서 구현한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 first-run orchestration/idempotency, logging foundation, migration, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-005 First-run initializer`의 orchestration/idempotency 부분이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 first-run initializer port, fake port idempotency test, product event outcome으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task007.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 14. Stop Conditions

다음 조건을 확인했다.

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
- [ ] 외부 정보, 권한, 비밀값, 접근 권한이 없어 진행할 수 없다.
- [ ] `AGENTS.md` 원칙과 충돌하는 요구사항이 발견되었다.
- [ ] 테스트 또는 검증 환경이 없어 완료 여부를 판단할 수 없다.
- [ ] 코드베이스 구조가 계획과 크게 달라 태스크 재설계가 필요하다.
- [ ] 사용자 결정이 필요한 아키텍처 선택지가 발생했다.

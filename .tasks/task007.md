# Task 007. First-run Initializer Orchestration과 Idempotency

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-005 First-run initializer`를 이어서 구현하는 것이다.
- [x] 이 태스크는 first-run plan을 실행하는 orchestration을 만들되, 실제 filesystem I/O는 port 뒤로 숨긴다.
- [x] 이 태스크 완료 후 프로젝트는 clean profile, idempotent profile, 실패 profile을 fake port로 검증한다.

## 2. Current Context

- [x] 현재 코드베이스는 `AppConfig`, `FirstRunPlan`, `FirstRunState`, `FirstRunEvent`를 가진다.
- [x] 이전 태스크 Task 006에서 first-run 상태 전이와 directory plan foundation을 완료했다.
- [x] 이번 태스크는 `MVP-005`의 initializer orchestration과 idempotency 검증을 진행했다.
- [x] 현재 확인된 제약 사항은 실제 filesystem adapter가 아직 없다는 점이다. 이번 태스크는 port trait과 fake implementation test로 제한했다.

## 3. Scope

### Included

- [x] first-run directory/metadata port trait을 추가한다.
- [x] `FirstRunInitializer` orchestration을 추가한다.
- [x] clean/idempotent/failure fake port 테스트를 추가한다.

### Excluded

- [x] 실제 filesystem adapter는 이번 태스크에서 구현하지 않았다.
- [x] Product logger 구현체는 이번 태스크에서 구현하지 않았다.
- [x] migration runner는 후속 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: first-run store port를 만들었다.
- [x] 입력: directory role/path와 metadata marker write 요청.
- [x] 출력: created/already-present/failure 결과.
- [x] 성공 조건: initializer가 port trait만 호출하고 filesystem을 직접 호출하지 않는다.
- [x] 실패 조건: core가 `std::fs` 또는 platform-specific API를 직접 사용한다.

### Functional Unit 2

- [x] 구현한 기능: clean profile initializer orchestration을 만들었다.
- [x] 입력: `AppConfig`, fake clean store.
- [x] 출력: completed outcome, created directory count, product event 후보.
- [x] 성공 조건: 모든 directory role을 생성 요청하고 metadata marker write까지 완료한다.
- [x] 실패 조건: 일부 directory role이 누락되거나 metadata write가 먼저 실행된다.

### Functional Unit 3

- [x] 구현한 기능: idempotent/failure flow를 만들었다.
- [x] 입력: fake already-initialized store, fake failing store.
- [x] 출력: completed 또는 failed outcome.
- [x] 성공 조건: 이미 존재하는 directory는 성공으로 처리하고 실패는 retryable error code를 포함한다.
- [x] 실패 조건: 재실행이 실패하거나 실패 상태가 boolean flag로만 표현된다.

## 5. Architecture Notes

- [x] 변경되는 계층은 core runtime foundation이다.
- [x] 실제 filesystem 구현은 port 뒤에 숨긴다.
- [x] initializer는 config와 port를 명시적 인자로 받는다.
- [x] domain/usecase crate는 변경하지 않았다.
- [x] environment, process args, network, DB 접근은 추가하지 않았다.
- [x] side effect는 port trait으로만 요청한다.

## 6. Configuration Rules

- [x] `AppConfig`를 명시적 인자로 받는다.
- [x] 외부 환경 값을 runtime 중간에 재조회하지 않는다.
- [x] 전역 config registry나 mutable singleton을 만들지 않는다.
- [x] 테스트는 environment를 변경하지 않고 explicit config를 생성한다.
- [x] runtime 중간에 설정 값을 삽입하거나 변경하는 API를 만들지 않는다.

## 7. Logging Requirements

### Product Log

- [x] logger implementation은 추가하지 않았다.
- [x] initializer outcome은 future Product Log event 후보를 enum으로 반환한다.
- [x] Product event 후보는 `FirstRunCompleted`, `FirstRunFailed`만 허용한다.
- [x] 문서 본문, 첨부 내용, secret, raw directory path를 event payload에 넣지 않았다.

### Field Debug Log

- [x] Field Debug Log 구현은 추가하지 않았다.
- [x] future diagnostic은 directory role과 state만 기록할 수 있도록 구조를 둔다.
- [x] raw path와 민감 정보는 후속 masking 정책에서 제한한다.

### Development Log

- [x] runtime Development Log를 추가하지 않았다.
- [x] fake port call 검증은 test assertion으로 처리했다.
- [x] 프로덕션 기본 동작에 포함되는 개발용 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] initializer는 `transition_first_run`을 통해 상태를 변경한다.
- [x] 정상 흐름은 `Completed`로 종료한다.
- [x] directory creation 실패는 `Failed { StoreCreationFailed, retryable: true }`로 종료한다.
- [x] metadata write 실패는 `Failed { MetadataWriteFailed, retryable: true }`로 종료한다.
- [x] 상태 변경 결과는 테스트 가능하다.
- [x] 상태 변경은 future Product Log event 후보와 연결된다.

## 9. TDD Plan

- [x] 실패하는 clean profile initializer test를 먼저 작성했다.
- [x] 실패하는 idempotent initializer test를 먼저 작성했다.
- [x] 실패하는 store failure test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 fake port로 대체했다.
- [x] 설정, 로그 outcome, 상태 전이를 검증 대상에 포함했다.
- [x] 구현 후 중복과 구조 문제를 정리했다.

## 10. Implementation Checklist

- [x] initializer tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] first-run port trait을 작성했다.
- [x] `FirstRunInitializer`를 작성했다.
- [x] fake port 테스트를 통과시켰다.
- [x] core가 filesystem/env/network를 직접 접근하지 않는지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] initializer가 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `FirstRunStore` port trait을 추가했다.
  - `FirstRunStoreStatus`, `FirstRunInitializer`, `FirstRunInitializationOutcome`, `FirstRunProductEvent`를 추가했다.
  - initializer가 `AppConfig`와 `FirstRunStore`를 명시적으로 받아 실행하도록 구현했다.
  - metadata marker write도 명시적 metadata directory path를 받도록 port 계약을 정리했다.
  - clean/idempotent/failure fake port 테스트를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/first_run.rs`
  - `crates/cabinet-core/tests/first_run_initializer_tests.rs`
  - `.tasks/task006.md`
  - `.tasks/task007.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-core first_run_initializer`: 최초 실행은 initializer/port 타입 없음으로 실패했고, 구현 후 3개 테스트가 통과했다.
  - `sh scripts/check_first_run_boundaries.sh`: 통과.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
- [x] 검증한 항목:
  - clean fake profile은 5개 directory role을 생성 요청하고 metadata marker를 쓴다.
  - already initialized fake profile은 재실행 시 idempotent하게 완료된다.
  - store creation failure는 retryable failed state와 `FirstRunFailed` event 후보를 반환한다.
  - Product event 후보에는 raw directory path나 민감 정보가 없다.
  - core에는 filesystem/env/network 직접 접근이 없다.
- [x] 남은 위험 요소:
  - 실제 filesystem adapter와 clean temp profile smoke는 아직 없다.
  - `MVP-005`의 required validation인 clean temp profile test는 다음 태스크에서 완료해야 한다.
  - Product logger port와 실제 log emission은 후속 logging foundation 태스크가 필요하다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-005 First-run initializer`를 실제 local filesystem adapter로 마무리한다.
  - `cabinet-adapters`에서 `FirstRunStore` 구현체를 만들고 clean temp profile test와 idempotent rerun test를 작성한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 first-run filesystem adapter, logging foundation, migration, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-005 First-run initializer`의 clean temp profile validation이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 local filesystem adapter, clean temp profile smoke, idempotent rerun smoke로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task008.md`다.
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

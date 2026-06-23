# Task 009. Local Migration Runner Foundation

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-006 Local migration runner`를 시작하는 것이다.
- [x] 이 태스크는 local migration 흐름을 명시적 상태머신과 version recording port로 표현한다.
- [x] 이 태스크 완료 후 프로젝트는 initial no-op migration plan과 fake store 기반 version 기록 테스트를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 bootstrap config와 first-run local adapter를 가진다.
- [x] 이전 태스크 Task 008에서 `MVP-005 First-run initializer`의 clean temp profile validation을 완료했다.
- [x] 이번 태스크는 Phase 2 exit gate에 필요한 local migration foundation을 시작했다.
- [x] 현재 확인된 제약 사항은 실제 metadata store adapter가 아직 없다는 점이다. 이번 태스크는 core migration runner와 fake store test로 제한했다.

## 3. Scope

### Included

- [x] migration 상태, 이벤트, 전이 결과를 추가한다.
- [x] initial no-op migration plan을 추가한다.
- [x] migration store port와 fake version recording test를 추가한다.

### Excluded

- [x] 실제 metadata store filesystem adapter는 후속 태스크로 넘긴다.
- [x] multi-process lock 구현은 후속 adapter/fault-injection 태스크로 넘긴다.
- [x] Product logger 구현체는 후속 logging foundation 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: migration state machine을 만들었다.
- [x] 입력: `MigrationState`, `MigrationEvent`.
- [x] 출력: `MigrationTransition`.
- [x] 성공 조건: lock, run, success, failure, retry 전이가 테스트로 검증된다.
- [x] 실패 조건: invalid transition이 조용히 성공한다.

### Functional Unit 2

- [x] 구현한 기능: initial no-op migration plan을 만들었다.
- [x] 입력: migration plan constructor.
- [x] 출력: version 1 initial no-op migration step.
- [x] 성공 조건: plan은 stable version과 name을 가진다.
- [x] 실패 조건: migration version이 free-form string으로만 관리된다.

### Functional Unit 3

- [x] 구현한 기능: fake store 기반 version recording runner를 만들었다.
- [x] 입력: `MigrationPlan`, `MigrationStore`.
- [x] 출력: completed outcome, recorded versions, product event 후보.
- [x] 성공 조건: 미적용 version은 기록되고 이미 적용된 version은 중복 기록되지 않는다.
- [x] 실패 조건: migration 상태가 flag 조합이나 숨겨진 global state로 관리된다.

## 5. Architecture Notes

- [x] 변경되는 계층은 core runtime foundation이다.
- [x] migration runner는 store port만 호출한다.
- [x] 실제 filesystem/DB/network 접근은 추가하지 않았다.
- [x] domain/usecase crate는 변경하지 않았다.
- [x] migration side effect는 port trait으로만 요청한다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] process environment를 읽지 않았다.
- [x] runtime 중간 설정 변경 API를 만들지 않았다.
- [x] migration runner는 config global singleton에 의존하지 않는다.
- [x] 테스트는 fake store를 명시적으로 생성한다.

## 7. Logging Requirements

### Product Log

- [x] logger implementation은 추가하지 않았다.
- [x] migration outcome은 future Product Log event 후보를 enum으로 반환한다.
- [x] Product event 후보는 `MigrationCompleted`, `MigrationFailed`만 허용한다.
- [x] raw path, 문서 본문, 첨부 내용, secret을 event payload에 넣지 않았다.

### Field Debug Log

- [x] Field Debug Log 구현은 추가하지 않았다.
- [x] future diagnostic은 migration state, version, error code만 기록할 수 있도록 구조를 둔다.
- [x] raw storage detail과 민감 정보는 후속 masking 정책에서 제한한다.

### Development Log

- [x] runtime Development Log를 추가하지 않았다.
- [x] fake store call 검증은 test assertion으로 처리했다.
- [x] 프로덕션 기본 동작에 포함되는 개발용 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] `MigrationState`는 `NotStarted`, `Locked`, `Running`, `Completed`, `Failed`, `Retrying`을 포함한다.
- [x] `MigrationEvent`는 `AcquireLock`, `RunMigration`, `MigrationSucceeded`, `MigrationFailed`, `Retry`, `ReleaseLock`을 포함한다.
- [x] invalid transition은 `MigrationError::InvalidTransition`으로 실패한다.
- [x] 실패 상태는 retry 가능 여부와 error code를 가진다.
- [x] 상태 전이는 unit test로 검증한다.
- [x] 상태머신은 UI, 외부 어댑터, 인프라에 의존하지 않는다.

## 9. TDD Plan

- [x] 실패하는 migration transition test를 먼저 작성했다.
- [x] 실패하는 migration plan test를 먼저 작성했다.
- [x] 실패하는 fake store version recording test를 먼저 작성했다.
- [x] 실패와 retry 전이 테스트를 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 fake store로 대체했다.
- [x] 설정, 로그 outcome, 상태 전이를 검증 대상에 포함했다.

## 10. Implementation Checklist

- [x] migration tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] migration module을 작성했다.
- [x] `cabinet-core`에서 module을 공개했다.
- [x] fake store 테스트를 통과시켰다.
- [x] core가 filesystem/env/network를 직접 접근하지 않는지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] migration runner가 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-core::migration` 모듈을 추가했다.
  - `MigrationState`, `MigrationEvent`, `MigrationTransition`, `MigrationErrorCode`, `MigrationError`를 추가했다.
  - `MigrationVersion`, `MigrationStep`, `MigrationPlan::initial`을 추가했다.
  - `MigrationStore` port trait과 `MigrationRunner`를 추가했다.
  - `MigrationProductEvent`와 `MigrationOutcome`을 추가했다.
  - migration core I/O 금지 경계 검사 `scripts/check_migration_boundaries.sh`를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/migration.rs`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-core/tests/migration_tests.rs`
  - `scripts/check_migration_boundaries.sh`
  - `.tasks/task008.md`
  - `.tasks/task009.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-core migration`: 최초 실행은 `cabinet_core::migration` 없음으로 실패했고, 구현 후 6개 migration 테스트가 통과했다.
  - `sh scripts/check_migration_boundaries.sh`: 통과.
  - `sh scripts/check_first_run_boundaries.sh`: 통과.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
- [x] 검증한 항목:
  - migration 상태 전이는 enum 기반 pure function이다.
  - invalid transition은 명시적 오류로 실패한다.
  - 실패 상태는 retry 가능 여부와 error code를 포함한다.
  - initial no-op migration은 stable version 1과 이름을 가진다.
  - 미적용 migration version은 fake store에 기록되고, 이미 적용된 version은 중복 기록되지 않는다.
  - migration core에는 filesystem/env/network 직접 접근이 없다.
- [x] 남은 위험 요소:
  - 실제 metadata filesystem adapter가 아직 없다.
  - lock의 multi-process 안전성은 아직 구현하지 않았다.
  - migration completed/failed Product Log event의 실제 emission은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MigrationStore`를 구현하는 local metadata adapter를 만들고 temp profile에서 version 기록을 검증한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 local migration adapter, logging foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 local migration metadata adapter다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 local migration store adapter, clean temp version record smoke, idempotent rerun smoke로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task010.md`다.
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

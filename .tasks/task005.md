# Task 005. Bootstrap Config Object와 Environment Snapshot

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-004 Bootstrap config object`를 시작하는 것이다.
- [x] 이 태스크는 외부 환경 값을 runtime 중간에 재조회하지 않고 explicit config object로 전달하는 구조를 만든다.
- [x] 이 태스크 완료 후 프로젝트는 `AppConfig`, `LocalPathsConfig`, `LoggingConfig`, `StorageConfig`, `SearchConfig`, `ExternalEnvironmentSnapshot`, `BootstrapConfigInput`을 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 Rust core workspace, frontend package skeleton, desktop shell boundary scaffold를 가진 상태다.
- [x] 이전 태스크 Task 004에서 desktop shell crate와 boundary check를 완료했고 Phase 1을 종료했다.
- [x] 이번 태스크는 Phase 2 runtime foundation의 첫 작업으로, 모든 runtime 설정이 명시적 입력으로 전달되도록 만들기 위해 시작했다.
- [x] 현재 확인된 제약 사항은 실제 process environment 읽기와 filesystem 초기화가 아직 없다는 점이다. 이번 태스크는 순수 config value object와 boundary check로 제한했다.

## 3. Scope

### Included

- [x] `cabinet-core`에 config value object를 추가한다.
- [x] explicit environment snapshot 기반 bootstrap input을 추가한다.
- [x] config validation test와 hidden env access boundary check를 추가한다.

### Excluded

- [x] 실제 process environment 읽기 구현은 이번 태스크에서 하지 않았다.
- [x] first-run initializer와 directory creation은 후속 태스크로 넘긴다.
- [x] logger implementation은 후속 logging foundation 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: config value objects를 만들었다.
- [x] 입력: app data path, workspace root path, logging/search/storage 정책.
- [x] 출력: `AppConfig`, `LocalPathsConfig`, `LoggingConfig`, `StorageConfig`, `SearchConfig`.
- [x] 성공 조건: config object가 명시적으로 생성되고 immutable value처럼 전달 가능하다.
- [x] 실패 조건: config가 global singleton 또는 hidden registry로 구현된다.

### Functional Unit 2

- [x] 구현한 기능: environment snapshot 기반 bootstrap input을 만들었다.
- [x] 입력: 시작 시점에 수집된 key/value snapshot.
- [x] 출력: `ExternalEnvironmentSnapshot`, `BootstrapConfigInput`.
- [x] 성공 조건: core config는 `std::env`를 직접 읽지 않는다.
- [x] 실패 조건: 함수 내부에서 environment를 재조회한다.

### Functional Unit 3

- [x] 구현한 기능: config validation과 boundary check를 만들었다.
- [x] 입력: config input과 core source files.
- [x] 출력: unit tests, `scripts/check_runtime_config_boundaries.sh`.
- [x] 성공 조건: invalid path와 hidden env access를 검출한다.
- [x] 실패 조건: invalid config나 `std::env` 접근이 통과한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 core runtime foundation이다.
- [x] domain crate는 변경하지 않았다.
- [x] usecase crate는 변경하지 않았다.
- [x] config object는 domain model이 아니다.
- [x] process env access는 core 내부에서 수행하지 않고 explicit snapshot으로만 받는다.
- [x] 전역 상태, 숨겨진 I/O, 암묵적 설정 접근을 피했다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] 환경 값은 프로그램 시작 시 최초 1회 수집된 snapshot으로만 전달한다.
- [x] 최초 수신 이후에는 환경 값을 전역 상수처럼 사용하지 않는다.
- [x] 환경 값은 명시적 input object와 config object로 전달한다.
- [x] 프로세스 중간에 환경 설정 값을 삽입하거나 변경하지 않는다.
- [x] 런타임 중간 재설정, 동적 환경 변경, 숨겨진 설정 조회를 금지한다.

## 7. Logging Requirements

### Product Log

- [x] 이번 태스크는 logger implementation을 추가하지 않았다.
- [x] config validation error는 후속 runtime/usecase에서 stable error code로 변환한다.
- [x] 민감 정보와 과도한 내부 상태를 기록하지 않았다.

### Field Debug Log

- [x] 이번 태스크는 Field Debug Log를 추가하지 않았다.
- [x] 활성화 조건은 후속 logging foundation 태스크에서 정의한다.
- [x] config의 비민감 요약만 future diagnostic 대상으로 허용한다.
- [x] 민감 정보 마스킹 기준을 위반하는 코드를 추가하지 않았다.

### Development Log

- [x] 이번 태스크는 런타임 Development Log 구현을 추가하지 않았다.
- [x] 검증 결과는 Completion Report에 기록했다.
- [x] 프로덕션 기본 동작에 포함되는 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신 구현이 필요하지 않다.
- [x] 복잡한 내부 흐름을 암묵적 플래그 조합으로 관리하지 않았다.
- [x] FirstRun state machine은 후속 태스크로 넘긴다.
- [x] Migration state machine은 후속 태스크로 넘긴다.
- [x] 상태 전이는 이번 태스크에서 구현하지 않았다.

## 9. TDD Plan

- [x] 실패하는 config test를 먼저 작성했다.
- [x] 테스트 대상은 config value object와 bootstrap input이다.
- [x] 정상 케이스 테스트를 작성했다.
- [x] 실패 케이스 테스트를 작성했다.
- [x] 경계값 테스트는 empty path와 missing app data path로 수행했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 설정 값 전달 방식 테스트를 작성했다.
- [x] 로그 정책 검증은 런타임 로그 코드 없음으로 검증했다.
- [x] 상태 전이가 없으므로 상태 전이 테스트는 작성하지 않았다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 테스트 통과 후 구조를 정리했다.

## 10. Implementation Checklist

- [x] config tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] 최소 config object 구현을 작성했다.
- [x] 계층 간 의존성을 확인했다.
- [x] 외부 의존성이 경계 계층에 추가되지 않았는지 확인했다.
- [x] 설정 값 전달 방식이 명시적인지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 상태 관리 구현을 추가하지 않았다.
- [x] 중복과 구조 문제를 정리했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] config construction이 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-core::config` 모듈을 추가했다.
  - `AppConfig`, `LocalPathsConfig`, `LoggingConfig`, `StorageConfig`, `SearchConfig`를 추가했다.
  - `ExternalEnvironmentSnapshot`과 `BootstrapConfigInput`을 추가했다.
  - 외부 환경 직접 접근을 금지하는 `scripts/check_runtime_config_boundaries.sh`를 추가했다.
  - config 생성, validation, bootstrap input 소비 테스트를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/config.rs`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-core/tests/config_tests.rs`
  - `scripts/check_runtime_config_boundaries.sh`
  - `.tasks/task005.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-core config`: 최초 실행은 `cabinet_core::config` 없음으로 실패했고, 구현 후 4개 config 테스트가 통과했다.
  - `cargo test -p cabinet-core bootstrap_config_input`: 최초 실행은 `BootstrapConfigInput` 없음으로 실패했고, 구현 후 통과했다.
  - `sh scripts/check_runtime_config_boundaries.sh`: 최초 실행은 `config.rs` 없음으로 실패했고, 구현 후 `runtime config boundaries ok`로 통과했다.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
- [x] 검증한 항목:
  - core/usecase/domain source에서 `std::env`, `process::Command`, `dotenv`, `env!` 직접 접근이 없다.
  - config는 snapshot을 명시적으로 받아 생성된다.
  - missing app data dir과 empty app data dir은 실패한다.
  - workspace root가 없으면 app data dir 하위 `workspaces`로 결정된다.
  - Product/Field/Development runtime log 구현은 아직 추가하지 않았다.
- [x] 남은 위험 요소:
  - 실제 process environment를 최초 1회 수집하는 bootstrap adapter는 아직 없다.
  - first-run directory creation과 idempotency는 아직 없다.
  - logging foundation과 stable error code mapping은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-005 First-run initializer`를 시작한다.
  - first-run은 filesystem I/O를 직접 수행하지 않고, 상태 전이와 directory plan을 먼저 순수 모델로 검증한다.
  - 실제 directory creation은 port/adapter contract가 준비되는 후속 태스크로 넘긴다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 first-run, logging foundation, migration, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-005 First-run initializer`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 first-run state machine, directory plan, validation으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task006.md`다.
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

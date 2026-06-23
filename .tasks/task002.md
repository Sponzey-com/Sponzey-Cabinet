# Task 002. Rust Core Workspace Skeleton과 Boundary Smoke Test

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 1의 `MVP-001 Core workspace scaffold`를 시작하는 것이다.
- [x] 이 태스크는 `plan.md`의 Layered Architecture, Clean Architecture, TDD, Dependency Boundary 기준에 기여한다.
- [x] 이 태스크 완료 후 프로젝트는 domain/usecase/port/adapter/platform 계층을 담을 Rust workspace skeleton과 최소 검증 명령을 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 문서와 task 운영 파일만 있는 상태에서 시작했다.
- [x] 이전 태스크인 Task 001은 Phase 0 작업 루프, phase gate tracker, decision record 기준을 완료했다.
- [x] 이번 태스크는 Phase 1 진입 후 가장 먼저 필요한 Rust core workspace boundary를 만들기 위해 시작했다.
- [x] 현재 확인된 제약 사항은 frontend workspace와 desktop shell이 아직 없다는 점이다.

## 3. Scope

### Included

- [x] Rust workspace와 core crate skeleton을 생성한다.
- [x] crate 간 의존 방향을 최소 Cargo dependency로 고정한다.
- [x] boundary smoke test와 workspace test를 실행한다.

### Excluded

- [x] frontend workspace scaffold는 이번 태스크에서 다루지 않는다.
- [x] Tauri desktop shell scaffold는 이번 태스크에서 다루지 않는다.
- [x] 실제 document domain model, usecase, storage adapter 구현은 후속 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: Rust workspace skeleton을 만들었다.
- [x] 입력: `.tasks/plan.md` Phase 1, `AGENTS.md` architecture rules.
- [x] 출력: root `Cargo.toml`과 `crates/cabinet-*` skeleton.
- [x] 성공 조건: `cargo test --workspace`가 실행 가능한 workspace가 된다.
- [x] 실패 조건: workspace가 빌드되지 않거나 domain crate가 외부 계층에 의존한다.

### Functional Unit 2

- [x] 구현한 기능: crate dependency direction을 고정했다.
- [x] 입력: Clean Architecture dependency direction.
- [x] 출력: domain, ports, usecases, core, adapters, platform crate dependency layout.
- [x] 성공 조건: domain은 외부 crate에 의존하지 않고 usecases는 domain과 ports에만 직접 의존한다.
- [x] 실패 조건: domain이 adapter/platform/framework에 의존하거나 usecase가 concrete adapter에 의존한다.

### Functional Unit 3

- [x] 구현한 기능: architecture boundary smoke test를 만들었다.
- [x] 입력: `Cargo.toml` dependency declarations.
- [x] 출력: `scripts/check_architecture_boundaries.sh`.
- [x] 성공 조건: boundary check script가 통과한다.
- [x] 실패 조건: 금지 dependency가 발견되어도 검증하지 못한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 workspace skeleton, domain, ports, usecases, core, adapters, platform이다.
- [x] domain crate는 외부 framework, filesystem, network, env, adapter, usecase, platform에 의존하지 않는다.
- [x] ports crate는 domain type을 사용할 수 있지만 adapter 구현체를 알지 않는다.
- [x] usecases crate는 domain과 ports에만 직접 의존한다.
- [x] adapters crate는 ports를 구현할 위치지만 이번 태스크에서는 구현하지 않았다.
- [x] platform crate는 shell boundary 위치지만 이번 태스크에서는 platform SDK를 추가하지 않았다.
- [x] 전역 상태, 숨겨진 I/O, 암묵적 설정 접근을 추가하지 않았다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] 환경 값은 프로그램 시작 시 최초 1회만 수신한다는 원칙을 코드로 위반하지 않았다.
- [x] 최초 수신 이후에는 환경 값을 전역 상수처럼 사용하지 않았다.
- [x] 이번 태스크에서는 환경 값 전달 구조를 구현하지 않았다.
- [x] 프로세스 중간에 환경 설정 값을 삽입하거나 변경하지 않았다.
- [x] 런타임 중간 재설정, 동적 환경 변경, 숨겨진 설정 조회를 추가하지 않았다.

## 7. Logging Requirements

### Product Log

- [x] 이번 태스크는 제품 런타임을 구현하지 않으므로 Product Log를 추가하지 않았다.
- [x] 사용자 영향, 핵심 상태 변화, 장애 원인 추적 로그는 후속 usecase/runtime 태스크에서 정의한다.
- [x] 민감 정보와 과도한 내부 상태를 기록하지 않았다.

### Field Debug Log

- [x] 이번 태스크는 Field Debug Log가 필요하지 않다.
- [x] 활성화 조건은 후속 logging foundation 태스크에서 정의한다.
- [x] 민감 정보 마스킹 기준을 위반하는 코드를 추가하지 않았다.
- [x] 보존 범위와 사용 범위는 후속 logging foundation 태스크에서 제한한다.

### Development Log

- [x] 이번 태스크는 런타임 Development Log 구현을 추가하지 않았다.
- [x] 검증 결과는 Completion Report에 기록했다.
- [x] 프로덕션 기본 동작에 포함되는 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신 구현이 필요하지 않다.
- [x] 복잡한 내부 흐름을 암묵적 플래그 조합으로 관리하지 않았다.
- [x] 상태 목록은 후속 domain/runtime 태스크에서 정의한다.
- [x] 이벤트 목록은 후속 domain/runtime 태스크에서 정의한다.
- [x] 전이 조건은 후속 domain/runtime 태스크에서 정의한다.
- [x] 실패 상태와 종료 상태는 후속 상태머신 태스크에서 정의한다.
- [x] 상태 전이는 이번 태스크에서 구현하지 않았다.

## 9. TDD Plan

- [x] 실패하는 boundary check를 먼저 작성했다.
- [x] 테스트 대상은 workspace skeleton과 crate dependency direction이다.
- [x] 정상 케이스 테스트로 각 crate의 smoke test를 작성했다.
- [x] 실패 케이스 테스트로 boundary script가 금지 dependency pattern을 검사하게 했다.
- [x] 경계값 테스트는 domain crate dependency가 비어 있는지 확인하는 방식으로 수행했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 설정 값 전달 방식 테스트는 이번 태스크 범위가 아니므로 설정 코드 없음으로 검증했다.
- [x] 로그 정책 검증은 런타임 로그 코드 없음으로 검증했다.
- [x] 상태 전이가 없으므로 상태 전이 테스트는 작성하지 않았다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 테스트 통과 후 구조를 정리했다.

## 10. Implementation Checklist

- [x] boundary check script를 먼저 작성했다.
- [x] workspace가 없어서 boundary check가 실패하는 것을 확인했다.
- [x] 최소 Rust workspace와 crate skeleton을 작성했다.
- [x] 계층 간 의존성을 확인했다.
- [x] 외부 의존성이 경계 계층에만 있는지 확인했다.
- [x] 설정 값 전달 방식이 변경되지 않았는지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 상태 관리 구현을 추가하지 않았다.
- [x] 중복과 구조 문제를 정리했다.
- [x] 모든 테스트를 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과했다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] 유스케이스 crate가 명시적 boundary를 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:

  - Rust workspace root `Cargo.toml`을 생성했다.
  - `cabinet-domain`, `cabinet-ports`, `cabinet-usecases`, `cabinet-core`, `cabinet-adapters`, `cabinet-platform` crate skeleton을 생성했다.
  - 각 crate에 최소 smoke test를 추가했다.
  - `scripts/check_architecture_boundaries.sh`를 추가했다.
  - `.tasks/phase-gates.md`의 Phase 1 상태를 `in-progress`로 갱신했다.
- [x] 생성하거나 수정한 파일:

  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/cabinet-domain/Cargo.toml`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-ports/Cargo.toml`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-usecases/Cargo.toml`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-core/Cargo.toml`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-adapters/Cargo.toml`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-platform/Cargo.toml`
  - `crates/cabinet-platform/src/lib.rs`
  - `scripts/check_architecture_boundaries.sh`
  - `.tasks/task001.md`
  - `.tasks/task002.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과:

  - `sh scripts/check_architecture_boundaries.sh`: 최초 실행은 `Cargo.toml` 없음으로 실패했고, 구현 후 `architecture boundaries ok`로 통과했다.
  - `cargo test --workspace`: 6개 crate smoke test 통과.
  - `cargo fmt --all --check`: 통과.
- [x] 검증한 항목:

  - domain crate는 외부 crate dependency가 없다.
  - ports crate는 domain에만 의존한다.
  - usecases crate는 domain과 ports에만 의존한다.
  - adapter/platform SDK 또는 framework dependency를 추가하지 않았다.
  - 환경 변수, runtime config, logging code, state machine code를 추가하지 않았다.
- [x] 남은 위험 요소:

  - Phase 1 exit gate는 아직 완료되지 않았다. frontend workspace와 desktop shell skeleton이 남아 있다.
  - boundary script는 Cargo manifest pattern 기반의 1차 검증이다. 향후 cargo metadata 기반 검증으로 강화할 수 있다.
  - crates는 skeleton만 있으며 실제 domain/usecase 계약은 후속 태스크에서 추가해야 한다.
- [x] 후속 태스크에서 이어받아야 할 내용:

  - 다음 태스크는 `MVP-002 Frontend workspace scaffold`를 다뤄야 한다.
  - package/client boundary는 domain rule을 직접 구현하지 않도록 정적 검증을 포함해야 한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 frontend workspace, desktop shell, runtime foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-002 Frontend workspace scaffold`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 package workspace, shared packages, static boundary check로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task003.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성한다.
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

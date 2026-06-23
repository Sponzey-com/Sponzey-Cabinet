# Task 008. Local First-run Store Adapter와 Clean Temp Profile Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-005 First-run initializer`를 clean temp profile validation까지 완료하는 것이다.
- [x] 이 태스크는 실제 filesystem I/O를 `cabinet-adapters` 구현체에만 배치한다.
- [x] 이 태스크 완료 후 프로젝트는 local directory 자동 생성과 재실행 idempotency를 실제 임시 경로에서 검증한다.

## 2. Current Context

- [x] 현재 코드베이스는 `FirstRunStore` port, `FirstRunInitializer`, fake port tests를 가진다.
- [x] 이전 태스크 Task 007에서 first-run orchestration과 idempotency를 fake port로 검증했다.
- [x] 이번 태스크는 MVP-005 required validation인 clean temp profile test를 실제 adapter로 수행했다.
- [x] 현재 확인된 제약 사항은 external temp helper crate를 추가하지 않는다는 점이다. 테스트는 `/tmp` 하위 고유 경로를 명시적으로 생성하고 정리한다.

## 3. Scope

### Included

- [x] `cabinet-adapters`에 local filesystem first-run store adapter를 추가한다.
- [x] clean temp profile first-run test를 추가한다.
- [x] same temp profile rerun idempotency test를 추가한다.

### Excluded

- [x] migration runner는 후속 태스크로 넘긴다.
- [x] Product logger 구현체는 후속 logging foundation 태스크로 넘긴다.
- [x] packaged app install smoke는 release gate 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: local filesystem first-run adapter를 만들었다.
- [x] 입력: directory role/path, metadata directory path.
- [x] 출력: `FirstRunStoreStatus::Created` 또는 `AlreadyPresent`.
- [x] 성공 조건: adapter만 `std::fs`를 사용한다.
- [x] 실패 조건: core/domain/usecase가 filesystem을 직접 사용한다.

### Functional Unit 2

- [x] 구현한 기능: clean temp profile smoke를 만들었다.
- [x] 입력: `/tmp` 하위 고유 app data path로 만든 `AppConfig`.
- [x] 출력: completed outcome과 실제 생성된 directory/metadata marker.
- [x] 성공 조건: metadata, version store, asset store, search index, workspace root가 실제로 생성된다.
- [x] 실패 조건: 수동 directory 생성이나 외부 설정 파일이 필요하다.

### Functional Unit 3

- [x] 구현한 기능: idempotent rerun smoke를 만들었다.
- [x] 입력: 이미 초기화된 동일 temp profile.
- [x] 출력: completed outcome, already-present directory count.
- [x] 성공 조건: 재실행이 실패하지 않고 기존 directory를 성공으로 처리한다.
- [x] 실패 조건: 이미 존재하는 directory 때문에 first-run이 실패한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 adapter layer와 adapter test다.
- [x] `cabinet-adapters`는 `cabinet-core`의 first-run port를 구현한다.
- [x] core/domain/usecase에는 filesystem access를 추가하지 않았다.
- [x] adapter test는 실제 filesystem을 사용해 adapter contract만 검증한다.
- [x] 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일을 요구하지 않는다.

## 6. Configuration Rules

- [x] `AppConfig`는 test에서 명시적으로 생성한다.
- [x] process environment를 읽지 않는다.
- [x] runtime 중간 설정 변경 API를 만들지 않는다.
- [x] adapter는 config를 전역으로 저장하지 않는다.
- [x] temp path는 test helper에서 명시적으로 생성하고 cleanup한다.

## 7. Logging Requirements

### Product Log

- [x] logger implementation은 추가하지 않았다.
- [x] adapter는 Product Log를 직접 기록하지 않는다.
- [x] outcome enum은 core initializer 결과를 그대로 사용한다.
- [x] raw path를 Product event payload에 추가하지 않았다.

### Field Debug Log

- [x] Field Debug Log 구현은 추가하지 않았다.
- [x] adapter 내부 path detail은 test assertion으로만 검증한다.
- [x] 운영 diagnostic은 후속 logging foundation 태스크에서 masking 정책과 함께 정의한다.

### Development Log

- [x] runtime Development Log를 추가하지 않았다.
- [x] 테스트용 filesystem cleanup 실패는 test failure가 아니라 test guard cleanup으로 처리한다.
- [x] 프로덕션 기본 동작에 포함되는 개발용 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] adapter는 상태 전이를 직접 구현하지 않는다.
- [x] 상태 전이는 `FirstRunInitializer`와 `transition_first_run`을 통해서만 발생한다.
- [x] clean temp profile은 `Completed`로 종료한다.
- [x] rerun profile은 `Completed`로 종료한다.
- [x] filesystem failure simulation은 이번 태스크 범위에서 제외하고 후속 fault-injection 태스크에서 다룬다.

## 9. TDD Plan

- [x] 실패하는 adapter clean temp profile test를 먼저 작성했다.
- [x] 실패하는 adapter idempotent rerun test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 adapter 구현만 작성했다.
- [x] 외부 의존성은 standard library filesystem으로만 제한했다.
- [x] core boundary check가 계속 통과하는지 확인했다.
- [x] 구현 후 중복과 구조 문제를 정리했다.

## 10. Implementation Checklist

- [x] adapter tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] `LocalFirstRunStore` adapter를 작성했다.
- [x] adapter crate dependency를 필요한 만큼만 추가했다.
- [x] clean temp profile smoke를 통과시켰다.
- [x] idempotent rerun smoke를 통과시켰다.
- [x] core가 filesystem/env/network를 직접 접근하지 않는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] adapter가 port/interface 뒤에 숨겨진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-adapters::local_first_run::LocalFirstRunStore`를 추가했다.
  - `FIRST_RUN_MARKER_FILE` marker 파일명을 명시했다.
  - `cabinet-adapters`가 `cabinet-core` first-run port를 구현하도록 dependency를 추가했다.
  - clean temp profile smoke와 idempotent rerun smoke를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-adapters/Cargo.toml`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/src/local_first_run.rs`
  - `crates/cabinet-adapters/tests/local_first_run_store_tests.rs`
  - `.tasks/task007.md`
  - `.tasks/task008.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-adapters local_first_run_store`: 최초 실행은 `cabinet_core` dependency와 `local_first_run` 모듈 없음으로 실패했고, 구현 후 2개 adapter smoke가 통과했다.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `sh scripts/check_first_run_boundaries.sh`: 통과.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - clean temp profile에서 metadata, version store, asset store, search index, workspace root가 생성된다.
  - metadata marker 파일이 metadata directory 아래 생성된다.
  - 동일 temp profile에서 재실행해도 실패하지 않고 already-present로 완료된다.
  - filesystem I/O는 adapter 계층에만 존재한다.
  - core first-run boundary에는 filesystem/env/network 접근이 없다.
- [x] 남은 위험 요소:
  - packaged desktop artifact 기준 first-run smoke는 release gate에서 별도 검증해야 한다.
  - filesystem permission failure와 partial initialization recovery는 fault-injection 태스크가 필요하다.
  - Product logger port와 실제 log emission은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - `MVP-005 First-run initializer`의 core/fake/local adapter validation은 완료했다.
  - 다음 태스크는 Phase 2의 `MVP-006 Local migration runner`를 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 migration runner, logging foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-006 Local migration runner`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 migration state machine, no-op migration plan, version recording port contract로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task009.md`다.
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

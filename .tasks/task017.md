# Task 017. Asset Lifecycle State Machine

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-015 Asset lifecycle state machine`을 구현하는 것이다.
- [x] 이 태스크는 asset 상태를 명시적 상태, 이벤트, 전이 결과로 표현한다.

## 2. Scope

- [x] `AssetLifecycleState`와 `AssetLifecycleEvent`를 추가한다.
- [x] valid transition function을 추가한다.
- [x] invalid transition error를 추가한다.

## 3. TDD Plan

- [x] 실패하는 valid transition test를 먼저 작성했다.
- [x] 실패하는 invalid transition test를 먼저 작성했다.

## 4. Completion Report

- [x] 수행한 변경 사항:
  - `AssetLifecycleState`, `AssetLifecycleEvent`, `AssetLifecycleTransition`을 추가했다.
  - `transition_asset_lifecycle` pure transition function을 추가했다.
  - `AssetError::InvalidLifecycleTransition`을 추가했다.
  - valid asset flow와 invalid transition tests를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/asset.rs`
  - `crates/cabinet-domain/tests/asset_lifecycle_tests.rs`
  - `.tasks/task016.md`
  - `.tasks/task017.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain asset_lifecycle`: 최초 실행은 lifecycle 타입/함수/error 없음으로 실패했고, 구현 후 2개 lifecycle 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
  - `cargo test --workspace`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - registered/link/unlink/archive/restore/missing 전이가 enum 기반으로 표현된다.
  - invalid transition은 state와 event를 포함한 domain error로 실패한다.
  - asset lifecycle transition은 storage/UI/logger에 의존하지 않는다.
- [x] 남은 위험 요소:
  - asset store adapter와 missing file fault handling은 후속 phase에서 필요하다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-016 Version domain model`을 시작한다.

## 5. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 다음 우선순위는 `MVP-016 Version domain model`이다.
- [x] 다음 태스크 파일명은 `.tasks/task018.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 059. Performance Benchmark Harness

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-100 Performance fixture design`을 구현하는 것이다.
- [x] 이 태스크는 p95 300ms release gate가 공통으로 사용할 fixture profile, measurement environment, sample report 모델을 정의한다.

## 2. Scope

- [x] small, medium, large fixture profile을 deterministic 값으로 정의한다.
- [x] measurement environment metadata를 명시적 값 객체로 정의한다.
- [x] benchmark target enum을 정의한다.
- [x] benchmark sample과 p95 계산 report를 정의한다.
- [x] p95 계산은 외부 환경, filesystem, clock, global state에 의존하지 않는다.
- [x] p95 300ms 판단 함수는 target별 sample만 사용한다.

## 3. TDD Plan

- [x] 실패하는 Rust performance harness 테스트를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `crates/cabinet-core/src/performance.rs`를 구현한다.
- [x] `cargo test -p cabinet-core --test performance_tests --quiet`를 실행한다.
- [x] 전체 workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] performance harness는 core 계층에 두고 외부 I/O를 수행하지 않는다.
- [x] benchmark 실행 adapter는 이후 태스크에서 port/usecase path를 주입받는다.
- [x] benchmark config는 명시적 객체로 전달한다.
- [x] Development Log raw samples는 production default artifact에 포함하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `PerformanceFixtureProfile`, `MeasurementEnvironment`, `PerformanceTarget`, `PerformanceSample`, `PerformanceReport`를 추가했다.
  - p95 nearest-rank 계산과 target별 300ms 목표 판정 함수를 추가했다.
  - fixture profile은 small/medium/large deterministic scale을 기록한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-core/src/performance.rs`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-core/tests/performance_tests.rs`
  - `.tasks/task059.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-core --test performance_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
  - `node scripts/check_platform_adapter_smoke.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
- [x] 다음 태스크를 결정한다.
  - Task 060. Query Performance Benchmarks

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

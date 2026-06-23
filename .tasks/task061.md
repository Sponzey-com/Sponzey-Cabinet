# Task 061. Query Performance Benchmarks

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-101`부터 `MVP-105`까지의 query p95 benchmark를 구현하는 것이다.
- [x] 이 태스크는 current, history, specific version, search, link/backlink, asset metadata lookup이 p95 300ms 목표를 만족하는지 release gate에서 검증한다.

## 2. Scope

- [x] platform integration test에서 local adapters와 usecases를 조합한다.
- [x] small deterministic fixture를 생성한다.
- [x] current document lookup p95를 측정한다.
- [x] history list lookup p95를 측정한다.
- [x] specific version lookup p95를 측정한다.
- [x] search lookup p95를 측정한다.
- [x] link/backlink lookup p95를 측정한다.
- [x] asset metadata lookup p95를 측정한다.

## 3. TDD Plan

- [x] 실패하는 platform query benchmark test를 먼저 작성한다.
- [x] 누락된 platform dependency와 fixture setup을 구현한다.
- [x] `cargo test -p cabinet-platform --test query_performance_benchmarks --quiet`를 실행한다.
- [x] 전체 workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] benchmark는 usecase와 adapter 경계를 우회하지 않는다.
- [x] benchmark fixture config는 명시적 fixture profile과 measurement environment로 전달한다.
- [x] benchmark는 runtime 중간 env 변경이나 hidden global config를 사용하지 않는다.
- [x] benchmark는 사용자-facing Git 개념이나 Git CLI를 사용하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `cabinet-platform` integration benchmark를 추가했다.
  - local document/version/search/link/document-asset adapters와 usecases를 조합해 small fixture를 생성한다.
  - current/history/specific version/search/link-backlink/asset metadata p95 300ms 목표를 `PerformanceReport`로 검증한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-platform/tests/query_performance_benchmarks.rs`
  - `crates/cabinet-platform/Cargo.toml`
  - `.tasks/task061.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-platform --test query_performance_benchmarks --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
  - `node scripts/check_platform_adapter_smoke.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
- [x] 다음 태스크를 결정한다.
  - Task 062. Clean Install Smoke

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

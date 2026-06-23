# Task 026. Local Setup Health Checker and Git CLI Absence Check

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 4의 `LocalSetupHealthChecker`를 구현하고 Git CLI 비의존 검증을 추가하는 것이다.
- [x] 이 태스크는 설치 1회 로컬 실행에 필요한 store directory와 metadata marker 상태를 명시적으로 점검한다.
- [x] 이 태스크는 내부 version store가 Git CLI 또는 user-facing Git 개념에 의존하지 않도록 정적 검증을 추가한다.

## 2. Scope

- [x] `LocalSetupHealthChecker`를 추가한다.
- [x] health role과 issue kind를 정의한다.
- [x] metadata, version store, asset store, search index, workspace root directory를 점검한다.
- [x] first-run metadata marker 존재 여부를 점검한다.
- [x] 누락 directory, file로 잘못 생성된 경로, marker 누락을 구분한다.
- [x] Git CLI 의존성 금지 check script를 추가한다.

## 3. TDD Plan

- [x] 실패하는 healthy first-run profile test를 먼저 작성한다.
- [x] 실패하는 missing directory issue test를 먼저 작성한다.
- [x] 실패하는 path is file issue test를 먼저 작성한다.
- [x] 실패하는 missing marker issue test를 먼저 작성한다.

## 4. Architecture Rules

- [x] filesystem 접근은 adapter 계층에만 둔다.
- [x] health checker는 bootstrap에서 검증된 `LocalPathsConfig`를 명시적으로 받는다.
- [x] health checker는 환경 변수를 읽지 않는다.
- [x] health checker는 setup 상태를 보고하고 자동 수정하지 않는다.
- [x] Git CLI 검증은 `git` 실행이 아니라 source scan으로 수행한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalSetupHealthChecker`를 추가해 first-run 이후 필수 local directory와 metadata marker 상태를 점검한다.
  - healthy, missing directory, path-is-file, missing marker 상태를 구분한다.
  - `scripts/check_no_git_cli_dependency.sh`를 추가해 Git CLI/process command/git implementation library 의존을 정적으로 거부한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_setup_health.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_setup_health_checker_tests.rs`
  - `scripts/check_no_git_cli_dependency.sh`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_setup_health_checker_tests --quiet`: initial fail, missing health module
  - `cargo test -p cabinet-adapters --test local_setup_health_checker_tests --quiet`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 027은 local atomic write utility와 recovery/failure tests를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 062. Clean Install Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-110 Clean machine install smoke`를 구현하는 것이다.
- [x] 이 태스크는 수동 설정 파일, 외부 DB, 외부 검색 서버, Git CLI, Node.js runtime 없이 local first-run이 완료되는지 검증한다.

## 2. Scope

- [x] platform release smoke API를 추가한다.
- [x] clean install smoke input은 명시적 app data dir만 받는다.
- [x] smoke는 `AppConfig`, `FirstRunInitializer`, `LocalFirstRunStore`, `LocalSetupHealthChecker`를 조합한다.
- [x] smoke 결과는 completed/healthy 상태와 생성된 directory 수를 보고한다.
- [x] smoke는 runtime 중간 env 변경이나 hidden global config를 사용하지 않는다.

## 3. TDD Plan

- [x] 실패하는 clean install smoke test를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 platform release smoke API를 구현한다.
- [x] `cargo test -p cabinet-platform --test clean_install_smoke --quiet`를 실행한다.
- [x] 전체 workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] release smoke 조합은 platform 계층에 둔다.
- [x] core/usecase/domain 계층은 filesystem이나 environment에 직접 접근하지 않는다.
- [x] clean install smoke는 local setup 입력을 명시적 값으로 전달한다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `cabinet-platform`에 clean install release smoke API를 추가했다.
  - smoke는 명시적 app data directory만 입력으로 받고 `ExternalEnvironmentSnapshot`을 내부 값으로 구성해 `AppConfig`, first-run initializer, local setup health checker를 조합한다.
  - 수동 설정 파일, 외부 DB, 외부 검색 서버, Git CLI, Node.js runtime 없이 최초 실행 local directory가 생성되고 health check가 통과하는지 검증한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-platform/src/release_smoke.rs`
  - `crates/cabinet-platform/src/lib.rs`
  - `crates/cabinet-platform/tests/clean_install_smoke.rs`
  - `.tasks/task062.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-platform --test clean_install_smoke --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
  - `node scripts/check_platform_adapter_smoke.mjs`: pass
  - `node scripts/check_frontend_boundaries.mjs`: pass
  - `sh scripts/check_desktop_shell_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 063은 local data preservation smoke를 구현한다. clean install 이후 문서, 이력, 링크, 검색 index, 첨부 metadata가 재실행 후에도 보존되는지 검증한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

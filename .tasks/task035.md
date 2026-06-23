# Task 035. RestoreDocumentVersion Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-027 RestoreDocumentVersion` usecase를 구현하는 것이다.
- [x] 이 태스크는 선택한 이력 version을 current snapshot으로 복원하고, 복원 자체를 새 version entry로 기록한다.

## 2. Scope

- [x] `RestoreDocumentVersionInput`과 `RestoreDocumentVersionOutput`을 추가한다.
- [x] current snapshot과 target version snapshot을 읽는다.
- [x] target version body로 restore version record를 append한다.
- [x] current snapshot을 target version body로 교체한다.
- [x] restore 성공 시 domain event와 Product Log event를 기록한다.
- [x] missing current 또는 target version을 `NotFound`로 보고한다.
- [x] current update 실패 시 기존 current snapshot이 보존되는지 테스트한다.

## 3. TDD Plan

- [x] 실패하는 restore success flow test를 먼저 작성한다.
- [x] 실패하는 missing target not found test를 먼저 작성한다.
- [x] 실패하는 current update failure preserves current test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`, `VersionStore`, event publisher, product logger port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI component를 import하지 않는다.
- [x] restore 절차는 명시적 상태 전이 결과를 output 또는 error에 포함한다.
- [x] restore는 사용자에게 Git, commit, PR 개념을 노출하지 않는다.
- [x] restore version id와 snapshot ref는 외부 clock/random 전역 접근 없이 명시적 input으로 받는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `RestoreDocumentVersionUsecase`를 추가했다.
  - restore는 current와 target version을 읽고, target body를 새 restore version으로 append한 뒤 current snapshot을 교체한다.
  - restore 성공 시 `DocumentRestored` event와 Product Log event를 남긴다.
  - missing target은 write 없이 `NotFound`를 반환한다.
  - current update 실패 시 기존 current snapshot이 유지되는 테스트를 추가했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/restore_document_version_tests.rs`
  - `.tasks/task035.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test restore_document_version_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 036은 `UpdateDocument` usecase를 구현한다.
  - update는 current snapshot을 새 body로 교체하고 새 version entry를 append해야 한다.
  - update 성공/실패 로그, event, current/history 분리 조건을 테스트한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 047. Markdown Folder Import Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-070 Markdown folder import`를 구현하는 것이다.
- [x] 이 태스크는 filesystem adapter가 넘긴 Markdown entries를 current document와 version history로 가져온다.

## 2. Scope

- [x] `ImportMarkdownFolderInput/Output`을 추가한다.
- [x] `ImportMarkdownEntryInput`을 추가한다.
- [x] import state machine 상태를 정의한다.
- [x] usecase는 filesystem을 직접 scan/read하지 않는다.
- [x] entry별 explicit document id, version id, snapshot ref를 input으로 받는다.
- [x] valid entry는 current document와 version record로 저장한다.
- [x] duplicate/conflict entry는 실패 item으로 기록하고 다른 entry import를 계속한다.
- [x] 모든 entry 실패 시 `Failed`, 일부 실패 시 `PartiallyFailed`, 모두 성공 시 `Completed`를 반환한다.

## 3. TDD Plan

- [x] 실패하는 import success stores current and versions test를 먼저 작성한다.
- [x] 실패하는 duplicate entry produces partial failure test를 먼저 작성한다.
- [x] 실패하는 invalid entry does not stop valid entry test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`와 `VersionStore` port만 받는다.
- [x] usecase는 filesystem, env, concrete adapter, UI import picker를 import하지 않는다.
- [x] id/version/snapshot 생성은 usecase 내부 전역 random/clock 접근 없이 input으로 받는다.
- [x] import state는 명시적 enum으로 표현한다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `ImportMarkdownFolderUsecase`를 추가했다.
  - usecase는 filesystem을 읽지 않고 explicit import entries를 current document와 version history로 저장한다.
  - entry별 conflict/invalid failure를 수집하고 import를 계속한다.
  - final state는 `Completed`, `PartiallyFailed`, `Failed`로 반환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/import.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/import_markdown_folder_tests.rs`
  - `.tasks/task047.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test import_markdown_folder_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 048은 Markdown export usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

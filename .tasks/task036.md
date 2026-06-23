# Task 036. UpdateDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-028 UpdateDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 문서 본문 변경을 current snapshot에 반영하고, 동일한 내용을 새 version entry로 기록한다.

## 2. Scope

- [x] `UpdateDocumentInput`과 `UpdateDocumentOutput`을 추가한다.
- [x] current snapshot을 먼저 조회해 document 존재 여부를 확인한다.
- [x] 새 body를 `DocumentBodyPolicy`로 검증하고 정규화한다.
- [x] 새 body를 version store에 append한다.
- [x] version append 성공 후 current snapshot을 교체한다.
- [x] update 성공 시 domain event와 Product Log event를 기록한다.
- [x] invalid body는 write 없이 `InvalidDocumentInput`으로 보고한다.
- [x] duplicate version 또는 version append 실패는 current snapshot 변경 없이 보고한다.

## 3. TDD Plan

- [x] 실패하는 update success flow test를 먼저 작성한다.
- [x] 실패하는 invalid body without writes test를 먼저 작성한다.
- [x] 실패하는 duplicate version preserves current test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`, `VersionStore`, event publisher, product logger port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI component를 import하지 않는다.
- [x] version id와 snapshot ref는 clock/random 전역 접근 없이 명시적 input으로 받는다.
- [x] update는 사용자에게 Git, commit, PR 개념을 노출하지 않는다.
- [x] current query와 history append는 port 경계 뒤에 유지한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `UpdateDocumentUsecase`를 추가했다.
  - update는 current 존재를 확인하고, 새 body를 검증 및 정규화한 뒤 version append를 먼저 수행한다.
  - version append 성공 후 current snapshot을 교체한다.
  - duplicate version conflict는 current 변경 없이 `VersionAlreadyExists`로 반환한다.
  - 성공 시 `DocumentUpdated` event와 Product Log event를 남긴다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/update_document_tests.rs`
  - `.tasks/task036.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test update_document_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 037은 `RenameDocument` usecase를 구현한다.
  - rename은 문서 identity를 유지하면서 title/path metadata만 변경하고 current body와 history 분리를 유지해야 한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

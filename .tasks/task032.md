# Task 032. GetDocumentHistory Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-024 GetDocumentHistory` usecase를 구현하는 것이다.
- [x] 이 태스크는 history 조회가 pagination contract를 사용하고 current snapshot 또는 specific snapshot 조회를 호출하지 않도록 고정한다.

## 2. Scope

- [x] `GetDocumentHistoryInput`과 `GetDocumentHistoryOutput`을 추가한다.
- [x] `GetDocumentHistoryUsecase`를 추가한다.
- [x] cursor와 limit을 `HistoryPageRequest`로 검증한다.
- [x] `VersionStore::list_history`만 사용한다.
- [x] invalid input을 store 호출 전에 거부한다.

## 3. TDD Plan

- [x] 실패하는 paginated history query test를 먼저 작성한다.
- [x] 실패하는 invalid limit skips store test를 먼저 작성한다.
- [x] 실패하는 invalid cursor skips store test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `VersionStore`만 받는다.
- [x] usecase는 `DocumentRepository`를 받거나 호출하지 않는다.
- [x] usecase는 `get_version_snapshot`을 호출하지 않는다.
- [x] usecase는 filesystem, env, concrete local adapter를 import하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `GetDocumentHistoryUsecase`를 추가해 cursor/limit 기반 history pagination을 구현했다.
  - usecase가 `VersionStore::list_history`만 호출하도록 하고 current repository와 snapshot read 경로를 배제했다.
  - invalid limit/cursor는 store 호출 전에 거부한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/get_document_history_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test get_document_history_tests --quiet`: initial fail, missing history usecase types
  - `cargo test -p cabinet-usecases --test get_document_history_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 033은 `CompareDocumentVersions` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

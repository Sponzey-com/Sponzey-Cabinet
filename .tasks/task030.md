# Task 030. GetCurrentDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-022 GetCurrentDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 current snapshot 조회가 version history store 또는 history scan에 의존하지 않도록 고정한다.
- [x] 이 태스크는 id 기반 조회와 path 기반 조회를 명시적 입력으로 분리한다.

## 2. Scope

- [x] `GetCurrentDocumentInput`과 `GetCurrentDocumentOutput`을 추가한다.
- [x] `GetCurrentDocumentUsecase`를 추가한다.
- [x] id 기반 current 조회를 구현한다.
- [x] path 기반 current 조회를 구현한다.
- [x] missing current document를 `NotFound`로 보고한다.
- [x] invalid input을 repository 호출 전에 거부한다.

## 3. TDD Plan

- [x] 실패하는 get current by id without history scan test를 먼저 작성한다.
- [x] 실패하는 get current by path without history scan test를 먼저 작성한다.
- [x] 실패하는 missing current not found test를 먼저 작성한다.
- [x] 실패하는 invalid input skips repository test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`만 받는다.
- [x] usecase는 `VersionStore`를 받거나 호출하지 않는다.
- [x] usecase는 filesystem, env, concrete local adapter를 import하지 않는다.
- [x] output은 명시적 record를 반환하고 UI DTO를 반환하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `GetCurrentDocumentUsecase`를 추가해 id/path 기반 current snapshot 조회를 구현했다.
  - usecase가 `DocumentRepository`만 받도록 하여 version history store 의존을 제거했다.
  - invalid input은 repository read 전에 거부하고, missing current snapshot은 `NotFound`로 반환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/get_current_document_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test get_current_document_tests --quiet`: initial fail, missing current usecase types
  - `cargo test -p cabinet-usecases --test get_current_document_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 031은 `GetDocumentVersion` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

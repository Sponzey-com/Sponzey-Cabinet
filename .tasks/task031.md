# Task 031. GetDocumentVersion Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-023 GetDocumentVersion` usecase를 구현하는 것이다.
- [x] 이 태스크는 특정 version snapshot 조회가 current repository나 full history load에 의존하지 않도록 고정한다.

## 2. Scope

- [x] `GetDocumentVersionInput`과 `GetDocumentVersionOutput`을 추가한다.
- [x] `GetDocumentVersionUsecase`를 추가한다.
- [x] `VersionStore::get_version_snapshot`만 사용한다.
- [x] missing version snapshot을 `NotFound`로 보고한다.
- [x] invalid input을 store 호출 전에 거부한다.

## 3. TDD Plan

- [x] 실패하는 get version snapshot without current repository test를 먼저 작성한다.
- [x] 실패하는 missing version not found test를 먼저 작성한다.
- [x] 실패하는 invalid input skips store test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `VersionStore`만 받는다.
- [x] usecase는 `DocumentRepository`를 받거나 호출하지 않는다.
- [x] usecase는 full history list를 호출하지 않는다.
- [x] usecase는 filesystem, env, concrete local adapter를 import하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `GetDocumentVersionUsecase`를 추가해 특정 version snapshot 조회를 구현했다.
  - usecase가 `VersionStore::get_version_snapshot`만 호출하도록 하고 current repository/history list 호출을 배제했다.
  - invalid input은 store 호출 전에 거부하고 missing snapshot은 `NotFound`로 반환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/get_document_version_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test get_document_version_tests --quiet`: initial fail, missing version usecase types
  - `cargo test -p cabinet-usecases --test get_document_version_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 032는 `GetDocumentHistory` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

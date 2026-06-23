# Task 046. SearchDocuments Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-062 SearchDocuments` usecase를 구현하는 것이다.
- [x] 이 태스크는 명시적 query/limit input을 받아 `SearchIndex` port를 통해 검색 결과를 반환한다.

## 2. Scope

- [x] `SearchDocumentsInput`과 `SearchDocumentsOutput`을 추가한다.
- [x] workspace id, query text, limit을 검증한다.
- [x] `SearchQuery`를 생성해 query validation을 port model에 위임한다.
- [x] `SearchIndex::search`를 호출한다.
- [x] invalid query/limit은 index call 없이 `InvalidInput`으로 보고한다.
- [x] search index failure는 `StorageUnavailable`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 search documents success delegates to search index test를 먼저 작성한다.
- [x] 실패하는 invalid query skips search index test를 먼저 작성한다.
- [x] 실패하는 search index failure maps to storage unavailable test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `SearchIndex` port만 받는다.
- [x] usecase는 filesystem, env, concrete adapter, UI search component를 import하지 않는다.
- [x] p95 search benchmark는 Phase 8 gate에서 별도로 측정한다.
- [x] Product Log 성공 이벤트는 만들지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `SearchDocumentsUsecase`를 추가했다.
  - usecase는 `SearchQuery`를 생성해 query/limit validation을 수행한 뒤 `SearchIndex::search`만 호출한다.
  - invalid query는 index call 없이 `InvalidInput`으로 실패한다.
  - search index storage failure는 `StorageUnavailable`로 매핑한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/search.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/search_documents_tests.rs`
  - `.tasks/task046.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test search_documents_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 047은 Markdown folder import state machine/usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

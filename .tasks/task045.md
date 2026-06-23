# Task 045. LocalSearchIndex Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-061 Local search index adapter`를 구현하는 것이다.
- [x] 이 태스크는 외부 검색 서버 없이 local embedded search index를 제공한다.

## 2. Scope

- [x] `LocalSearchIndex` adapter를 추가한다.
- [x] upsert된 문서를 workspace/document 기준으로 저장한다.
- [x] delete된 문서는 이후 search result에서 제외한다.
- [x] title, path, body에서 case-insensitive query match를 수행한다.
- [x] query limit을 준수한다.
- [x] result snippet은 document body line 또는 metadata에서 만든다.

## 3. TDD Plan

- [x] 실패하는 local search returns matching documents with snippets test를 먼저 작성한다.
- [x] 실패하는 local search respects limit and ranking test를 먼저 작성한다.
- [x] 실패하는 delete removes document from search results test를 먼저 작성한다.

## 4. Architecture Rules

- [x] adapter는 `SearchIndex` port만 구현한다.
- [x] adapter는 external search server, network, env, Git CLI에 의존하지 않는다.
- [x] adapter는 UI search component를 import하지 않는다.
- [x] search p95 benchmark는 Phase 8 gate에서 별도로 측정한다.
- [x] local index는 문서 저장소의 source of truth가 아니며 projection으로 취급한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalSearchIndex` adapter를 추가했다.
  - local search는 workspace/document 기준 in-memory projection으로 동작한다.
  - title/path/body case-insensitive match, simple score ranking, limit, delete exclusion을 구현했다.
  - 외부 검색 서버나 네트워크 의존성은 추가하지 않았다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_search_index.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_search_index_tests.rs`
  - `.tasks/task045.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_search_index_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 046은 `SearchDocuments` usecase를 구현한다.
  - usecase는 `SearchIndex` port만 받아 query/limit을 명시적으로 전달한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

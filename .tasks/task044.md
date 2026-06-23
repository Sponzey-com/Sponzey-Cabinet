# Task 044. SearchIndex Port Contract

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-060 Search index port`를 구현하는 것이다.
- [x] 이 태스크는 문서 검색 index의 upsert/delete/query 경계와 query/result model을 정의한다.

## 2. Scope

- [x] `SearchIndex` port를 추가한다.
- [x] `SearchDocumentRecord`를 정의한다.
- [x] `SearchQuery`를 정의하고 empty query, invalid limit을 거부한다.
- [x] `SearchResult`와 `SearchPage`를 정의한다.
- [x] search result는 document id, title, path, snippet, score만 포함한다.
- [x] index port는 document body object나 storage adapter를 직접 소유하지 않는다.

## 3. TDD Plan

- [x] 실패하는 search query validation contract test를 먼저 작성한다.
- [x] 실패하는 search document record exposes metadata and content test를 먼저 작성한다.
- [x] 실패하는 search result rejects empty snippet test를 먼저 작성한다.

## 4. Architecture Rules

- [x] port는 domain document/workspace value object만 참조한다.
- [x] port는 filesystem, env, network, concrete search engine을 import하지 않는다.
- [x] search query limit은 explicit input model에서 검증한다.
- [x] p95 search benchmark는 Phase 8 gate에서 별도로 측정한다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `SearchIndex` port를 추가했다.
  - `SearchDocumentRecord`, `SearchQuery`, `SearchResult`, `SearchPage`를 정의했다.
  - query empty/limit 검증과 result snippet 검증을 추가했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/search_index.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/search_index_contract_tests.rs`
  - `.tasks/task044.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test search_index_contract_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 045는 `LocalSearchIndex` adapter를 구현한다.
  - local search는 외부 검색 서버 없이 embedded/in-memory index로 시작한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

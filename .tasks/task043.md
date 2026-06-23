# Task 043. GraphLiteProjection Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-042 Graph-lite projection`을 구현하는 것이다.
- [x] 이 태스크는 center document 기준 depth 1 관계도를 반환한다.

## 2. Scope

- [x] `LinkIndex::get_document_links` query를 추가한다.
- [x] `GraphLiteProjectionInput`과 `GraphLiteProjectionOutput`을 추가한다.
- [x] center document node를 반환한다.
- [x] center의 outgoing resolved link edge를 반환한다.
- [x] center로 들어오는 incoming backlink edge를 반환한다.
- [x] center의 unresolved link를 unresolved node/edge로 반환한다.
- [x] known document list에 없는 resolved target은 missing/deleted node로 표시한다.

## 3. TDD Plan

- [x] 실패하는 graph projection includes incoming outgoing unresolved test를 먼저 작성한다.
- [x] 실패하는 graph projection marks missing target test를 먼저 작성한다.
- [x] 실패하는 local link index get document links test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `LinkIndex` port만 받는다.
- [x] usecase는 filesystem, env, concrete adapter, UI graph renderer를 import하지 않는다.
- [x] graph output은 UI DTO가 아니라 usecase output model이다.
- [x] depth 1 graph query는 LinkIndex projection을 scan하되 document body나 version history를 읽지 않는다.
- [x] p95 graph benchmark는 Phase 8 gate에서 별도로 측정한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LinkIndex::get_document_links` query를 추가했다.
  - `GraphLiteProjectionUsecase`를 추가했다.
  - graph-lite는 center, outgoing, incoming, unresolved, missing target node/edge를 depth 1로 반환한다.
  - usecase는 `LinkIndex` port만 사용하고 document body, version history, UI renderer에 접근하지 않는다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/link_index.rs`
  - `crates/cabinet-adapters/src/local_link_index.rs`
  - `crates/cabinet-adapters/tests/local_link_index_tests.rs`
  - `crates/cabinet-usecases/src/graph.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/graph_lite_projection_tests.rs`
  - `.tasks/task043.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_link_index_tests --quiet` 통과
  - `cargo test -p cabinet-usecases --test graph_lite_projection_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 044는 `SearchIndex` port를 구현한다.
  - 이후 local search adapter와 `SearchDocuments` usecase로 이어간다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

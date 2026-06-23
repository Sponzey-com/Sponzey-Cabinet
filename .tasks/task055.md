# Task 055. Search UI

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-085 Search UI`를 구현하는 것이다.
- [x] 이 태스크는 search query/result client contract와 search result UI view model을 정의한다.

## 2. Scope

- [x] client-core에 search documents query contract를 정의한다.
- [x] client-core에 search result/page contract를 정의한다.
- [x] UI package에 search panel view model을 정의한다.
- [x] search result click command는 current document query로 변환한다.
- [x] search UI는 history query나 document body 전체를 사용하지 않는다.
- [x] 정적 smoke script로 search UI와 current open command를 검증한다.

## 3. TDD Plan

- [x] 실패하는 search UI static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/client-core/src/index.ts`와 `packages/ui/src/index.ts`를 구현한다.
- [x] search UI smoke와 frontend boundary check를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] UI는 search result를 열 때 current document query만 생성한다.
- [x] UI는 search result를 열기 위해 history query를 호출하지 않는다.
- [x] UI는 검색 성능 문제를 loading 표시로 숨기지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `SearchDocumentsQuery`, `SearchResultView`, `SearchResultsPage`와 `searchDocuments` client method를 추가했다.
  - `SearchPanelViewModel`과 `SearchResultItemViewModel`을 추가했다.
  - search result open command가 history가 아니라 `createCurrentDocumentQuery`로 변환되도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/client-core/src/index.ts`
  - `packages/ui/src/index.ts`
  - `scripts/check_search_ui.mjs`
  - `package.json`
  - `.tasks/task055.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `node scripts/check_search_ui.mjs` 통과
  - `node scripts/check_current_history_ui.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
  - `node scripts/check_ui_shell.mjs` 통과
  - `node scripts/check_editor_integration.mjs` 통과
  - `node scripts/check_wikilink_editor.mjs` 통과
  - `node scripts/check_attachment_editor.mjs` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 056. Link and Backlink UI

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

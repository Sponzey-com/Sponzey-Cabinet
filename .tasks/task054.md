# Task 054. Current and History UI Split

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-084 Current/history UI split`을 구현하는 것이다.
- [x] 이 태스크는 current document view와 history panel이 서로 다른 client query contract를 사용하도록 분리한다.

## 2. Scope

- [x] client-core에 current document query contract를 정의한다.
- [x] client-core에 document history query contract를 정의한다.
- [x] UI package에 current document view model을 정의한다.
- [x] UI package에 history panel view model을 정의한다.
- [x] current view model은 history entry나 history query를 참조하지 않는다.
- [x] history panel view model은 document body를 직접 포함하지 않는다.
- [x] 정적 smoke script로 current/history query 분리를 검증한다.

## 3. TDD Plan

- [x] 실패하는 current/history UI static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/client-core/src/index.ts`와 `packages/ui/src/index.ts`를 구현한다.
- [x] current/history smoke와 frontend boundary check를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] UI는 current 문서를 만들기 위해 version history 전체를 조회하지 않는다.
- [x] UI는 history panel을 만들기 위해 current document body를 조회하지 않는다.
- [x] UI는 domain/usecase/adapters를 import하지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `CurrentDocumentQuery`, `DocumentHistoryQuery`, `CurrentDocumentView`, `DocumentHistoryPage`, `CabinetDocumentClient`를 client-core에 추가했다.
  - `CurrentDocumentViewModel`과 `HistoryPanelViewModel`을 UI package에 추가했다.
  - current view factory가 history query/entry를 참조하지 않고, history panel factory가 document body를 포함하지 않도록 smoke로 검증했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/client-core/src/index.ts`
  - `packages/ui/src/index.ts`
  - `scripts/check_current_history_ui.mjs`
  - `package.json`
  - `.tasks/task054.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
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
  - Task 055. Search UI

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

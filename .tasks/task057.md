# Task 057. Asset UI

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-087 Asset UI`를 구현하는 것이다.
- [x] 이 태스크는 document asset list, missing asset 표시, attach command client contract와 UI view model을 정의한다.

## 2. Scope

- [x] client-core에 document asset list query contract를 정의한다.
- [x] client-core에 attach asset command contract를 정의한다.
- [x] UI package에 asset panel view model을 정의한다.
- [x] missing asset은 UI status로 표시하되 존재 여부 판단은 client/usecase 결과를 따른다.
- [x] UI는 파일시스템, file picker, asset store에 직접 접근하지 않는다.
- [x] 정적 smoke script로 asset UI와 attach command boundary를 검증한다.

## 3. TDD Plan

- [x] 실패하는 asset UI static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/client-core/src/index.ts`와 `packages/ui/src/index.ts`를 구현한다.
- [x] asset UI smoke와 기존 frontend smoke를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] UI는 asset id, label, metadata만 표시한다.
- [x] UI는 local path, filesystem object, platform picker object를 저장하지 않는다.
- [x] UI는 missing asset 판단을 직접 수행하지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `ListDocumentAssetsQuery`, `AssetView`, `DocumentAssetsPage`, `SelectedAssetDraft`, `AttachAssetCommand`를 추가했다.
  - `AssetPanelViewModel`과 `AssetItemViewModel`을 추가했다.
  - UI attach command factory가 filesystem/picker에 접근하지 않고 metadata command만 만들도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/client-core/src/index.ts`
  - `packages/ui/src/index.ts`
  - `scripts/check_asset_ui.mjs`
  - `package.json`
  - `.tasks/task057.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `node scripts/check_asset_ui.mjs` 통과
  - `node scripts/check_link_ui.mjs` 통과
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
  - Task 058. Platform Adapter Smoke

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

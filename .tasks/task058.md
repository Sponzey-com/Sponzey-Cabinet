# Task 058. Platform Adapter Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 platform adapter smoke를 구현하는 것이다.
- [x] 이 태스크는 Web/Desktop shell이 같은 client-core 계약을 사용하고 platform picker 결과를 명시적 value object로만 전달하도록 검증한다.

## 2. Scope

- [x] Web shell에 shared workspace shell model과 editor boundary descriptor를 연결한다.
- [x] Desktop shell에 shared workspace shell model과 editor boundary descriptor를 연결한다.
- [x] Web shell에 selected asset value object to attach command mapper를 추가한다.
- [x] Desktop shell에 selected asset value object to attach command mapper를 추가한다.
- [x] app shell은 filesystem, environment, Tauri API, Rust domain/usecase/adapters를 직접 import하지 않는다.
- [x] desktop Rust shell boundary check를 platform adapter smoke에 포함한다.

## 3. TDD Plan

- [x] 실패하는 platform adapter static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `apps/web/src/index.ts`와 `apps/desktop/src/index.ts`를 구현한다.
- [x] platform adapter smoke와 기존 frontend smoke를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] app shell은 client-core, ui, editor package만 직접 사용한다.
- [x] app shell은 selected asset metadata를 `SelectedAssetDraft`로 매핑한다.
- [x] app shell은 file picker object나 local path를 durable state에 넣지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.

  - Web/Desktop shell이 `createWorkspaceShellModel`과 `createEditorBoundaryDescriptor`를 함께 사용하도록 했다.
  - Web/Desktop selected asset value object를 `SelectedAssetDraft`로 매핑하고 attach command를 생성하는 mapper를 추가했다.
  - platform adapter smoke가 Web/Desktop shell, desktop Rust shell boundary, forbidden pattern을 함께 확인하도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.

  - `apps/web/src/index.ts`
  - `apps/desktop/src/index.ts`
  - `scripts/check_platform_adapter_smoke.mjs`
  - `package.json`
  - `.tasks/task058.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.

  - `node scripts/check_platform_adapter_smoke.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
  - `sh scripts/check_desktop_shell_boundaries.sh` 통과
  - `node scripts/check_asset_ui.mjs` 통과
  - `node scripts/check_link_ui.mjs` 통과
  - `node scripts/check_search_ui.mjs` 통과
  - `node scripts/check_current_history_ui.mjs` 통과
  - `node scripts/check_ui_shell.mjs` 통과
  - `node scripts/check_editor_integration.mjs` 통과
  - `node scripts/check_wikilink_editor.mjs` 통과
  - `node scripts/check_attachment_editor.mjs` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.

  - Task 059. Performance Benchmark Harness

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
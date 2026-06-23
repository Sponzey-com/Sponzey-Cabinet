# Task 050. Shared Shell UI Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-080 Shared shell UI`를 구현하는 것이다.
- [x] 이 태스크는 Web/Desktop이 공유할 shell layout model을 UI package boundary에 정의한다.

## 2. Scope

- [x] shared shell layout model을 추가한다.
- [x] document list, editor, side panel, status bar, command palette placeholder zone을 명시한다.
- [x] UI model은 client-core capability를 입력으로 받는다.
- [x] UI는 env, filesystem, Tauri, Rust crate를 import하지 않는다.
- [x] 정적 smoke script로 shell model export와 zone 구성을 검증한다.

## 3. TDD Plan

- [x] 실패하는 UI shell static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/ui/src/index.ts`를 구현한다.
- [x] frontend boundary check와 UI shell smoke를 실행한다.

## 4. Architecture Rules

- [x] UI package는 domain/usecase/adapters를 import하지 않는다.
- [x] UI shell은 document rule을 구현하지 않는다.
- [x] UI shell은 표시 모델만 정의하고 client action은 client-core contract로 위임한다.
- [x] UI shell은 visible instructional copy를 포함하지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - Web/Desktop이 공유할 `WorkspaceShellModel`과 `WorkspaceShellZone`을 `packages/ui`에 추가했다.
  - shell zone은 document list, editor, metadata, history, status, command palette로 고정했다.
  - UI shell이 환경 변수, filesystem, Tauri, Rust crate, Git/commit/PR 사용자 노출 문자열을 포함하지 않도록 정적 smoke를 추가했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/ui/src/index.ts`
  - `scripts/check_ui_shell.mjs`
  - `package.json`
  - `.tasks/task050.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `node scripts/check_ui_shell.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 051. Code Editor Integration

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

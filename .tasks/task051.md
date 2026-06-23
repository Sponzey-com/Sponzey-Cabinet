# Task 051. Code Editor Integration

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-081 Code editor integration`을 구현하는 것이다.
- [x] 이 태스크는 CodeMirror 기반 UI가 사용할 editor session model과 load/save/dirty operation boundary를 정의한다.

## 2. Scope

- [x] editor package에 document body string 기반 session model을 추가한다.
- [x] clean/dirty state를 editor-local state로 표현한다.
- [x] load operation, content change operation, save command 변환 함수를 추가한다.
- [x] save command는 document id와 최신 body string만 포함한다.
- [x] editor package는 domain/usecase/adapters, filesystem, environment, Tauri, Rust crate를 import하지 않는다.
- [x] CodeMirror state를 domain document model이나 durable 저장 모델로 노출하지 않는다.
- [x] 정적 smoke script로 editor integration API와 금지 패턴을 검증한다.

## 3. TDD Plan

- [x] 실패하는 editor integration static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/editor/src/index.ts`를 구현한다.
- [x] editor integration smoke, frontend boundary check, UI shell smoke를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] editor package는 client-core boundary만 import한다.
- [x] editor state는 document domain model이 아니다.
- [x] dirty state는 UI/editor-local state로만 표현한다.
- [x] save/restore conflict 판단은 editor package에서 수행하지 않는다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `EditorSessionModel`, `EditorDirtyState`, `EditorDocumentSnapshot`, `EditorSaveCommand`를 추가했다.
  - editor load operation, content change, save command 변환 함수를 추가했다.
  - editor package가 CodeMirror runtime state, domain/usecase/adapter, filesystem/env/platform SDK에 의존하지 않는지 smoke로 검증했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/editor/src/index.ts`
  - `scripts/check_editor_integration.mjs`
  - `package.json`
  - `.tasks/task051.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `node scripts/check_editor_integration.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
  - `node scripts/check_ui_shell.mjs` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 052. Wikilink Editor Extension

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 052. Wikilink Editor Extension

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 7의 `MVP-082 Wikilink editor extension`을 구현하는 것이다.
- [x] 이 태스크는 editor package에 wikilink decoration model과 insert/open command 변환을 추가한다.

## 2. Scope

- [x] `[[target]]`와 `[[target|label]]` 형태의 wikilink decoration model을 정의한다.
- [x] wikilink source range를 editor-local range로 표현한다.
- [x] asset reference `![[asset:...]]`는 wikilink decoration으로 처리하지 않는다.
- [x] wikilink 삽입 operation을 만든다.
- [x] wikilink click/open command를 만든다.
- [x] unresolved/resolved 판단은 editor package에서 수행하지 않는다.
- [x] 정적 smoke script로 wikilink API와 금지 패턴을 검증한다.

## 3. TDD Plan

- [x] 실패하는 wikilink editor static smoke script를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 `packages/editor/src/index.ts`를 구현한다.
- [x] wikilink smoke, editor integration smoke, frontend boundary check를 실행한다.
- [x] Rust workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] editor package는 wikilink parser 결과를 domain object로 만들지 않는다.
- [x] editor package는 link resolution, permission, navigation policy를 판단하지 않는다.
- [x] editor package는 CodeMirror event를 serializable command/operation으로 변환한다.
- [x] 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `WikilinkDecoration`, `WikilinkOpenCommand`, `EditorSourceRange`를 추가했다.
  - `findWikilinkDecorations`, `createInsertWikilinkOperation`, `createOpenWikilinkCommand`를 추가했다.
  - asset reference `![[asset:...]]`를 wikilink decoration에서 제외했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `packages/editor/src/index.ts`
  - `scripts/check_wikilink_editor.mjs`
  - `package.json`
  - `.tasks/task052.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `node scripts/check_wikilink_editor.mjs` 통과
  - `node scripts/check_editor_integration.mjs` 통과
  - `node scripts/check_frontend_boundaries.mjs` 통과
  - `node scripts/check_ui_shell.mjs` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 053. Attachment Editor Extension

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

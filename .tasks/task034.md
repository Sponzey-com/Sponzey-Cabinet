# Task 034. PreviewDocumentRestore Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-026 PreviewDocumentRestore` usecase를 구현하는 것이다.
- [x] 이 태스크는 실제 current snapshot 변경 없이 restore 대상 version과 diff preview를 제공한다.

## 2. Scope

- [x] `PreviewDocumentRestoreInput`과 `PreviewDocumentRestoreOutput`을 추가한다.
- [x] current snapshot과 target version snapshot을 읽는다.
- [x] preview line diff를 반환한다.
- [x] preview 단계에서 document repository write와 version append를 수행하지 않는다.
- [x] missing current 또는 target version을 `NotFound`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 restore preview diff without writes test를 먼저 작성한다.
- [x] 실패하는 missing target not found test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`와 `VersionStore` port만 받는다.
- [x] preview usecase는 write method를 호출하지 않는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI diff component를 import하지 않는다.
- [x] preview output은 UI DTO가 아니라 usecase output model이다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `PreviewDocumentRestoreUsecase`를 추가했다.
  - usecase는 current snapshot과 target version snapshot을 읽고 `LineDiff` 목록을 반환한다.
  - preview 단계에서는 `DocumentRepository::put_current`와 `VersionStore::append_version`을 호출하지 않는다.
  - missing current 또는 missing target version은 `PreviewDocumentRestoreError::NotFound`로 반환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/preview_document_restore_tests.rs`
  - `.tasks/task034.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test preview_document_restore_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 035는 `RestoreDocumentVersion` usecase를 구현한다.
  - restore는 preview와 달리 target version을 current snapshot으로 복원하고 새 version entry를 append해야 한다.
  - restore 절차는 명시적 상태 전이로 표현하고 실패 시 current snapshot 보존을 테스트한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

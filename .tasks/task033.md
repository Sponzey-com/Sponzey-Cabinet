# Task 033. CompareDocumentVersions Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-025 CompareDocumentVersions` usecase를 구현하는 것이다.
- [x] 이 태스크는 current-vs-version과 version-vs-version 비교를 명시적 입력으로 분리한다.

## 2. Scope

- [x] `CompareDocumentVersionsInput`과 `CompareDocumentVersionsOutput`을 추가한다.
- [x] `LineDiff`와 `LineDiffKind`를 추가한다.
- [x] current-vs-version 비교를 구현한다.
- [x] version-vs-version 비교를 구현한다.
- [x] missing current 또는 version snapshot을 `NotFound`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 current-vs-version diff test를 먼저 작성한다.
- [x] 실패하는 version-vs-version diff test를 먼저 작성한다.
- [x] 실패하는 missing target not found test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`와 `VersionStore` port만 받는다.
- [x] usecase는 history list를 호출하지 않는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI diff component를 import하지 않는다.
- [x] diff output은 UI DTO가 아니라 usecase output model이다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `CompareDocumentVersionsUsecase`를 추가해 current-vs-version과 version-vs-version line diff를 구현했다.
  - diff usecase가 specific snapshot/current read만 사용하고 history list를 호출하지 않도록 했다.
  - `LineDiff`/`LineDiffKind` output model을 추가했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/compare_document_versions_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test compare_document_versions_tests --quiet`: initial fail, missing compare usecase types
  - `cargo test -p cabinet-usecases --test compare_document_versions_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 034는 `PreviewDocumentRestore` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

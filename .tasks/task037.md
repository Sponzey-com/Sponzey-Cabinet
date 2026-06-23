# Task 037. RenameDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-029 RenameDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 document identity를 유지하면서 title/path metadata를 변경한다.

## 2. Scope

- [x] `RenameDocumentInput`과 `RenameDocumentOutput`을 추가한다.
- [x] current snapshot을 조회해 document 존재 여부를 확인한다.
- [x] 새 title과 path를 domain value object로 검증한다.
- [x] document id와 body는 유지하고 metadata만 교체한다.
- [x] rename 성공 시 domain event와 Product Log event를 기록한다.
- [x] invalid title/path는 write 없이 `InvalidDocumentInput`으로 보고한다.
- [x] missing current는 write 없이 `NotFound`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 rename success keeps identity and body test를 먼저 작성한다.
- [x] 실패하는 invalid path without writes test를 먼저 작성한다.
- [x] 실패하는 missing current not found test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`, event publisher, product logger port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI component를 import하지 않는다.
- [x] rename은 version history store를 직접 수정하지 않는다.
- [x] rename은 사용자에게 Git, commit, PR 개념을 노출하지 않는다.
- [x] path/title validation은 usecase 내부 문자열 검사가 아니라 domain value object를 사용한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `RenameDocumentUsecase`를 추가했다.
  - rename은 current snapshot을 조회하고 document id와 body는 유지한 채 title/path metadata만 교체한다.
  - version history store는 수정하지 않는다.
  - invalid path와 missing current는 write 없이 실패한다.
  - 성공 시 `DocumentRenamed` event와 Product Log event를 남긴다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/rename_document_tests.rs`
  - `.tasks/task037.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test rename_document_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 038은 `DeleteDocument` usecase를 구현한다.
  - delete는 current snapshot에서 제거하되 history 보존 정책과 Product Log를 테스트해야 한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

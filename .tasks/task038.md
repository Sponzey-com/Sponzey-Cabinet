# Task 038. DeleteDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-030 DeleteDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 current snapshot에서 문서를 제거하되 version history store는 삭제하지 않는 recoverable delete 정책을 고정한다.

## 2. Scope

- [x] `DeleteDocumentInput`과 `DeleteDocumentOutput`을 추가한다.
- [x] current snapshot을 조회해 document 존재 여부를 확인한다.
- [x] 존재하는 문서는 `DocumentRepository::delete_current`로 current 조회 대상에서 제거한다.
- [x] delete 성공 시 domain event와 Product Log event를 기록한다.
- [x] missing current는 delete 호출 없이 `NotFound`로 보고한다.
- [x] repository delete failure는 Product Log failure로 보고한다.
- [x] delete usecase는 `VersionStore`를 받거나 수정하지 않는다.

## 3. TDD Plan

- [x] 실패하는 delete success removes current test를 먼저 작성한다.
- [x] 실패하는 missing current skips delete test를 먼저 작성한다.
- [x] 실패하는 repository delete failure preserves current test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`, event publisher, product logger port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI component를 import하지 않는다.
- [x] delete는 version history store를 직접 수정하지 않는다.
- [x] delete는 사용자에게 Git, commit, PR 개념을 노출하지 않는다.
- [x] current/history 분리 정책을 깨지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `DeleteDocumentUsecase`를 추가했다.
  - delete는 current snapshot 존재를 확인한 뒤 `DocumentRepository::delete_current`만 호출한다.
  - delete usecase는 `VersionStore`를 받지 않아 version history store를 수정하지 않는다.
  - missing current는 delete 호출 없이 `NotFound`로 실패한다.
  - repository delete failure는 current 보존과 Product Log failure를 테스트했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/delete_document_tests.rs`
  - `.tasks/task038.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test delete_document_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Phase 5의 남은 asset usecase 범위를 확인한 뒤 `AttachFileToDocument` 또는 Phase 6 parser/index 작업으로 진행한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

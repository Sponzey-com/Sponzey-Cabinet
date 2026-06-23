# Task 039. AttachFileToDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-051 AttachFileToDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 첨부 파일 binary/object를 문서 본문과 분리해 저장하고, 문서별 asset reference만 연결한다.

## 2. Scope

- [x] `DocumentAssetRepository` port를 추가한다.
- [x] `DocumentAssetRecord`와 attach outcome/error를 정의한다.
- [x] `AttachFileToDocumentInput`과 `AttachFileToDocumentOutput`을 추가한다.
- [x] target document current snapshot 존재를 확인한다.
- [x] asset metadata/object를 `AssetStore`에 저장한다.
- [x] asset 저장 성공 후 document-asset association을 저장한다.
- [x] 실패 시 document current snapshot을 수정하지 않는다.
- [x] 성공 시 domain event와 Product Log event를 기록한다.

## 3. TDD Plan

- [x] 실패하는 document asset record mismatch contract test를 먼저 작성한다.
- [x] 실패하는 attach success stores asset and association test를 먼저 작성한다.
- [x] 실패하는 missing document skips asset storage test를 먼저 작성한다.
- [x] 실패하는 asset store failure skips association test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`, `AssetStore`, `DocumentAssetRepository`, event publisher, product logger port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI file picker를 import하지 않는다.
- [x] asset id는 content hash 계산 adapter에서 생성되어 명시적 input으로 전달된다.
- [x] document body에는 original bytes를 삽입하지 않는다.
- [x] Product Log payload에는 file bytes, full path, content를 넣지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `DocumentAssetRepository` port와 `DocumentAssetRecord`를 추가했다.
  - `AttachFileToDocumentUsecase`를 추가했다.
  - attach는 document current 존재 확인 후 asset object/metadata를 저장하고, 성공한 경우에만 document-asset association을 저장한다.
  - missing document와 asset store failure는 document current snapshot을 수정하지 않는다.
  - 성공 시 `DocumentAssetAttached` event와 Product Log event를 남긴다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/document_asset_repository.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/document_asset_repository_contract_tests.rs`
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/attach_file_to_document_tests.rs`
  - `.tasks/task039.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test document_asset_repository_contract_tests --quiet` 통과
  - `cargo test -p cabinet-usecases --test attach_file_to_document_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 040은 `ListDocumentAssets` usecase를 구현한다.
  - list는 metadata/reference만 반환하고 object bytes를 읽지 않아야 한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 040. ListDocumentAssets Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-052 ListDocumentAssets` usecase를 구현하는 것이다.
- [x] 이 태스크는 문서에 연결된 첨부 파일 목록을 object bytes 없이 metadata/reference만 조회한다.

## 2. Scope

- [x] `ListDocumentAssetsInput`과 `ListDocumentAssetsOutput`을 추가한다.
- [x] target document current snapshot 존재를 확인한다.
- [x] `DocumentAssetRepository::list_assets`로 문서별 asset metadata/reference 목록을 조회한다.
- [x] asset object bytes를 읽기 위한 `AssetStore`를 받지 않는다.
- [x] missing document는 association list 조회 없이 `NotFound`로 보고한다.
- [x] association repository failure는 `StorageUnavailable`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 list success returns metadata without object store test를 먼저 작성한다.
- [x] 실패하는 missing document skips association list test를 먼저 작성한다.
- [x] 실패하는 repository failure maps to storage unavailable test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 `DocumentRepository`와 `DocumentAssetRepository` port만 받는다.
- [x] usecase는 filesystem, env, concrete local adapter, UI component를 import하지 않는다.
- [x] usecase는 asset binary/object bytes를 조회하지 않는다.
- [x] list query는 Product Log 성공 이벤트를 만들지 않는다.
- [x] current/history/asset association 조회 경계를 분리한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `ListDocumentAssetsUsecase`를 추가했다.
  - usecase는 target document current 존재를 확인한 뒤 `DocumentAssetRepository::list_assets`만 호출한다.
  - `AssetStore`를 받지 않아 object bytes 조회 경로가 없다.
  - missing document는 association list 조회 없이 `NotFound`로 실패한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/tests/list_document_assets_tests.rs`
  - `.tasks/task040.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test list_document_assets_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Phase 5를 완료 처리하고 Task 041에서 Phase 6 `MarkdownParser` port와 parser adapter를 시작한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Task 060. Local Document Asset Repository

## 1. Task Purpose

- [x] 이 태스크의 목적은 Phase 8 query benchmark와 release smoke가 사용할 local document-asset association adapter를 구현하는 것이다.
- [x] 이 태스크는 `DocumentAssetRepository` 포트의 로컬 구현을 추가해 첨부 metadata 조회가 fake repository에 의존하지 않도록 한다.

## 2. Scope

- [x] `LocalDocumentAssetRepository`를 추가한다.
- [x] document별 asset association을 local metadata file로 저장한다.
- [x] attach는 같은 asset id 중복 등록 시 `AlreadyAttached`를 반환한다.
- [x] list는 asset object bytes 없이 metadata와 reference만 반환한다.
- [x] adapter는 domain rule을 재구현하지 않고 value object 생성 실패를 corrupted metadata로 매핑한다.
- [x] local adapter 테스트를 추가한다.

## 3. TDD Plan

- [x] 실패하는 local document asset repository 테스트를 먼저 작성한다.
- [x] 실패 원인을 확인한 뒤 adapter를 구현한다.
- [x] adapter test와 전체 workspace 품질 게이트를 실행한다.

## 4. Architecture Rules

- [x] adapter는 filesystem 접근을 `cabinet-adapters` 안에만 둔다.
- [x] adapter는 document body나 asset object bytes를 association file에 저장하지 않는다.
- [x] adapter는 Git CLI나 사용자-facing Git 개념을 사용하지 않는다.
- [x] adapter format은 local install 1회 원칙을 깨는 외부 daemon이나 DB를 요구하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalDocumentAssetRepository`를 추가했다.
  - document별 association을 `assets.tsv`로 저장하고 label/file/media type은 hex encoding으로 보존한다.
  - 중복 attach는 `AlreadyAttached`를 반환하고 list는 metadata/reference만 반환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_document_asset_repository.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_document_asset_repository_tests.rs`
  - `.tasks/task060.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_document_asset_repository_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 061. Query Performance Benchmarks

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

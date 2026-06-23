# Task 025. Local AssetStore Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 Task 024에서 정의한 `AssetStore` port의 local adapter를 구현하는 것이다.
- [x] 이 태스크는 첨부 metadata와 object bytes를 local filesystem layout에서 분리 저장한다.
- [x] 이 태스크는 metadata-only 조회가 object bytes 존재 여부에 종속되지 않도록 검증한다.

## 2. Scope

- [x] `LocalAssetStore`를 추가한다.
- [x] asset store root를 생성자 인자로 명시적으로 받는다.
- [x] asset metadata layout을 구현한다.
- [x] content-addressed object layout을 구현한다.
- [x] duplicate asset registration을 `AlreadyPresent`로 보고한다.
- [x] object file missing 상태를 `MissingObject`로 보고한다.
- [x] 손상된 metadata를 `CorruptedMetadata`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 put/get metadata without object read test를 먼저 작성한다.
- [x] 실패하는 get object bytes test를 먼저 작성한다.
- [x] 실패하는 duplicate registration test를 먼저 작성한다.
- [x] 실패하는 missing object test를 먼저 작성한다.
- [x] 실패하는 corrupted metadata test를 먼저 작성한다.

## 4. Architecture Rules

- [x] filesystem 접근은 adapter 계층에만 둔다.
- [x] adapter는 `AssetStore` port를 구현하고 document body 저장소를 호출하지 않는다.
- [x] adapter는 Git CLI, process command, external DB, external object storage SDK에 의존하지 않는다.
- [x] adapter 생성자는 bootstrap에서 검증된 asset store root를 명시적으로 받는다.
- [x] adapter 내부에서 환경 변수를 읽지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalAssetStore`를 추가하고 `AssetStore` port를 구현했다.
  - asset metadata는 별도 metadata layout에 저장하고 object bytes는 content-addressed object layout에 저장했다.
  - metadata-only 조회가 object file 누락과 독립적으로 동작하도록 했다.
  - missing object와 corrupted metadata를 명시적 port error로 보고한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_asset_store.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_asset_store_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_asset_store_tests --quiet`: initial fail, missing adapter module
  - `cargo test -p cabinet-adapters --test local_asset_store_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 026은 `LocalSetupHealthChecker`와 Git CLI 비의존 검증을 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

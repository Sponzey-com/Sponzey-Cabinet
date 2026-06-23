# Task 024. AssetStore Port and Metadata/Object Separation Contract

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 4의 `AssetStore` port를 정의하는 것이다.
- [x] 이 태스크는 첨부 파일 metadata와 원본 object bytes를 분리 조회하는 계약을 고정한다.
- [x] 이 태스크는 document body에 첨부 원본을 포함하지 않는 구조를 저장소 port에서도 유지한다.

## 2. Scope

- [x] `cabinet-ports`에 `asset_store` module을 추가한다.
- [x] `AssetObject`를 정의한다.
- [x] `AssetRecord`를 metadata와 object bytes를 묶는 port-level record로 정의한다.
- [x] `AssetStorePutOutcome`을 정의한다.
- [x] `AssetStore` trait를 정의한다.
- [x] metadata id와 object id 불일치를 거부한다.
- [x] metadata byte size와 object byte length 불일치를 거부한다.
- [x] metadata-only 조회 계약을 정의한다.

## 3. TDD Plan

- [x] 실패하는 mismatched asset id test를 먼저 작성한다.
- [x] 실패하는 mismatched byte size test를 먼저 작성한다.
- [x] 실패하는 metadata-only lookup test를 먼저 작성한다.
- [x] 실패하는 duplicate registration outcome test를 먼저 작성한다.

## 4. Architecture Rules

- [x] asset port는 document body나 editor state를 참조하지 않는다.
- [x] asset port는 filesystem path, object storage key, DB row를 노출하지 않는다.
- [x] asset metadata 조회는 object bytes 조회를 요구하지 않는다.
- [x] asset port는 external object storage 구현체를 import하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `AssetStore` port를 추가하고 metadata-only 조회와 object bytes 조회를 분리했다.
  - `AssetRecord`가 metadata id/object id 및 byte size/object length 불일치를 거부하도록 했다.
  - duplicate registration을 `AlreadyPresent` outcome으로 표현했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/asset_store.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/asset_store_contract_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test asset_store_contract_tests --quiet`: initial fail, missing port module
  - `cargo test -p cabinet-ports --test asset_store_contract_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 025는 `AssetStore` port의 local adapter를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

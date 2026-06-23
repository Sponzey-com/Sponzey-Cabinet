# Task 023. Local VersionStore Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 Task 022에서 정의한 `VersionStore` port의 local adapter를 구현하는 것이다.
- [x] 이 태스크는 version snapshot과 history entry를 current snapshot repository와 분리 저장한다.
- [x] 이 태스크는 특정 version snapshot 조회와 cursor 기반 history pagination을 local layout으로 검증한다.

## 2. Scope

- [x] `LocalVersionStore`를 추가한다.
- [x] version store root를 생성자 인자로 명시적으로 받는다.
- [x] version entry metadata와 snapshot body를 분리 저장한다.
- [x] 특정 version snapshot direct lookup을 구현한다.
- [x] cursor 기반 history pagination을 구현한다.
- [x] duplicate version append를 `Conflict`로 보고한다.
- [x] 손상된 version metadata를 `CorruptedHistory`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 append/get snapshot test를 먼저 작성한다.
- [x] 실패하는 history pagination test를 먼저 작성한다.
- [x] 실패하는 duplicate version conflict test를 먼저 작성한다.
- [x] 실패하는 corrupted version metadata test를 먼저 작성한다.

## 4. Architecture Rules

- [x] filesystem 접근은 adapter 계층에만 둔다.
- [x] adapter는 `VersionStore` port를 구현하고 current snapshot repository를 호출하지 않는다.
- [x] adapter는 Git CLI, process command, external DB, external search engine에 의존하지 않는다.
- [x] adapter 생성자는 bootstrap에서 검증된 version store root를 명시적으로 받는다.
- [x] adapter 내부에서 환경 변수를 읽지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalVersionStore`를 추가하고 `VersionStore` port를 구현했다.
  - version entry metadata와 snapshot body를 current snapshot layout과 분리 저장했다.
  - 특정 version 조회는 version id 기반 direct lookup으로 처리하고, history 목록은 cursor/limit 기반으로 읽는다.
  - duplicate version id는 `Conflict`, 손상된 version metadata는 `CorruptedHistory`로 보고한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_version_store.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_version_store_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_version_store_tests --quiet`: initial fail, missing adapter module
  - `cargo test -p cabinet-adapters --test local_version_store_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 024는 `AssetStore` port와 asset metadata/object 분리 계약을 정의한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

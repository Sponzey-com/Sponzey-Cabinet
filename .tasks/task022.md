# Task 022. VersionStore Port and History Contract

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 4의 `VersionStore` port를 정의하는 것이다.
- [x] 이 태스크는 current snapshot repository와 version history store를 명확히 분리한다.
- [x] 이 태스크는 특정 version snapshot 조회와 history pagination 계약을 테스트로 고정한다.

## 2. Scope

- [x] `cabinet-ports`에 `version_store` module을 추가한다.
- [x] `VersionSnapshot`을 정의한다.
- [x] `VersionRecord`를 entry와 snapshot을 묶는 port-level record로 정의한다.
- [x] `HistoryCursor`, `HistoryPageRequest`, `HistoryPage`를 정의한다.
- [x] `VersionStore` trait를 정의한다.
- [x] entry와 snapshot의 document id 또는 snapshot ref 불일치를 거부한다.
- [x] history page limit 검증 규칙을 정의한다.

## 3. TDD Plan

- [x] 실패하는 mismatched version record identity test를 먼저 작성한다.
- [x] 실패하는 specific version snapshot retrieval test를 먼저 작성한다.
- [x] 실패하는 history pagination contract test를 먼저 작성한다.
- [x] 실패하는 invalid page limit test를 먼저 작성한다.

## 4. Architecture Rules

- [x] version port는 current snapshot repository를 호출하지 않는다.
- [x] version port는 Git commit, branch, repository 같은 사용자-facing Git 개념을 노출하지 않는다.
- [x] version port는 filesystem, process command, external DB, external search engine에 의존하지 않는다.
- [x] pagination은 full history load를 기본 조회 계약으로 삼지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `VersionStore` port를 추가하고 version append, 특정 snapshot 조회, history pagination 계약을 정의했다.
  - `VersionRecord`가 entry와 snapshot의 document id/snapshot ref 불일치를 거부하도록 했다.
  - `HistoryPageRequest`가 page limit 1..100 범위만 허용하도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/version_store.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/version_store_contract_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test version_store_contract_tests --quiet`: initial fail, missing port module
  - `cargo test -p cabinet-ports --test version_store_contract_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 023은 `VersionStore` port의 local adapter를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

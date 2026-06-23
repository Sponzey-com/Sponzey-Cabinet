# Task 021. Local DocumentRepository Current Snapshot Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 Task 020에서 정의한 `DocumentRepository` port의 local current snapshot adapter를 구현하는 것이다.
- [x] 이 태스크는 current snapshot을 version history와 분리된 local layout에 저장한다.
- [x] 이 태스크는 id 조회와 path 조회가 각각 direct lookup으로 동작해야 한다는 저장 계약을 테스트한다.

## 2. Scope

- [x] `LocalDocumentRepository`를 추가한다.
- [x] workspace root를 생성자 인자로 명시적으로 받는다.
- [x] id 기반 current snapshot layout을 구현한다.
- [x] path 기반 direct lookup index를 구현한다.
- [x] current snapshot metadata와 body를 분리 저장한다.
- [x] delete 시 id lookup과 path lookup을 함께 제거한다.
- [x] 손상된 metadata를 `CorruptedMetadata`로 보고한다.

## 3. TDD Plan

- [x] 실패하는 write/read by id test를 먼저 작성한다.
- [x] 실패하는 write/read by path test를 먼저 작성한다.
- [x] 실패하는 delete current test를 먼저 작성한다.
- [x] 실패하는 corrupted metadata test를 먼저 작성한다.

## 4. Architecture Rules

- [x] filesystem 접근은 adapter 계층에만 둔다.
- [x] adapter는 `DocumentRepository` port를 구현하고 usecase orchestration rule을 포함하지 않는다.
- [x] adapter는 Git CLI, process command, external DB, external search engine에 의존하지 않는다.
- [x] adapter 생성자는 bootstrap에서 검증된 path를 명시적으로 받는다.
- [x] adapter 내부에서 환경 변수를 읽지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LocalDocumentRepository`를 추가하고 `DocumentRepository` port를 구현했다.
  - current snapshot metadata와 body를 `by-id` layout에 분리 저장하고, path lookup은 `by-path` direct index로 처리했다.
  - delete 시 id layout과 path index를 함께 제거하도록 했다.
  - 손상된 metadata/body/index는 port error로 변환한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_document_repository.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_document_repository_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_document_repository_tests --quiet`: initial fail, missing adapter module
  - `cargo test -p cabinet-adapters --test local_document_repository_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 022는 `VersionStore` port와 history pagination/snapshot 조회 계약을 정의한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

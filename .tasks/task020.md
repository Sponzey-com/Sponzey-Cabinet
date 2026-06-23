# Task 020. DocumentRepository Port and Current Snapshot Contract

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 4의 `DocumentRepository` port와 current snapshot direct read 계약을 정의하는 것이다.
- [x] 이 태스크는 local adapter 구현 전에 usecase가 의존할 저장소 경계를 먼저 고정한다.
- [x] 이 태스크는 현재 문서 조회가 version history scan에 의존하지 않아야 한다는 계약을 테스트로 표현한다.

## 2. Scope

- [x] `cabinet-ports`에 `document_repository` module을 추가한다.
- [x] `CurrentDocumentRecord`를 metadata와 current snapshot을 묶는 port-level record로 정의한다.
- [x] `DocumentRepository` trait를 정의한다.
- [x] id 기반 current snapshot 조회 계약을 정의한다.
- [x] path 기반 current snapshot 조회 계약을 정의한다.
- [x] metadata id와 snapshot id가 다른 record 생성을 거부한다.

## 3. TDD Plan

- [x] 실패하는 mismatched metadata/snapshot identity test를 먼저 작성한다.
- [x] 실패하는 current snapshot direct read by id contract test를 먼저 작성한다.
- [x] 실패하는 current snapshot direct read by path contract test를 먼저 작성한다.
- [x] 실패하는 missing current snapshot contract test를 먼저 작성한다.

## 4. Architecture Rules

- [x] port는 domain type을 사용하되 storage schema, filesystem path, DB row, Git concept를 노출하지 않는다.
- [x] port는 adapter 구현체를 import하지 않는다.
- [x] current snapshot 조회 port는 version history 조회 port와 분리한다.
- [x] Product Log, Field Debug Log, Development Log 구현체는 이 태스크 범위에 포함하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.

  - `DocumentRepository` port를 추가하고 current snapshot read/write/delete 계약을 정의했다.
  - `CurrentDocumentRecord`가 metadata와 snapshot의 document id 불일치를 거부하도록 했다.
  - current snapshot 조회 계약을 id/path 기준으로 분리하고 history scan 없이 동작해야 함을 테스트로 표현했다.
- [x] 생성하거나 수정한 파일을 기록한다.

  - `crates/cabinet-ports/src/document_repository.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/document_repository_contract_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.

  - `cargo test -p cabinet-ports --test document_repository_contract_tests --quiet`: initial fail, missing port module
  - `cargo test -p cabinet-ports --test document_repository_contract_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.

  - Task 021은 `DocumentRepository` port의 local current snapshot adapter를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
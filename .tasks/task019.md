# Task 019. Link and Backlink Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-017 Link domain model`을 구현하는 것이다.
- [x] 이 태스크는 문서 링크를 단순 문자열이 아니라 source, target, status, range를 가진 value object로 정의한다.

## 2. Scope

- [x] `SourceRange`, `LinkTarget`, `LinkStatus`, `DocumentLink`를 추가한다.
- [x] `Backlink`를 추가한다.
- [x] resolved/unresolved link와 invalid range tests를 추가한다.

## 3. TDD Plan

- [x] 실패하는 resolved link test를 먼저 작성한다.
- [x] 실패하는 unresolved link test를 먼저 작성한다.
- [x] 실패하는 invalid range test를 먼저 작성한다.

## 4. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - 문서 링크, 백링크, resolved/unresolved target, source range value object를 domain 계층에 추가했다.
  - invalid source range는 생성 시점에 거부하도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-domain/src/link.rs`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-domain/tests/link_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo fmt --all --check`: pass
  - `sh scripts/check_domain_boundaries.sh`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 020은 Phase 4의 시작 작업으로 `DocumentRepository` port와 current snapshot 저장 계약을 정의한다.

## 5. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

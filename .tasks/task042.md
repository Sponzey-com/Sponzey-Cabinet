# Task 042. LinkIndex Port and Local Projection Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-041 Link index`를 구현하는 것이다.
- [x] 이 태스크는 문서별 link projection을 저장하고 backlink, unresolved link, orphan document query를 제공한다.

## 2. Scope

- [x] `LinkIndex` port를 추가한다.
- [x] source document별 `LinkProjectionRecord`를 정의한다.
- [x] backlink source identity mismatch를 거부한다.
- [x] unresolved link 목록에는 unresolved target만 허용한다.
- [x] local in-memory link index adapter를 추가한다.
- [x] backlinks query를 구현한다.
- [x] unresolved links query를 구현한다.
- [x] orphan documents query를 구현한다.

## 3. TDD Plan

- [x] 실패하는 link projection record validation contract test를 먼저 작성한다.
- [x] 실패하는 local link index replace and backlink query test를 먼저 작성한다.
- [x] 실패하는 unresolved and orphan query test를 먼저 작성한다.

## 4. Architecture Rules

- [x] port는 domain link/document/workspace value object만 참조한다.
- [x] adapter는 port를 구현하고 UI, env, network, Git CLI를 import하지 않는다.
- [x] replace operation은 source document의 기존 projection을 새 projection으로 교체한다.
- [x] orphan query는 주어진 document id 목록 기준으로 incoming backlink가 없는 문서를 반환한다.
- [x] p95 projection benchmark는 Phase 8 gate에서 별도로 측정한다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `LinkIndex` port와 `LinkProjectionRecord`를 추가했다.
  - projection record는 source mismatch와 resolved link in unresolved list를 거부한다.
  - `LocalLinkIndex` adapter를 추가했다.
  - local adapter는 source document별 projection replacement, backlinks, unresolved links, orphan documents query를 지원한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/link_index.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/link_index_contract_tests.rs`
  - `crates/cabinet-adapters/src/local_link_index.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_link_index_tests.rs`
  - `.tasks/task042.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test link_index_contract_tests --quiet` 통과
  - `cargo test -p cabinet-adapters --test local_link_index_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 043은 `GraphLiteProjection` query를 구현한다.
  - depth 1 graph query는 LinkIndex 위의 usecase로 구성한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

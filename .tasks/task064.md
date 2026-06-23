# Task 064. MVP End-to-End Flow Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-112 MVP end-to-end flow`를 구현하는 것이다.
- [x] 사용자가 설치 후 문서를 만들고, 편집하고, 링크/검색/첨부/복원 흐름을 수행할 수 있는지 자동 smoke로 검증한다.
- [x] end-to-end smoke는 단위 테스트를 대체하지 않고 release gate의 사용자-facing 흐름만 검증한다.

## 2. Current Context

- [x] Task 062에서 clean install smoke가 완료되었다.
- [x] Task 063에서 문서/버전/첨부 데이터 보존 smoke가 완료되었다.
- [x] 기존 usecase와 local adapter는 문서 CRUD, version history, markdown parsing, search index, link index, asset attach, restore preview/restore를 개별적으로 검증하고 있다.
- [x] 검색/링크 index는 파생 데이터로 다루며, 이번 smoke 안에서 현재 문서 상태를 기반으로 명시적으로 갱신한다.

## 3. Scope

### Included

- [x] platform release smoke API에 MVP end-to-end smoke를 추가한다.
- [x] smoke는 clean install, migration, create document, edit document, markdown parse, search index update, link projection update, asset attach, restore preview, restore를 수행한다.
- [x] smoke는 검색 결과, backlink projection, asset reference parsing, asset metadata listing, restore final state, restored current document를 검증한다.

### Excluded

- [x] UI automation은 이번 태스크에서 다루지 않는다.
- [x] 검색/링크 인덱스 영속화는 이번 태스크에서 다루지 않는다.
- [x] 릴리스 게이트 통합 스크립트와 문서 업데이트는 후속 태스크로 분리한다.

## 4. Functional Units

### Functional Unit 1

- [x] `run_mvp_end_to_end_smoke` 입력은 명시적 app data dir만 받는다.
- [x] 성공 조건은 first-run과 migration이 완료되는 것이다.
- [x] 실패 조건은 config invalid, first-run 실패, migration 실패다.

### Functional Unit 2

- [x] smoke는 source document와 target document를 생성하고 source document를 wikilink, 검색 키워드, asset reference가 포함된 body로 수정한다.
- [x] smoke는 markdown parser 결과로 wikilink와 asset reference를 확인한다.
- [x] smoke는 search/link 파생 인덱스를 현재 문서 상태에서 명시적으로 갱신한다.

### Functional Unit 3

- [x] smoke는 검색 결과에서 source document를 찾는다.
- [x] smoke는 target document 중심 graph에서 source backlink를 찾는다.
- [x] smoke는 첨부 metadata 조회와 restore preview/restore/current read-back을 검증한다.

## 5. Architecture Notes

- [x] release smoke 조합은 platform 계층에 둔다.
- [x] domain/usecase/core 계층은 filesystem, UI, platform SDK에 직접 접근하지 않는다.
- [x] local adapter는 port 구현체로만 사용한다.
- [x] search/link index 갱신은 파생 projection update로 다루고, domain rule을 UI나 test에 복제하지 않는다.

## 6. Configuration Rules

- [x] app data dir은 smoke input으로 명시적으로 전달한다.
- [x] smoke 중간에 env 값을 읽거나 변경하지 않는다.
- [x] config는 `AppConfig`로 한 번 변환하고 이후 path/config는 명시적 인자로 전달한다.

## 7. Logging Requirements

- [x] Product Log collector는 document body와 asset bytes가 product event에 포함되지 않는지 확인한다.
- [x] Field Debug Log는 새로 활성화하지 않는다.
- [x] Development Log는 추가하지 않는다.

## 8. State Machine Requirements

- [x] first-run state가 `Completed`인지 확인한다.
- [x] migration state가 `Completed`인지 확인한다.
- [x] restore state가 `Completed`인지 확인한다.
- [x] 복잡한 흐름은 report boolean 조합이 아니라 usecase/state 결과를 기준으로 검증한다.

## 9. TDD Plan

- [x] 실패하는 MVP end-to-end smoke test를 먼저 작성한다.
- [x] 테스트는 명시적 temp app data dir로 smoke를 실행한다.
- [x] 테스트는 create/edit/search/link/asset/restore 결과를 검증한다.
- [x] 실패 원인을 확인한 뒤 platform release smoke API를 구현한다.
- [x] 관련 테스트와 전체 품질 게이트를 실행한다.

## 10. Implementation Checklist

- [x] `crates/cabinet-platform/tests/mvp_end_to_end_smoke.rs`를 추가한다.
- [x] `MvpEndToEndSmokeInput`을 추가한다.
- [x] `MvpEndToEndSmokeReport`를 추가한다.
- [x] `run_mvp_end_to_end_smoke`를 구현한다.
- [x] release smoke 내부에 current document를 search record로 변환하는 helper를 추가한다.
- [x] release smoke 내부에 parsed wikilink를 link projection으로 반영하는 helper를 추가한다.
- [x] restore 후 current document가 target version body와 일치하는지 검증한다.

## 11. Validation Checklist

- [x] `cargo test -p cabinet-platform --test mvp_end_to_end_smoke --quiet`가 통과한다.
- [x] `cargo fmt --all --check`가 통과한다.
- [x] `cargo test --workspace --quiet`가 통과한다.
- [x] `sh scripts/check_architecture_boundaries.sh`가 통과한다.
- [x] `sh scripts/check_no_git_cli_dependency.sh`가 통과한다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] 유스케이스가 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 I/O가 boundary 계층에만 존재한다.

## 12. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `run_mvp_end_to_end_smoke`를 추가해 clean install, migration, create target/source, edit source, markdown parse, search/link projection update, asset attach/list, restore preview, restore, current read-back을 검증한다.
  - Product Log collector가 document body와 asset bytes를 포함하지 않는지 확인한다.
  - source document history가 create/update/restore 3개 entry를 보유하는지 확인한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-platform/src/release_smoke.rs`
  - `crates/cabinet-platform/tests/mvp_end_to_end_smoke.rs`
  - `.tasks/task064.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-platform --test mvp_end_to_end_smoke --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 남은 위험 요소를 기록한다.
  - UI automation은 아직 release gate에 포함되지 않았다. 현재는 platform-level 사용자 흐름 smoke로 검증한다.
  - 검색/링크 index는 파생 데이터이며 persistence가 아니라 현재 문서 기반 projection update로 검증한다.
- [x] 후속 태스크에서 이어받아야 할 내용을 기록한다.
  - Task 065는 MVP release gate script와 release documentation update를 구현한다.

## 13. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인한다.
- [x] 도달했다면 추가 태스크를 생성하지 않는다.
- [x] 도달하지 못했다면 남은 목표를 정리한다.
- [x] 다음 우선순위 작업을 선택한다.
- [x] 다음 태스크가 기능 2~3개 단위를 넘지 않도록 범위를 제한한다.
- [x] 다음 태스크를 `taskXXX.md`로 생성한다.
- [x] 다음 태스크를 즉시 실행한다.

## 14. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

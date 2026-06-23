# Task 063. Local Data Preservation Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 8의 `MVP-111 Local data preservation smoke`를 구현하는 것이다.
- [x] clean install 이후 문서 현재본, 문서 이력, 특정 버전 snapshot, 첨부 metadata, 첨부 object가 앱 재초기화 후에도 보존되는지 검증한다.
- [x] migration 재실행이 idempotent이며 기존 사용자 데이터를 변경하지 않는지 검증한다.

## 2. Current Context

- [x] Task 062에서 explicit app data dir 기반 clean install smoke가 구현되었다.
- [x] `LocalDocumentRepository`, `LocalVersionStore`, `LocalAssetStore`, `LocalDocumentAssetRepository`는 파일 기반 데이터를 보존할 수 있다.
- [x] `LocalSearchIndex`, `LocalLinkIndex`는 현재 메모리 기반 파생 인덱스이므로 권위 데이터 보존 범위에서 제외하고, 후속 MVP end-to-end flow에서 재색인/검색 흐름을 검증한다.

## 3. Scope

### Included

- [x] platform release smoke API에 data preservation smoke를 추가한다.
- [x] smoke는 first-run, migration, create/update document, attach asset, adapter 재생성, read-back 검증을 수행한다.
- [x] smoke는 문서/버전/첨부 데이터 보존과 migration idempotency를 report로 반환한다.

### Excluded

- [x] 검색 인덱스 영속화 구현은 이번 태스크에서 다루지 않는다.
- [x] 링크 인덱스 영속화 구현은 이번 태스크에서 다루지 않는다.
- [x] 전체 MVP end-to-end flow smoke는 후속 태스크로 분리한다.

## 4. Functional Units

### Functional Unit 1

- [x] `run_data_preservation_smoke` 입력은 명시적 app data dir만 받는다.
- [x] 성공 조건은 clean install과 initial migration이 완료되는 것이다.
- [x] 실패 조건은 config invalid, first-run 실패, migration 실패다.

### Functional Unit 2

- [x] smoke는 문서 생성, 문서 수정, 첨부 연결을 실제 usecase와 local adapter로 수행한다.
- [x] 입력은 고정된 fixture workspace/document/version/asset id다.
- [x] 성공 조건은 파일 기반 adapter에 데이터가 기록되는 것이다.
- [x] 실패 조건은 usecase 오류 또는 local storage 오류다.

### Functional Unit 3

- [x] smoke는 adapter를 새로 생성해 현재 문서, history, 특정 version, 첨부 metadata, 첨부 object를 다시 조회한다.
- [x] 성공 조건은 기대한 document body, version count, asset metadata, asset bytes가 모두 보존되는 것이다.
- [x] 실패 조건은 누락, 변조, migration 중복 기록이다.

## 5. Architecture Notes

- [x] release smoke 조합은 platform 계층에 둔다.
- [x] domain/usecase/core 계층은 filesystem에 직접 접근하지 않는다.
- [x] 외부 I/O는 local adapter에만 위치한다.
- [x] smoke는 usecase input/output과 port 구현체를 통해 동작을 검증한다.
- [x] 검색/링크 파생 인덱스의 재색인 책임은 후속 end-to-end smoke에서 검증한다.

## 6. Configuration Rules

- [x] 외부 환경 값은 테스트에서 직접 읽지 않는다.
- [x] app data dir은 smoke input으로 명시적으로 전달한다.
- [x] smoke 내부는 runtime 중간 env 변경이나 hidden global config를 사용하지 않는다.
- [x] migration과 adapter 경로는 `AppConfig`에서 파생된 명시적 path만 사용한다.

## 7. Logging Requirements

- [x] Product Log는 usecase 성공/실패 이벤트가 문서 본문과 첨부 bytes를 포함하지 않는지 확인할 수 있는 no-op collector로 검증한다.
- [x] Field Debug Log는 이번 태스크에서 새로 활성화하지 않는다.
- [x] Development Log는 이번 태스크에서 추가하지 않는다.

## 8. State Machine Requirements

- [x] first-run state가 `Completed`인지 확인한다.
- [x] migration state가 `Completed`인지 확인한다.
- [x] migration 재실행은 추가 version record 없이 `Completed`가 되어야 한다.
- [x] 별도 플래그 조합으로 절차 상태를 관리하지 않는다.

## 9. TDD Plan

- [x] 실패하는 `data_preservation_smoke` 통합 테스트를 먼저 작성한다.
- [x] 테스트는 명시적 temp app data dir로 smoke를 실행한다.
- [x] 테스트는 current document, history count, specific version, asset metadata, asset object, migration idempotency를 검증한다.
- [x] 실패 원인을 확인한 뒤 platform release smoke API를 구현한다.
- [x] 관련 테스트와 전체 품질 게이트를 실행한다.

## 10. Implementation Checklist

- [x] `crates/cabinet-platform/tests/data_preservation_smoke.rs`를 추가한다.
- [x] `DataPreservationSmokeInput`을 추가한다.
- [x] `DataPreservationSmokeReport`를 추가한다.
- [x] `run_data_preservation_smoke`를 구현한다.
- [x] release smoke 내부에 local adapter 조립 helper를 추가한다.
- [x] Product Log용 no-op collector가 민감 데이터를 저장하지 않도록 한다.
- [x] migration 재실행이 중복 기록을 만들지 않는지 검증한다.

## 11. Validation Checklist

- [x] `cargo test -p cabinet-platform --test data_preservation_smoke --quiet`가 통과한다.
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
  - `run_data_preservation_smoke`를 추가해 clean install, initial migration, 문서 생성/수정, 첨부 연결, adapter 재생성 후 read-back 검증을 수행한다.
  - read-back은 현재 문서 body, history 2건, 최초 version snapshot, 첨부 metadata, 첨부 object bytes, migration idempotency를 확인한다.
  - Product Log collector는 문서 본문과 첨부 bytes가 product event에 포함되지 않는지 확인한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-platform/src/release_smoke.rs`
  - `crates/cabinet-platform/tests/data_preservation_smoke.rs`
  - `.tasks/task063.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-platform --test data_preservation_smoke --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 남은 위험 요소를 기록한다.
  - `LocalSearchIndex`, `LocalLinkIndex`는 현재 메모리 기반 파생 인덱스다. 검색/링크 사용자 흐름은 후속 MVP end-to-end flow에서 재색인 또는 fixture seeding 방식으로 검증해야 한다.
- [x] 후속 태스크에서 이어받아야 할 내용을 기록한다.
  - Task 064는 MVP end-to-end flow smoke를 구현한다. create/edit/link/search/restore/asset 흐름을 한 번의 release gate로 검증한다.

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

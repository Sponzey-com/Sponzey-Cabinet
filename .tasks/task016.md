# Task 016. Asset Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-014 Asset domain model`을 구현하는 것이다.
- [x] 이 태스크는 첨부 파일 원본을 document body와 분리하고 content-addressed asset identity를 domain으로 정의한다.
- [x] 이 태스크 완료 후 프로젝트는 `AssetId`, `AssetMetadata`, `AssetReference`와 validation tests를 가진다.

## 2. Scope

- [x] content-addressed `AssetId`를 추가한다.
- [x] `AssetFileName`, `AssetMediaType`, `AssetMetadata`를 추가한다.
- [x] document body 원본 저장 없이 asset을 가리키는 `AssetReference`를 추가한다.

## 3. TDD Plan

- [x] 실패하는 asset id validation test를 먼저 작성했다.
- [x] 실패하는 asset metadata validation test를 먼저 작성했다.
- [x] 실패하는 asset reference separation test를 먼저 작성했다.

## 4. Architecture Rules

- [x] 변경 계층은 pure domain이다.
- [x] domain은 filesystem, DB, network, env, logger, MIME detector에 의존하지 않는다.
- [x] asset 원본 bytes는 domain metadata/reference에 포함하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항:
  - `cabinet-domain::asset` 모듈을 추가했다.
  - `AssetId`, `AssetFileName`, `AssetMediaType`, `AssetMetadata`, `AssetReference`, `AssetError`를 추가했다.
  - SHA-256 hex content identity validation을 추가했다.
  - asset metadata와 reference separation tests를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/asset.rs`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-domain/tests/asset_tests.rs`
  - `.tasks/task015.md`
  - `.tasks/task016.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain asset`: 최초 실행은 `cabinet_domain::asset` 없음으로 실패했고, 구현 후 3개 asset 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
  - `cargo test --workspace`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - asset id는 64자 SHA-256 hex만 허용한다.
  - asset file name은 path traversal/path separator를 허용하지 않는다.
  - media type과 byte size를 검증한다.
  - asset reference는 asset id와 label만 포함하고 원본 bytes를 포함하지 않는다.
- [x] 남은 위험 요소:
  - asset lifecycle state machine은 아직 없다.
  - asset store adapter와 duplicate registration 정책은 후속 phase에서 필요하다.
  - content hash 계산은 adapter 책임이며 아직 구현하지 않았다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-015 Asset lifecycle state machine`을 시작한다.

## 6. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 다음 우선순위는 `MVP-015 Asset lifecycle state machine`이다.
- [x] 다음 태스크 파일명은 `.tasks/task017.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 7. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
- [ ] 외부 정보, 권한, 비밀값, 접근 권한이 없어 진행할 수 없다.

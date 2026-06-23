# Task 018. Version Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-016 Version domain model`을 구현하는 것이다.
- [x] 이 태스크는 current document snapshot과 history version entry를 domain에서 명확히 분리한다.

## 2. Scope

- [x] `CurrentDocumentSnapshot`을 추가한다.
- [x] `VersionId`, `VersionAuthor`, `VersionSummary`, `DocumentSnapshotRef`, `VersionEntry`를 추가한다.
- [x] current/history separation tests를 추가한다.

## 3. TDD Plan

- [x] 실패하는 current snapshot test를 먼저 작성했다.
- [x] 실패하는 version entry validation test를 먼저 작성했다.
- [x] 실패하는 current/history separation test를 먼저 작성했다.

## 4. Completion Report

- [x] 수행한 변경 사항:
  - `cabinet-domain::version` 모듈을 추가했다.
  - `CurrentDocumentSnapshot`과 `VersionEntry`를 별도 타입으로 추가했다.
  - `VersionId`, `VersionAuthor`, `VersionSummary`, `DocumentSnapshotRef`, `VersionError`를 추가했다.
  - current snapshot과 history entry separation tests를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/version.rs`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-domain/tests/version_tests.rs`
  - `.tasks/task017.md`
  - `.tasks/task018.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain version`: 최초 실행은 `cabinet_domain::version` 없음으로 실패했고, 구현 후 version filter 테스트가 통과했다.
  - `cargo test -p cabinet-domain --test version_tests`: 3개 version 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo fmt --all --check`: 통과.
  - `cargo test --workspace`: 통과.
- [x] 검증한 항목:
  - current snapshot은 document id와 body만 가진다.
  - history entry는 version id, document id, snapshot ref, author, summary를 가진다.
  - current snapshot과 history entry는 별도 타입으로 분리된다.
  - Git 개념을 domain/user-facing type 이름으로 노출하지 않는다.
- [x] 남은 위험 요소:
  - version store adapter와 history pagination은 아직 없다.
  - restore preview model은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-017 Link domain model`을 시작한다.

## 5. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 다음 우선순위는 `MVP-017 Link domain model`이다.
- [x] 다음 태스크 파일명은 `.tasks/task019.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

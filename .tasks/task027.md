# Task 027. Local Atomic File Utility and Recovery Tests

## 1. Task Purpose

- [x] 이 태스크의 목적은 Phase 4 저장소 adapter의 atomic write 정책을 공통 유틸과 테스트로 고정하는 것이다.
- [x] 이 태스크는 임시 파일 write, sync, replace, recovery 상태를 명시적으로 표현한다.
- [x] 이 태스크는 local document/version/asset adapter가 중복 atomic write 구현을 갖지 않도록 정리한다.

## 2. Scope

- [x] `local_atomic_file` module을 추가한다.
- [x] `AtomicWriteState`, `AtomicWriteOutcome`, `AtomicRecoveryOutcome`, `AtomicWriteError`를 정의한다.
- [x] `write_bytes_atomically`와 `write_text_atomically`을 구현한다.
- [x] stale temp file recovery를 구현한다.
- [x] document/version/asset local adapter의 중복 atomic write 함수를 공통 유틸 호출로 교체한다.

## 3. TDD Plan

- [x] 실패하는 atomic write completed state test를 먼저 작성한다.
- [x] 실패하는 stale temp recovery test를 먼저 작성한다.
- [x] 실패하는 parent path is file failure test를 먼저 작성한다.

## 4. Architecture Rules

- [x] filesystem 접근은 adapter 계층에만 둔다.
- [x] atomic write utility는 domain/usecase/port를 import하지 않는다.
- [x] adapter error mapping은 adapter 내부에서 수행한다.
- [x] 기능 변경과 정리는 같은 목적 안에서만 수행하고 public port 계약은 변경하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `local_atomic_file` 공통 module을 추가하고 atomic write, stale temp recovery, failure state를 테스트했다.
  - local document/version/asset adapter의 중복 atomic write 구현을 공통 유틸 호출로 교체했다.
  - public port 계약은 변경하지 않고 adapter 내부 error mapping만 유지했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-adapters/src/local_atomic_file.rs`
  - `crates/cabinet-adapters/src/local_document_repository.rs`
  - `crates/cabinet-adapters/src/local_version_store.rs`
  - `crates/cabinet-adapters/src/local_asset_store.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_atomic_file_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-adapters --test local_atomic_file_tests --quiet`: initial fail, missing atomic module
  - `cargo test -p cabinet-adapters --test local_atomic_file_tests --quiet`: pass
  - `cargo test -p cabinet-adapters --test local_document_repository_tests --quiet`: pass
  - `cargo test -p cabinet-adapters --test local_version_store_tests --quiet`: pass
  - `cargo test -p cabinet-adapters --test local_asset_store_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 028은 Phase 5의 시작으로 `CreateWorkspace` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

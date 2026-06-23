# Task 010. Local Migration Store Adapter와 Version Record Smoke

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-006 Local migration runner`를 local metadata adapter validation까지 완료하는 것이다.
- [x] 이 태스크는 migration version 기록 I/O를 `cabinet-adapters` 구현체에만 배치한다.
- [x] 이 태스크 완료 후 프로젝트는 first-run 이후 local metadata directory에 migration version을 실제로 기록하고 재실행 idempotency를 검증한다.

## 2. Current Context

- [x] 현재 코드베이스는 `MigrationStore` port, `MigrationRunner`, initial no-op migration plan을 가진다.
- [x] 이전 태스크 Task 009에서 migration 상태머신과 fake store version recording을 완료했다.
- [x] 이번 태스크는 `MVP-006`의 local adapter evidence를 추가했다.
- [x] 현재 확인된 제약 사항은 multi-process lock 안전성은 아직 구현하지 않는다는 점이다. 이번 태스크는 single-process local metadata smoke로 제한했다.

## 3. Scope

### Included

- [x] `cabinet-adapters`에 local filesystem migration store adapter를 추가한다.
- [x] first-run 이후 clean temp profile migration version record smoke를 추가한다.
- [x] 같은 temp profile migration rerun idempotency smoke를 추가한다.

### Excluded

- [x] multi-process lock contention 처리는 후속 fault-injection 태스크로 넘긴다.
- [x] migration file format versioning은 후속 storage hardening 태스크로 넘긴다.
- [x] Product logger 구현체는 후속 logging foundation 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: local migration store adapter를 만들었다.
- [x] 입력: metadata directory path.
- [x] 출력: lock acquire/release, applied versions read, version record.
- [x] 성공 조건: adapter만 filesystem을 사용한다.
- [x] 실패 조건: core/domain/usecase가 migration file I/O를 직접 수행한다.

### Functional Unit 2

- [x] 구현한 기능: clean temp profile migration record smoke를 만들었다.
- [x] 입력: first-run으로 초기화된 temp profile.
- [x] 출력: completed outcome과 version record file.
- [x] 성공 조건: initial no-op version 1이 metadata directory에 기록된다.
- [x] 실패 조건: 수동 file 생성이나 외부 설정 파일이 필요하다.

### Functional Unit 3

- [x] 구현한 기능: migration rerun idempotency smoke를 만들었다.
- [x] 입력: 이미 version 1이 기록된 temp profile.
- [x] 출력: completed outcome, 추가 기록 없음.
- [x] 성공 조건: 재실행이 version 1을 중복 기록하지 않는다.
- [x] 실패 조건: 같은 migration version이 중복 기록된다.

## 5. Architecture Notes

- [x] 변경되는 계층은 adapter layer와 adapter test다.
- [x] `cabinet-adapters`는 `cabinet-core`의 `MigrationStore` port를 구현한다.
- [x] core/domain/usecase에는 filesystem access를 추가하지 않았다.
- [x] adapter test는 first-run adapter와 migration adapter를 함께 사용해 local sequence를 검증한다.
- [x] 외부 DB, 외부 검색 서버, Git CLI, Node.js, 수동 env, 수동 설정 파일을 요구하지 않는다.

## 6. Configuration Rules

- [x] `AppConfig`는 test에서 명시적으로 생성한다.
- [x] process environment를 읽지 않았다.
- [x] runtime 중간 설정 변경 API를 만들지 않았다.
- [x] adapter는 config를 전역으로 저장하지 않는다.
- [x] temp path는 test helper에서 명시적으로 생성하고 cleanup한다.

## 7. Logging Requirements

### Product Log

- [x] logger implementation은 추가하지 않았다.
- [x] adapter는 Product Log를 직접 기록하지 않는다.
- [x] migration outcome enum은 core runner 결과를 그대로 사용한다.
- [x] raw path를 Product event payload에 추가하지 않았다.

### Field Debug Log

- [x] Field Debug Log 구현은 추가하지 않았다.
- [x] adapter 내부 file detail은 test assertion으로만 검증한다.
- [x] 운영 diagnostic은 후속 logging foundation 태스크에서 masking 정책과 함께 정의한다.

### Development Log

- [x] runtime Development Log를 추가하지 않았다.
- [x] 테스트용 filesystem cleanup 실패는 test guard cleanup으로 처리한다.
- [x] 프로덕션 기본 동작에 포함되는 개발용 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] adapter는 상태 전이를 직접 구현하지 않는다.
- [x] 상태 전이는 `MigrationRunner`와 `transition_migration`을 통해서만 발생한다.
- [x] clean temp profile migration은 `Completed`로 종료한다.
- [x] rerun migration은 `Completed`로 종료한다.
- [x] lock contention/failure simulation은 이번 태스크 범위에서 제외한다.

## 9. TDD Plan

- [x] 실패하는 local migration record smoke test를 먼저 작성했다.
- [x] 실패하는 local migration rerun idempotency test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 adapter 구현만 작성했다.
- [x] 외부 의존성은 standard library filesystem으로만 제한했다.
- [x] core boundary check가 계속 통과하는지 확인했다.
- [x] 구현 후 중복과 구조 문제를 정리했다.

## 10. Implementation Checklist

- [x] adapter tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] `LocalMigrationStore` adapter를 작성했다.
- [x] migration version value accessor를 필요한 최소 범위로 추가했다.
- [x] clean temp profile smoke를 통과시켰다.
- [x] rerun idempotency smoke를 통과시켰다.
- [x] core가 filesystem/env/network를 직접 접근하지 않는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] adapter가 port/interface 뒤에 숨겨진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-adapters::local_migration::LocalMigrationStore`를 추가했다.
  - `MIGRATION_LOCK_FILE`, `MIGRATION_VERSIONS_FILE` 파일명을 명시했다.
  - `MigrationVersion::value()` accessor를 추가했다.
  - first-run 이후 migration version record smoke를 추가했다.
  - migration rerun idempotency smoke를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/migration.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/src/local_migration.rs`
  - `crates/cabinet-adapters/tests/local_migration_store_tests.rs`
  - `.tasks/task009.md`
  - `.tasks/task010.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-adapters local_migration_store`: 최초 실행은 `local_migration` 모듈 없음으로 실패했고, 구현 후 2개 adapter smoke가 통과했다.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
  - `sh scripts/check_migration_boundaries.sh`: 통과.
  - `sh scripts/check_first_run_boundaries.sh`: 통과.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - first-run 이후 metadata directory에 migration version 1이 기록된다.
  - migration lock file은 runner 완료 후 제거된다.
  - 같은 temp profile에서 재실행해도 version 1이 중복 기록되지 않는다.
  - migration filesystem I/O는 adapter 계층에만 존재한다.
  - migration core에는 filesystem/env/network 직접 접근이 없다.
- [x] 남은 위험 요소:
  - multi-process lock contention은 아직 안전하지 않다.
  - corrupted migration version file recovery는 아직 없다.
  - Product logger port와 실제 log emission은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - `MVP-006 Local migration runner`의 core/fake/local adapter validation은 완료했다.
  - 다음 태스크는 Phase 2의 `MVP-007 Logging foundation`을 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 logging foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-007 Logging foundation`이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 log event model, Product/Field/Development sink ports, 민감정보 배제 테스트로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task011.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 14. Stop Conditions

다음 조건을 확인했다.

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
- [ ] 외부 정보, 권한, 비밀값, 접근 권한이 없어 진행할 수 없다.
- [ ] `AGENTS.md` 원칙과 충돌하는 요구사항이 발견되었다.
- [ ] 테스트 또는 검증 환경이 없어 완료 여부를 판단할 수 없다.
- [ ] 코드베이스 구조가 계획과 크게 달라 태스크 재설계가 필요하다.
- [ ] 사용자 결정이 필요한 아키텍처 선택지가 발생했다.

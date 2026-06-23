# Task 011. Logging Foundation과 Sensitive Data Exclusion

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 2의 `MVP-007 Logging foundation`을 시작하는 것이다.
- [x] 이 태스크는 Product Log, Field Debug Log, Development Log를 타입과 port 수준에서 분리한다.
- [x] 이 태스크 완료 후 프로젝트는 stable event name, error code, 민감정보 배제 테스트를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 bootstrap config, first-run, migration foundation을 가진다.
- [x] 이전 태스크 Task 010에서 local migration adapter와 version record smoke를 완료했다.
- [x] 이번 태스크는 Phase 2 exit gate에 필요한 logging foundation을 시작했다.
- [x] 현재 확인된 제약 사항은 실제 logging backend 또는 file writer를 구현하지 않는다는 점이다. 이번 태스크는 core event model과 port contract로 제한했다.

## 3. Scope

### Included

- [x] Product/Field Debug/Development log event model을 추가한다.
- [x] Product/Field Debug/Development logger port trait을 분리한다.
- [x] sensitive key/value/raw path 배제 테스트를 추가한다.

### Excluded

- [x] 실제 log file writer는 후속 adapter 태스크로 넘긴다.
- [x] runtime log routing은 후속 composition 태스크로 넘긴다.
- [x] external observability integration은 MVP 범위 밖으로 둔다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: stable log event model을 만들었다.
- [x] 입력: first-run/migration outcome event.
- [x] 출력: `LogRecord` with level, event name, optional error code.
- [x] 성공 조건: Product event는 free-form message나 raw payload를 받지 않는다.
- [x] 실패 조건: document body, attachment content, secret, raw path가 Product event에 들어갈 수 있다.

### Functional Unit 2

- [x] 구현한 기능: 3단계 logger port를 분리했다.
- [x] 입력: Product/Field Debug/Development event.
- [x] 출력: `ProductLogger`, `FieldDebugLogger`, `DevelopmentLogger` trait.
- [x] 성공 조건: 각 logger는 자기 event type만 받는다.
- [x] 실패 조건: 하나의 generic logger가 모든 log level과 payload를 처리한다.

### Functional Unit 3

- [x] 구현한 기능: sensitive field sanitizer와 default activation policy를 만들었다.
- [x] 입력: key/value field 후보, `LoggingConfig`.
- [x] 출력: sanitized field 또는 rejection, default disabled flags.
- [x] 성공 조건: sensitive key, secret-like value, raw path는 reject된다.
- [x] 실패 조건: Field Debug 또는 Development 로그가 production default에서 활성화된다.

## 5. Architecture Notes

- [x] 변경되는 계층은 core runtime foundation이다.
- [x] logging backend 구현은 adapter 계층으로 미룬다.
- [x] logger port는 외부 I/O를 수행하지 않는다.
- [x] Product event model은 사용자 영향과 핵심 상태 변화만 표현한다.
- [x] Field Debug event model은 scope와 TTL을 요구한다.
- [x] Development event model은 production default에서 활성화되지 않는다.

## 6. Configuration Rules

- [x] `LoggingConfig` default는 Product Log enabled, Field Debug disabled, Development disabled로 유지한다.
- [x] runtime 중간에 logging 설정을 변경하는 API를 만들지 않았다.
- [x] process environment를 읽지 않았다.
- [x] log activation은 config object로 명시적으로 전달되는 값만 사용한다.
- [x] 전역 logger singleton을 만들지 않았다.

## 7. Logging Requirements

### Product Log

- [x] 목적은 사용자 영향, 핵심 상태 변화, 장애 원인 추적에 필요한 최소 정보다.
- [x] 허용 정보는 stable event name, stable error code, 비민감 count/version이다.
- [x] 금지 정보는 문서 본문, 첨부 내용, secret, token, password, raw path, free-form debug message다.
- [x] 사용 위치는 first-run completed/failed, migration completed/failed, usecase failed, local setup unhealthy다.
- [x] 리뷰 기준은 Product event 생성자가 free-form payload를 받지 않는지 확인하는 것이다.

### Field Debug Log

- [x] 목적은 운영 또는 고객 환경에서 제한적 문제 재현과 상태 확인이다.
- [x] 허용 정보는 scope, TTL, state, version, path role, cache/index diagnostic name이다.
- [x] 금지 정보는 문서 본문, 첨부 내용, secret, raw path, 영구 보존 debug dump다.
- [x] 사용 위치는 first-run step, migration state, cache/index diagnostic이다.
- [x] 리뷰 기준은 활성화 scope와 TTL이 없는 Field Debug event를 거부하는 것이다.

### Development Log

- [x] 목적은 로컬 개발, 테스트, 검증 과정의 확인이다.
- [x] 허용 정보는 local test setup, fake port call, parser intermediate result, benchmark detail이다.
- [x] 금지 정보는 production default build에서 활성화되는 development log path다.
- [x] 사용 위치는 test/dev-only code path다.
- [x] 리뷰 기준은 `LoggingConfig::default().development_log_enabled == false`를 유지하는 것이다.

## 8. State Machine Requirements

- [x] 이번 태스크는 새 상태머신을 추가하지 않는다.
- [x] log event는 first-run/migration state outcome과 연결 가능하다.
- [x] 상태 전이는 logging code에서 직접 수행하지 않는다.
- [x] logging code는 상태 flag를 조합해 절차를 관리하지 않는다.

## 9. TDD Plan

- [x] 실패하는 Product log event record test를 먼저 작성했다.
- [x] 실패하는 logger port separation test를 먼저 작성했다.
- [x] 실패하는 sensitive field rejection test를 먼저 작성했다.
- [x] 실패하는 default logging activation test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 구현 후 중복과 구조 문제를 정리했다.

## 10. Implementation Checklist

- [x] logging tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] logging module을 작성했다.
- [x] `cabinet-core`에서 module을 공개했다.
- [x] logger port trait을 분리했다.
- [x] 민감정보 배제 테스트를 통과시켰다.
- [x] core가 filesystem/env/network를 직접 접근하지 않는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] logging event가 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준에 맞게 분리되어 있다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-core::logging` 모듈을 추가했다.
  - `LogLevel`, `LogRecord`, `LogField`, `LogFieldError`, `LogErrorCode`를 추가했다.
  - `ProductLogEvent`, `FieldDebugLogEvent`, `DevelopmentLogEvent`와 각 event name enum을 추가했다.
  - `ProductLogger`, `FieldDebugLogger`, `DevelopmentLogger` port trait을 분리했다.
  - Field Debug scope와 TTL 모델을 추가했다.
  - logging core I/O 금지 경계 검사 `scripts/check_logging_boundaries.sh`를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-core/src/logging.rs`
  - `crates/cabinet-core/src/lib.rs`
  - `crates/cabinet-core/tests/logging_tests.rs`
  - `scripts/check_logging_boundaries.sh`
  - `.tasks/task010.md`
  - `.tasks/task011.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-core logging`: 최초 실행은 `cabinet_core::logging` 없음으로 실패했고, 구현 후 logging config filter 테스트가 통과했다.
  - `cargo test -p cabinet-core --test logging_tests`: 4개 logging 테스트가 통과했다.
  - `sh scripts/check_logging_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
  - `sh scripts/check_first_run_boundaries.sh`: 통과.
  - `sh scripts/check_migration_boundaries.sh`: 통과.
  - `sh scripts/check_runtime_config_boundaries.sh`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - Product Log event는 stable event name과 optional stable error code만 record로 변환한다.
  - Product/Field Debug/Development logger port가 타입 수준에서 분리되어 있다.
  - Field Debug event는 scope와 TTL을 포함한다.
  - sensitive key, secret-like value, raw path value가 rejected 된다.
  - `LoggingConfig::default()`는 Product enabled, Field Debug disabled, Development disabled다.
  - logging core에는 filesystem/env/network 직접 접근이 없다.
- [x] 남은 위험 요소:
  - 실제 log writer adapter는 아직 없다.
  - first-run/migration runner가 Product logger port를 실제로 호출하지는 않는다.
  - Field Debug 활성화 scope와 TTL의 runtime routing은 아직 없다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - Phase 2 runtime foundation은 exit gate를 충족했다.
  - 다음 태스크는 Phase 3의 `MVP-010 Workspace domain model`을 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-010 Workspace domain model`이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 workspace id/name/path policy와 validation, pure domain tests로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task012.md`다.
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

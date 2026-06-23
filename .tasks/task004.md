# Task 004. Desktop Shell Boundary Scaffold

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 1의 `MVP-003 Tauri desktop shell scaffold`를 시작하는 것이다.
- [x] 이 태스크는 desktop shell이 domain/usecase를 직접 우회하지 않고 platform boundary를 통해 호출하도록 구조를 만든다.
- [x] 이 태스크 완료 후 프로젝트는 `apps/desktop/src-tauri` Rust shell crate와 desktop boundary 정적 검증을 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 Rust core workspace와 frontend package skeleton을 가진 상태다.
- [x] 이전 태스크 Task 003에서 package workspace, UI/editor/client-core skeleton, frontend boundary check를 완료했다.
- [x] 이번 태스크는 Phase 1 exit gate에 필요한 desktop shell skeleton을 만들기 위해 시작했다.
- [x] 현재 확인된 제약 사항은 Tauri CLI가 설치되어 있지 않다는 점이다. 따라서 이번 태스크는 external dependency 없이 가능한 shell boundary scaffold로 제한했다.

## 3. Scope

### Included

- [x] `apps/desktop/src-tauri` shell crate skeleton을 생성한다.
- [x] desktop shell command boundary placeholder를 생성한다.
- [x] desktop shell boundary check script를 생성하고 실행한다.

### Excluded

- [x] Tauri CLI 설치는 이번 태스크에서 수행하지 않는다.
- [x] 실제 Tauri command macro와 window runtime 구현은 이번 태스크에서 수행하지 않는다.
- [x] local workspace filesystem access 구현은 후속 runtime/adapter 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: desktop shell Rust crate skeleton을 만들었다.
- [x] 입력: Phase 1 desktop shell scope, existing `cabinet-platform` crate.
- [x] 출력: `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/src/lib.rs`.
- [x] 성공 조건: `cargo test --workspace`에 desktop shell crate가 포함된다.
- [x] 실패 조건: desktop shell crate가 domain/usecase/adapters를 직접 우회한다.

### Functional Unit 2

- [x] 구현한 기능: command boundary placeholder를 만들었다.
- [x] 입력: desktop command는 DTO mapping과 platform boundary 호출만 수행해야 한다는 AGENTS 기준.
- [x] 출력: command request/response placeholder와 shell layer smoke test.
- [x] 성공 조건: shell boundary는 business rule을 포함하지 않는다.
- [x] 실패 조건: shell boundary가 document lifecycle, storage, version rule을 직접 구현한다.

### Functional Unit 3

- [x] 구현한 기능: desktop shell boundary check를 만들었다.
- [x] 입력: src-tauri manifest와 Rust source.
- [x] 출력: `scripts/check_desktop_shell_boundaries.sh`.
- [x] 성공 조건: forbidden direct dependency/import pattern을 검증한다.
- [x] 실패 조건: shell이 domain/usecase를 직접 의존해도 검출하지 못한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 desktop platform shell boundary다.
- [x] domain, usecases, adapters crate 내부 구현은 변경하지 않았다.
- [x] desktop shell은 `cabinet-platform` boundary만 직접 의존한다.
- [x] shell command는 DTO placeholder와 boundary routing만 가진다.
- [x] filesystem, environment, network 접근은 추가하지 않았다.
- [x] 전역 상태, 숨겨진 I/O, 암묵적 설정 접근을 추가하지 않았다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] 환경 값은 이번 태스크에서 읽지 않았다.
- [x] 최초 수신 이후에는 환경 값을 전역 상수처럼 사용하지 않았다.
- [x] 환경 값은 후속 bootstrap config 태스크에서 명시적으로 전달한다.
- [x] 프로세스 중간에 환경 설정 값을 삽입하거나 변경하지 않았다.
- [x] 런타임 중간 재설정, 동적 환경 변경, 숨겨진 설정 조회를 추가하지 않았다.

## 7. Logging Requirements

### Product Log

- [x] 이번 태스크는 제품 런타임을 구현하지 않으므로 Product Log를 추가하지 않았다.
- [x] 사용자 영향, 핵심 상태 변화, 장애 원인 추적 로그는 후속 usecase/runtime 태스크에서 정의한다.
- [x] 민감 정보와 과도한 내부 상태를 기록하지 않았다.

### Field Debug Log

- [x] 이번 태스크는 Field Debug Log가 필요하지 않다.
- [x] 활성화 조건은 후속 logging foundation 태스크에서 정의한다.
- [x] 민감 정보 마스킹 기준을 위반하는 코드를 추가하지 않았다.
- [x] 보존 범위와 사용 범위는 후속 logging foundation 태스크에서 제한한다.

### Development Log

- [x] 이번 태스크는 런타임 Development Log 구현을 추가하지 않았다.
- [x] 검증 결과는 Completion Report에 기록했다.
- [x] 프로덕션 기본 동작에 포함되는 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신 구현이 필요하지 않다.
- [x] 복잡한 내부 흐름을 암묵적 플래그 조합으로 관리하지 않았다.
- [x] 상태 목록은 후속 runtime/domain 태스크에서 정의한다.
- [x] 이벤트 목록은 후속 runtime/domain 태스크에서 정의한다.
- [x] 전이 조건은 후속 runtime/domain 태스크에서 정의한다.
- [x] 실패 상태와 종료 상태는 후속 상태머신 태스크에서 정의한다.
- [x] 상태 전이는 이번 태스크에서 구현하지 않았다.

## 9. TDD Plan

- [x] 실패하는 desktop boundary check를 먼저 작성했다.
- [x] 테스트 대상은 desktop shell manifest와 source boundary다.
- [x] 정상 케이스 테스트로 desktop shell smoke test를 작성했다.
- [x] 실패 케이스 테스트로 forbidden direct dependency pattern을 검사했다.
- [x] 경계값 테스트는 shell crate가 `cabinet-platform`만 직접 의존하는지 확인했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 설정 값 전달 방식 테스트는 이번 태스크 범위가 아니므로 설정 코드 없음으로 검증했다.
- [x] 로그 정책 검증은 런타임 로그 코드 없음으로 검증했다.
- [x] 상태 전이가 없으므로 상태 전이 테스트는 작성하지 않았다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 테스트 통과 후 구조를 정리했다.

## 10. Implementation Checklist

- [x] desktop boundary check script를 먼저 작성했다.
- [x] src-tauri crate가 없어서 boundary check가 실패하는 것을 확인했다.
- [x] 최소 desktop shell crate와 command boundary placeholder를 작성했다.
- [x] 계층 간 의존성을 확인했다.
- [x] 외부 의존성이 경계 계층에 추가되지 않았는지 확인했다.
- [x] 설정 값 전달 방식이 변경되지 않았는지 확인했다.
- [x] 런타임 로그 코드를 추가하지 않았다.
- [x] 상태 관리 구현을 추가하지 않았다.
- [x] 중복과 구조 문제를 정리했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과했다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] desktop shell이 명시적 boundary를 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `apps/desktop/src-tauri` Rust shell crate를 workspace member로 추가했다.
  - `DesktopShellRequest`, `DesktopShellResponse`, `route_desktop_command` placeholder를 추가했다.
  - `apps/desktop/src-tauri/tauri.conf.json` placeholder를 추가했다.
  - `scripts/check_desktop_shell_boundaries.sh`를 추가했다.
  - `.tasks/phase-gates.md`의 Phase 1 상태를 `complete`로 갱신했다.
- [x] 생성하거나 수정한 파일:
  - `Cargo.toml`
  - `Cargo.lock`
  - `apps/desktop/src-tauri/Cargo.toml`
  - `apps/desktop/src-tauri/src/lib.rs`
  - `apps/desktop/src-tauri/tauri.conf.json`
  - `scripts/check_desktop_shell_boundaries.sh`
  - `.tasks/task003.md`
  - `.tasks/task004.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo tauri --version`: 실패. Tauri CLI 없음.
  - `tauri --version`: 실패. Tauri CLI 없음.
  - `sh scripts/check_desktop_shell_boundaries.sh`: 최초 실행은 `src-tauri` 없음으로 실패했고, 구현 후 `desktop shell boundaries ok`로 통과했다.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `npm run check:frontend-boundaries`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
- [x] 검증한 항목:
  - desktop shell crate는 `cabinet-platform`만 직접 의존한다.
  - desktop shell source에는 direct filesystem, environment, network, DB access가 없다.
  - desktop shell source에는 business/infrastructure rule 이름이 없다.
  - Phase 1의 core/frontend/desktop skeleton evidence가 존재한다.
- [x] 남은 위험 요소:
  - Tauri CLI와 Tauri runtime dependency는 아직 설치/빌드하지 않았다.
  - 실제 Tauri command macro와 app launch smoke는 후속 태스크가 필요하다.
  - desktop shell은 아직 placeholder boundary다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 Phase 2 `MVP-004 Bootstrap config object`로 넘어가야 한다.
  - Tauri CLI 설치와 real launch smoke는 dependency 설치가 필요한 별도 태스크로 다룬다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 runtime foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-004 Bootstrap config object`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 AppConfig, LocalPathsConfig, bootstrap env-read-once 검증으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task005.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성한다.
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

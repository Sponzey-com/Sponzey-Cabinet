# Task 012. Workspace Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-010 Workspace domain model`을 시작하는 것이다.
- [x] 이 태스크는 workspace의 identity, display name, logical path를 외부 storage와 분리된 pure domain value object로 정의한다.
- [x] 이 태스크 완료 후 프로젝트는 `Workspace`, `WorkspaceId`, `WorkspaceName`, `WorkspacePath`와 validation test를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 Phase 2 runtime foundation을 완료했다.
- [x] 이전 태스크 Task 011에서 logging foundation과 Phase 2 exit gate를 완료했다.
- [x] 이번 태스크는 Phase 3 domain core의 첫 작업이다.
- [x] 현재 확인된 제약 사항은 workspace path가 filesystem path가 아니라 사용자/도메인 logical path여야 한다는 점이다.

## 3. Scope

### Included

- [x] `cabinet-domain`에 workspace value object를 추가한다.
- [x] workspace name validation을 추가한다.
- [x] workspace logical path validation을 추가한다.

### Excluded

- [x] filesystem path resolver는 이번 태스크에서 구현하지 않았다.
- [x] workspace repository port는 후속 usecase/adapter 태스크로 넘긴다.
- [x] UI workspace selector는 후속 UI 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: workspace identity를 만들었다.
- [x] 입력: caller-provided stable string id.
- [x] 출력: `WorkspaceId`.
- [x] 성공 조건: empty id와 whitespace id는 거부된다.
- [x] 실패 조건: id가 filesystem path나 external DB id에 종속된다.

### Functional Unit 2

- [x] 구현한 기능: workspace display name을 만들었다.
- [x] 입력: user-facing name.
- [x] 출력: `WorkspaceName`.
- [x] 성공 조건: trim, empty rejection, length limit, control character rejection이 테스트된다.
- [x] 실패 조건: name validation이 UI 또는 adapter에 흩어진다.

### Functional Unit 3

- [x] 구현한 기능: workspace logical path와 aggregate를 만들었다.
- [x] 입력: logical path string, id, name.
- [x] 출력: `WorkspacePath`, `Workspace`.
- [x] 성공 조건: absolute filesystem path, traversal segment, empty segment가 거부된다.
- [x] 실패 조건: domain이 `std::fs`, platform path, config를 읽는다.

## 5. Architecture Notes

- [x] 변경되는 계층은 pure domain이다.
- [x] domain은 framework, filesystem, DB, network, env, logger에 의존하지 않는다.
- [x] workspace path는 storage path가 아니라 logical domain path다.
- [x] validation error는 stable domain error enum으로 표현한다.
- [x] domain type은 adapter schema와 UI state에 종속되지 않는다.

## 6. Configuration Rules

- [x] domain은 config object를 직접 읽지 않는다.
- [x] process environment를 읽지 않는다.
- [x] validation policy는 type 내부 상수로 표현한다.
- [x] runtime 중간 설정 변경 API를 만들지 않는다.
- [x] 테스트는 외부 파일이나 환경 값 없이 실행한다.

## 7. Logging Requirements

### Product Log

- [x] domain은 Product Logger를 호출하지 않는다.
- [x] domain error는 후속 usecase가 Product Log error code로 변환할 수 있도록 stable enum으로 둔다.
- [x] 문서 본문, 첨부 내용, secret, raw path를 domain log payload로 만들지 않는다.

### Field Debug Log

- [x] domain은 Field Debug Logger를 호출하지 않는다.
- [x] workspace validation detail은 테스트 assertion으로만 검증한다.
- [x] runtime diagnostic은 후속 usecase/logging 태스크에서 처리한다.

### Development Log

- [x] domain은 Development Logger를 호출하지 않는다.
- [x] 테스트용 출력 코드를 추가하지 않는다.
- [x] 프로덕션 기본 동작에 포함되는 개발용 로그 코드를 추가하지 않는다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신을 추가하지 않는다.
- [x] workspace lifecycle은 후속 domain/usecase 태스크에서 필요할 때 정의한다.
- [x] boolean flag 조합으로 workspace 절차를 관리하지 않는다.

## 9. TDD Plan

- [x] 실패하는 workspace id validation test를 먼저 작성했다.
- [x] 실패하는 workspace name validation test를 먼저 작성했다.
- [x] 실패하는 workspace path validation test를 먼저 작성했다.
- [x] 실패하는 workspace aggregate construction test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 구현 후 중복과 구조 문제를 정리했다.

## 10. Implementation Checklist

- [x] domain tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] workspace module을 작성했다.
- [x] `cabinet-domain`에서 module을 공개했다.
- [x] domain boundary check가 계속 통과하는지 확인했다.
- [x] domain에 filesystem/env/network/logger 접근이 없는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] workspace domain model이 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 domain에 직접 포함되지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 가능한 한 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-domain::workspace` 모듈을 추가했다.
  - `Workspace`, `WorkspaceId`, `WorkspaceName`, `WorkspacePath`, `WorkspaceError`를 추가했다.
  - workspace id/name/path validation tests를 추가했다.
  - domain source I/O/framework/logger 금지 경계 검사 `scripts/check_domain_boundaries.sh`를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/workspace.rs`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-domain/tests/workspace_tests.rs`
  - `scripts/check_domain_boundaries.sh`
  - `.tasks/task011.md`
  - `.tasks/task012.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain workspace`: 최초 실행은 `cabinet_domain::workspace` 없음으로 실패했고, 구현 후 4개 workspace 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - workspace id는 empty/whitespace 값을 거부한다.
  - workspace name은 trim, empty rejection, 80자 제한, control character rejection을 수행한다.
  - workspace path는 logical path이며 absolute filesystem path, empty segment, traversal segment를 거부한다.
  - workspace aggregate는 id/name/path를 명시적으로 받는다.
  - domain source에는 filesystem/env/network/logger/framework 접근이 없다.
- [x] 남은 위험 요소:
  - workspace repository/usecase는 아직 없다.
  - workspace logical path normalization 정책은 현재 최소 수준이며, rename/move 요구가 나오면 확장해야 한다.
  - workspace lifecycle state는 아직 정의하지 않았다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-011 Document identity and metadata`를 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 document identity/body/lifecycle, asset/version/link domain, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-011 Document identity and metadata`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 document id/title/path/slug와 metadata validation으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task013.md`다.
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

# Task 003. Frontend Workspace Skeleton과 Client Boundary Check

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 1의 `MVP-002 Frontend workspace scaffold`를 시작하는 것이다.
- [x] 이 태스크는 Web과 desktop shell이 공유할 UI/editor/client package boundary를 만든다.
- [x] 이 태스크 완료 후 프로젝트는 package workspace skeleton과 frontend boundary 정적 검증을 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 Rust core workspace skeleton을 가진 상태다.
- [x] 이전 태스크 Task 002에서 `cabinet-domain`, `cabinet-ports`, `cabinet-usecases`, `cabinet-core`, `cabinet-adapters`, `cabinet-platform` crate skeleton과 Rust boundary check를 완료했다.
- [x] 이번 태스크는 Phase 1의 다음 작업인 shared frontend package boundary를 만들기 위해 시작했다.
- [x] 현재 확인된 제약 사항은 npm install과 외부 package 다운로드를 수행하지 않았다는 점이다.

## 3. Scope

### Included

- [x] root package workspace manifest를 생성한다.
- [x] `packages/client-core`, `packages/ui`, `packages/editor`, `apps/web`, `apps/desktop` skeleton을 생성한다.
- [x] frontend boundary check script를 생성하고 실행한다.

### Excluded

- [x] npm install과 외부 package 다운로드는 이번 태스크에서 수행하지 않는다.
- [x] React render test와 CodeMirror mount test는 이번 태스크에서 수행하지 않는다.
- [x] Tauri shell 구현은 후속 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: frontend workspace manifest를 만들었다.
- [x] 입력: `.tasks/plan.md` Phase 1, `PROJECT.md` 기술 방향.
- [x] 출력: root `package.json`과 workspace package manifests.
- [x] 성공 조건: workspace package 목록이 Web, desktop, ui, editor, client-core 경계를 표현한다.
- [x] 실패 조건: app package만 있고 shared package 경계가 없다.

### Functional Unit 2

- [x] 구현한 기능: client-core/ui/editor skeleton을 만들었다.
- [x] 입력: UI와 editor는 domain rule을 직접 구현하지 않는다는 AGENTS 기준.
- [x] 출력: 각 package의 `src/index.ts`.
- [x] 성공 조건: UI와 editor package가 client-core boundary만 사용한다.
- [x] 실패 조건: UI 또는 editor가 Rust domain, filesystem, environment, platform SDK를 직접 참조한다.

### Functional Unit 3

- [x] 구현한 기능: frontend architecture boundary check를 만들었다.
- [x] 입력: package manifests와 source files.
- [x] 출력: `scripts/check_frontend_boundaries.mjs`.
- [x] 성공 조건: 금지 import와 package boundary 위반을 정적 검사한다.
- [x] 실패 조건: 금지 import가 있어도 검증하지 못한다.

## 5. Architecture Notes

- [x] 변경되는 계층은 client workspace skeleton, UI package, editor package, client-core package, app shell placeholder다.
- [x] 도메인, 유스케이스, 어댑터 Rust crate는 변경하지 않았다.
- [x] UI package는 domain rule을 직접 구현하지 않는다.
- [x] editor package는 editor event를 document operation으로 변환하는 경계만 가진다.
- [x] apps/web과 apps/desktop은 shared package를 조합하는 app boundary다.
- [x] 전역 상태, 숨겨진 I/O, 암묵적 설정 접근을 추가하지 않았다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] 환경 값은 이번 태스크에서 읽지 않았다.
- [x] 최초 수신 이후에는 환경 값을 전역 상수처럼 사용하지 않았다.
- [x] 환경 값은 후속 runtime/client config 태스크에서 명시적으로 전달한다.
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
- [x] 상태 목록은 후속 domain/runtime 태스크에서 정의한다.
- [x] 이벤트 목록은 후속 domain/runtime 태스크에서 정의한다.
- [x] 전이 조건은 후속 domain/runtime 태스크에서 정의한다.
- [x] 실패 상태와 종료 상태는 후속 상태머신 태스크에서 정의한다.
- [x] 상태 전이는 이번 태스크에서 구현하지 않았다.

## 9. TDD Plan

- [x] 실패하는 frontend boundary check를 먼저 작성했다.
- [x] 테스트 대상은 package workspace skeleton과 source import boundary다.
- [x] 정상 케이스 테스트로 required package manifests와 source files를 검사한다.
- [x] 실패 케이스 테스트로 forbidden import pattern을 검사한다.
- [x] 경계값 테스트는 package naming과 workspace 목록 확인으로 수행했다.
- [x] 외부 의존성은 설치하지 않았다.
- [x] 설정 값 전달 방식 테스트는 이번 태스크 범위가 아니므로 설정 코드 없음으로 검증했다.
- [x] 로그 정책 검증은 런타임 로그 코드 없음으로 검증했다.
- [x] 상태 전이가 없으므로 상태 전이 테스트는 작성하지 않았다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 테스트 통과 후 구조를 정리했다.

## 10. Implementation Checklist

- [x] frontend boundary check script를 먼저 작성했다.
- [x] package workspace가 없어서 boundary check가 실패하는 것을 확인했다.
- [x] 최소 package workspace와 source skeleton을 작성했다.
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
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다. 이번 태스크는 Rust domain crate를 변경하지 않았다.
- [x] client packages가 명시적 boundary를 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 분리되었다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - root `package.json` workspace manifest를 생성했다.
  - `packages/client-core`, `packages/ui`, `packages/editor`, `apps/web`, `apps/desktop` package skeleton을 생성했다.
  - React와 CodeMirror는 peer/dependency manifest 경계에 선언하되 설치하지 않았다.
  - `scripts/check_frontend_boundaries.mjs`를 추가했다.
  - `.tasks/phase-gates.md`에 frontend skeleton evidence를 반영했다.
- [x] 생성하거나 수정한 파일:
  - `package.json`
  - `packages/client-core/package.json`
  - `packages/client-core/src/index.ts`
  - `packages/ui/package.json`
  - `packages/ui/src/index.ts`
  - `packages/editor/package.json`
  - `packages/editor/src/index.ts`
  - `apps/web/package.json`
  - `apps/web/src/index.ts`
  - `apps/desktop/package.json`
  - `apps/desktop/src/index.ts`
  - `scripts/check_frontend_boundaries.mjs`
  - `.tasks/task002.md`
  - `.tasks/task003.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과:
  - `node scripts/check_frontend_boundaries.mjs`: 최초 실행은 `package.json` 없음으로 실패했고, 구현 후 `frontend boundaries ok`로 통과했다.
  - `npm run check:frontend-boundaries`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
- [x] 검증한 항목:
  - root package workspaces는 `packages/*`, `apps/*`를 포함한다.
  - `client-core`는 dependency가 없다.
  - `ui`와 `editor`는 `client-core` boundary를 사용한다.
  - source files에 filesystem, process env, Tauri SDK, Rust crate 직접 참조가 없다.
  - 런타임 설정, 로그, 상태머신 구현을 추가하지 않았다.
- [x] 남은 위험 요소:
  - npm dependencies는 아직 설치하지 않았다.
  - React render test와 CodeMirror mount test는 아직 없다.
  - desktop package는 아직 Tauri shell이 아니다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-003 Tauri desktop shell scaffold`를 다뤄야 한다.
  - 외부 dependency 설치가 필요한 경우 별도 검증과 승인 경로를 사용해야 한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 desktop shell, runtime foundation, domain model, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-003 Tauri desktop shell scaffold`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 desktop shell manifest, command boundary placeholder, static validation으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task004.md`다.
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

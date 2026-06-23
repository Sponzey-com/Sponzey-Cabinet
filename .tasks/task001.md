# Task 001. Phase 0 작업 루프와 품질 게이트 기반 구축

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md`의 Phase 0을 실제 작업 루프에서 실행할 수 있게 만드는 것이다.
- [x] 이 태스크는 `plan.md`의 Phase Gate Rules, Decision Records, TDD Strategy, Review Checklist를 실제 추적 파일로 연결한다.
- [x] 이 태스크 완료 후 프로젝트는 다음 태스크를 정의할 수 있는 `.tasks` 운영 구조를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 문서 중심 상태다. 확인된 추적 파일은 `.tasks/plan.md`, `AGENTS.md`, `ROADMAP.md`, `RESEARCH.md`, `PROJECT.md`다.
- [x] 이전 태스크는 없다. 이 파일이 첫 번째 태스크다.
- [x] 이번 태스크는 Phase 0의 작업 루프와 품질 게이트를 실제 파일로 고정하기 위해 시작했다.
- [x] 현재 확인된 제약 사항은 제품 코드, Rust workspace, frontend workspace, desktop shell이 아직 없다는 점이다.

## 3. Scope

### Included

- [x] `task001.md`를 필수 구조에 맞게 생성한다.
- [x] phase entry/exit gate를 추적하는 `.tasks/phase-gates.md`를 생성한다.
- [x] decision record 운영 기준을 담은 `.tasks/decisions/README.md`를 생성한다.

### Excluded

- [x] Rust workspace scaffold는 이번 태스크에서 만들지 않는다.
- [x] frontend workspace scaffold는 이번 태스크에서 만들지 않는다.
- [x] 제품 기능, domain model, usecase, adapter 구현은 후속 태스크로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: 태스크 루프의 첫 실행 단위인 `task001.md`를 만들었다.
- [x] 입력: `.tasks/plan.md`, `AGENTS.md`, 현재 저장소 상태.
- [x] 출력: 체크박스 기반 실행/검증/완료 보고 구조를 가진 `.tasks/task001.md`.
- [x] 성공 조건: 태스크 문서가 필수 14개 섹션을 모두 포함한다.
- [x] 실패 조건: 필수 섹션 누락, 체크박스 누락, 검증 기준 누락.

### Functional Unit 2

- [x] 구현한 기능: Phase Gate 추적 파일을 만들었다.
- [x] 입력: `.tasks/plan.md`의 Phase Gate Rules.
- [x] 출력: `.tasks/phase-gates.md`.
- [x] 성공 조건: Phase 0부터 Phase 8까지 entry gate, exit gate, status, evidence를 추적할 수 있다.
- [x] 실패 조건: phase 선후관계가 불명확하거나 검증 증거 칸이 없다.

### Functional Unit 3

- [x] 구현한 기능: Decision Record 기반을 만들었다.
- [x] 입력: `.tasks/plan.md`의 Decision Records 섹션.
- [x] 출력: `.tasks/decisions/README.md`.
- [x] 성공 조건: decision record 형식, 필수 결정 목록, 거부 기준, 리뷰 기준이 기록된다.
- [x] 실패 조건: 설정, 로그, 상태머신, 테스트 영향 기록 기준이 빠져 있다.

## 5. Architecture Notes

- [x] 변경되는 계층은 documentation/task operation 계층이다.
- [x] 도메인, 유스케이스, 어댑터, 인프라 코드는 이번 태스크에서 변경하지 않았다.
- [x] 의존성 방향을 코드로 변경하지 않았다.
- [x] 외부 시스템 접근을 추가하지 않았다.
- [x] 필요한 인터페이스, 포트, 어댑터는 정의하지 않았다.
- [x] 전역 상태, 숨겨진 I/O, 암묵적 설정 접근을 추가하지 않았다.

## 6. Configuration Rules

- [x] 외부 설정 파일 의존을 추가하지 않았다.
- [x] 환경 값은 이번 태스크에서 읽지 않았다.
- [x] 최초 수신 이후 환경 값을 전역 상수처럼 사용하는 코드를 만들지 않았다.
- [x] 환경 값 전달 구조를 구현하지 않았다.
- [x] 프로세스 중간에 환경 설정 값을 삽입하거나 변경하지 않았다.
- [x] 런타임 중간 재설정, 동적 환경 변경, 숨겨진 설정 조회를 추가하지 않았다.

## 7. Logging Requirements

### Product Log

- [x] 이번 태스크는 제품 런타임을 구현하지 않으므로 Product Log를 추가하지 않았다.
- [x] 사용자 영향, 핵심 상태 변화, 장애 원인 추적 로그는 후속 구현 태스크에서 정의한다.
- [x] 민감 정보와 과도한 내부 상태를 기록하지 않았다.

### Field Debug Log

- [x] 이번 태스크는 현장 확인용 디버그 로그가 필요하지 않다.
- [x] Field Debug Log 활성화 조건은 후속 logging foundation 태스크에서 정의한다.
- [x] 민감 정보 마스킹 기준은 `.tasks/decisions/README.md`에 decision review 기준으로 남겼다.
- [x] 보존 범위와 사용 범위는 후속 logging foundation 태스크에서 제한한다.

### Development Log

- [x] 이번 태스크는 개발 로그 구현을 추가하지 않았다.
- [x] 검증 결과는 이 문서의 Completion Report에 기록했다.
- [x] 프로덕션 기본 동작에 포함되는 로그 코드를 추가하지 않았다.

## 8. State Machine Requirements

- [x] 이번 태스크는 런타임 상태머신 구현이 필요하지 않다.
- [x] 복잡한 내부 흐름을 암묵적 플래그 조합으로 관리하는 코드를 추가하지 않았다.
- [x] FirstRun, Migration, DocumentLifecycle 등 상태머신 대상은 `.tasks/phase-gates.md`와 `.tasks/decisions/README.md`에 추적 기준으로 연결했다.
- [x] 상태 전이 구현은 후속 Phase 2와 Phase 3 태스크로 넘긴다.

## 9. TDD Plan

- [x] 실패하는 테스트를 먼저 작성할 제품 코드가 없는 문서 태스크임을 명시했다.
- [x] 테스트 대상 유스케이스는 없다.
- [x] 정상 케이스 검증은 `rg`와 `git status`로 수행했다.
- [x] 실패 케이스 검증은 필수 섹션/금지 표현 검색으로 수행했다.
- [x] 경계값 테스트는 task 번호 zero-padding과 파일명 규칙 확인으로 대체했다.
- [x] 외부 의존성은 추가하지 않았다.
- [x] 설정 값 전달 방식 테스트는 이번 태스크 범위가 아니므로 설정 변경 없음으로 검증했다.
- [x] 로그 정책 검증은 로그 코드가 추가되지 않았음을 확인했다.
- [x] 상태 전이 구현이 없으므로 상태 전이 테스트를 만들지 않았다.
- [x] 테스트를 통과하는 최소 구현은 문서와 추적 파일 생성이었다.
- [x] 테스트 통과 후 체크박스와 Completion Report를 갱신했다.

## 10. Implementation Checklist

- [x] 테스트 파일을 먼저 작성할 제품 코드가 없는지 확인했다.
- [x] 실패하는 테스트 대신 필수 문서 구조 검증 기준을 먼저 정의했다.
- [x] 최소 구현으로 `.tasks/task001.md`, `.tasks/phase-gates.md`, `.tasks/decisions/README.md`를 작성했다.
- [x] 계층 간 의존성이 변경되지 않았는지 확인했다.
- [x] 외부 의존성이 경계 계층에 추가되지 않았는지 확인했다.
- [x] 설정 값 전달 방식이 변경되지 않았는지 확인했다.
- [x] 런타임 로그 코드가 추가되지 않았는지 확인했다.
- [x] 상태 관리 구현이 추가되지 않았는지 확인했다.
- [x] 중복과 구조 문제를 정리했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 문서 검증 명령이 모두 통과했다.
- [x] 실패 테스트가 먼저 작성될 제품 코드가 없음을 명시했다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다. 이번 태스크는 제품 코드를 만들지 않았다.
- [x] 유스케이스가 명시적 입력과 출력을 가지는 제품 코드 변경이 없다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다. 이번 태스크는 환경 값을 읽지 않았다.
- [x] 외부 환경 값이 전역 상수처럼 사용되지 않는다.
- [x] 로그가 Product Log, Field Debug Log, Development Log 기준을 위반하지 않는다. 이번 태스크는 런타임 로그 코드를 추가하지 않았다.
- [x] 개발용 로그가 프로덕션 기본 동작에 포함되지 않는다.
- [x] 복잡한 흐름이 플래그 조합으로 새로 구현되지 않았다.
- [x] 리팩터링과 기능 변경이 섞이지 않았다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:

  - `.tasks/task001.md`를 생성하고 실행 결과를 기록했다.
  - `.tasks/phase-gates.md`를 생성해 Phase 0부터 Phase 8까지 entry/exit gate와 evidence를 추적하게 했다.
  - `.tasks/decisions/README.md`를 생성해 decision record 형식, 필수 결정 목록, 거부 기준을 기록했다.
  - `.gitignore`를 조정해 `.tasks/plan.md`, `.tasks/task*.md`, `.tasks/phase-gates.md`, `.tasks/decisions/**`가 추적 가능하도록 했다.
- [x] 생성하거나 수정한 파일:

  - `.tasks/task001.md`
  - `.tasks/phase-gates.md`
  - `.tasks/decisions/README.md`
  - `.gitignore`
- [x] 실행한 테스트 명령과 결과:

  - `rg -n "^## [0-9]+\\." .tasks/task001.md`: 14개 필수 섹션 확인.
  - 금지 표현 검색 명령: 결과 없음.
  - `rg -n "Phase 0|Phase 8|Local metadata store|Internal version store|Product/Field/Development|Decision Record Template" .tasks/phase-gates.md .tasks/decisions/README.md`: 핵심 추적 항목 확인.
  - `git status --short -- .tasks .gitignore`: `.tasks/`와 `.gitignore` 변경 확인.
- [x] 검증한 항목:

  - task 번호는 `task001.md` zero-padding 규칙을 따른다.
  - task 문서는 필수 14개 섹션을 가진다.
  - phase gate와 decision record 추적 파일이 존재한다.
  - 제품 코드, 설정 코드, 로그 코드, 상태머신 코드를 추가하지 않았다.
- [x] 남은 위험 요소:

  - 실제 제품 코드 scaffold가 아직 없다.
  - Phase 1 skeleton을 시작하기 전에 concrete repository layout과 Rust workspace 구성 방식을 확정해야 한다.
  - Product/Field/Development log event naming decision은 Phase 2 전에 작성해야 한다.
- [x] 후속 태스크에서 이어받아야 할 내용:

  - 다음 태스크는 Phase 1의 `MVP-001 Core workspace scaffold`를 시작해야 한다.
  - 다음 태스크는 domain/usecase/port/adapter skeleton과 최소 테스트 runner를 포함해야 한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다. MVP 제품 코드는 아직 없다.
- [x] 남은 목표는 Phase 1부터 Phase 8까지의 product scaffold, runtime foundation, domain, adapter, usecase, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-001 Core workspace scaffold`다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 Rust/core skeleton, dependency boundary, smoke test로 제한해야 한다.
- [x] 다음 태스크는 테스트와 검증을 포함해야 한다.
- [x] 다음 태스크는 `AGENTS.md` 원칙과 충돌하지 않는다.
- [x] 다음 태스크 파일명은 `.tasks/task002.md`가 되어야 한다.
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
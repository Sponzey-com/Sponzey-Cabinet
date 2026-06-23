# Task 015. Document Lifecycle State Machine

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-013 Document lifecycle state machine`을 구현하는 것이다.
- [x] 이 태스크는 문서 lifecycle을 boolean flag 조합이 아니라 명시적 상태, 이벤트, 전이 결과로 표현한다.
- [x] 이 태스크 완료 후 프로젝트는 document lifecycle valid/invalid transition tests를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 document metadata와 body domain model을 가진다.
- [x] 이전 태스크 Task 014에서 `DocumentBody`를 완료했다.
- [x] 이번 태스크는 document state transition foundation을 추가했다.
- [x] 현재 확인된 제약 사항은 상태 전이가 storage, UI, logger, editor state에 의존하면 안 된다는 점이다.

## 3. Scope

### Included

- [x] `DocumentLifecycleState`를 추가한다.
- [x] `DocumentLifecycleEvent`를 추가한다.
- [x] valid/invalid transition function을 추가한다.

### Excluded

- [x] repository persistence는 후속 phase로 넘긴다.
- [x] usecase command orchestration은 후속 phase로 넘긴다.
- [x] Product Log emission은 후속 usecase 태스크로 넘긴다.

## 4. Functional Units

- [x] 구현한 기능: lifecycle state/event enum을 만들었다.
- [x] 구현한 기능: create/save/edit/archive/delete/restore 정상 전이를 만들었다.
- [x] 구현한 기능: invalid transition error를 만들었다.

## 5. Architecture Notes

- [x] 변경되는 계층은 pure domain이다.
- [x] transition function은 pure function으로 유지한다.
- [x] domain은 filesystem, DB, network, env, logger, editor state에 의존하지 않는다.

## 6. Configuration Rules

- [x] domain은 config object를 직접 읽지 않는다.
- [x] process environment를 읽지 않는다.
- [x] transition policy는 type 내부 규칙으로만 표현한다.

## 7. Logging Requirements

- [x] domain은 logger를 호출하지 않는다.
- [x] transition result는 후속 usecase가 log event로 변환할 수 있는 stable error를 제공한다.

## 8. State Machine Requirements

- [x] states: `Draft`, `Saved`, `Editing`, `Archived`, `Deleted`, `Restored`.
- [x] events: `Create`, `Save`, `StartEdit`, `Archive`, `Delete`, `Restore`.
- [x] invalid transition은 state와 event를 포함한 domain error로 실패한다.
- [x] 상태 전이는 unit test로 검증한다.

## 9. TDD Plan

- [x] 실패하는 valid transition test를 먼저 작성했다.
- [x] 실패하는 invalid transition test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 추가하지 않았다.

## 10. Implementation Checklist

- [x] domain tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] lifecycle type과 transition function을 작성했다.
- [x] domain boundary check가 계속 통과하는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] 상태 전이가 flag 조합 없이 enum으로 표현된다.

## 12. Completion Report

- [x] 수행한 변경 사항:
  - `DocumentLifecycleState`, `DocumentLifecycleEvent`, `DocumentLifecycleTransition`을 추가했다.
  - `transition_document_lifecycle` pure transition function을 추가했다.
  - `DocumentError::InvalidLifecycleTransition`을 추가했다.
  - valid lifecycle flow와 invalid transition tests를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/document.rs`
  - `crates/cabinet-domain/tests/document_lifecycle_tests.rs`
  - `.tasks/task014.md`
  - `.tasks/task015.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain lifecycle`: 최초 실행은 lifecycle 타입/함수/error 없음으로 실패했고, 구현 후 2개 lifecycle 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo fmt --all --check`: 포맷 적용 후 통과.
  - `cargo test --workspace`: 통과.
- [x] 검증한 항목:
  - create/save/edit/archive/delete/restore 정상 전이가 명시적 enum으로 표현된다.
  - invalid transition은 state와 event를 포함한 domain error로 실패한다.
  - lifecycle transition은 storage/UI/logger/editor state에 의존하지 않는다.
- [x] 남은 위험 요소:
  - usecase orchestration과 persistence는 아직 없다.
  - restore preview/current-history 연결은 version domain과 usecase에서 필요하다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-014 Asset domain model`을 시작한다.

## 13. Next Task Decision Hook

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 다음 우선순위는 `MVP-014 Asset domain model`이다.
- [x] 다음 태스크 파일명은 `.tasks/task016.md`다.
- [x] 다음 태스크를 `taskXXX.md`로 생성했다.
- [x] 다음 태스크 생성을 완료한 뒤 즉시 실행을 시작한다.

## 14. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.
- [ ] 외부 정보, 권한, 비밀값, 접근 권한이 없어 진행할 수 없다.
- [ ] `AGENTS.md` 원칙과 충돌하는 요구사항이 발견되었다.
- [ ] 테스트 또는 검증 환경이 없어 완료 여부를 판단할 수 없다.
- [ ] 코드베이스 구조가 계획과 크게 달라 태스크 재설계가 필요하다.
- [ ] 사용자 결정이 필요한 아키텍처 선택지가 발생했다.

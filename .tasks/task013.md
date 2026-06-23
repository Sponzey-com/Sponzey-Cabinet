# Task 013. Document Identity and Metadata Domain Model

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 3의 `MVP-011 Document identity and metadata`를 구현하는 것이다.
- [x] 이 태스크는 문서의 identity, title, logical path, slug, metadata를 storage/editor와 분리된 pure domain model로 정의한다.
- [x] 이 태스크 완료 후 프로젝트는 `DocumentId`, `DocumentTitle`, `DocumentPath`, `DocumentSlug`, `DocumentMetadata`와 validation test를 가진다.

## 2. Current Context

- [x] 현재 코드베이스는 workspace domain model을 가진다.
- [x] 이전 태스크 Task 012에서 Phase 3 workspace value object를 완료했다.
- [x] 이번 태스크는 문서 identity와 path/title/slug validation을 시작했다.
- [x] 현재 확인된 제약 사항은 document path가 filesystem path가 아니라 workspace 내부 logical markdown path여야 한다는 점이다.

## 3. Scope

### Included

- [x] document id/title/path/slug value object를 추가한다.
- [x] title 기반 slug normalization을 추가한다.
- [x] document metadata aggregate와 title 변경 시 identity/path 유지 테스트를 추가한다.

### Excluded

- [x] document body는 후속 태스크로 넘긴다.
- [x] document lifecycle state machine은 후속 태스크로 넘긴다.
- [x] repository/storage adapter는 후속 phase로 넘긴다.

## 4. Functional Units

이번 태스크는 기능 3개 단위로만 구성했다.

### Functional Unit 1

- [x] 구현한 기능: document identity/title value object를 만들었다.
- [x] 입력: stable id string, user-facing title.
- [x] 출력: `DocumentId`, `DocumentTitle`.
- [x] 성공 조건: empty/whitespace/control character/length limit이 검증된다.
- [x] 실패 조건: validation이 UI 또는 adapter에 흩어진다.

### Functional Unit 2

- [x] 구현한 기능: document logical path와 slug를 만들었다.
- [x] 입력: logical markdown path와 title.
- [x] 출력: `DocumentPath`, `DocumentSlug`.
- [x] 성공 조건: absolute path, traversal, empty segment, non-markdown extension을 거부하고 slug를 안정적으로 정규화한다.
- [x] 실패 조건: filesystem path나 editor route가 domain path로 섞인다.

### Functional Unit 3

- [x] 구현한 기능: document metadata aggregate를 만들었다.
- [x] 입력: id, title, path.
- [x] 출력: `DocumentMetadata`.
- [x] 성공 조건: title 변경 시 identity/path는 유지되고 slug만 title 기반으로 변경된다.
- [x] 실패 조건: rename이 document id를 바꾸거나 version/history 개념과 섞인다.

## 5. Architecture Notes

- [x] 변경되는 계층은 pure domain이다.
- [x] domain은 framework, filesystem, DB, network, env, logger, editor state에 의존하지 않는다.
- [x] document path는 storage path가 아니라 workspace logical markdown path다.
- [x] document metadata는 current snapshot이나 history entry가 아니다.
- [x] Git 또는 version store 개념을 사용자-facing domain 이름으로 노출하지 않는다.

## 6. Configuration Rules

- [x] domain은 config object를 직접 읽지 않는다.
- [x] process environment를 읽지 않는다.
- [x] validation policy는 type 내부 상수로만 표현한다.
- [x] runtime 중간 설정 변경 API를 만들지 않는다.
- [x] 테스트는 외부 파일이나 환경 값 없이 실행한다.

## 7. Logging Requirements

### Product Log

- [x] domain은 Product Logger를 호출하지 않는다.
- [x] domain error는 후속 usecase가 Product Log error code로 변환할 수 있도록 stable enum으로 둔다.
- [x] 문서 본문, 첨부 내용, secret, raw path를 domain log payload로 만들지 않는다.

### Field Debug Log

- [x] domain은 Field Debug Logger를 호출하지 않는다.
- [x] validation detail은 테스트 assertion으로만 검증한다.

### Development Log

- [x] domain은 Development Logger를 호출하지 않는다.
- [x] 테스트용 출력 코드를 추가하지 않는다.

## 8. State Machine Requirements

- [x] 이번 태스크는 상태머신을 추가하지 않는다.
- [x] document lifecycle state machine은 후속 태스크로 넘긴다.
- [x] boolean flag 조합으로 document 절차를 관리하지 않는다.

## 9. TDD Plan

- [x] 실패하는 document id/title validation test를 먼저 작성했다.
- [x] 실패하는 document path validation test를 먼저 작성했다.
- [x] 실패하는 slug normalization test를 먼저 작성했다.
- [x] 실패하는 metadata rename separation test를 먼저 작성했다.
- [x] 테스트를 통과하는 최소 구현만 작성했다.
- [x] 외부 의존성은 추가하지 않았다.

## 10. Implementation Checklist

- [x] domain tests를 먼저 작성했다.
- [x] 구현 전 테스트 실패를 확인했다.
- [x] document module을 작성했다.
- [x] `cabinet-domain`에서 module을 공개했다.
- [x] domain boundary check가 계속 통과하는지 확인했다.
- [x] 모든 관련 검증 명령을 실행했다.

## 11. Validation Checklist

- [x] 기능 요구사항이 충족되었다.
- [x] 테스트가 모두 통과한다.
- [x] 실패 테스트가 먼저 작성되었다.
- [x] 도메인 계층이 외부 프레임워크에 의존하지 않는다.
- [x] document metadata가 명시적 입력과 출력을 가진다.
- [x] 외부 환경 값이 런타임 중간에 재조회되지 않는다.
- [x] 로그가 domain에 직접 포함되지 않는다.
- [x] current/history/version 개념이 metadata와 섞이지 않았다.

## 12. Completion Report

태스크 완료 결과는 다음이다.

- [x] 수행한 변경 사항:
  - `cabinet-domain::document` 모듈을 추가했다.
  - `DocumentId`, `DocumentTitle`, `DocumentPath`, `DocumentSlug`, `DocumentMetadata`, `DocumentError`를 추가했다.
  - document id/title/path validation tests를 추가했다.
  - title 기반 slug normalization과 metadata title 변경 테스트를 추가했다.
- [x] 생성하거나 수정한 파일:
  - `crates/cabinet-domain/src/document.rs`
  - `crates/cabinet-domain/src/lib.rs`
  - `crates/cabinet-domain/tests/document_metadata_tests.rs`
  - `.tasks/task012.md`
  - `.tasks/task013.md`
- [x] 실행한 테스트 명령과 결과:
  - `cargo test -p cabinet-domain document`: 최초 실행은 `cabinet_domain::document` 없음으로 실패했고, 구현 후 4개 document metadata 테스트가 통과했다.
  - `sh scripts/check_domain_boundaries.sh`: 통과.
  - `cargo test --workspace`: 통과.
  - `cargo fmt --all --check`: 통과.
  - `sh scripts/check_architecture_boundaries.sh`: 통과.
- [x] 검증한 항목:
  - document id는 empty/whitespace 값을 거부한다.
  - document title은 trim, control character rejection, 120자 제한을 수행한다.
  - document path는 workspace logical markdown path이며 absolute/traversal/non-markdown path를 거부한다.
  - slug는 title에서 안정적으로 정규화된다.
  - title 변경 시 document id와 path는 유지되고 slug는 새 title 기준으로 변경된다.
- [x] 남은 위험 요소:
  - document body model은 아직 없다.
  - document lifecycle state machine은 아직 없다.
  - current snapshot/history version 구분은 후속 version domain에서 다룬다.
- [x] 후속 태스크에서 이어받아야 할 내용:
  - 다음 태스크는 `MVP-012 Document body model`을 시작한다.

## 13. Next Task Decision Hook

이 태스크 완료 후 판단 결과는 다음이다.

- [x] `plan.md`의 최종 목표에 도달했는지 확인했다.
- [x] 최종 목표에는 도달하지 못했다.
- [x] 남은 목표는 document body/lifecycle, asset/version/link domain, adapters, usecases, UI, release gate다.
- [x] 남은 목표 중 가장 우선순위가 높은 작업은 `MVP-012 Document body model`이다.
- [x] 다음 태스크는 기능 2~3개 단위를 넘지 않도록 line ending normalization, size limit, Unicode preservation으로 제한한다.
- [x] 다음 태스크가 테스트와 검증을 포함하도록 정의한다.
- [x] 다음 태스크가 `AGENTS.md` 원칙과 충돌하지 않는지 확인했다.
- [x] 다음 태스크 파일명은 `.tasks/task014.md`다.
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

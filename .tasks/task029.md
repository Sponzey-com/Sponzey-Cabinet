# Task 029. CreateDocument Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-021 CreateDocument` usecase를 구현하는 것이다.
- [x] 이 태스크는 문서 생성 시 current snapshot 저장, version entry 생성, indexing event 발행을 usecase 경계에서 조율한다.
- [x] 이 태스크는 document body가 Product Log에 포함되지 않도록 검증한다.

## 2. Scope

- [x] `CreateDocumentInput`과 `CreateDocumentOutput`을 추가한다.
- [x] `CreateDocumentUsecase`를 추가한다.
- [x] explicit `DocumentBodyPolicy`를 생성자 인자로 받는다.
- [x] `DocumentRepository`에 current snapshot을 저장한다.
- [x] `VersionStore`에 initial version record를 저장한다.
- [x] search/link indexing을 위한 `DocumentChangeEventPublisher`를 정의한다.
- [x] document created와 usecase failed Product Log event를 기록한다.

## 3. TDD Plan

- [x] 실패하는 create document stores current and version test를 먼저 작성한다.
- [x] 실패하는 invalid body rejects before write test를 먼저 작성한다.
- [x] 실패하는 current repository failure skips version/event test를 먼저 작성한다.
- [x] 실패하는 product log excludes body test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 explicit input/output을 사용한다.
- [x] usecase는 domain과 port에만 의존한다.
- [x] usecase는 filesystem, env, concrete local adapter, parser/search implementation을 import하지 않는다.
- [x] indexing은 concrete index 호출이 아니라 domain-neutral event request로 발행한다.
- [x] Product Log event에는 document body, full title, path 원문을 넣지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `CreateDocumentUsecase`를 추가해 current snapshot 저장, initial version 저장, document change event 발행을 조율했다.
  - `DocumentBodyPolicy`를 생성자 인자로 명시적으로 받도록 했다.
  - Product Log event가 document id와 error code만 포함하고 body/title/path 원문을 제외하도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-usecases/src/document.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/create_document_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test create_document_tests --quiet`: initial fail, missing document usecase module
  - `cargo test -p cabinet-usecases --test create_document_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 030은 `GetCurrentDocument` usecase와 current/history 분리 테스트를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

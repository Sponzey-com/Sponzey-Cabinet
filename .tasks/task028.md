# Task 028. CreateWorkspace Usecase

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 5의 `MVP-020 CreateWorkspace` usecase를 구현하는 것이다.
- [x] 이 태스크는 workspace 생성 입력/출력, repository port, Product Log 요청을 명시적으로 정의한다.
- [x] 이 태스크는 usecase가 filesystem, environment, concrete adapter를 직접 참조하지 않도록 검증한다.

## 2. Scope

- [x] `WorkspaceRepository` port를 추가한다.
- [x] `CreateWorkspaceInput`과 `CreateWorkspaceOutput`을 추가한다.
- [x] `CreateWorkspaceUsecase`를 추가한다.
- [x] workspace id/name/path value object validation을 usecase 경계에서 수행한다.
- [x] duplicate workspace를 conflict error로 보고한다.
- [x] workspace created와 usecase failed Product Log event를 기록한다.

## 3. TDD Plan

- [x] 실패하는 workspace create persists workspace test를 먼저 작성한다.
- [x] 실패하는 duplicate workspace conflict test를 먼저 작성한다.
- [x] 실패하는 invalid input logs failure test를 먼저 작성한다.
- [x] 실패하는 product log excludes workspace path test를 먼저 작성한다.

## 4. Architecture Rules

- [x] usecase는 explicit input을 받고 explicit output을 반환한다.
- [x] usecase는 domain과 port에만 의존한다.
- [x] usecase는 filesystem, env, concrete local adapter를 import하지 않는다.
- [x] Product Log event에는 workspace path 원문을 넣지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `WorkspaceRepository` port와 `CreateWorkspaceUsecase`를 추가했다.
  - usecase가 명시적 input/output을 사용하고 fake repository/logger로 검증되도록 했다.
  - Product Log event는 workspace id와 error code만 담고 workspace path 원문을 포함하지 않도록 했다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/workspace_repository.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-usecases/src/workspace.rs`
  - `crates/cabinet-usecases/src/lib.rs`
  - `crates/cabinet-usecases/tests/create_workspace_tests.rs`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-usecases --test create_workspace_tests --quiet`: initial fail, missing port/usecase module
  - `cargo test -p cabinet-usecases --test create_workspace_tests --quiet`: pass
  - `cargo fmt --all --check`: pass
  - `cargo test --workspace --quiet`: pass
  - `sh scripts/check_architecture_boundaries.sh`: pass
  - `sh scripts/check_no_git_cli_dependency.sh`: pass
- [x] 다음 태스크를 결정한다.
  - Task 029는 `CreateDocument` usecase를 구현한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

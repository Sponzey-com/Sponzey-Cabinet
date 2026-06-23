# Task 041. MarkdownParser Port and Local Adapter

## 1. Task Purpose

- [x] 이 태스크의 목적은 `.tasks/plan.md` Phase 6의 `MVP-040 Markdown parser adapter`를 구현하는 것이다.
- [x] 이 태스크는 Markdown body에서 heading, wikilink, asset reference를 추출하는 parser port와 local adapter를 제공한다.

## 2. Scope

- [x] `MarkdownParser` port를 추가한다.
- [x] parsed heading, wikilink, asset reference output model을 정의한다.
- [x] local Markdown parser adapter를 추가한다.
- [x] heading 문법은 `#`부터 `######`까지 지원한다.
- [x] wikilink 문법은 `[[Target]]`와 `[[Target|Label]]`을 지원한다.
- [x] asset reference 문법은 `![[asset:<sha256>|Label]]`을 지원한다.
- [x] parser는 filesystem, network, env, UI, DB에 접근하지 않는다.

## 3. TDD Plan

- [x] 실패하는 parser output validation contract test를 먼저 작성한다.
- [x] 실패하는 local parser extracts headings wikilinks asset references test를 먼저 작성한다.
- [x] 실패하는 invalid asset reference is ignored without failing whole parse test를 먼저 작성한다.

## 4. Architecture Rules

- [x] parser port는 domain value object만 참조한다.
- [x] adapter는 port를 구현하고 concrete storage를 import하지 않는다.
- [x] parser는 document body 문자열만 입력으로 사용한다.
- [x] parser output은 UI DTO가 아니라 index/update usecase 입력으로 재사용 가능한 boundary model이다.
- [x] parser는 사용자에게 Git, commit, PR 개념을 노출하지 않는다.

## 5. Completion Report

- [x] 수행한 변경 사항을 요약한다.
  - `MarkdownParser` port와 parsed heading/wikilink/asset reference output model을 추가했다.
  - `LocalMarkdownParser` adapter를 추가했다.
  - heading, `[[Target]]`, `[[Target|Label]]`, `![[asset:<sha256>|Label]]` 추출을 구현했다.
  - invalid asset reference는 전체 parse 실패 없이 무시한다.
- [x] 생성하거나 수정한 파일을 기록한다.
  - `crates/cabinet-ports/src/markdown_parser.rs`
  - `crates/cabinet-ports/src/lib.rs`
  - `crates/cabinet-ports/tests/markdown_parser_contract_tests.rs`
  - `crates/cabinet-adapters/src/local_markdown_parser.rs`
  - `crates/cabinet-adapters/src/lib.rs`
  - `crates/cabinet-adapters/tests/local_markdown_parser_tests.rs`
  - `.tasks/task041.md`
  - `.tasks/phase-gates.md`
- [x] 실행한 테스트 명령과 결과를 기록한다.
  - `cargo test -p cabinet-ports --test markdown_parser_contract_tests --quiet` 통과
  - `cargo test -p cabinet-adapters --test local_markdown_parser_tests --quiet` 통과
  - `cargo fmt --all --check` 통과
  - `cargo test --workspace --quiet` 통과
  - `sh scripts/check_architecture_boundaries.sh` 통과
  - `sh scripts/check_no_git_cli_dependency.sh` 통과
- [x] 다음 태스크를 결정한다.
  - Task 042는 `LinkIndex` port를 구현한다.
  - LinkIndex는 backlinks, unresolved links, orphan query를 위한 projection 경계를 제공한다.

## 6. Stop Conditions

- [ ] `plan.md`의 최종 목표에 도달했다.
- [ ] 필수 요구사항이 불명확하여 더 이상 안전하게 진행할 수 없다.

# Markdown WYSIWYG Editor Library Research

작성일: 2026-07-23  
문서 목적: Sponzey Cabinet의 문서 편집기를 대체하거나 보강할 수 있는 Markdown WYSIWYG/Live Preview editor library를 조사하고, 현재 제품 요구조건에 맞는 채택 후보와 제외 후보를 정리한다.

## 1. Research Goal

현재 Cabinet의 자체 React WYSIWYG 구현은 Obsidian과 같은 자연스러운 Markdown Live Preview 경험을 제공하기에 부족하다. 이 문서는 다음 목표를 만족하는 외부 editor library 또는 editor framework를 찾기 위한 조사 기록이다.

- 개인용 로컬 macOS 데스크톱 앱에서 동작해야 한다.
- React/Tauri 기반 desktop UI에 통합 가능해야 한다.
- 저장 canonical form은 Markdown source여야 한다.
- 사용자는 기본 화면에서 Markdown marker를 직접 보지 않고 문서를 읽고 수정할 수 있어야 한다.
- `원문 편집`은 CodeMirror 기반 plain text editor로 별도 제공할 수 있어야 한다.
- 첫 번째 물리적 줄에서 문서 제목을 파생하는 Cabinet 규칙을 유지해야 한다.
- Wikilink, Markdown link, backlink, attachment reference, table, checklist, code block, blockquote를 확장 가능해야 한다.
- `[[문서]]`, `[[문서|별칭]]`, `![[asset:...|label]]` 같은 Cabinet/Obsidian 유사 문법을 안전한 chip 또는 inline widget으로 표현할 수 있어야 한다.
- 문서 저장, diff, restore, search, Graph projection은 Markdown body 기준으로 유지해야 한다.
- editor runtime이 domain/usecase/Rust/Tauri storage 계층으로 누출되면 안 된다.
- 외부 설정 파일이나 서버 의존 없이 설치 1회 후 로컬에서 동작해야 한다.

## 2. Cabinet Editor Requirements

### 2.1 Hard Requirements

다음 항목은 반드시 만족해야 한다.

- Markdown string을 입력으로 받고 Markdown string을 출력할 수 있어야 한다.
- Markdown round-trip에서 heading, paragraph, ordered/unordered list, checklist, table, code block, blockquote, link가 안정적으로 보존되어야 한다.
- WYSIWYG 또는 Live Preview surface에서 문법 marker를 기본 노출하지 않아야 한다.
- raw Markdown source editor를 별도 plain text editing path로 유지할 수 있어야 한다.
- custom inline syntax를 추가할 수 있어야 한다.
- custom block syntax 또는 block widget을 추가할 수 있어야 한다.
- React 앱 안에서 제어 가능한 component 또는 adapter로 사용할 수 있어야 한다.
- editor state 변경을 외부 `onChange(markdown: string)`로 전달할 수 있어야 한다.
- 저장 시점의 canonical body를 Markdown으로 확정할 수 있어야 한다.
- 테스트에서 editor runtime을 presentation adapter로 격리할 수 있어야 한다.

### 2.2 Cabinet-Specific Requirements

다음 항목은 일반 Markdown editor에는 보통 없으므로 확장 가능성이 중요하다.

- Wikilink: `[[Target Document]]`
- Wikilink alias: `[[Target Document|Display Label]]`
- Attachment reference: `![[asset:asset-id|Display Label]]`
- Attachment chip click action: 열기, 미리보기, 연결 해제, 원문에서 편집
- Link chip click action: 문서 열기, link/backlink projection refresh
- First-line title derivation: 별도 title field 없이 첫 번째 물리적 줄에서 제목 파생
- Internal ID hiding: document ID, asset ID, version ID, snapshot path, Git 용어를 일반 UI에 노출하지 않음
- Diff/restore compatibility: editor 내부 state가 아니라 Markdown body로 version diff와 restore 수행
- Graph projection compatibility: Markdown link extraction과 editor widget 표현이 같은 parse rule을 사용

### 2.3 Preferred Qualities

- MIT 또는 Apache-2.0처럼 제품 적용이 쉬운 license
- 활발한 유지보수
- TypeScript 지원
- React integration
- plugin architecture
- schema/serializer 확장 가능
- table editing UX
- checklist UX
- keyboard shortcut 확장 가능
- CSS를 Cabinet design system에 맞게 교체 가능
- server/collaboration 기능을 강제하지 않음

## 3. Summary Decision

현재 요구조건을 완전히 충족하는 단일 라이브러리는 없다.

가장 현실적인 선택지는 다음 순서다.

1. **Milkdown**: Cabinet의 장기 editor foundation 후보로 가장 적합하다.
2. **MDXEditor**: React 기반 Markdown WYSIWYG를 빠르게 검증하기 좋은 후보다.
3. **Tiptap + Markdown extension**: 확장성은 좋지만 Markdown extension의 성숙도와 round-trip 리스크를 검증해야 한다.
4. **Lexical 직접 구현**: 장기적으로 강력하지만 현재 문제 해결에는 구현량이 너무 크다.
5. **CodeMirror rich preview 계열**: 기존 CodeMirror 기반 source editor를 Live Preview로 확장하는 실험 후보이나, 제품 기본 editor로 바로 채택하기에는 위험하다.

BlockNote, TOAST UI Editor, ByteMD, Syncfusion/CKEditor류 Markdown mode는 Cabinet의 canonical Markdown, Obsidian-style extension, local-first 제품 계약과 완전히 맞지 않아 기본 후보에서 제외한다.

## 4. Candidate Matrix

| Candidate               | Fit         | Markdown Canonical | WYSIWYG/Live Preview | Custom Wikilink/Asset | React/Tauri Fit | Risk                              |
| ----------------------- | ----------- | ------------------ | -------------------- | --------------------- | --------------- | --------------------------------- |
| Milkdown                | High        | High               | High                 | High                  | Medium-High     | Custom plugin 구현 필요               |
| MDXEditor               | Medium-High | High               | High                 | Medium                | High            | MDX/Lexical plugin 이해 필요          |
| Tiptap + Markdown       | Medium      | Medium             | High                 | High                  | High            | Markdown extension beta/edge case |
| Lexical 직접 구현           | Medium      | Medium             | High                 | High                  | High            | 구현량 큼                             |
| codemirror-rich-markdoc | Low-Medium  | High               | Medium               | Medium                | High            | 실험성/성능/known issue                |
| BlockNote               | Low         | Low                | High                 | Medium                | High            | Markdown import/export lossy      |
| TOAST UI Editor         | Low-Medium  | Medium             | Medium               | Low-Medium            | Medium          | 유지보수/확장성 리스크                      |
| ByteMD                  | Low         | High               | Low                  | Medium                | Medium          | WYSIWYG가 아니라 split/preview 중심     |

## 5. Candidate Details

## 5.1 Milkdown

Source:

- Website: https://milkdown.dev/
- GitHub: https://github.com/Milkdown/milkdown
- npm package: `@milkdown/core`
- 조사 시점 npm version: `7.21.3`
- License: MIT

### Description

Milkdown은 ProseMirror와 Remark 기반의 plugin-driven WYSIWYG Markdown editor framework다. 공식 설명에서도 WYSIWYG Markdown editor framework로 소개하며, Typora와 유사한 Markdown writing experience를 목표로 한다.

### Strengths for Cabinet

- Markdown 중심 editor다.
- ProseMirror 기반이므로 schema, node view, plugin, transaction을 깊게 확장할 수 있다.
- Remark 기반 parse/stringify 흐름을 활용하므로 Cabinet의 Markdown canonical 정책과 잘 맞는다.
- WYSIWYG surface와 Markdown serialization을 함께 다룰 수 있다.
- table, checklist, link, code block 같은 기본 Markdown 확장이 가능하다.
- headless 성격이 강해 Cabinet UI/UX와 Penpot 20260721 design을 적용하기 쉽다.
- Y.js 연동 기반이 있으나 현재 제품 범위에서는 collaboration 기능을 끄고 local-only editor로 사용할 수 있다.

### Gaps

- Cabinet 전용 `[[...]]`, `![[asset:...]]` 문법은 기본 제공되지 않는다.
- Wikilink와 asset chip을 위한 custom parser, serializer, node/mark view가 필요하다.
- ProseMirror transaction과 Markdown serializer 사이의 round-trip 테스트가 필수다.
- 현재 hand-rolled React WYSIWYG를 바로 대체하려면 integration layer를 새로 설계해야 한다.

### Architecture Fit

Milkdown은 `apps/desktop` presentation adapter 안에만 위치해야 한다. `packages/editor`에는 Milkdown runtime type을 넣지 않는다. 공통 editor package에는 Markdown parse contract, Cabinet token extraction, title derivation, link extraction처럼 editor-runtime-independent logic만 둔다.

Allowed:

```text
apps/desktop MilkdownAdapter -> packages/editor markdown contracts
MilkdownAdapter -> onChange(markdown)
UseCase -> Markdown body string
```

Forbidden:

```text
packages/editor domain parser -> Milkdown Editor
Rust storage -> ProseMirror JSON
UseCase -> Milkdown transaction
```

### Cabinet Extension Plan

- `cabinetWikilinkPlugin`: `[[document]]`, `[[document|label]]` token을 inline atom 또는 mark view로 표시한다.
- `cabinetAssetReferencePlugin`: `![[asset:...|label]]` token을 attachment chip으로 표시한다.
- `cabinetFallbackSourcePlugin`: 파싱 불가 또는 round-trip 위험 블록은 읽기 전용 fallback block과 `원문에서 편집` action을 제공한다.
- `cabinetMarkdownSerializer`: editor state를 Markdown string으로 serialize한다.
- `cabinetMarkdownRoundTripTests`: Markdown input -> editor state -> Markdown output의 안정성을 검증한다.

### Recommendation

Milkdown을 1순위 PoC 대상으로 선택한다. 현재 자체 WYSIWYG를 계속 확장하는 것보다 유지보수성과 사용자 경험 측면에서 더 낫다.

## 5.2 MDXEditor

Source:

- Website: https://mdxeditor.dev/
- Overview: https://mdxeditor.dev/editor/docs/overview
- GitHub: https://github.com/mdx-editor/editor
- npm package: `@mdxeditor/editor`
- 조사 시점 npm version: `4.1.0`
- License: MIT

### Description

MDXEditor는 React component 형태의 Markdown/MDX WYSIWYG editor다. 공식 문서 기준으로 Markdown string을 받고 내보내며, 중간 HTML representation 없이 Markdown syntax constraints에 맞춰 편집한다.

### Strengths for Cabinet

- React component로 바로 붙이기 쉽다.
- Markdown input/output 계약이 명확하다.
- Lexical 기반이라 editing core가 현대적이다.
- CodeMirror dependency를 포함하고 있어 code block/source/diff 계열 기능과 친화적이다.
- table, code block, link, frontmatter, directive, MDX JSX 등 풍부한 plugin 생태가 있다.
- 빠른 PoC에 적합하다.

### Gaps

- MDX/JSX 중심 기능이 Cabinet에는 불필요하거나 과하다.
- `[[...]]`, `![[asset:...]]`는 직접 import/export customization이 필요하다.
- MDXEditor 내부 plugin model을 이해하고 Cabinet 문법을 안정적으로 끼워야 한다.
- Cabinet의 Obsidian식 Live Preview 감각과 완전히 같지는 않을 수 있다.

### Architecture Fit

MDXEditor 역시 `apps/desktop` adapter에만 위치해야 한다. `@mdxeditor/editor`, Lexical, CodeMirror runtime type은 domain/usecase/common editor package에 누출하지 않는다.

### Recommendation

MDXEditor는 2순위 후보다. 빠르게 사용자-facing WYSIWYG 품질을 검증하기에는 좋지만, Cabinet 전용 Markdown 문법의 장기 소유권을 고려하면 Milkdown보다 약간 낮다.

## 5.3 Tiptap + Markdown Extension

Source:

- Tiptap Markdown docs: https://tiptap.dev/docs/editor/markdown
- Tiptap GitHub: https://github.com/ueberdosis/tiptap
- npm package: `@tiptap/core`
- 조사 시점 npm version: `3.28.0`
- License: MIT

### Description

Tiptap은 ProseMirror 기반 rich text editor framework다. Markdown extension은 Markdown string을 Tiptap JSON으로 parse하고 editor content를 Markdown으로 serialize하는 기능을 제공한다.

### Strengths for Cabinet

- ProseMirror 기반으로 확장성이 높다.
- React integration이 성숙하다.
- custom node, custom mark, node view, command, shortcut 구현이 쉽다.
- Notion-like UX, bubble menu, slash command, floating menu 등을 만들기 좋다.

### Gaps

- 공식 Markdown extension이 early release/beta 성격으로 안내되어 edge case 리스크가 있다.
- Tiptap의 canonical internal state는 ProseMirror JSON이다. Cabinet의 canonical Markdown 정책과 맞추려면 import/export round-trip을 강하게 검증해야 한다.
- Markdown을 제품 저장소의 단일 진실로 유지하려면 serializer 품질이 핵심 리스크가 된다.

### Recommendation

Tiptap은 custom UX가 매우 중요하고 Markdown round-trip을 직접 통제할 준비가 있을 때 선택한다. 현재 Cabinet에는 Milkdown이나 MDXEditor를 먼저 검증하는 것이 더 낫다.

## 5.4 Lexical Direct Implementation

Source:

- Lexical docs: https://lexical.dev/
- Markdown package: https://lexical.dev/docs/packages/lexical-markdown
- GitHub: https://github.com/facebook/lexical
- npm package: `lexical`
- 조사 시점 npm version: `0.48.0`
- License: MIT

### Description

Lexical은 Meta가 개발한 extensible text editor framework다. `@lexical/markdown`은 Markdown import/export helper와 shortcut을 제공한다.

### Strengths for Cabinet

- editing core의 성능과 접근성이 좋다.
- React integration이 좋다.
- custom node와 plugin을 통해 Cabinet 전용 chip, block, command를 만들 수 있다.
- MDXEditor가 Lexical 기반이므로 ecosystem 검증이 이미 어느 정도 되어 있다.

### Gaps

- Markdown WYSIWYG product를 직접 만들어야 한다.
- table, wikilink, asset chip, source fallback, Markdown serializer를 상당 부분 직접 구현해야 한다.
- 현재 문제는 editor UX 안정화이므로 낮은 수준의 framework를 직접 쓰면 개발량이 너무 커진다.

### Recommendation

장기적으로 완전한 Cabinet-native editor를 만들 때만 고려한다. 현재는 MDXEditor를 통해 Lexical 기반 접근을 간접 검증하는 것이 더 실용적이다.

## 5.5 codemirror-rich-markdoc

Source:

- GitHub: https://github.com/segphault/codemirror-rich-markdoc
- npm package: `codemirror-rich-markdoc`
- 조사 시점 npm version: `0.0.2`
- License: MIT

### Description

CodeMirror 6 plugin으로 Markdown/Markdoc content에 rich editing presentation을 추가한다. Markdown 문법 문자를 숨기고, 커서가 위치한 요소의 문법 문자만 드러내는 hybrid editing mode를 제공한다.

### Strengths for Cabinet

- 현재 Cabinet이 이미 CodeMirror를 사용하므로 adapter 통합은 상대적으로 쉽다.
- Markdown source를 그대로 유지하는 접근과 잘 맞는다.
- Obsidian Live Preview와 비슷한 방향의 cursor-local syntax reveal 모델을 제공한다.
- `apps/desktop/src/codemirror_document_editor.ts`에 실험 feature로 붙이기 쉽다.

### Gaps

- GitHub README 기준 known issue가 많다.
- image syntax 지원이 부족하다.
- bracketed text를 link로 오인식하는 문제가 있다.
- rendered block replacement가 최적화되지 않았고 매 operation마다 다시 계산하는 문제가 있다.
- nested Markdoc tag rendering 문제가 있다.
- release가 없고 commit 수가 적다.
- Cabinet의 기본 editor로 채택하기에는 제품 안정성 리스크가 크다.

### Recommendation

기본 editor 후보로 선택하지 않는다. CodeMirror 기반 Live Preview 가능성을 탐색하는 실험 PoC로만 취급한다.

## 5.6 BlockNote

Source:

- Website: https://www.blocknotejs.org/
- Markdown export docs: https://www.blocknotejs.org/docs/features/export/markdown
- Markdown import docs: https://www.blocknotejs.org/docs/features/import/markdown
- npm package: `@blocknote/react`
- 조사 시점 npm version: `0.52.1`
- License: MPL-2.0

### Description

BlockNote는 Notion-like block-based rich text editor다. React component와 ready-to-use UI를 제공하고 real-time collaboration 확장도 지원한다.

### Strengths for Cabinet

- 사용자 경험은 Notion에 가깝고 완성도가 높다.
- block 기반 편집, slash command, rich UI를 빠르게 제공할 수 있다.
- React integration이 좋다.

### Gaps

- 공식 문서 기준 Markdown import는 lossy일 수 있다.
- non-lossy 저장은 BlockNote 자체 JSON을 권장한다.
- Cabinet은 Markdown source가 canonical이어야 하므로 저장 모델과 충돌한다.
- MPL-2.0 license 검토가 필요하다.
- Obsidian-style Markdown source ownership과 맞지 않는다.

### Recommendation

현재 Cabinet editor foundation으로 사용하지 않는다. Notion-like block UX 참고 대상으로만 둔다.

## 5.7 TOAST UI Editor

Source:

- Website: https://ui.toast.com/tui-editor/
- npm package: `@toast-ui/editor`
- 조사 시점 npm version: `3.2.2`
- License: MIT

### Description

TOAST UI Editor는 Markdown editor와 WYSIWYG editor를 함께 제공하는 JavaScript editor다.

### Strengths for Cabinet

- Markdown/WYSIWYG 전환 기능이 이미 있다.
- table, syntax highlighting, live preview 같은 기본 기능이 있다.
- MIT license다.

### Gaps

- 최신 npm publish가 오래되었다.
- 현대 React/Tauri architecture에 깊게 맞추기 어렵다.
- Cabinet 전용 wikilink/asset chip UX 확장성이 Milkdown, Tiptap, Lexical보다 낮다.
- Obsidian-style Live Preview라기보다 Markdown editor + WYSIWYG mode 전환에 가깝다.

### Recommendation

제품 기본 editor 후보에서 제외한다. 단순 Markdown/WYSIWYG reference로만 참고한다.

## 5.8 ByteMD

Source:

- Website: https://bytemd.js.org/

### Description

ByteMD는 Svelte 기반 Markdown editor component이며 React/Vue/Angular에서도 사용할 수 있다고 안내한다.

### Strengths for Cabinet

- Markdown source 중심이다.
- plugin system이 있다.
- 비교적 가볍다.

### Gaps

- WYSIWYG가 아니라 Markdown source + preview 중심이다.
- Cabinet이 원하는 Obsidian-style Live Preview 기본 편집 경험과 다르다.
- React/Tauri에서 쓰려면 wrapper integration을 검증해야 한다.

### Recommendation

현재 Cabinet 요구조건에는 맞지 않는다.

## 6. Source Editor Strategy

현재 Cabinet은 CodeMirror 기반 `원문 편집` modal을 제공한다. 외부 WYSIWYG editor를 채택하더라도 이 구조는 유지한다.

### Required Rule

- WYSIWYG editor는 기본 편집 surface다.
- CodeMirror는 plain text Markdown source editor다.
- WYSIWYG와 CodeMirror는 같은 canonical Markdown body를 공유한다.
- CodeMirror source editor에 rich preview plugin을 기본 적용하지 않는다.
- plain text editor는 Markdown marker를 숨기면 안 된다.

### Reason

사용자가 `원문 편집`을 열었을 때는 실제 저장될 Markdown source를 확인하고 수정해야 한다. 이 영역까지 WYSIWYG로 만들면 제품의 source ownership 원칙이 약해진다.

## 7. Recommended Architecture

### 7.1 Editor Boundary

Editor library는 desktop UI adapter 경계에 둔다.

```text
React Document Workbench
  -> CabinetEditorAdapter
    -> Milkdown or MDXEditor runtime
    -> onChange(markdown)
  -> CodeMirrorSourceModal
    -> onChange(markdown)

UseCase
  <- markdown body string

Domain
  <- title/link/asset extraction independent of editor runtime
```

### 7.2 Required Interfaces

```text
CabinetEditorAdapter:
  mount(parent, initialMarkdown, callbacks)
  setMarkdown(markdown, revision)
  focus()
  destroy()

CabinetEditorCallbacks:
  onMarkdownChange(markdown, sourceRevision)
  onOpenSourceEditor(reason, targetRange)
  onOpenDocumentLink(displayReference)
  onOpenAssetReference(displayReference)
```

### 7.3 Runtime Isolation

다음 import는 `apps/desktop` adapter 밖에 나오면 안 된다.

```text
@milkdown/*
@mdxeditor/editor
@tiptap/*
lexical
@lexical/*
prosemirror-*
```

`packages/editor`에는 다음만 허용한다.

```text
Markdown token model
Markdown parse result
Wikilink extraction
Asset reference extraction
Title derivation
Round-trip test fixtures
Editor-agnostic command contract
```

## 8. PoC Plan

### 8.1 PoC A: Milkdown

Goal:

- Milkdown으로 Cabinet 문서 하나를 편집하고 Markdown body round-trip을 검증한다.

Scope:

- heading
- paragraph
- unordered list
- checklist
- table
- code block
- blockquote
- Markdown link
- wikilink fallback text
- asset reference fallback text

Success Criteria:

- `initialMarkdown -> editor -> exportedMarkdown` 결과가 의미적으로 동일하다.
- 첫 줄 제목 파생이 유지된다.
- table alignment row가 사용자 기본 화면에 직접 노출되지 않는다.
- checklist toggle이 Markdown body를 갱신한다.
- source modal에서 수정한 Markdown이 Milkdown surface에 반영된다.
- Milkdown runtime import가 `apps/desktop` adapter로만 제한된다.

Failure Criteria:

- Markdown round-trip에서 Cabinet 문법이 손실된다.
- table, checklist, link가 불안정하게 serialize된다.
- editor state를 저장 canonical으로 삼아야만 기능이 동작한다.
- domain/usecase에 ProseMirror/Milkdown type이 누출된다.

### 8.2 PoC B: MDXEditor

Goal:

- MDXEditor가 Cabinet의 Markdown canonical 저장 정책과 custom syntax 확장에 충분한지 검증한다.

Scope:

- markdown input/output
- table
- checklist
- code block
- source/diff 관련 내장 기능 검토
- custom directive 또는 custom plugin으로 wikilink/asset 표시 가능성 확인

Success Criteria:

- Markdown string input/output이 stable하다.
- React integration이 단순하다.
- Cabinet custom syntax를 손실하지 않는다.
- source modal과 default WYSIWYG를 분리할 수 있다.

Failure Criteria:

- MDX/JSX parser가 Cabinet Markdown을 의도치 않게 변환한다.
- custom syntax를 HTML/MDX node로 바꾸지 않으면 유지할 수 없다.
- Markdown output이 Cabinet source diff에 불리하게 계속 재포맷된다.

### 8.3 PoC C: CodeMirror Live Preview Experiment

Goal:

- CodeMirror source editor에 Live Preview extension을 붙이는 방식이 Obsidian-like UX에 충분한지 확인한다.

Scope:

- `codemirror-rich-markdoc`
- heading marker hiding
- link marker hiding
- table/block widget
- cursor-local syntax reveal

Success Criteria:

- 기존 CodeMirror adapter 변경만으로 실험 가능하다.
- Markdown source가 그대로 보존된다.
- 커서 주변 문법 reveal이 사용 가능하다.

Failure Criteria:

- table/block replacement 성능이 문서 크기에 따라 불안정하다.
- Cabinet asset reference와 wikilink가 잘못 파싱된다.
- 원문 편집 modal의 plain text 정체성을 해친다.

## 9. Required Tests Before Adoption

### 9.1 Markdown Round-Trip Tests

다음 fixture를 모든 후보에 대해 검증한다.

- 빈 문서
- 한글 제목
- `# 제목` 첫 줄 제목
- heading marker 없는 첫 줄 제목
- paragraph 여러 개
- ordered list
- unordered list
- checklist checked/unchecked
- nested list
- GFM table
- table alignment
- fenced code block
- blockquote
- Markdown link
- Wikilink
- Wikilink alias
- Asset reference
- raw HTML
- malformed Markdown
- 긴 줄
- Unicode emoji와 조합형 한글

### 9.2 Cabinet Behavior Tests

- 첫 줄 수정 후 Home, Document, Graph, Canvas, Assets 연결 문서 표시의 제목이 동일하게 갱신된다.
- WYSIWYG에서 checklist를 toggle하면 Markdown body의 `[ ]`/`[x]`가 갱신된다.
- WYSIWYG에서 table cell을 수정하면 alignment row가 손실되지 않는다.
- Wikilink chip을 클릭하면 문서 열기 action만 발생하고 내부 document ID가 노출되지 않는다.
- Asset chip을 클릭하면 asset action이 열리고 asset ID가 일반 UI에 노출되지 않는다.
- source modal에서 Markdown을 수정하면 WYSIWYG surface가 같은 body로 갱신된다.
- stale WYSIWYG transaction은 거부되거나 재시도 가능한 conflict로 처리된다.
- 저장 후 durable readback body가 editor output과 동일하다.
- diff는 editor internal state가 아니라 Markdown body 기준으로 계산된다.

### 9.3 Performance Tests

표준 fixture:

- 100개 문단
- 20개 table
- 100개 wikilink
- 50개 asset reference
- 30개 checklist

측정:

- initial render
- keystroke latency
- Markdown export
- source modal sync
- document switching

목표:

- 표준 fixture에서 사용자-facing 조회와 편집 반응이 p95 300ms 기준을 해치지 않아야 한다.
- 대용량 문서는 전체 UI를 멈추지 않고 progressive 또는 bounded update를 사용해야 한다.

## 10. Adoption Recommendation

### 10.1 Primary Recommendation

Milkdown을 1순위로 PoC한다.

이유:

- Markdown-first editor framework다.
- ProseMirror와 Remark 기반이라 Cabinet custom syntax와 serializer를 소유하기 좋다.
- headless/plugin architecture가 Cabinet 디자인과 잘 맞는다.
- 장기적으로 Obsidian-like editing, safe chip, custom syntax, source fallback을 만들 수 있는 통제권이 가장 높다.

### 10.2 Secondary Recommendation

MDXEditor를 2순위로 PoC한다.

이유:

- React integration이 빠르다.
- Markdown string in/out이 명확하다.
- 기본 WYSIWYG 품질을 빠르게 확인할 수 있다.
- 다만 MDX 중심 설계가 Cabinet에 과할 수 있으므로 custom syntax round-trip을 반드시 검증한다.

### 10.3 Do Not Adopt As Foundation

다음은 기본 foundation으로 채택하지 않는다.

- BlockNote: Markdown canonical과 충돌한다.
- TOAST UI Editor: 현대 Cabinet custom editor requirements에 비해 확장성과 유지보수 리스크가 있다.
- ByteMD: WYSIWYG 요구조건과 맞지 않는다.
- codemirror-rich-markdoc: 실험 후보로만 사용한다.

## 11. Migration Strategy

### Phase 1. Editor Adapter Boundary

- 기존 자체 WYSIWYG code와 storage/usecase 경계를 다시 확인한다.
- `CabinetEditorAdapter` interface를 desktop presentation 계층에 정의한다.
- 기존 CodeMirror source modal은 유지한다.
- 기존 React WYSIWYG는 adapter 구현체 중 하나로 감싼다.

### Phase 2. Milkdown PoC

- `MilkdownCabinetEditorAdapter`를 추가한다.
- feature flag는 환경 변수나 외부 설정이 아니라 개발 build 내부 상수 또는 test-only composition으로 둔다.
- PoC는 제품 기본 경로로 노출하지 않는다.
- Markdown round-trip fixture를 먼저 작성한다.

### Phase 3. Cabinet Syntax Plugins

- Wikilink parser/serializer를 추가한다.
- Asset reference parser/serializer를 추가한다.
- Safe chip node view를 추가한다.
- Source fallback action을 추가한다.

### Phase 4. Product Integration

- Document authoring workbench의 default editor를 Milkdown adapter로 교체한다.
- `원문 편집` modal은 CodeMirror plain text editor로 유지한다.
- save coordinator와 stale patch guard를 Markdown body 기준으로 유지한다.

### Phase 5. Regression Gate

- document authoring smoke
- packaged UI smoke
- title propagation
- graph projection refresh
- attachment reference
- diff/restore
- accessibility
- performance benchmark

## 12. Final Decision Rule

다음 조건을 모두 만족한 후보만 Cabinet 기본 editor로 채택한다.

- Markdown source를 canonical 저장 포맷으로 유지한다.
- Cabinet custom syntax가 손실 없이 round-trip된다.
- 일반 UI에 Markdown marker와 내부 ID를 불필요하게 노출하지 않는다.
- source modal은 plain text editor로 유지된다.
- editor runtime이 presentation adapter 밖으로 누출되지 않는다.
- 표준 fixture에서 p95 300ms 사용자-facing 기준을 해치지 않는다.
- 문서 저장, diff, restore, Graph projection과 같은 기존 유스케이스 계약을 변경하지 않는다.

현재 기준으로 이 조건을 만족할 가능성이 가장 높은 후보는 **Milkdown**이다.

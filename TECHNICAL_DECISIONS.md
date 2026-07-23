# Technical Decisions

## TD-001. CommonMark Link Event Parser

- Status: Accepted
- Date: 2026-07-18
- Scope: local Markdown adapter에서 standard Markdown external link와 source range를 추출한다.

### Decision

`cabinet-adapters`에 `pulldown-cmark` 0.13.4를 고정 범위 dependency로 추가한다. crate의 event와 tag 타입은 `LocalMarkdownParser` 내부에서만 사용하고 `cabinet-ports`에는 parser-neutral value object만 반환한다.

### Evidence

- pulldown-cmark 0.13.4는 CommonMark pull parser이며 MIT 라이선스이고 Rust 1.71.1 이상을 지원한다.
- comrak 0.53.0은 CommonMark/GFM AST와 rendering까지 제공하지만, 현재 필요한 link event와 source offset보다 넓은 기능과 dependency surface를 가진다.
- Cabinet은 HTML rendering을 이 parser로 교체하지 않는다. CodeMirror 편집, Markdown preview, custom Wikilink/asset syntax는 기존 경계를 유지한다.

### Constraints

- `http`, `https`, `mailto` target만 external graph relation으로 분류한다.
- URL 원문, credential, query, fragment를 Product/Field Debug Log에 기록하지 않는다.
- parser crate 타입을 domain, usecase, port public API에 노출하지 않는다.
- dependency는 build-time에 고정하며 외부 프로세스, 서비스, 환경 변수, 설정 파일을 요구하지 않는다.

### Rejected Alternative

comrak은 향후 full GFM AST 변환이나 Markdown rendering 통합이 필요할 때 재평가한다. 현재 범위에서는 AST arena와 renderer 기능이 불필요하므로 채택하지 않는다.

### Rollback

`LocalMarkdownParser`의 standard-link adapter와 dependency를 제거하고 동일한 `ParsedExternalLink` port를 구현하는 다른 adapter로 교체한다. domain graph, projection usecase, durable graph schema, UI DTO는 변경하지 않는다.

## TD-002. Local Topology Renderer And Background Layout

- Status: Accepted
- Date: 2026-07-18
- Scope: macOS 개인용 로컬 앱의 문서 토폴로지를 가속 렌더링하고 layout 계산을 UI thread에서 분리한다.

### Decision

presentation adapter에서 `sigma` 3.0.3, `graphology` 0.26.0, `graphology-layout-forceatlas2` 0.10.1을 exact version으로 사용한다. Sigma와 Graphology 타입은 presentation adapter 내부에만 두고 application-owned `TopologyRendererAdapter`와 `TopologyLayoutAdapter` 뒤로 숨긴다. ForceAtlas2는 worker entry point를 사용하고, layout request의 generation이 현재 generation과 일치할 때만 결과를 적용한다.

### Candidate Matrix

| Criterion            | Sigma + Graphology + ForceAtlas2                                      | Cytoscape.js                                                                               |
| -------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Evaluated version    | 3.0.3 + 0.26.0 + 0.10.1                                               | 3.34.0                                                                                     |
| License              | MIT / MIT / MIT                                                       | MIT                                                                                        |
| Renderer             | WebGL graph renderer                                                  | Canvas renderer with integrated graph API                                                  |
| Background layout    | First-party Graphology ForceAtlas2 worker entry point                 | Layout behavior varies by selected core/extension layout                                   |
| Lifecycle            | Explicit renderer `kill()` API                                        | Explicit instance `destroy()` API                                                          |
| 1,000-node target    | WebGL renderer and bounded graph policy align with the current target | Suitable candidate, but renderer, graph model, and layout surface are more tightly bundled |
| Cabinet boundary fit | Renderer and layout are separately replaceable behind two owned ports | Requires a broader all-in-one presentation adapter                                         |
| Decision             | Accepted                                                              | Retained as rollback candidate                                                             |

### Evidence And Constraints

- npm registry metadata reports all accepted packages and Cytoscape.js as MIT licensed at the evaluated exact versions.
- Sigma documentation defines a WebGL renderer over a Graphology graph and an explicit `kill()` lifecycle that releases renderer resources and bindings.
- Graphology ForceAtlas2 documentation exposes a worker implementation, allowing iterative layout outside the UI thread.
- Cytoscape.js remains maintained and has an explicit destroy lifecycle, but this phase prefers independent renderer and layout ports and the direct first-party ForceAtlas2 worker path.
- The production adapter must prove a nonblank 1,000-node render, bounded update latency, stale-generation rejection, listener/observer/worker disposal, and WebView compatibility before replacing the current visual renderer.
- The semantic DOM node list remains available without Sigma, WebGL, or worker success. It is the keyboard and assistive-technology fallback and must not derive its labels from renderer internals.
- No runtime environment variable, hidden setting, remote service, external process, or dynamic vendor selection is introduced.
- Product Log may contain only stable renderer error codes and node/edge count buckets. It must not contain document titles, content, filenames, paths, or graph payloads.

### Pinning And Provenance

- Pin `sigma` to `3.0.3`.
- Pin `graphology` to `0.26.0`.
- Pin `graphology-layout-forceatlas2` to `0.10.1`.
- Commit the resolved package lock whenever dependencies are committed.
- Upgrade only in a dedicated dependency task that repeats license, nonblank render, lifecycle, bundle, and 1,000-node performance checks.

### Rejected Alternative

Cytoscape.js 3.34.0 is not selected for this phase because Cabinet needs an independently replaceable WebGL renderer and background layout adapter with explicit generation cancellation. It remains the first evaluated replacement candidate if Sigma fails the production WebView, lifecycle, or performance gates.

### Rollback

Remove the three vendor dependencies and their presentation adapters, then bind the existing semantic topology list and previous SVG renderer to the same application-owned graph model. Domain entities, graph projection, query usecases, controller state, opaque node identity, navigation, filters, and accessibility labels must not change during rollback.

## TD-003. Root-Owned Workspace Sidebar Document Shortcuts

- Status: Accepted
- Date: 2026-07-21
- Scope: macOS 개인용 로컬 데스크톱 앱의 workspace shell route navigation과 좌측 하단 문서 바로가기 일관성.

### Decision

좌측 하단 문서 바로가기는 각 route component가 자체적으로 계산하지 않는다. `desktop_entry` 같은 root composition 지점에서 현재 workspace의 recent document projection을 기준으로 한 번 계산한 `documentShortcuts`를 Home, Search, Document, Graph, Canvas, Assets, Backup route에 동일하게 전달한다.

Document 메뉴는 Search route의 별칭이 아니다. 마지막 작업 문서가 있으면 해당 문서를 재개하고, 마지막 작업 문서가 없을 때만 문서 선택 또는 빈 상태를 표시한다. 검색은 상단 검색 또는 명시적 Search navigation action으로만 진입한다.

### Rationale

route별 컴포넌트가 sidebar document list를 각각 계산하면 메뉴 전환만으로 좌측 하단 문서 영역이 검색 결과, 현재 문서, Graph 중심 문서, Asset 선택 상태 또는 빈 목록으로 바뀐다. 사용자는 이 영역을 탐색 기준으로 인식하므로 route 전환에 따라 내용이 바뀌면 앱의 위치 감각이 무너지고 문서 메뉴의 정체성이 모호해진다.

### Constraints

- route component는 `documentShortcuts`를 표시만 하고, 검색 결과나 route-local state에서 재생성하지 않는다.
- root-owned shortcuts는 사용자-facing title을 사용하고 내부 document ID, 문서 파일명, snapshot path를 label로 노출하지 않는다.
- Search route의 결과 목록은 main content 영역에서만 표시한다.
- Graph, Canvas, Assets route의 선택 context는 해당 main surface 또는 inspector에만 표시한다.
- route 변경 regression test는 Home, Search, Document, Graph, Canvas, Assets, Backup 전체 route를 포함한다.

### Evidence

- Desktop route integration tests verify that all routed surfaces receive the same root-owned sidebar document shortcuts.
- Shared shell tests verify that route actions do not mutate the lower sidebar document list.
- Authoring route tests verify that Document navigation preserves the current working document context.

### Rejected Alternative

route별로 문서 바로가기를 계산하는 방식은 거부한다. Search route는 검색 결과를, Document route는 현재 문서를, Graph/Canvas/Assets route는 각 surface의 selection을 보여줄 수 있지만, 그 정보는 main content 또는 inspector에 속한다. 공통 sidebar shortcut 영역의 의미를 route마다 바꾸면 사용자가 메뉴를 이동할 때 같은 위치에서 다른 개념을 보게 된다.

### Rollback

이 결정을 되돌리려면 먼저 사용자-facing navigation model을 다시 정의하고, 좌측 하단 영역의 이름과 목적을 바꾼 뒤 전체 route visual regression을 갱신해야 한다. 단순히 route component 내부 계산으로 되돌리는 rollback은 허용하지 않는다.

## TD-004. Markdown Editor Modes And Obsidian-Like Live Preview Boundary

- Status: Accepted
- Date: 2026-07-22
- Scope: macOS 개인용 로컬 데스크톱 앱의 문서 편집 경험과 CodeMirror extension 경계.

### Decision

현재 릴리스 범위의 문서 편집기는 CodeMirror 기반 Markdown source editor와 별도 preview 또는 split view를 제공한다. 사용자는 Markdown 원문을 직접 편집하고, preview panel에서 rendered Markdown을 확인한다. Obsidian Live Preview처럼 편집기 안에서 Markdown 문법을 일부 숨기거나 inline widget으로 렌더링하는 방식은 현재 완료 기능으로 주장하지 않는다.

Obsidian과 유사한 Live Preview를 구현할 경우, 저장 데이터의 canonical form은 Markdown source로 유지한다. inline preview, checkbox toggle, link chip, attachment chip, table widget, heading rendering, syntax hiding은 CodeMirror presentation adapter의 `ViewPlugin`, `Decoration`, `WidgetType`, command extension과 React presenter 경계에서 구현한다. Domain, usecase, port는 Markdown text, document operation, attachment association, link projection 계약만 소유한다.

### Rationale

현재 앱은 문서 저장, 첫 줄 제목 파생, diff, restore, link projection, Graph, Canvas, attachment association을 Markdown source 기준으로 검증한다. 편집기 내부 렌더링 방식을 도메인 모델에 섞으면 diff와 restore의 canonical input이 흔들리고, future platform adapter가 같은 문서 계약을 재사용하기 어렵다. 따라서 Obsidian-like UX는 presentation concern으로 격리한다.

### Constraints

- Markdown source는 저장, diff, restore, search, link projection의 단일 원천으로 유지한다.
- Live Preview 구현은 source/split/preview mode를 제거하지 않는다.
- CodeMirror vendor 타입은 presentation adapter 내부에 둔다.
- domain/usecase/port public API에 `Decoration`, `WidgetType`, DOM node, React component, editor state 타입을 노출하지 않는다.
- table, attachment, wikilink, checkbox widget은 원문 Markdown으로 round-trip 가능해야 한다.
- Product Log에는 editor input 원문, selected text, document body, attachment filename/path를 기록하지 않는다.
- Live Preview 테스트는 source text round-trip, cursor 주변 syntax visibility, widget command, keyboard navigation, screen-reader fallback, internal ID 비노출을 포함해야 한다.

### Rejected Alternative

Markdown을 HTML 또는 block tree로 즉시 변환해 저장하는 WYSIWYG editor는 현재 범위에서 거부한다. 이 방식은 existing diff/restore/link projection 계약과 충돌하고, 사용자가 자신의 Markdown 문서를 직접 소유한다는 제품 목표를 약화한다.

### Rollback

Live Preview extension을 비활성화해도 source/split/preview 편집, 저장, diff, restore, attachment, Graph, Canvas 기능은 그대로 동작해야 한다. rollback은 CodeMirror extension binding과 presenter state만 되돌리며 domain/usecase/storage schema를 변경하지 않는다.

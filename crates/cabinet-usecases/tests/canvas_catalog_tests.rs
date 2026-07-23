use cabinet_domain::canvas::{CanvasId, CanvasLifecycleState, CanvasRevision, CanvasTitle};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::canvas_catalog::{
    CanvasCatalogEntry, CanvasCatalogError, CanvasCatalogPort, LastCanvasSelectionError,
    LastCanvasSelectionPort,
};
use cabinet_usecases::canvas_catalog::{
    ResolveInitialCanvasError, ResolveInitialCanvasInput, ResolveInitialCanvasUsecase,
    ResolvedCanvasSelectionSource, SelectCanvasError, SelectCanvasInput, SelectCanvasUsecase,
};

#[test]
fn active_last_used_canvas_is_selected_from_the_bounded_catalog() {
    let catalog = Catalog::new([
        entry("canvas-a", "첫 Canvas", CanvasLifecycleState::Saved, 2),
        entry("canvas-b", "최근 Canvas", CanvasLifecycleState::Updated, 4),
    ]);
    let output = ResolveInitialCanvasUsecase::new()
        .execute(input(20, true), &catalog, &Selection::found("canvas-b"))
        .unwrap();

    assert_eq!(output.entries().len(), 2);
    assert_eq!(output.selected_canvas_id(), Some("canvas-b"));
    assert_eq!(
        output.selection_source(),
        ResolvedCanvasSelectionSource::LastUsed
    );
    assert_eq!(catalog.requested_limit.get(), 20);
    assert!(catalog.include_archived.get());
}

#[test]
fn missing_or_archived_last_used_canvas_falls_back_to_the_first_active_entry() {
    for last in ["missing-canvas", "canvas-archived"] {
        let catalog = Catalog::new([
            entry(
                "canvas-archived",
                "보관 Canvas",
                CanvasLifecycleState::Archived,
                8,
            ),
            entry(
                "canvas-active",
                "작업 Canvas",
                CanvasLifecycleState::Saved,
                3,
            ),
        ]);
        let output = ResolveInitialCanvasUsecase::new()
            .execute(input(10, true), &catalog, &Selection::found(last))
            .unwrap();

        assert_eq!(output.selected_canvas_id(), Some("canvas-active"));
        assert_eq!(
            output.selection_source(),
            ResolvedCanvasSelectionSource::Fallback
        );
    }
}

#[test]
fn catalog_without_an_active_canvas_returns_an_explicit_empty_selection() {
    let catalog = Catalog::new([entry(
        "canvas-archived",
        "보관 Canvas",
        CanvasLifecycleState::Archived,
        2,
    )]);
    let output = ResolveInitialCanvasUsecase::new()
        .execute(input(10, true), &catalog, &Selection::none())
        .unwrap();

    assert_eq!(output.selected_canvas_id(), None);
    assert_eq!(
        output.selection_source(),
        ResolvedCanvasSelectionSource::Empty
    );
}

#[test]
fn invalid_limit_and_boundary_failures_return_stable_errors() {
    let invalid = ResolveInitialCanvasUsecase::new()
        .execute(input(0, false), &Catalog::new([]), &Selection::none())
        .unwrap_err();
    assert_eq!(invalid, ResolveInitialCanvasError::InvalidInput);
    assert_eq!(invalid.code(), "canvas_catalog.invalid_input");

    let catalog_failure = ResolveInitialCanvasUsecase::new()
        .execute(input(10, false), &Catalog::failed(), &Selection::none())
        .unwrap_err();
    assert_eq!(
        catalog_failure,
        ResolveInitialCanvasError::CatalogUnavailable
    );

    let selection_failure = ResolveInitialCanvasUsecase::new()
        .execute(input(10, false), &Catalog::new([]), &Selection::failed())
        .unwrap_err();
    assert_eq!(
        selection_failure,
        ResolveInitialCanvasError::SelectionUnavailable
    );
}

#[test]
fn selecting_an_active_canvas_persists_only_the_validated_identity() {
    let catalog = Catalog::new([entry(
        "canvas-active",
        "작업 Canvas",
        CanvasLifecycleState::Saved,
        3,
    )]);
    let mut selection = RecordingSelection::default();

    let output = SelectCanvasUsecase::new()
        .execute(
            SelectCanvasInput::new("workspace-1", "canvas-active", 10),
            &catalog,
            &mut selection,
        )
        .unwrap();

    assert_eq!(output.selected_canvas_id(), "canvas-active");
    assert_eq!(selection.saved, vec!["canvas-active"]);
}

#[test]
fn selecting_missing_or_archived_canvas_is_rejected_without_a_write() {
    for (canvas_id, expected) in [
        ("missing", SelectCanvasError::CanvasNotFound),
        ("canvas-archived", SelectCanvasError::CanvasArchived),
    ] {
        let catalog = Catalog::new([entry(
            "canvas-archived",
            "보관 Canvas",
            CanvasLifecycleState::Archived,
            2,
        )]);
        let mut selection = RecordingSelection::default();
        let error = SelectCanvasUsecase::new()
            .execute(
                SelectCanvasInput::new("workspace-1", canvas_id, 10),
                &catalog,
                &mut selection,
            )
            .unwrap_err();
        assert_eq!(error, expected);
        assert!(selection.saved.is_empty());
    }
}

fn input(limit: usize, include_archived: bool) -> ResolveInitialCanvasInput {
    ResolveInitialCanvasInput::new("workspace-1", limit, include_archived)
}

fn entry(
    id: &str,
    title: &str,
    lifecycle: CanvasLifecycleState,
    revision: u64,
) -> CanvasCatalogEntry {
    CanvasCatalogEntry::new(
        CanvasId::new(id).unwrap(),
        CanvasTitle::new(title).unwrap(),
        lifecycle,
        CanvasRevision::new(revision).unwrap(),
    )
}

struct Catalog {
    entries: Result<Vec<CanvasCatalogEntry>, CanvasCatalogError>,
    requested_limit: std::cell::Cell<usize>,
    include_archived: std::cell::Cell<bool>,
}

impl Catalog {
    fn new<const N: usize>(entries: [CanvasCatalogEntry; N]) -> Self {
        Self {
            entries: Ok(entries.into()),
            requested_limit: std::cell::Cell::new(0),
            include_archived: std::cell::Cell::new(false),
        }
    }

    fn failed() -> Self {
        Self {
            entries: Err(CanvasCatalogError::StorageUnavailable),
            requested_limit: std::cell::Cell::new(0),
            include_archived: std::cell::Cell::new(false),
        }
    }
}

impl CanvasCatalogPort for Catalog {
    fn list_canvas_entries(
        &self,
        _: &WorkspaceId,
        limit: usize,
        include_archived: bool,
    ) -> Result<Vec<CanvasCatalogEntry>, CanvasCatalogError> {
        self.requested_limit.set(limit);
        self.include_archived.set(include_archived);
        self.entries.clone()
    }
}

struct Selection(Result<Option<CanvasId>, LastCanvasSelectionError>);

impl Selection {
    fn found(id: &str) -> Self {
        Self(Ok(Some(CanvasId::new(id).unwrap())))
    }
    fn none() -> Self {
        Self(Ok(None))
    }
    fn failed() -> Self {
        Self(Err(LastCanvasSelectionError::StorageUnavailable))
    }
}

impl LastCanvasSelectionPort for Selection {
    fn load_last_canvas_id(
        &self,
        _: &WorkspaceId,
    ) -> Result<Option<CanvasId>, LastCanvasSelectionError> {
        self.0.clone()
    }

    fn save_last_canvas_id(
        &mut self,
        _: &WorkspaceId,
        _: &CanvasId,
    ) -> Result<(), LastCanvasSelectionError> {
        unreachable!()
    }
}

#[derive(Default)]
struct RecordingSelection {
    saved: Vec<String>,
}

impl LastCanvasSelectionPort for RecordingSelection {
    fn load_last_canvas_id(
        &self,
        _: &WorkspaceId,
    ) -> Result<Option<CanvasId>, LastCanvasSelectionError> {
        Ok(None)
    }

    fn save_last_canvas_id(
        &mut self,
        _: &WorkspaceId,
        canvas_id: &CanvasId,
    ) -> Result<(), LastCanvasSelectionError> {
        self.saved.push(canvas_id.as_str().to_string());
        Ok(())
    }
}

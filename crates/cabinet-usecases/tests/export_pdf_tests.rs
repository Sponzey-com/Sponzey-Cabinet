use cabinet_usecases::export::{ExportPdfOutput, ExportPdfUsecase};

#[test]
fn export_pdf_returns_unsupported_without_external_renderer() {
    let usecase = ExportPdfUsecase::new();

    let output = usecase.execute();

    assert_eq!(output, ExportPdfOutput::Unsupported);
}

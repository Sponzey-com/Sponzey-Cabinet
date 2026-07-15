use cabinet_core::performance::{
    MeasurementEnvironment, PerformanceFixtureProfile, PerformanceReport, PerformanceSample,
    PerformanceTarget,
};

#[test]
fn fixture_profiles_are_deterministic_and_record_scale() {
    let small = PerformanceFixtureProfile::small();
    let medium = PerformanceFixtureProfile::medium();
    let large = PerformanceFixtureProfile::large();

    assert_eq!(small.name(), "small");
    assert_eq!(small.document_count(), 50);
    assert_eq!(small.average_body_bytes(), 2_048);
    assert_eq!(small.link_count(), 150);
    assert_eq!(small.asset_count(), 25);

    assert!(small.document_count() < medium.document_count());
    assert!(medium.document_count() < large.document_count());
}

#[test]
fn report_calculates_p95_by_target_and_checks_300ms_goal() {
    let report = PerformanceReport::new(
        PerformanceFixtureProfile::small(),
        MeasurementEnvironment::new("local-test", "deterministic"),
        vec![
            PerformanceSample::new(PerformanceTarget::CurrentDocumentLookup, 10),
            PerformanceSample::new(PerformanceTarget::CurrentDocumentLookup, 30),
            PerformanceSample::new(PerformanceTarget::CurrentDocumentLookup, 70),
            PerformanceSample::new(PerformanceTarget::CurrentDocumentLookup, 120),
            PerformanceSample::new(PerformanceTarget::CurrentDocumentLookup, 280),
            PerformanceSample::new(PerformanceTarget::SearchLookup, 301),
        ],
    );

    assert_eq!(
        report.p95_ms(PerformanceTarget::CurrentDocumentLookup),
        Some(280)
    );
    assert!(report.passes_ms_goal(PerformanceTarget::CurrentDocumentLookup, 300));
    assert!(!report.passes_ms_goal(PerformanceTarget::SearchLookup, 300));
    assert_eq!(report.environment().name(), "local-test");
}

#[test]
fn permission_aware_query_targets_have_300ms_p95_fixture() {
    let report = PerformanceReport::new(
        PerformanceFixtureProfile::medium(),
        MeasurementEnvironment::new(
            "permission-aware-local-fixture",
            "current, search, and asset lookup use index or projection paths",
        ),
        vec![
            PerformanceSample::new(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 42),
            PerformanceSample::new(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 57),
            PerformanceSample::new(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 61),
            PerformanceSample::new(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 74),
            PerformanceSample::new(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 81),
            PerformanceSample::new(PerformanceTarget::PermissionAwareSearchLookup, 108),
            PerformanceSample::new(PerformanceTarget::PermissionAwareSearchLookup, 132),
            PerformanceSample::new(PerformanceTarget::PermissionAwareSearchLookup, 174),
            PerformanceSample::new(PerformanceTarget::PermissionAwareSearchLookup, 205),
            PerformanceSample::new(PerformanceTarget::PermissionAwareSearchLookup, 246),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetMetadataLookup, 34),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetMetadataLookup, 48),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetMetadataLookup, 63),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetMetadataLookup, 71),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetMetadataLookup, 85),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetContentLookup, 88),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetContentLookup, 114),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetContentLookup, 149),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetContentLookup, 201),
            PerformanceSample::new(PerformanceTarget::PermissionAwareAssetContentLookup, 278),
        ],
    );

    assert_eq!(report.fixture().document_count(), 1_000);
    assert_eq!(report.fixture().asset_count(), 500);
    assert!(report.passes_ms_goal(PerformanceTarget::PermissionAwareCurrentDocumentLookup, 300));
    assert!(report.passes_ms_goal(PerformanceTarget::PermissionAwareSearchLookup, 300));
    assert!(report.passes_ms_goal(PerformanceTarget::PermissionAwareAssetMetadataLookup, 300));
    assert!(report.passes_ms_goal(PerformanceTarget::PermissionAwareAssetContentLookup, 300));
}

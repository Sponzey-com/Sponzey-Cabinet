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

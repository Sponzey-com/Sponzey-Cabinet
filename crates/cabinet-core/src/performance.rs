#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceTarget {
    CurrentDocumentLookup,
    HistoryListLookup,
    SpecificVersionLookup,
    SearchLookup,
    LinkBacklinkLookup,
    UnresolvedLinkLookup,
    OrphanDocumentLookup,
    AssetMetadataLookup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceFixtureProfile {
    name: &'static str,
    document_count: usize,
    average_body_bytes: usize,
    link_count: usize,
    asset_count: usize,
}

impl PerformanceFixtureProfile {
    pub const fn small() -> Self {
        Self {
            name: "small",
            document_count: 50,
            average_body_bytes: 2_048,
            link_count: 150,
            asset_count: 25,
        }
    }

    pub const fn medium() -> Self {
        Self {
            name: "medium",
            document_count: 1_000,
            average_body_bytes: 4_096,
            link_count: 5_000,
            asset_count: 500,
        }
    }

    pub const fn large() -> Self {
        Self {
            name: "large",
            document_count: 10_000,
            average_body_bytes: 8_192,
            link_count: 75_000,
            asset_count: 5_000,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn document_count(&self) -> usize {
        self.document_count
    }

    pub const fn average_body_bytes(&self) -> usize {
        self.average_body_bytes
    }

    pub const fn link_count(&self) -> usize {
        self.link_count
    }

    pub const fn asset_count(&self) -> usize {
        self.asset_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementEnvironment {
    name: String,
    description: String,
}

impl MeasurementEnvironment {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.trim().to_string(),
            description: description.trim().to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerformanceSample {
    target: PerformanceTarget,
    elapsed_ms: u64,
}

impl PerformanceSample {
    pub const fn new(target: PerformanceTarget, elapsed_ms: u64) -> Self {
        Self { target, elapsed_ms }
    }

    pub const fn target(self) -> PerformanceTarget {
        self.target
    }

    pub const fn elapsed_ms(self) -> u64 {
        self.elapsed_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceReport {
    fixture: PerformanceFixtureProfile,
    environment: MeasurementEnvironment,
    samples: Vec<PerformanceSample>,
}

impl PerformanceReport {
    pub fn new(
        fixture: PerformanceFixtureProfile,
        environment: MeasurementEnvironment,
        samples: Vec<PerformanceSample>,
    ) -> Self {
        Self {
            fixture,
            environment,
            samples,
        }
    }

    pub fn fixture(&self) -> &PerformanceFixtureProfile {
        &self.fixture
    }

    pub fn environment(&self) -> &MeasurementEnvironment {
        &self.environment
    }

    pub fn samples(&self) -> &[PerformanceSample] {
        &self.samples
    }

    pub fn p95_ms(&self, target: PerformanceTarget) -> Option<u64> {
        let mut elapsed: Vec<u64> = self
            .samples
            .iter()
            .filter(|sample| sample.target() == target)
            .map(|sample| sample.elapsed_ms())
            .collect();
        if elapsed.is_empty() {
            return None;
        }

        elapsed.sort_unstable();
        let nearest_rank = (elapsed.len() * 95).div_ceil(100);
        elapsed.get(nearest_rank.saturating_sub(1)).copied()
    }

    pub fn passes_ms_goal(&self, target: PerformanceTarget, goal_ms: u64) -> bool {
        self.p95_ms(target)
            .map(|elapsed_ms| elapsed_ms <= goal_ms)
            .unwrap_or(false)
    }
}

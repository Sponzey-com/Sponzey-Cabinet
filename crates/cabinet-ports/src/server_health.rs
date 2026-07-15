#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthComponentState {
    Healthy,
    Degraded,
    Unavailable,
}

impl HealthComponentState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthComponentSummary {
    name: String,
    state: HealthComponentState,
    error_code: Option<String>,
}

impl HealthComponentSummary {
    pub fn new(
        name: &str,
        state: HealthComponentState,
        error_code: Option<&str>,
    ) -> Result<Self, &'static str> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err("component name is required");
        }
        let error_code = error_code
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        Ok(Self {
            name: trimmed_name.to_string(),
            state,
            error_code,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> HealthComponentState {
        self.state
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }
}

pub trait ServerHealthProbe {
    fn check_components(&self) -> Vec<HealthComponentSummary>;
}

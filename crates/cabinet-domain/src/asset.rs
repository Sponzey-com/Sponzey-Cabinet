#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetId {
    value: String,
}

impl AssetId {
    pub fn from_sha256_hex(value: &str) -> Result<Self, AssetError> {
        let trimmed = value.trim();
        if trimmed.len() != 64
            || !trimmed
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(AssetError::InvalidContentHash);
        }
        Ok(Self {
            value: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFileName {
    value: String,
}

impl AssetFileName {
    pub fn new(value: &str) -> Result<Self, AssetError> {
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed == "."
            || trimmed == ".."
            || trimmed.contains("..")
            || trimmed.chars().any(char::is_control)
        {
            return Err(AssetError::InvalidFileName);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMediaType {
    value: String,
}

impl AssetMediaType {
    pub fn new(value: &str) -> Result<Self, AssetError> {
        let trimmed = value.trim();
        let mut parts = trimmed.split('/');
        let top_level = parts.next().unwrap_or_default();
        let subtype = parts.next().unwrap_or_default();
        if top_level.is_empty()
            || subtype.is_empty()
            || parts.next().is_some()
            || trimmed.contains(';')
            || trimmed.chars().any(char::is_control)
        {
            return Err(AssetError::InvalidMediaType);
        }
        Ok(Self {
            value: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetMetadata {
    id: AssetId,
    file_name: AssetFileName,
    media_type: AssetMediaType,
    byte_size: u64,
}

impl AssetMetadata {
    pub fn new(
        id: AssetId,
        file_name: AssetFileName,
        media_type: AssetMediaType,
        byte_size: u64,
    ) -> Result<Self, AssetError> {
        if byte_size == 0 {
            return Err(AssetError::InvalidByteSize);
        }
        Ok(Self {
            id,
            file_name,
            media_type,
            byte_size,
        })
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn file_name(&self) -> &AssetFileName {
        &self.file_name
    }

    pub fn media_type(&self) -> &AssetMediaType {
        &self.media_type
    }

    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReference {
    asset_id: AssetId,
    label: String,
}

impl AssetReference {
    pub fn new(asset_id: AssetId, label: &str) -> Result<Self, AssetError> {
        let trimmed = label.trim();
        if trimmed.is_empty() {
            return Err(AssetError::EmptyReferenceLabel);
        }
        Ok(Self {
            asset_id,
            label: trimmed.to_string(),
        })
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLifecycleState {
    Registered,
    Linked,
    Unlinked,
    Archived,
    Restored,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetLifecycleEvent {
    Register,
    Link,
    Unlink,
    Archive,
    Restore,
    MarkMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetLifecycleTransition {
    pub previous_state: AssetLifecycleState,
    pub event: AssetLifecycleEvent,
    pub next_state: AssetLifecycleState,
}

pub fn transition_asset_lifecycle(
    state: AssetLifecycleState,
    event: AssetLifecycleEvent,
) -> Result<AssetLifecycleTransition, AssetError> {
    let next_state = match (state, event) {
        (AssetLifecycleState::Registered, AssetLifecycleEvent::Register) => {
            AssetLifecycleState::Registered
        }
        (
            AssetLifecycleState::Registered
            | AssetLifecycleState::Unlinked
            | AssetLifecycleState::Restored,
            AssetLifecycleEvent::Link,
        ) => AssetLifecycleState::Linked,
        (AssetLifecycleState::Linked, AssetLifecycleEvent::Unlink) => AssetLifecycleState::Unlinked,
        (
            AssetLifecycleState::Registered
            | AssetLifecycleState::Linked
            | AssetLifecycleState::Unlinked
            | AssetLifecycleState::Restored,
            AssetLifecycleEvent::Archive,
        ) => AssetLifecycleState::Archived,
        (
            AssetLifecycleState::Archived | AssetLifecycleState::Missing,
            AssetLifecycleEvent::Restore,
        ) => AssetLifecycleState::Restored,
        (
            AssetLifecycleState::Registered
            | AssetLifecycleState::Linked
            | AssetLifecycleState::Unlinked
            | AssetLifecycleState::Restored,
            AssetLifecycleEvent::MarkMissing,
        ) => AssetLifecycleState::Missing,
        _ => {
            return Err(AssetError::InvalidLifecycleTransition { state, event });
        }
    };

    Ok(AssetLifecycleTransition {
        previous_state: state,
        event,
        next_state,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    InvalidContentHash,
    InvalidFileName,
    InvalidMediaType,
    InvalidByteSize,
    EmptyReferenceLabel,
    InvalidLifecycleTransition {
        state: AssetLifecycleState,
        event: AssetLifecycleEvent,
    },
}

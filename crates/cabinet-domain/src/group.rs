use crate::user::UserId;
use crate::workspace::WorkspaceId;

const GROUP_NAME_MAX_LEN: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    id: GroupId,
    workspace_id: WorkspaceId,
    name: GroupName,
}

impl Group {
    pub fn new(id: GroupId, workspace_id: WorkspaceId, name: GroupName) -> Self {
        Self {
            id,
            workspace_id,
            name,
        }
    }

    pub fn id(&self) -> &GroupId {
        &self.id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn name(&self) -> &GroupName {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupId {
    value: String,
}

impl GroupId {
    pub fn new(value: &str) -> Result<Self, GroupError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GroupError::EmptyId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(GroupError::InvalidId);
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
pub struct GroupName {
    value: String,
}

impl GroupName {
    pub fn new(value: &str) -> Result<Self, GroupError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GroupError::EmptyName);
        }
        if trimmed.chars().count() > GROUP_NAME_MAX_LEN {
            return Err(GroupError::NameTooLong {
                max: GROUP_NAME_MAX_LEN,
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(GroupError::InvalidNameCharacter);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn duplicate_key(&self) -> String {
        self.value.to_ascii_lowercase()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMembership {
    group_id: GroupId,
    user_id: UserId,
}

impl GroupMembership {
    pub fn new(group_id: GroupId, user_id: UserId) -> Self {
        Self { group_id, user_id }
    }

    pub fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupError {
    EmptyId,
    InvalidId,
    EmptyName,
    NameTooLong { max: usize },
    InvalidNameCharacter,
}

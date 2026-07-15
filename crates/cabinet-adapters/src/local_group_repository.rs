use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::group::{Group, GroupId, GroupMembership, GroupName};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::group_repository::{
    GroupRepository, GroupRepositoryError, MembershipMutationResult,
};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_GROUPS_DIR: &str = "groups";
pub const LOCAL_GROUPS_BY_ID_DIR: &str = "by-id";
pub const LOCAL_GROUPS_BY_NAME_DIR: &str = "by-name";
pub const LOCAL_GROUP_MEMBERSHIPS_DIR: &str = "memberships";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalGroupRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalGroupRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalGroupRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalGroupRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn workspace_root(&self, workspace_id: &WorkspaceId) -> PathBuf {
        self.root
            .join(LOCAL_GROUPS_DIR)
            .join(hex_encode(workspace_id.as_str()))
    }

    fn group_path(&self, workspace_id: &WorkspaceId, group_id: &GroupId) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_GROUPS_BY_ID_DIR)
            .join(format!("{}.group", hex_encode(group_id.as_str())))
    }

    fn name_index_path(&self, workspace_id: &WorkspaceId, name: &GroupName) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_GROUPS_BY_NAME_DIR)
            .join(format!("{}.idx", hex_encode(&name.duplicate_key())))
    }

    fn membership_path(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> PathBuf {
        self.workspace_root(workspace_id)
            .join(LOCAL_GROUP_MEMBERSHIPS_DIR)
            .join(hex_encode(group_id.as_str()))
            .join(format!("{}.member", hex_encode(user_id.as_str())))
    }

    fn lookup_name_index(
        &self,
        workspace_id: &WorkspaceId,
        name: &GroupName,
    ) -> Result<Option<GroupId>, GroupRepositoryError> {
        let path = self.name_index_path(workspace_id, name);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(GroupRepositoryError::StorageUnavailable),
        };
        let group_id = hex_decode(content.trim())?;
        GroupId::new(&group_id)
            .map(Some)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)
    }
}

impl GroupRepository for LocalGroupRepository {
    fn find_group_by_name(
        &self,
        workspace_id: &WorkspaceId,
        name: &GroupName,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        let Some(group_id) = self.lookup_name_index(workspace_id, name)? else {
            return Ok(None);
        };
        self.get_group(workspace_id, &group_id)?
            .ok_or(GroupRepositoryError::StorageUnavailable)
            .map(Some)
    }

    fn get_group(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
    ) -> Result<Option<Group>, GroupRepositoryError> {
        let path = self.group_path(workspace_id, group_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(GroupRepositoryError::StorageUnavailable),
        };
        decode_group(&content).map(Some)
    }

    fn save_group(
        &mut self,
        workspace_id: &WorkspaceId,
        group: Group,
    ) -> Result<(), GroupRepositoryError> {
        if group.workspace_id() != workspace_id {
            return Err(GroupRepositoryError::StorageUnavailable);
        }
        if self.group_path(workspace_id, group.id()).exists()
            || self
                .lookup_name_index(workspace_id, group.name())?
                .is_some()
        {
            return Err(GroupRepositoryError::Conflict);
        }
        write_text_atomically(
            &self.group_path(workspace_id, group.id()),
            encode_group(&group),
        )
        .map(|_| ())
        .map_err(|_| GroupRepositoryError::StorageUnavailable)?;
        write_text_atomically(
            &self.name_index_path(workspace_id, group.name()),
            format!("{}\n", hex_encode(group.id().as_str())),
        )
        .map(|_| ())
        .map_err(|_| GroupRepositoryError::StorageUnavailable)
    }

    fn has_membership(
        &self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<bool, GroupRepositoryError> {
        let path = self.membership_path(workspace_id, group_id, user_id);
        if !path.exists() {
            return Ok(false);
        }
        decode_membership(
            &fs::read_to_string(path).map_err(|_| GroupRepositoryError::StorageUnavailable)?,
        )?;
        Ok(true)
    }

    fn add_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        membership: GroupMembership,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        let path = self.membership_path(workspace_id, membership.group_id(), membership.user_id());
        if path.exists() {
            decode_membership(
                &fs::read_to_string(path).map_err(|_| GroupRepositoryError::StorageUnavailable)?,
            )?;
            return Ok(MembershipMutationResult::AlreadyApplied);
        }
        write_text_atomically(&path, encode_membership(&membership))
            .map(|_| MembershipMutationResult::Changed)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)
    }

    fn remove_membership(
        &mut self,
        workspace_id: &WorkspaceId,
        group_id: &GroupId,
        user_id: &UserId,
    ) -> Result<MembershipMutationResult, GroupRepositoryError> {
        let path = self.membership_path(workspace_id, group_id, user_id);
        if !path.exists() {
            return Ok(MembershipMutationResult::Missing);
        }
        fs::remove_file(path)
            .map(|_| MembershipMutationResult::Changed)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)
    }

    fn list_workspace_memberships(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<GroupMembership>, GroupRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_GROUP_MEMBERSHIPS_DIR);
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut memberships = Vec::new();
        collect_memberships(&root, &mut memberships)?;
        memberships.sort_by(|left, right| {
            left.group_id()
                .as_str()
                .cmp(right.group_id().as_str())
                .then(left.user_id().as_str().cmp(right.user_id().as_str()))
        });
        Ok(memberships)
    }

    fn list_workspace_groups(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Group>, GroupRepositoryError> {
        let root = self
            .workspace_root(workspace_id)
            .join(LOCAL_GROUPS_BY_ID_DIR);
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(GroupRepositoryError::StorageUnavailable),
        };
        let mut groups = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|_| GroupRepositoryError::StorageUnavailable)?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("group") {
                continue;
            }
            groups.push(decode_group(
                &fs::read_to_string(path).map_err(|_| GroupRepositoryError::StorageUnavailable)?,
            )?);
        }
        groups.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(groups)
    }
}

fn collect_memberships(
    root: &Path,
    memberships: &mut Vec<GroupMembership>,
) -> Result<(), GroupRepositoryError> {
    for entry in fs::read_dir(root).map_err(|_| GroupRepositoryError::StorageUnavailable)? {
        let path = entry
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?
            .path();
        if path.is_dir() {
            collect_memberships(&path, memberships)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("member") {
            memberships.push(decode_membership(
                &fs::read_to_string(path).map_err(|_| GroupRepositoryError::StorageUnavailable)?,
            )?);
        }
    }
    Ok(())
}

fn encode_group(group: &Group) -> String {
    format!(
        "id={}\nworkspace_id={}\nname={}\n",
        hex_encode(group.id().as_str()),
        hex_encode(group.workspace_id().as_str()),
        hex_encode(group.name().as_str())
    )
}

fn decode_group(content: &str) -> Result<Group, GroupRepositoryError> {
    let mut id = None;
    let mut workspace_id = None;
    let mut name = None;
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(GroupRepositoryError::StorageUnavailable)?;
        match key {
            "id" => id = Some(hex_decode(value)?),
            "workspace_id" => workspace_id = Some(hex_decode(value)?),
            "name" => name = Some(hex_decode(value)?),
            _ => return Err(GroupRepositoryError::StorageUnavailable),
        }
    }
    Ok(Group::new(
        GroupId::new(&id.ok_or(GroupRepositoryError::StorageUnavailable)?)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?,
        WorkspaceId::new(&workspace_id.ok_or(GroupRepositoryError::StorageUnavailable)?)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?,
        GroupName::new(&name.ok_or(GroupRepositoryError::StorageUnavailable)?)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?,
    ))
}

fn encode_membership(membership: &GroupMembership) -> String {
    format!(
        "group_id={}\nuser_id={}\n",
        hex_encode(membership.group_id().as_str()),
        hex_encode(membership.user_id().as_str())
    )
}

fn decode_membership(content: &str) -> Result<GroupMembership, GroupRepositoryError> {
    let mut group_id = None;
    let mut user_id = None;
    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(GroupRepositoryError::StorageUnavailable)?;
        match key {
            "group_id" => group_id = Some(hex_decode(value)?),
            "user_id" => user_id = Some(hex_decode(value)?),
            _ => return Err(GroupRepositoryError::StorageUnavailable),
        }
    }
    Ok(GroupMembership::new(
        GroupId::new(&group_id.ok_or(GroupRepositoryError::StorageUnavailable)?)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?,
        UserId::new(&user_id.ok_or(GroupRepositoryError::StorageUnavailable)?)
            .map_err(|_| GroupRepositoryError::StorageUnavailable)?,
    ))
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, GroupRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(GroupRepositoryError::StorageUnavailable);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| GroupRepositoryError::StorageUnavailable)?;
    String::from_utf8(bytes).map_err(|_| GroupRepositoryError::StorageUnavailable)
}

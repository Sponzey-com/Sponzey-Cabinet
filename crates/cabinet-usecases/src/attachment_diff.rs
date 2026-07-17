use std::collections::BTreeMap;

use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentDiff {
    Known(KnownAttachmentDiff),
    LegacyUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownAttachmentDiff {
    added: Vec<AssetReference>,
    removed: Vec<AssetReference>,
    relabeled: Vec<AttachmentLabelChange>,
    unchanged_count: usize,
}

impl KnownAttachmentDiff {
    pub fn added(&self) -> &[AssetReference] {
        &self.added
    }

    pub fn removed(&self) -> &[AssetReference] {
        &self.removed
    }

    pub fn relabeled(&self) -> &[AttachmentLabelChange] {
        &self.relabeled
    }

    pub const fn unchanged_count(&self) -> usize {
        self.unchanged_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentLabelChange {
    asset_id: AssetId,
    before_label: String,
    after_label: String,
}

impl AttachmentLabelChange {
    pub const fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn before_label(&self) -> &str {
        &self.before_label
    }

    pub fn after_label(&self) -> &str {
        &self.after_label
    }
}

pub fn compare_attachment_snapshots(
    left: &AttachmentSnapshotState,
    right: &AttachmentSnapshotState,
) -> AttachmentDiff {
    let (Some(left), Some(right)) = (left.references(), right.references()) else {
        return AttachmentDiff::LegacyUnknown;
    };
    let left = by_asset_id(left);
    let right = by_asset_id(right);
    let mut removed = Vec::new();
    let mut relabeled = Vec::new();
    let mut unchanged_count = 0;

    for (asset_id, previous) in &left {
        match right.get(asset_id) {
            None => removed.push(previous.clone()),
            Some(next) if next.label() == previous.label() => unchanged_count += 1,
            Some(next) => relabeled.push(AttachmentLabelChange {
                asset_id: previous.asset_id().clone(),
                before_label: previous.label().to_string(),
                after_label: next.label().to_string(),
            }),
        }
    }
    let added = right
        .iter()
        .filter(|(asset_id, _)| !left.contains_key(*asset_id))
        .map(|(_, reference)| reference.clone())
        .collect();

    AttachmentDiff::Known(KnownAttachmentDiff {
        added,
        removed,
        relabeled,
        unchanged_count,
    })
}

fn by_asset_id(references: &[AssetReference]) -> BTreeMap<String, AssetReference> {
    references
        .iter()
        .map(|reference| (reference.asset_id().as_str().to_string(), reference.clone()))
        .collect()
}

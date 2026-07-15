use crate::local_atomic_file::write_text_atomically;
use cabinet_domain::document::{DocumentId, DocumentSlug, DocumentTitle};
use cabinet_domain::link::{Backlink, DocumentLink, LinkTarget, SourceRange};
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::link_index::{
    BacklinkPage, BacklinkPageReader, BacklinkPageRequest, LinkIndex, LinkIndexError,
    LinkProjectionRecord,
};
use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
const HEADER: &str = "schema\t1";
#[derive(Debug, Clone)]
pub struct DurableLocalLinkIndex {
    root: PathBuf,
}
impl DurableLocalLinkIndex {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    fn workspace_root(&self, w: &WorkspaceId) -> PathBuf {
        self.root.join("link-projections").join(hex(w.as_str()))
    }
    fn path(&self, w: &WorkspaceId, d: &DocumentId) -> PathBuf {
        self.workspace_root(w)
            .join(format!("{}.snapshot", hex(d.as_str())))
    }
    fn records(&self, w: &WorkspaceId) -> Result<Vec<LinkProjectionRecord>, LinkIndexError> {
        let entries = match fs::read_dir(self.workspace_root(w)) {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(vec![]),
            Err(_) => return Err(LinkIndexError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|e| {
                e.map(|x| x.path())
                    .map_err(|_| LinkIndexError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        paths
            .into_iter()
            .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("snapshot"))
            .map(|p| read(&p))
            .collect()
    }

    fn sorted_projection_paths(&self, w: &WorkspaceId) -> Result<Vec<PathBuf>, LinkIndexError> {
        let entries = match fs::read_dir(self.workspace_root(w)) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(LinkIndexError::StorageUnavailable),
        };
        let mut paths = entries
            .map(|entry| {
                entry
                    .map(|value| value.path())
                    .map_err(|_| LinkIndexError::StorageUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("snapshot"));
        paths.sort();
        Ok(paths)
    }
}

impl BacklinkPageReader for DurableLocalLinkIndex {
    fn list_backlinks_page(
        &self,
        workspace_id: &WorkspaceId,
        target_document_id: &DocumentId,
        request: BacklinkPageRequest,
    ) -> Result<BacklinkPage, LinkIndexError> {
        let mut matched = 0;
        let mut records = Vec::with_capacity(request.limit() + 1);
        'paths: for path in self.sorted_projection_paths(workspace_id)? {
            let projection = read(&path)?;
            for backlink in projection.backlinks() {
                if backlink.target_document_id() != target_document_id {
                    continue;
                }
                if matched < request.offset() {
                    matched += 1;
                    continue;
                }
                records.push(backlink.clone());
                matched += 1;
                if records.len() > request.limit() {
                    break 'paths;
                }
            }
        }
        let has_more = records.len() > request.limit();
        records.truncate(request.limit());
        let next_offset = has_more.then_some(request.offset() + records.len());
        Ok(BacklinkPage::new(records, next_offset))
    }
}
impl LinkIndex for DurableLocalLinkIndex {
    fn replace_document_links(
        &mut self,
        w: &WorkspaceId,
        r: LinkProjectionRecord,
    ) -> Result<(), LinkIndexError> {
        write_text_atomically(&self.path(w, r.source_document_id()), encode(&r))
            .map(|_| ())
            .map_err(|_| LinkIndexError::StorageUnavailable)
    }
    fn delete_document_links(
        &mut self,
        w: &WorkspaceId,
        d: &DocumentId,
    ) -> Result<(), LinkIndexError> {
        match fs::remove_file(self.path(w, d)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(_) => Err(LinkIndexError::StorageUnavailable),
        }
    }
    fn get_document_links(
        &self,
        w: &WorkspaceId,
        d: &DocumentId,
    ) -> Result<Option<LinkProjectionRecord>, LinkIndexError> {
        match fs::read_to_string(self.path(w, d)) {
            Ok(t) => decode(&t).map(Some),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(LinkIndexError::StorageUnavailable),
        }
    }
    fn list_backlinks(
        &self,
        w: &WorkspaceId,
        t: &DocumentId,
    ) -> Result<Vec<Backlink>, LinkIndexError> {
        Ok(self
            .records(w)?
            .into_iter()
            .flat_map(|r| r.backlinks().to_vec())
            .filter(|b| b.target_document_id() == t)
            .collect())
    }
    fn list_unresolved_links(&self, w: &WorkspaceId) -> Result<Vec<DocumentLink>, LinkIndexError> {
        Ok(self
            .records(w)?
            .into_iter()
            .flat_map(|r| r.unresolved_links().to_vec())
            .collect())
    }
    fn list_orphan_documents(
        &self,
        w: &WorkspaceId,
        ids: &[DocumentId],
    ) -> Result<Vec<DocumentId>, LinkIndexError> {
        let incoming = self
            .records(w)?
            .into_iter()
            .flat_map(|r| r.backlinks().to_vec())
            .map(|b| b.target_document_id().as_str().to_string())
            .collect::<HashSet<_>>();
        Ok(ids
            .iter()
            .filter(|id| !incoming.contains(id.as_str()))
            .cloned()
            .collect())
    }
}
fn encode(r: &LinkProjectionRecord) -> String {
    let mut lines = vec![format!("source\t{}", hex(r.source_document_id().as_str()))];
    lines.extend(r.backlinks().iter().map(|b| {
        format!(
            "backlink\t{}\t{}\t{}",
            hex(b.target_document_id().as_str()),
            b.source_range().start(),
            b.source_range().end()
        )
    }));
    lines.extend(
        r.unresolved_links()
            .iter()
            .filter_map(|l| match l.target() {
                LinkTarget::Unresolved(s) => Some(format!(
                    "unresolved\t{}\t{}\t{}",
                    hex(s.as_str()),
                    l.source_range().start(),
                    l.source_range().end()
                )),
                _ => None,
            }),
    );
    let payload = format!("{}\n", lines.join("\n"));
    format!(
        "{HEADER}\nchecksum\t{:016x}\n{payload}",
        checksum(payload.as_bytes())
    )
}
fn read(p: &Path) -> Result<LinkProjectionRecord, LinkIndexError> {
    fs::read_to_string(p)
        .map_err(|_| LinkIndexError::StorageUnavailable)
        .and_then(|t| decode(&t))
}
fn decode(t: &str) -> Result<LinkProjectionRecord, LinkIndexError> {
    let mut lines = t.lines();
    if lines.next() != Some(HEADER) {
        return Err(LinkIndexError::CorruptedProjection);
    }
    let expected = lines
        .next()
        .and_then(|l| l.strip_prefix("checksum\t"))
        .and_then(|v| u64::from_str_radix(v, 16).ok())
        .ok_or(LinkIndexError::CorruptedProjection)?;
    let payload = format!("{}\n", lines.collect::<Vec<_>>().join("\n"));
    if checksum(payload.as_bytes()) != expected {
        return Err(LinkIndexError::CorruptedProjection);
    }
    let mut source = None;
    let mut backlinks = vec![];
    let mut unresolved = vec![];
    for line in payload.lines() {
        let f = line.split('\t').collect::<Vec<_>>();
        match f.as_slice() {
            ["source", v] if source.is_none() => source = Some(doc(v)?),
            ["backlink", target, start, end] => {
                let s = source.clone().ok_or(LinkIndexError::CorruptedProjection)?;
                backlinks.push(Backlink::new(s, doc(target)?, range(start, end)?))
            }
            ["unresolved", slug, start, end] => {
                let s = source.clone().ok_or(LinkIndexError::CorruptedProjection)?;
                let title = DocumentTitle::new(&unhex(slug)?)
                    .map_err(|_| LinkIndexError::CorruptedProjection)?;
                let slug = DocumentSlug::from_title(&title)
                    .map_err(|_| LinkIndexError::CorruptedProjection)?;
                unresolved.push(DocumentLink::new(
                    s,
                    LinkTarget::unresolved(slug),
                    range(start, end)?,
                ))
            }
            _ => return Err(LinkIndexError::CorruptedProjection),
        }
    }
    LinkProjectionRecord::new(
        source.ok_or(LinkIndexError::CorruptedProjection)?,
        backlinks,
        unresolved,
    )
    .map_err(|_| LinkIndexError::CorruptedProjection)
}
fn range(a: &str, b: &str) -> Result<SourceRange, LinkIndexError> {
    SourceRange::new(
        a.parse().map_err(|_| LinkIndexError::CorruptedProjection)?,
        b.parse().map_err(|_| LinkIndexError::CorruptedProjection)?,
    )
    .map_err(|_| LinkIndexError::CorruptedProjection)
}
fn doc(v: &str) -> Result<DocumentId, LinkIndexError> {
    DocumentId::new(&unhex(v)?).map_err(|_| LinkIndexError::CorruptedProjection)
}
fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325, |h, x| {
        (h ^ u64::from(*x)).wrapping_mul(0x100000001b3)
    })
}
fn hex(v: &str) -> String {
    v.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(v: &str) -> Result<String, LinkIndexError> {
    if v.len() % 2 != 0 {
        return Err(LinkIndexError::CorruptedProjection);
    }
    let bytes = v
        .as_bytes()
        .chunks_exact(2)
        .map(|p| {
            std::str::from_utf8(p)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
                .ok_or(LinkIndexError::CorruptedProjection)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| LinkIndexError::CorruptedProjection)
}

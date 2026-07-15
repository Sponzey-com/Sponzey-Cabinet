use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_import_source::{
    LocalAssetImportSource, LocalAssetImportSourceConfig,
};
use cabinet_domain::asset::AssetImportHandle;
use cabinet_ports::asset_import_source::{AssetImportSource, AssetImportSourceError};

#[test]
fn registry_returns_safe_descriptor_and_bounded_chunks_without_exposing_path() {
    let temp = TempRoot::new("bounded");
    let path = temp.path().join("notes.txt");
    fs::write(&path, b"hello").expect("fixture");
    let handle = AssetImportHandle::new("picker:one").expect("handle");
    let mut source =
        LocalAssetImportSource::new(LocalAssetImportSourceConfig::new(2).expect("config"));

    let descriptor = source
        .register_selected_file(handle.clone(), &path)
        .expect("register");
    let first = source.read_chunk(&handle, 0, 2).expect("first");
    let last = source.read_chunk(&handle, 4, 2).expect("last");

    assert_eq!(descriptor.file_name().as_str(), "notes.txt");
    assert_eq!(descriptor.media_type().as_str(), "text/plain");
    assert_eq!(descriptor.byte_size(), 5);
    assert_eq!(source.describe(&handle).expect("describe"), descriptor);
    assert_eq!(first.bytes(), b"he");
    assert!(!first.is_eof());
    assert_eq!(last.bytes(), b"o");
    assert!(last.is_eof());
    assert!(!format!("{descriptor:?}").contains(temp.path().to_string_lossy().as_ref()));
}

#[test]
fn registry_rejects_invalid_config_unsafe_file_types_and_zero_byte_file() {
    assert_eq!(
        LocalAssetImportSourceConfig::new(0).expect_err("zero config"),
        AssetImportSourceError::InvalidChunkLimit
    );
    let temp = TempRoot::new("unsafe");
    let empty = temp.path().join("empty.txt");
    fs::write(&empty, []).expect("empty fixture");
    let mut source =
        LocalAssetImportSource::new(LocalAssetImportSourceConfig::new(4).expect("config"));
    assert_eq!(
        source
            .register_selected_file(
                AssetImportHandle::new("picker:empty").expect("handle"),
                &empty
            )
            .expect_err("empty"),
        AssetImportSourceError::UnsafeSource
    );
    assert_eq!(
        source
            .register_selected_file(
                AssetImportHandle::new("picker:dir").expect("handle"),
                temp.path()
            )
            .expect_err("directory"),
        AssetImportSourceError::UnsafeSource
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, b"data").expect("target");
        symlink(&target, &link).expect("symlink");
        assert_eq!(
            source
                .register_selected_file(
                    AssetImportHandle::new("picker:link").expect("handle"),
                    &link
                )
                .expect_err("symlink"),
            AssetImportSourceError::UnsafeSource
        );
    }
}

#[test]
fn registry_rejects_oversized_reads_and_detects_source_mutation() {
    let temp = TempRoot::new("mutation");
    let path = temp.path().join("mutable.bin");
    fs::write(&path, b"1234").expect("fixture");
    let handle = AssetImportHandle::new("picker:mutable").expect("handle");
    let mut source =
        LocalAssetImportSource::new(LocalAssetImportSourceConfig::new(2).expect("config"));
    source
        .register_selected_file(handle.clone(), &path)
        .expect("register");

    assert_eq!(
        source.read_chunk(&handle, 0, 3).expect_err("oversized"),
        AssetImportSourceError::InvalidChunkLimit
    );
    fs::write(&path, b"changed").expect("mutate");
    assert_eq!(
        source.read_chunk(&handle, 0, 2).expect_err("changed"),
        AssetImportSourceError::SourceChanged
    );
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "sponzey-asset-import-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp root");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

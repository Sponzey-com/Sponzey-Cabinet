use std::fs;
use std::path::PathBuf;

use cabinet_core::migration::{
    Phase002FixtureError, Phase002FixtureRecord, Phase002FixtureRecordKind,
    Phase002MigrationFixture,
};

pub const PHASE002_FIXTURE_FILE: &str = "phase002-self-host-fixture.tsv";
const PHASE002_FIXTURE_TEMP_FILE: &str = "phase002-self-host-fixture.tsv.tmp";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPhase002MigrationFixtureStore {
    metadata_dir: PathBuf,
}

impl LocalPhase002MigrationFixtureStore {
    pub fn new(metadata_dir: PathBuf) -> Self {
        Self { metadata_dir }
    }

    pub fn save_fixture(
        &self,
        fixture: &Phase002MigrationFixture,
    ) -> Result<(), LocalPhase002FixtureStoreError> {
        self.save_fixture_with_failure(fixture, None)
    }

    pub fn save_fixture_with_failure_for_test(
        &self,
        fixture: &Phase002MigrationFixture,
        failure: LocalPhase002FixtureFailure,
    ) -> Result<(), LocalPhase002FixtureStoreError> {
        self.save_fixture_with_failure(fixture, Some(failure))
    }

    pub fn load_fixture(&self) -> Result<Phase002MigrationFixture, LocalPhase002FixtureStoreError> {
        let content = fs::read_to_string(self.fixture_path())
            .map_err(|_| LocalPhase002FixtureStoreError::ReadFailed)?;
        decode_fixture(&content)
    }

    fn save_fixture_with_failure(
        &self,
        fixture: &Phase002MigrationFixture,
        failure: Option<LocalPhase002FixtureFailure>,
    ) -> Result<(), LocalPhase002FixtureStoreError> {
        if !self.metadata_dir.is_dir() {
            return Err(LocalPhase002FixtureStoreError::DirectoryMissing);
        }
        fixture
            .validate()
            .map_err(LocalPhase002FixtureStoreError::CorruptedFixture)?;

        let temp_path = self.temp_path();
        fs::write(&temp_path, encode_fixture(fixture))
            .map_err(|_| LocalPhase002FixtureStoreError::WriteFailed)?;
        if failure == Some(LocalPhase002FixtureFailure::BeforeCommit) {
            return Err(LocalPhase002FixtureStoreError::WriteFailed);
        }

        fs::rename(temp_path, self.fixture_path())
            .map_err(|_| LocalPhase002FixtureStoreError::WriteFailed)?;
        Ok(())
    }

    fn fixture_path(&self) -> PathBuf {
        self.metadata_dir.join(PHASE002_FIXTURE_FILE)
    }

    fn temp_path(&self) -> PathBuf {
        self.metadata_dir.join(PHASE002_FIXTURE_TEMP_FILE)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPhase002FixtureFailure {
    BeforeCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPhase002FixtureStoreError {
    DirectoryMissing,
    WriteFailed,
    ReadFailed,
    CorruptedFixture(Phase002FixtureError),
}

fn encode_fixture(fixture: &Phase002MigrationFixture) -> String {
    fixture
        .records()
        .iter()
        .map(encode_record)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn encode_record(record: &Phase002FixtureRecord) -> String {
    let fields = record
        .public_fields()
        .iter()
        .map(|(key, value)| format!("{}={}", hex_encode(key), hex_encode(value)))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{}\t{}\t{}\t{}",
        record.kind().as_str(),
        hex_encode(record.id()),
        record
            .sensitive_payload()
            .map_or_else(String::new, hex_encode),
        fields
    )
}

fn decode_fixture(
    content: &str,
) -> Result<Phase002MigrationFixture, LocalPhase002FixtureStoreError> {
    let records = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(decode_record)
        .collect::<Result<Vec<_>, LocalPhase002FixtureStoreError>>()?;
    Phase002MigrationFixture::new(records).map_err(LocalPhase002FixtureStoreError::CorruptedFixture)
}

fn decode_record(line: &str) -> Result<Phase002FixtureRecord, LocalPhase002FixtureStoreError> {
    let columns = line.split('\t').collect::<Vec<_>>();
    if columns.len() != 4 {
        return Err(LocalPhase002FixtureStoreError::CorruptedFixture(
            Phase002FixtureError::CorruptedRecord,
        ));
    }
    let kind = Phase002FixtureRecordKind::parse(columns[0]).ok_or(
        LocalPhase002FixtureStoreError::CorruptedFixture(Phase002FixtureError::CorruptedRecord),
    )?;
    let id = hex_decode(columns[1])?;
    let sensitive_payload = if columns[2].is_empty() {
        None
    } else {
        Some(hex_decode(columns[2])?)
    };
    let public_fields = if columns[3].is_empty() {
        Vec::new()
    } else {
        columns[3]
            .split(';')
            .map(|pair| {
                let (key, value) = pair.split_once('=').ok_or(
                    LocalPhase002FixtureStoreError::CorruptedFixture(
                        Phase002FixtureError::CorruptedRecord,
                    ),
                )?;
                Ok((hex_decode(key)?, hex_decode(value)?))
            })
            .collect::<Result<Vec<_>, LocalPhase002FixtureStoreError>>()?
    };
    Phase002FixtureRecord::new(
        kind,
        &id,
        public_fields
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect(),
        sensitive_payload.as_deref(),
    )
    .map_err(LocalPhase002FixtureStoreError::CorruptedFixture)
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, LocalPhase002FixtureStoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(LocalPhase002FixtureStoreError::CorruptedFixture(
            Phase002FixtureError::CorruptedRecord,
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for index in (0..value.len()).step_by(2) {
        let byte = u8::from_str_radix(&value[index..index + 2], 16).map_err(|_| {
            LocalPhase002FixtureStoreError::CorruptedFixture(Phase002FixtureError::CorruptedRecord)
        })?;
        bytes.push(byte);
    }
    String::from_utf8(bytes).map_err(|_| {
        LocalPhase002FixtureStoreError::CorruptedFixture(Phase002FixtureError::CorruptedRecord)
    })
}

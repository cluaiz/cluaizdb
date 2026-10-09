use std::io::Read;
use std::path::Path;

use tracing::{info, warn};

use cluaizdb_errors::StorageError;

use crate::log_entry::{WalEntry, WalOperation};

/// Result of replaying a single WAL segment.
pub struct RecoveryResult {
    /// Total entries found in the WAL.
    pub total_entries: usize,
    /// Entries that passed checksum validation and were replayed.
    pub replayed: usize,
    /// Entries that failed checksum validation and were skipped.
    pub skipped_corrupt: usize,
}

/// Scan all WAL segment files in the given directory and replay
/// any uncommitted entries back into the storage engine.
///
/// This is called once at startup, before the server begins accepting requests.
///
/// ## Recovery Logic
/// 1. Find all `wal_NNNNN.log` files in `wal_dir`, sorted by sequence number.
/// 2. For each file, deserialize entries one by one.
/// 3. Validate each entry's checksum. Skip corrupt entries with a warning.
/// 4. Apply valid `Write` entries to LMDB and discard `Delete` entries
///    (LMDB handles its own deletions via the engine).
///
/// # Errors
/// Returns `StorageError::WalRecoveryFailed` if the WAL directory cannot be read.
pub fn recover_from_wal(
    wal_dir: &Path,
    apply_write: &mut dyn FnMut(WalEntry) -> Result<(), StorageError>,
) -> Result<RecoveryResult, StorageError> {
    if !wal_dir.exists() {
        info!("No WAL directory found — clean startup, skipping recovery.");
        return Ok(RecoveryResult {
            total_entries: 0,
            replayed: 0,
            skipped_corrupt: 0,
        });
    }

    // Collect and sort all WAL segment files by name.
    let mut segment_files: Vec<std::fs::DirEntry> = std::fs::read_dir(wal_dir)
        .map_err(|e| StorageError::WalRecoveryFailed(e.to_string()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("wal_")
        })
        .collect();

    segment_files.sort_by_key(|e| e.file_name());

    let mut total = 0usize;
    let mut replayed = 0usize;
    let mut skipped = 0usize;

    for segment in &segment_files {
        let path = segment.path();
        info!(path = %path.display(), "Replaying WAL segment");

        let mut file = std::fs::File::open(&path)
            .map_err(|e| StorageError::WalRecoveryFailed(e.to_string()))?;

        loop {
            // Read the 4-byte length prefix.
            let mut len_buf = [0u8; 4];
            match file.read_exact(&mut len_buf) {
                Ok(_) => {}
                // Clean EOF — no more entries in this segment.
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    warn!(error = %e, "WAL read error — stopping segment replay early");
                    break;
                }
            }

            let entry_len = u32::from_le_bytes(len_buf) as usize;
            let mut entry_buf = vec![0u8; entry_len];

            if let Err(e) = file.read_exact(&mut entry_buf) {
                warn!(error = %e, "WAL entry body truncated — skipping");
                skipped += 1;
                continue;
            }

            total += 1;

            // Deserialize the WAL entry.
            let entry: WalEntry = match serde_json::from_slice(&entry_buf) {
                Ok(e) => e,
                Err(e) => {
                    warn!(error = %e, "WAL entry deserialization failed — corrupt entry, skipping");
                    skipped += 1;
                    continue;
                }
            };

            // Validate checksum.
            if !entry.is_valid() {
                warn!(sequence = entry.sequence, "WAL entry checksum mismatch — skipping");
                skipped += 1;
                continue;
            }

            // Only replay Write operations. Delete operations are idempotent.
            match &entry.operation {
                WalOperation::Write { .. } => {
                    if let Err(e) = apply_write(entry) {
                        warn!(error = %e, "WAL replay write failed — skipping entry");
                        skipped += 1;
                        continue;
                    }
                    replayed += 1;
                }
                WalOperation::Delete { .. } => {
                    // Deletes are handled by LMDB itself — no replay needed.
                    replayed += 1;
                }
            }
        }
    }

    info!(
        total_entries = total,
        replayed = replayed,
        skipped_corrupt = skipped,
        "WAL recovery complete"
    );

    Ok(RecoveryResult {
        total_entries: total,
        replayed,
        skipped_corrupt: skipped,
    })
}

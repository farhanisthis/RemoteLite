//! File transfer session implementations for sender and receiver.
//!
//! Provides streamed chunking, flow control, cryptographic integrity
//! verification via SHA-256, and atomic .part file staging to prevent
//! corrupted or incomplete files from entering the user's filesystem.

use crate::error::TransferError;
use crate::integrity::sha256_hex_reader;
use crate::path_safety::{resolve_safe_path, sanitize_filename};
use crate::{CHUNK_SIZE, MAX_FILE_SIZE};
use openremote_protocol::messages::{
    TransferAck, TransferAnnounce, TransferCancel, TransferComplete, TransferDirection,
};
use openremote_protocol::types::TransferId;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// A single discrete chunk of file data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub transfer_id: TransferId,
    pub sequence: u32,
    pub byte_offset: u64,
    pub data: Vec<u8>,
}

/// Lifecycle status of an active file transfer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed(String),
}

/// Dynamic progress metrics for a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub transfer_id: TransferId,
    pub file_name: String,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub status: TransferStatus,
}

impl TransferProgress {
    /// Percentage completed between 0.0 and 100.0.
    pub fn percent(&self) -> f32 {
        if self.total_bytes == 0 {
            100.0
        } else {
            ((self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0) as f32
        }
    }
}

/// Handles sending a local file in discrete, bounded chunks.
pub struct FileSender {
    transfer_id: TransferId,
    source_path: PathBuf,
    file_name: String,
    file_size: u64,
    sha256: String,
    direction: TransferDirection,
    file: File,
    current_sequence: u32,
    transferred_bytes: u64,
    status: TransferStatus,
}

impl FileSender {
    /// Initialize a new file sender for the specified source file.
    pub fn new<P: AsRef<Path>>(
        source_path: P,
        direction: TransferDirection,
    ) -> Result<Self, TransferError> {
        let source_path = source_path.as_ref().to_path_buf();
        if !source_path.exists() {
            return Err(TransferError::FileNotFound {
                path: source_path.to_string_lossy().into(),
            });
        }

        let metadata = fs::metadata(&source_path)?;
        let file_size = metadata.len();

        if file_size > MAX_FILE_SIZE {
            return Err(TransferError::FileTooLarge {
                size: file_size,
                max: MAX_FILE_SIZE,
            });
        }

        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(sanitize_filename)
            .transpose()?
            .ok_or_else(|| TransferError::InvalidFilename {
                name: "missing filename".into(),
            })?;

        // Calculate full SHA-256 digest
        let file_for_hash = File::open(&source_path)?;
        let sha256 = sha256_hex_reader(file_for_hash)?;

        let file = File::open(&source_path)?;
        let transfer_id = TransferId::new();

        info!(
            transfer_id = %transfer_id,
            file_name = %file_name,
            file_size = file_size,
            sha256 = %sha256,
            "Initialized FileSender"
        );

        Ok(Self {
            transfer_id,
            source_path,
            file_name,
            file_size,
            sha256,
            direction,
            file,
            current_sequence: 0,
            transferred_bytes: 0,
            status: TransferStatus::Pending,
        })
    }

    pub fn transfer_id(&self) -> TransferId {
        self.transfer_id.clone()
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Generate the announcement message to initiate transfer on the peer side.
    pub fn announce_message(&self) -> TransferAnnounce {
        TransferAnnounce {
            transfer_id: self.transfer_id.clone(),
            file_name: self.file_name.clone(),
            file_size: self.file_size,
            mime_type: None,
            sha256: self.sha256.clone(),
            direction: self.direction.clone(),
        }
    }

    /// Read the next chunk from the source file. Returns `None` when EOF is reached.
    pub fn read_next_chunk(&mut self, max_size: usize) -> Result<Option<FileChunk>, TransferError> {
        if self.status == TransferStatus::Cancelled {
            return Err(TransferError::Cancelled);
        }

        let chunk_size = max_size.min(CHUNK_SIZE);
        let mut buffer = vec![0u8; chunk_size];

        let bytes_read = self.file.read(&mut buffer)?;
        if bytes_read == 0 {
            if self.transferred_bytes >= self.file_size {
                self.status = TransferStatus::Completed;
            }
            return Ok(None);
        }

        buffer.truncate(bytes_read);
        let offset = self.transferred_bytes;
        let sequence = self.current_sequence;

        self.current_sequence += 1;
        self.transferred_bytes += bytes_read as u64;
        self.status = TransferStatus::InProgress;

        Ok(Some(FileChunk {
            transfer_id: self.transfer_id.clone(),
            sequence,
            byte_offset: offset,
            data: buffer,
        }))
    }

    /// Generate completion notification.
    pub fn complete_message(&self) -> TransferComplete {
        TransferComplete {
            transfer_id: self.transfer_id.clone(),
        }
    }

    /// Cancel the ongoing transfer.
    pub fn cancel(&mut self, reason: &str) -> TransferCancel {
        self.status = TransferStatus::Cancelled;
        warn!(transfer_id = %self.transfer_id, reason = %reason, "Transfer cancelled by sender");
        TransferCancel {
            transfer_id: self.transfer_id.clone(),
            reason: reason.to_string(),
        }
    }

    /// Current transfer progress.
    pub fn progress(&self) -> TransferProgress {
        TransferProgress {
            transfer_id: self.transfer_id.clone(),
            file_name: self.file_name.clone(),
            total_bytes: self.file_size,
            transferred_bytes: self.transferred_bytes,
            status: self.status.clone(),
        }
    }
}

/// Handles receiving a file, staging it into a .part file, verifying SHA-256,
/// and atomically renaming it to the final destination upon completion.
pub struct FileReceiver {
    announce: TransferAnnounce,
    destination_path: PathBuf,
    part_path: PathBuf,
    part_file: Option<File>,
    transferred_bytes: u64,
    last_sequence: Option<u32>,
    status: TransferStatus,
}

impl FileReceiver {
    /// Initialize a new file receiver for the incoming transfer announcement.
    pub fn new<P: AsRef<Path>>(
        approved_dir: P,
        announce: TransferAnnounce,
    ) -> Result<Self, TransferError> {
        let approved_dir = approved_dir.as_ref();
        if announce.file_size > MAX_FILE_SIZE {
            return Err(TransferError::FileTooLarge {
                size: announce.file_size,
                max: MAX_FILE_SIZE,
            });
        }

        // Validate filename & resolve safe path
        let destination_path = resolve_safe_path(approved_dir, &announce.file_name)?;
        let part_filename = format!("{}.part", sanitize_filename(&announce.file_name)?);
        let part_path = approved_dir.join(&part_filename);

        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create or truncate the .part staging file
        let part_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&part_path)?;

        info!(
            transfer_id = %announce.transfer_id,
            target = ?destination_path,
            part = ?part_path,
            "Initialized FileReceiver"
        );

        Ok(Self {
            announce,
            destination_path,
            part_path,
            part_file: Some(part_file),
            transferred_bytes: 0,
            last_sequence: None,
            status: TransferStatus::Pending,
        })
    }

    pub fn transfer_id(&self) -> TransferId {
        self.announce.transfer_id.clone()
    }

    /// Write an incoming file chunk to the .part file.
    pub fn write_chunk(&mut self, chunk: FileChunk) -> Result<TransferAck, TransferError> {
        if self.status == TransferStatus::Cancelled {
            return Err(TransferError::Cancelled);
        }

        if chunk.transfer_id != self.announce.transfer_id {
            return Err(TransferError::NotFound {
                id: chunk.transfer_id.to_string(),
            });
        }

        let file = self
            .part_file
            .as_mut()
            .ok_or_else(|| TransferError::Io(std::io::Error::other("File closed")))?;

        // Seek to chunk offset and write data
        file.seek(SeekFrom::Start(chunk.byte_offset))?;
        file.write_all(&chunk.data)?;

        self.transferred_bytes += chunk.data.len() as u64;
        self.last_sequence = Some(chunk.sequence);
        self.status = TransferStatus::InProgress;

        Ok(TransferAck {
            transfer_id: self.announce.transfer_id.clone(),
            last_received_sequence: chunk.sequence,
        })
    }

    /// Finalize the transfer: flush staging file, compute SHA-256, verify integrity,
    /// and atomically rename `.part` to destination file.
    pub fn finish(&mut self) -> Result<PathBuf, TransferError> {
        if let Some(mut file) = self.part_file.take() {
            file.flush()?;
            file.seek(SeekFrom::Start(0))?;

            // Compute SHA-256 of the completed .part file
            let actual_sha256 = sha256_hex_reader(file)?;

            if actual_sha256 != self.announce.sha256 {
                error!(
                    expected = %self.announce.sha256,
                    actual = %actual_sha256,
                    "File integrity check failed! Purging corrupted staging file"
                );
                self.status = TransferStatus::Failed("Integrity check failed".into());
                let _ = fs::remove_file(&self.part_path);
                return Err(TransferError::IntegrityCheckFailed {
                    expected: self.announce.sha256.clone(),
                    actual: actual_sha256,
                });
            }

            // Atomic rename from .part to destination
            fs::rename(&self.part_path, &self.destination_path)?;
            self.status = TransferStatus::Completed;

            info!(
                transfer_id = %self.announce.transfer_id,
                destination = ?self.destination_path,
                "Transfer completed and verified successfully"
            );

            Ok(self.destination_path.clone())
        } else {
            Err(TransferError::Io(std::io::Error::other(
                "Staging file already closed",
            )))
        }
    }

    /// Cancel the transfer and remove the .part staging file immediately.
    pub fn cancel(&mut self, reason: &str) -> Result<(), TransferError> {
        self.status = TransferStatus::Cancelled;
        self.part_file.take(); // Close handle before removal
        if self.part_path.exists() {
            let _ = fs::remove_file(&self.part_path);
        }
        warn!(
            transfer_id = %self.announce.transfer_id,
            reason = %reason,
            "Transfer cancelled by receiver and staging file purged"
        );
        Ok(())
    }

    /// Current receiver progress.
    pub fn progress(&self) -> TransferProgress {
        TransferProgress {
            transfer_id: self.announce.transfer_id.clone(),
            file_name: self.announce.file_name.clone(),
            total_bytes: self.announce.file_size,
            transferred_bytes: self.transferred_bytes,
            status: self.status.clone(),
        }
    }
}

impl Drop for FileReceiver {
    fn drop(&mut self) {
        // If dropped before completion (e.g. panic or abrupt disconnect), clean up staging file
        if matches!(
            self.status,
            TransferStatus::Pending | TransferStatus::InProgress
        ) {
            self.part_file.take();
            if self.part_path.exists() {
                let _ = fs::remove_file(&self.part_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::sha256_hex;

    #[test]
    fn test_sender_and_receiver_round_trip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_path = temp_dir.path().join("source.txt");
        let test_data = b"Hello, OpenRemote secure file transfer!";
        fs::write(&src_path, test_data).unwrap();

        // 1. Sender initializes
        let mut sender = FileSender::new(&src_path, TransferDirection::HostToClient).unwrap();
        let announce = sender.announce_message();
        assert_eq!(announce.file_name, "source.txt");
        assert_eq!(announce.file_size, test_data.len() as u64);
        assert_eq!(announce.sha256, sha256_hex(test_data));

        // 2. Receiver initializes in a separate output dir
        let dest_dir = temp_dir.path().join("downloads");
        let mut receiver = FileReceiver::new(&dest_dir, announce).unwrap();

        // 3. Stream chunks
        while let Some(chunk) = sender.read_next_chunk(16).unwrap() {
            let ack = receiver.write_chunk(chunk).unwrap();
            assert_eq!(ack.transfer_id, sender.transfer_id());
        }

        // 4. Finalize & verify
        let dest_path = receiver.finish().expect("finish must succeed");
        assert!(dest_path.exists());
        assert_eq!(fs::read(&dest_path).unwrap(), test_data);

        // Verify .part staging file is gone
        assert!(!dest_dir.join("source.txt.part").exists());
    }

    #[test]
    fn test_corrupted_transfer_fails_integrity_and_cleans_up() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_path = temp_dir.path().join("original.dat");
        let test_data = vec![0xAB; 1024];
        fs::write(&src_path, &test_data).unwrap();

        let mut sender = FileSender::new(&src_path, TransferDirection::ClientToHost).unwrap();
        let announce = sender.announce_message();

        let dest_dir = temp_dir.path().join("received");
        let mut receiver = FileReceiver::new(&dest_dir, announce).unwrap();

        while let Some(mut chunk) = sender.read_next_chunk(256).unwrap() {
            // Corrupt byte in transit
            if chunk.sequence == 0 {
                chunk.data[0] ^= 0xFF;
            }
            receiver.write_chunk(chunk).unwrap();
        }

        let res = receiver.finish();
        assert!(matches!(
            res,
            Err(TransferError::IntegrityCheckFailed { .. })
        ));

        // Destination file must NOT exist
        assert!(!dest_dir.join("original.dat").exists());
        // Part staging file must be purged
        assert!(!dest_dir.join("original.dat.part").exists());
    }

    #[test]
    fn test_cancellation_purges_staging_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_path = temp_dir.path().join("large_file.dat");
        let test_data = vec![0x42; 2048];
        fs::write(&src_path, &test_data).unwrap();

        let mut sender = FileSender::new(&src_path, TransferDirection::HostToClient).unwrap();
        let announce = sender.announce_message();

        let dest_dir = temp_dir.path().join("downloads");
        let mut receiver = FileReceiver::new(&dest_dir, announce).unwrap();

        // Write first chunk
        let chunk = sender.read_next_chunk(512).unwrap().unwrap();
        receiver.write_chunk(chunk).unwrap();
        assert!(dest_dir.join("large_file.dat.part").exists());

        // Cancel
        receiver.cancel("user aborted").unwrap();

        // Staging file must be purged
        assert!(!dest_dir.join("large_file.dat.part").exists());
        assert!(!dest_dir.join("large_file.dat").exists());
    }
}

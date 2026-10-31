//! End-to-end integration tests for Milestone M5 (Bidirectional File Transfer).
//!
//! Validates:
//! 1. PC-to-Android download with full chunk streaming and SHA-256 integrity check.
//! 2. Android-to-PC upload with atomic staging and rename.
//! 3. Tampering / bit corruption detection with automatic staging file purge.
//! 4. Directory containment and path traversal attack rejection.
//! 5. Clean mid-transfer cancellation with immediate resource cleanup.

use openremote_file_transfer::{FileReceiver, FileSender, TransferError, CHUNK_SIZE};
use openremote_protocol::messages::TransferDirection;
use std::fs;
use std::path::PathBuf;

fn create_test_file(path: &PathBuf, size_bytes: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size_bytes);
    for i in 0..size_bytes {
        data.push((i % 256) as u8);
    }
    fs::write(path, &data).unwrap();
    data
}

#[test]
fn test_e2e_pc_to_android_download_streamed_chunks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let host_dir = temp_dir.path().join("host_files");
    let android_dir = temp_dir.path().join("android_downloads");
    fs::create_dir_all(&host_dir).unwrap();
    fs::create_dir_all(&android_dir).unwrap();

    // 1. Create a 2 MB test file on PC
    let source_file = host_dir.join("desktop_presentation.pdf");
    let original_bytes = create_test_file(&source_file, 2 * 1024 * 1024);

    // 2. PC starts FileSender
    let mut sender = FileSender::new(&source_file, TransferDirection::HostToClient).unwrap();
    let announce = sender.announce_message();
    assert_eq!(announce.file_name, "desktop_presentation.pdf");
    assert_eq!(announce.file_size, original_bytes.len() as u64);

    // 3. Android receives announcement and initializes FileReceiver
    let mut receiver = FileReceiver::new(&android_dir, announce).unwrap();

    // 4. Stream chunks (256 KiB chunks)
    let mut chunk_count = 0;
    while let Some(chunk) = sender.read_next_chunk(CHUNK_SIZE).unwrap() {
        let ack = receiver.write_chunk(chunk).unwrap();
        assert_eq!(ack.last_received_sequence, chunk_count);
        chunk_count += 1;
    }

    assert!(chunk_count >= 8); // 2 MB / 256 KiB = 8 chunks

    // 5. Finalize transfer
    let completed_path = receiver.finish().expect("Verification must succeed");
    assert!(completed_path.exists());
    assert_eq!(completed_path, android_dir.join("desktop_presentation.pdf"));

    // Verify content byte-for-byte
    let received_bytes = fs::read(&completed_path).unwrap();
    assert_eq!(received_bytes.len(), original_bytes.len());
    assert_eq!(received_bytes, original_bytes);

    // Verify .part staging file is deleted
    assert!(!android_dir.join("desktop_presentation.pdf.part").exists());

    let prog = receiver.progress();
    assert_eq!(prog.percent(), 100.0);
}

#[test]
fn test_e2e_android_to_pc_upload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let phone_dir = temp_dir.path().join("phone_storage");
    let pc_dir = temp_dir.path().join("pc_received");
    fs::create_dir_all(&phone_dir).unwrap();
    fs::create_dir_all(&pc_dir).unwrap();

    // Phone prepares photo upload
    let photo_file = phone_dir.join("camera_photo.jpg");
    let original_photo = create_test_file(&photo_file, 512 * 1024); // 512 KiB

    let mut sender = FileSender::new(&photo_file, TransferDirection::ClientToHost).unwrap();
    let announce = sender.announce_message();

    let mut receiver = FileReceiver::new(&pc_dir, announce).unwrap();

    while let Some(chunk) = sender.read_next_chunk(64 * 1024).unwrap() {
        receiver.write_chunk(chunk).unwrap();
    }

    let final_path = receiver.finish().unwrap();
    assert_eq!(fs::read(final_path).unwrap(), original_photo);
    assert!(!pc_dir.join("camera_photo.jpg.part").exists());
}

#[test]
fn test_e2e_corrupted_data_in_transit_detected_and_purged() {
    let temp_dir = tempfile::tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();

    let file_path = src_dir.join("critical_data.bin");
    create_test_file(&file_path, 64 * 1024);

    let mut sender = FileSender::new(&file_path, TransferDirection::HostToClient).unwrap();
    let announce = sender.announce_message();
    let mut receiver = FileReceiver::new(&dest_dir, announce).unwrap();

    while let Some(mut chunk) = sender.read_next_chunk(16 * 1024).unwrap() {
        // Tamper with middle chunk
        if chunk.sequence == 1 {
            chunk.data[42] ^= 0x55;
        }
        receiver.write_chunk(chunk).unwrap();
    }

    // Finish must fail integrity check
    let result = receiver.finish();
    assert!(matches!(
        result,
        Err(TransferError::IntegrityCheckFailed { .. })
    ));

    // Target file must never be created
    assert!(!dest_dir.join("critical_data.bin").exists());
    // Staging .part file must be purged
    assert!(!dest_dir.join("critical_data.bin.part").exists());
}

#[test]
fn test_e2e_path_traversal_attack_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let approved_dir = temp_dir.path().join("safe_downloads");
    fs::create_dir_all(&approved_dir).unwrap();

    // Malicious announce attempting directory traversal
    let mut announce = openremote_protocol::messages::TransferAnnounce {
        transfer_id: openremote_protocol::types::TransferId::new(),
        file_name: "../../../Windows/System32/calc.exe".to_string(),
        file_size: 1024,
        mime_type: None,
        sha256: "dummy-hash".to_string(),
        direction: TransferDirection::ClientToHost,
    };

    let result = FileReceiver::new(&approved_dir, announce.clone());
    assert!(result.is_err());

    // Backslash variant
    announce.file_name = "..\\..\\evil.bat".to_string();
    assert!(FileReceiver::new(&approved_dir, announce).is_err());
}

#[test]
fn test_e2e_cancellation_cleans_up_staging_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let host_dir = temp_dir.path().join("host");
    let dest_dir = temp_dir.path().join("dest");
    fs::create_dir_all(&host_dir).unwrap();
    fs::create_dir_all(&dest_dir).unwrap();

    let file_path = host_dir.join("aborted_file.iso");
    create_test_file(&file_path, 1024 * 1024);

    let mut sender = FileSender::new(&file_path, TransferDirection::HostToClient).unwrap();
    let announce = sender.announce_message();
    let mut receiver = FileReceiver::new(&dest_dir, announce).unwrap();

    // Stream first 2 chunks
    let chunk0 = sender.read_next_chunk(64 * 1024).unwrap().unwrap();
    let chunk1 = sender.read_next_chunk(64 * 1024).unwrap().unwrap();
    receiver.write_chunk(chunk0).unwrap();
    receiver.write_chunk(chunk1).unwrap();

    // Verify .part staging file exists while in progress
    assert!(dest_dir.join("aborted_file.iso.part").exists());

    // Cancel transfer
    receiver.cancel("Connection dropped").unwrap();

    // Staging file must be purged immediately
    assert!(!dest_dir.join("aborted_file.iso.part").exists());
    assert!(!dest_dir.join("aborted_file.iso").exists());
}

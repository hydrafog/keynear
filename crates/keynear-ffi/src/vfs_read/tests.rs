use super::*;

#[test]
fn relative_path_validation_accepts_safe_paths() {
    assert_eq!(
        validate_relative_path("  folder/file.txt  ").expect("valid path"),
        "folder/file.txt"
    );
    assert_eq!(validate_relative_path("a/b/c").expect("nested"), "a/b/c");
}

#[test]
fn relative_path_validation_rejects_unsafe_paths() {
    for raw in [
        "",
        "   ",
        "../secret",
        "/etc/passwd",
        "a\0b",
        &"x".repeat(257),
    ] {
        assert!(
            matches!(
                validate_relative_path(raw),
                Err(FfiError::Validation { .. })
            ),
            "{raw:?} must be rejected"
        );
    }
}

#[tokio::test]
async fn request_roundtrip_preserves_path_offset_and_bound() {
    let mut bytes = Vec::new();
    write_vfs_read_request(&mut bytes, "notes.txt", 4096, 8192)
        .await
        .expect("write request");
    let (path, offset, max_bytes) = read_vfs_read_request(&mut bytes.as_slice())
        .await
        .expect("read request");
    assert_eq!(path, "notes.txt");
    assert_eq!(offset, 4096);
    assert_eq!(max_bytes, 8192);
}

#[tokio::test]
async fn request_rejects_bad_bounds_and_malformed_paths() {
    let mut bytes = Vec::new();
    assert!(matches!(
        write_vfs_read_request(&mut bytes, "a", 0, 0).await,
        Err(KeynearError::Validation { .. })
    ));
    let oversized = STREAM_CHUNK_BYTES as u32 + 1;
    assert!(matches!(
        write_vfs_read_request(&mut bytes, "a", 0, oversized).await,
        Err(KeynearError::Validation { .. })
    ));
    let bad = [0u8, 1u8, b'x'];
    assert!(read_vfs_read_request(&mut bad.as_slice()).await.is_err());
    let mut traversal = Vec::new();
    traversal.extend_from_slice(&5u16.to_be_bytes());
    traversal.extend_from_slice(b"../x");
    traversal.extend_from_slice(&0u64.to_be_bytes());
    traversal.extend_from_slice(&16u32.to_be_bytes());
    assert!(matches!(
        read_vfs_read_request(&mut traversal.as_slice()).await,
        Err(KeynearError::Protocol(_))
    ));
}

#[tokio::test]
async fn reply_roundtrip_and_bounds() {
    let mut bytes = Vec::new();
    write_vfs_read_reply(&mut bytes, b"payload")
        .await
        .expect("reply");
    let payload = read_vfs_read_reply(&mut bytes.as_slice(), 16)
        .await
        .expect("read reply");
    assert_eq!(payload, b"payload");

    let empty = 0u32.to_be_bytes();
    assert!(matches!(
        read_vfs_read_reply(&mut empty.as_slice(), 16).await,
        Err(KeynearError::Protocol(_))
    ));
    let oversized = 32u32.to_be_bytes();
    assert!(matches!(
        read_vfs_read_reply(&mut oversized.as_slice(), 16).await,
        Err(KeynearError::FrameTooLarge { .. })
    ));
}

#[tokio::test]
async fn bounded_chunk_stays_inside_incoming_directory() {
    let dir = std::env::temp_dir().join(format!(
        "keynear-vfs-read-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    tokio::fs::create_dir_all(&dir).await.expect("create dir");
    tokio::fs::write(dir.join("file.txt"), b"hello world")
        .await
        .expect("write file");
    let engine = TransferEngine::new(dir.clone(), false);
    let bytes = read_bounded_chunk(&engine, "file.txt", 6, 5)
        .await
        .expect("bounded chunk");
    assert_eq!(bytes, b"world");
    assert!(read_bounded_chunk(&engine, "../outside", 0, 5)
        .await
        .is_none());
    tokio::fs::remove_dir_all(&dir).await.expect("cleanup");
}

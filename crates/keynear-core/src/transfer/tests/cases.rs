use super::*;

use super::setup::{connected_pair, temp_dir};

#[tokio::test]
async fn streams_60kb_chunks_and_stored_zip_directory_skipping_symlinks() {
    let dir = temp_dir("stream-ok");
    let src_dir = dir.join("src");
    let dest_dir = dir.join("dest");
    fs::create_dir_all(src_dir.join("nested")).expect("create src");
    fs::create_dir_all(&dest_dir).expect("create dest");

    let single_file = dir.join("payload.bin");
    let single_bytes: Vec<u8> = (0..(STREAM_CHUNK_BYTES * 2 + 12_345))
        .map(|i| (i % 251) as u8)
        .collect();
    fs::write(&single_file, &single_bytes).expect("write single file");

    let nested_file = src_dir.join("nested").join("hello.txt");
    fs::write(&nested_file, b"orion stored zip content").expect("write nested file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&single_file, src_dir.join("escape-link")).expect("create symlink");

    let (server, client, server_conn, client_conn) = connected_pair().await;
    let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));
    let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

    let recv_clone = Arc::clone(&receiver_engine);
    let recv_task = tokio::spawn(async move {
        let (s1, r1) = server_conn.accept_bi().await.expect("accept file stream");
        let session1 = recv_clone
            .handle_incoming_stream("client-1", s1, r1)
            .await
            .expect("receive single file");
        let (s2, r2) = server_conn.accept_bi().await.expect("accept zip stream");
        let session2 = recv_clone
            .handle_incoming_stream("client-1", s2, r2)
            .await
            .expect("receive zip dir");
        (session1, session2)
    });

    let send_file_session = sender_engine
        .send_paths_on_connection(&client_conn, "server-1", &[single_file])
        .await
        .expect("send single file");
    assert_eq!(send_file_session.state, TransferState::Completed);
    assert_eq!(
        send_file_session.transferred_bytes,
        single_bytes.len() as u64
    );

    let send_dir_session = sender_engine
        .send_paths_on_connection(&client_conn, "server-1", std::slice::from_ref(&src_dir))
        .await
        .expect("send directory");
    assert_eq!(send_dir_session.state, TransferState::Completed);

    let (recv_s1, recv_s2) = recv_task.await.expect("recv join");
    assert_eq!(recv_s1.state, TransferState::Completed);
    assert_eq!(recv_s2.state, TransferState::Completed);
    assert_eq!(
        fs::read(dest_dir.join("payload.bin")).expect("read received file"),
        single_bytes
    );
    assert_eq!(
        fs::read(dest_dir.join("src").join("nested").join("hello.txt"))
            .expect("read received nested"),
        b"orion stored zip content"
    );
    assert!(!dest_dir.join("src").join("escape-link").exists());

    client.close().await;
    server.close().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn pending_consent_blocks_writes_unless_auto_accept() {
    let dir = temp_dir("consent");
    let src_file = dir.join("secret.txt");
    let dest_dir = dir.join("dest");
    fs::create_dir_all(&dir).expect("create dir");
    fs::create_dir_all(&dest_dir).expect("create dest");
    fs::write(&src_file, b"consent-gated payload").expect("write src");

    let (server, client, server_conn, client_conn) = connected_pair().await;
    let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), false));
    receiver_engine
        .set_peer_auto_accept("client-1", false)
        .expect("set peer auto accept false");
    let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

    let recv_clone = Arc::clone(&receiver_engine);
    let server_conn_1 = server_conn.clone();
    let recv_task = tokio::spawn(async move {
        let (s, r) = server_conn_1.accept_bi().await.expect("accept stream");
        recv_clone.handle_incoming_stream("client-1", s, r).await
    });

    let send_clone = Arc::clone(&sender_engine);
    let client_conn_1 = client_conn.clone();
    let src_1 = src_file.clone();
    let send_task = tokio::spawn(async move {
        send_clone
            .send_paths_on_connection(&client_conn_1, "server-1", &[src_1])
            .await
    });

    let pending_id = loop {
        let list = receiver_engine.list_transfers();
        if let Some(s) = list
            .into_iter()
            .find(|s| s.state == TransferState::PendingConsent)
        {
            break s.transfer_id;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(!dest_dir.join("secret.txt").exists());
    let declined = receiver_engine
        .respond_consent(&pending_id, false)
        .expect("decline");
    assert_eq!(declined.state, TransferState::Cancelled);
    assert!(matches!(
        recv_task.await.expect("join"),
        Err(KeynearError::TransferCancelled(_))
    ));
    assert!(matches!(
        send_task.await.expect("join"),
        Err(KeynearError::TransferCancelled(_))
    ));
    assert!(!dest_dir.join("secret.txt").exists());

    receiver_engine
        .set_peer_auto_accept("client-1", true)
        .expect("enable peer auto accept");
    let recv_clone_2 = Arc::clone(&receiver_engine);
    let recv_task_2 = tokio::spawn(async move {
        let (s, r) = server_conn.accept_bi().await.expect("accept stream");
        recv_clone_2.handle_incoming_stream("client-1", s, r).await
    });
    let sent = sender_engine
        .send_paths_on_connection(&client_conn, "server-1", &[src_file])
        .await
        .expect("auto-accepted send completes");
    assert_eq!(sent.state, TransferState::Completed);
    let recvd = recv_task_2.await.expect("join").expect("recv completes");
    assert_eq!(recvd.state, TransferState::Completed);
    assert_eq!(
        fs::read(dest_dir.join("secret.txt")).expect("read file"),
        b"consent-gated payload"
    );

    client.close().await;
    server.close().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn rejects_zip_slip_symlink_and_byte_overflow() {
    assert!(sanitize_relative_path("../escape.txt").is_err());
    assert!(sanitize_relative_path("/etc/passwd").is_err());
    assert!(sanitize_relative_path("a\\b.txt").is_err());
    assert!(sanitize_relative_path("").is_err());
    assert_eq!(
        sanitize_relative_path("folder/item.txt").unwrap(),
        PathBuf::from("folder/item.txt")
    );

    let dir = temp_dir("security-reject");
    let dest_dir = dir.join("dest");
    fs::create_dir_all(&dest_dir).expect("create dest");

    let mut zip_bytes = Vec::new();
    {
        let mut zw = ZipWriter::new(Cursor::new(&mut zip_bytes));
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zw.start_file("../evil.txt", opts).expect("start evil");
        zw.write_all(b"evil").expect("write evil");
        zw.finish().expect("finish");
    }

    let (server, client, server_conn, client_conn) = connected_pair().await;
    let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));

    let recv_clone = Arc::clone(&receiver_engine);
    let server_conn_1 = server_conn.clone();
    let recv_task = tokio::spawn(async move {
        let (s, r) = server_conn_1.accept_bi().await.expect("accept");
        recv_clone.handle_incoming_stream("client-1", s, r).await
    });

    let (mut send, mut recv) = client_conn.open_bi().await.expect("open bi");
    write_stream_kind(&mut send, QuicStreamKind::StoredZipArchive)
        .await
        .unwrap();
    let offer = v1::TransferOffer {
        transfer_id: "tx-zipslip".to_string(),
        items: vec![v1::TransferItemMeta {
            name: "safe.txt".to_string(),
            mime_type: "text/plain".to_string(),
            size_bytes: 4,
            is_directory: true,
        }],
        total_bytes: 4,
    };
    write_control_envelope(
        &mut send,
        &wrap_payload(v1::envelope::Payload::TransferOffer(offer)),
    )
    .await
    .unwrap();
    await_transfer_consent(&mut recv, "tx-zipslip")
        .await
        .unwrap();
    write_data_chunk(&mut send, &zip_bytes).await.unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&zip_bytes);
    let digest: [u8; 32] = hasher.finalize().into();
    let _ = send_completion_and_wait_ack(
        &mut send,
        &mut recv,
        CompletionDetails {
            transfer_id: "tx-zipslip",
            bytes_transferred: zip_bytes.len() as u64,
            digest: &digest,
        },
    )
    .await;

    let res = recv_task.await.expect("join");
    assert!(matches!(res, Err(KeynearError::Transfer(_))));
    assert!(!dir.join("evil.txt").exists());

    let recv_clone_2 = Arc::clone(&receiver_engine);
    let recv_task_2 = tokio::spawn(async move {
        let (s, r) = server_conn.accept_bi().await.expect("accept 2");
        recv_clone_2.handle_incoming_stream("client-1", s, r).await
    });
    let (mut send2, mut recv2) = client_conn.open_bi().await.expect("open bi 2");
    write_stream_kind(&mut send2, QuicStreamKind::RawFileChunk)
        .await
        .unwrap();
    let offer2 = v1::TransferOffer {
        transfer_id: "tx-overflow".to_string(),
        items: vec![v1::TransferItemMeta {
            name: "bomb.bin".to_string(),
            mime_type: "application/octet-stream".to_string(),
            size_bytes: 4,
            is_directory: false,
        }],
        total_bytes: 4,
    };
    write_control_envelope(
        &mut send2,
        &wrap_payload(v1::envelope::Payload::TransferOffer(offer2)),
    )
    .await
    .unwrap();
    await_transfer_consent(&mut recv2, "tx-overflow")
        .await
        .unwrap();
    let _ = write_data_chunk(&mut send2, &[0xAA; 100]).await;
    let _ = send2.finish();

    let res2 = recv_task_2.await.expect("join 2");
    assert!(matches!(res2, Err(KeynearError::Transfer(_))));
    assert!(!dest_dir.join("bomb.bin").exists());

    client.close().await;
    server.close().await;
    let _ = fs::remove_dir_all(&dir);
}

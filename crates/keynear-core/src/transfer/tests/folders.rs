use super::*;

use super::setup::{connected_pair, temp_dir};

#[tokio::test]
async fn preserves_empty_folders_subdirs_and_avoids_folder_collision_scattering() {
    let dir = temp_dir("folder-semantics");
    let empty_folder = dir.join("empty_root");
    let project_dir = dir.join("v1.2");
    let dest_dir = dir.join("dest");
    fs::create_dir_all(&empty_folder).expect("create empty root");
    fs::create_dir_all(project_dir.join("nested")).expect("create nested");
    fs::create_dir_all(project_dir.join("empty_sub")).expect("create empty sub");
    fs::create_dir_all(&dest_dir).expect("create dest");
    fs::write(project_dir.join("nested").join("note.txt"), b"folder body").expect("write note");

    let (server, client, server_conn, client_conn) = connected_pair().await;
    let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));
    let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

    let recv_clone = Arc::clone(&receiver_engine);
    let recv_task = tokio::spawn(async move {
        let mut sessions = Vec::new();
        for _ in 0..3 {
            let (s, r) = server_conn.accept_bi().await.expect("accept stream");
            let session = recv_clone
                .handle_incoming_stream("client-1", s, r)
                .await
                .expect("receive folder");
            sessions.push(session);
        }
        sessions
    });

    let s1 = sender_engine
        .send_paths_on_connection(
            &client_conn,
            "server-1",
            std::slice::from_ref(&empty_folder),
        )
        .await
        .expect("send empty folder");
    let s2 = sender_engine
        .send_paths_on_connection(&client_conn, "server-1", std::slice::from_ref(&project_dir))
        .await
        .expect("send project first");
    let s3 = sender_engine
        .send_paths_on_connection(&client_conn, "server-1", std::slice::from_ref(&project_dir))
        .await
        .expect("send project second");
    assert_eq!(s1.state, TransferState::Completed);
    assert_eq!(s2.state, TransferState::Completed);
    assert_eq!(s3.state, TransferState::Completed);

    let received = recv_task.await.expect("join recv");
    assert_eq!(received.len(), 3);
    assert_eq!(
        received[0].items[0].target_path.as_deref(),
        Some(dest_dir.join("empty_root").to_string_lossy().as_ref())
    );
    assert!(dest_dir.join("empty_root").is_dir());

    let first_target = dest_dir.join("v1.2");
    assert_eq!(
        received[1].items[0].target_path.as_deref(),
        Some(first_target.to_string_lossy().as_ref())
    );
    assert!(first_target.join("empty_sub").is_dir());
    assert_eq!(
        fs::read(first_target.join("nested").join("note.txt")).expect("read first"),
        b"folder body"
    );

    let second_target = dest_dir.join("v1.2 (1)");
    assert_eq!(
        received[2].items[0].target_path.as_deref(),
        Some(second_target.to_string_lossy().as_ref())
    );
    assert!(second_target.join("empty_sub").is_dir());
    assert_eq!(
        fs::read(second_target.join("nested").join("note.txt")).expect("read second"),
        b"folder body"
    );
    assert!(!first_target.join("nested").join("note (1).txt").exists());

    client.close().await;
    server.close().await;
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn send_paths_handles_colon_filenames_mime_types_and_drive_prefixes() {
    use crate::transfer::io::sanitize_relative_path;

    assert_eq!(
        sanitize_relative_path("1:2.txt").expect("unix colon ok"),
        PathBuf::from("1:2.txt")
    );
    assert!(sanitize_relative_path("C:/windows.txt").is_err());
    assert!(sanitize_relative_path("C:\\windows.txt").is_err());
    assert!(sanitize_relative_path("C:").is_err());
    assert!(sanitize_relative_path("../escape").is_err());
    assert!(sanitize_relative_path("/abs").is_err());
    assert!(sanitize_relative_path("").is_err());

    let dir = temp_dir("folder-retry-and-colon");
    let src_dir = dir.join("batch");
    let dest_dir = dir.join("dest");
    fs::create_dir_all(&src_dir).expect("mkdir src");
    fs::create_dir_all(&dest_dir).expect("mkdir dest");
    fs::write(src_dir.join("1:2.txt"), b"colon ok").expect("write colon");
    let mut ext_files = Vec::new();
    for ext in [
        "apk", "pdf", "png", "jpg", "gif", "webp", "svg", "mp4", "mkv", "webm", "mp3", "ogg",
        "wav", "flac", "html", "json", "xml", "zip", "tar", "gz", "7z", "bin",
    ] {
        let p = src_dir.join(format!("sample.{ext}"));
        fs::write(&p, b"x").expect("write ext");
        ext_files.push(p);
    }
    let multi_plan = crate::transfer::io::prepare_send_plan(&ext_files).expect("multi plan");
    assert_eq!(multi_plan.items.len(), ext_files.len());
    let single_plan = crate::transfer::io::prepare_send_plan(std::slice::from_ref(&ext_files[0]))
        .expect("single plan");
    assert_eq!(
        single_plan.items[0].mime_type,
        "application/vnd.android.package-archive"
    );

    let (server, client, server_conn, _client_conn) = connected_pair().await;
    let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));
    let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

    let recv_clone = Arc::clone(&receiver_engine);
    let recv_task = tokio::spawn(async move {
        let mut out = Vec::new();
        for _ in 0..2 {
            let (s, r) = server_conn.accept_bi().await.expect("accept");
            out.push(
                recv_clone
                    .handle_incoming_stream("client-1", s, r)
                    .await
                    .expect("recv"),
            );
        }
        out
    });

    assert!(sender_engine
        .send_paths(&client, "server-1", &[])
        .await
        .is_err());
    let sent = sender_engine
        .send_paths(&client, "server-1", std::slice::from_ref(&src_dir))
        .await
        .expect("send_paths ok");
    assert_eq!(sent.state, TransferState::Completed);

    let handle_file = fs::File::open(&ext_files[1]).expect("open pdf");
    let sent_fd = sender_engine
        .send_file_handle(
            &client,
            "server-1",
            "doc.pdf".to_string(),
            "application/pdf".to_string(),
            1,
            handle_file,
        )
        .await
        .expect("send_file_handle ok");
    assert_eq!(sent_fd.state, TransferState::Completed);

    let received = recv_task.await.expect("join");
    assert_eq!(received.len(), 2);
    assert_eq!(received[0].state, TransferState::Completed);
    assert_eq!(received[1].state, TransferState::Completed);
    assert_eq!(
        fs::read(dest_dir.join("batch").join("1:2.txt")).expect("read colon"),
        b"colon ok"
    );

    client.close().await;
    server.close().await;
    let _ = fs::remove_dir_all(&dir);
}

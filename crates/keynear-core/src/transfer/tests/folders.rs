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

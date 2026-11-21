use super::*;
use crate::STREAM_CHUNK_BYTES;

#[test]
fn invalid_remote_node_name_is_rejected() {
    let err = RemoteVfsNode::child("phone-1", "../escape", false, 10)
        .expect_err("traversal must be rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));

    let err = RemoteVfsNode::child("phone-1", &"x".repeat(129), false, 10)
        .expect_err("oversized name must be rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn child_document_id_is_bounded_and_opaque() {
    let node = RemoteVfsNode::child("phone-1", "docs/report.txt", false, 42).expect("valid child");
    assert_eq!(node.document_id, "phone-1/docs/report.txt");
    assert_eq!(node.name, "docs/report.txt");
    assert!(!node.is_directory);
}

#[test]
fn root_node_is_a_directory() {
    let root = RemoteVfsNode::root("phone-1", "Phone").expect("valid root");
    assert_eq!(root.document_id, "phone-1");
    assert!(root.is_directory);
    assert_eq!(root.size_bytes, 0);
}

#[test]
fn page_documents_clamps_to_available_rows() {
    let nodes: Vec<RemoteVfsNode> = (0..5)
        .map(|idx| {
            RemoteVfsNode::child("phone-1", &format!("file-{idx}.txt"), false, idx)
                .expect("valid node")
        })
        .collect();
    assert_eq!(page_documents(&nodes, 0, 2).len(), 2);
    assert_eq!(page_documents(&nodes, 4, 4).len(), 1);
    assert_eq!(page_documents(&nodes, 9, 4).len(), 0);
    assert_eq!(page_documents(&nodes, 0, 0).len(), 0);
}

#[test]
fn read_plan_clamps_to_file_size_and_chunk() {
    let plan = read_plan(10_000, 9_000, STREAM_CHUNK_BYTES).expect("plan");
    assert_eq!(plan.offset, 9_000);
    assert_eq!(plan.length, 1_000);
    assert_eq!(plan.chunks, 1);

    let clamped = read_plan(10_000, 20_000, 1_000).expect("plan");
    assert_eq!(clamped.offset, 10_000);
    assert_eq!(clamped.length, 0);
    assert_eq!(clamped.chunks, 0);
}

#[test]
fn read_plan_rejects_zero_and_oversized_requests() {
    assert!(matches!(
        read_plan(10_000, 0, 0),
        Err(KeynearError::Validation { .. })
    ));
    assert!(matches!(
        read_plan(10_000, 0, STREAM_CHUNK_BYTES + 1),
        Err(KeynearError::Validation { .. })
    ));
}

use super::*;

fn offer(
    transfer_id: &str,
    items: Vec<v1::TransferItemMeta>,
    total_bytes: u64,
) -> v1::TransferOffer {
    v1::TransferOffer {
        transfer_id: transfer_id.to_string(),
        items,
        total_bytes,
    }
}

fn item(name: &str, size_bytes: u64) -> v1::TransferItemMeta {
    v1::TransferItemMeta {
        name: name.to_string(),
        mime_type: "application/octet-stream".to_string(),
        size_bytes,
        is_directory: false,
    }
}

#[test]
fn validate_offer_accepts_a_consistent_offer() {
    let valid = offer(
        "tx-1",
        vec![item("notes.txt", 12), item("photo.bin", 30)],
        42,
    );
    assert!(validate_offer(&valid).is_ok());
}

#[test]
fn validate_offer_rejects_empty_and_oversized_transfer_ids() {
    let too_long = "x".repeat(65);
    for transfer_id in ["", "   ", &too_long[..]] {
        let err = validate_offer(&offer(transfer_id, vec![item("a.txt", 1)], 1))
            .expect_err("invalid transfer id");
        match err {
            KeynearError::Validation { field, message } => {
                assert_eq!(field, "transferId");
                assert_eq!(message, "transfer_id must be 1..64 bytes");
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
    }
}

#[test]
fn validate_offer_rejects_an_empty_item_list() {
    let err = validate_offer(&offer("tx-1", Vec::new(), 0)).expect_err("empty items");
    match err {
        KeynearError::Validation { field, message } => {
            assert_eq!(field, "items");
            assert!(message.contains("at least one item"));
        }
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn validate_offer_rejects_unsafe_item_names() {
    for name in ["../escape.txt", "/etc/passwd", "C:/secret.txt", "dir\\file"] {
        let err =
            validate_offer(&offer("tx-1", vec![item(name, 1)], 1)).expect_err("unsafe item name");
        assert!(matches!(err, KeynearError::Validation { field, .. } if field == "path"));
    }
}

#[test]
fn validate_offer_rejects_size_overflow() {
    let err = validate_offer(&offer(
        "tx-1",
        vec![item("a.bin", u64::MAX), item("b.bin", 1)],
        u64::MAX,
    ))
    .expect_err("overflow");
    assert!(matches!(err, KeynearError::Transfer(message) if message.contains("size overflow")));
}

#[test]
fn validate_offer_rejects_a_total_that_disagrees_with_the_items() {
    let err =
        validate_offer(&offer("tx-1", vec![item("a.txt", 12)], 13)).expect_err("total mismatch");
    match err {
        KeynearError::Validation { field, message } => {
            assert_eq!(field, "totalBytes");
            assert!(message.contains("does not match item sum"));
        }
        other => panic!("expected a validation error, got {other:?}"),
    }
}

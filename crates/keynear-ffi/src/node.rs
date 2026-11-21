use crate::*;
use zeroize::Zeroizing;

#[derive(uniffi::Object)]
pub struct OrionNodeHandle {
    pub(crate) runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) node: OrionQuicNode,
    pub(crate) engine: Arc<TransferEngine>,
    pub(crate) clipboard: Arc<ClipboardEngine>,
    pub(crate) synced_peers: Mutex<Vec<String>>,
    pub(crate) accept_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

pub(crate) struct OrionNodeInit {
    pub(crate) device_id: String,
    pub(crate) display_name: String,
    pub(crate) capability_flags: u32,
    pub(crate) quic_secret_key: Vec<u8>,
    pub(crate) incoming_directory: String,
    pub(crate) auto_accept_from_paired: bool,
    pub(crate) require_explicit_fd_for_single_file: bool,
    pub(crate) bind_addr: Option<SocketAddr>,
}

impl OrionNodeHandle {
    pub(crate) fn build_with_init(init: OrionNodeInit) -> Result<Self, FfiError> {
        let key_bytes = to_key_32(&init.quic_secret_key, "quicSecretKey")?;
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|err| KeynearError::Transport(format!("tokio runtime failed: {err}")))?,
        );
        let config = OrionNodeConfig {
            device_id: init.device_id,
            display_name: init.display_name,
            capability_flags: init.capability_flags,
            quic_secret_key: Zeroizing::new(key_bytes),
        };
        let node = runtime.block_on(OrionQuicNode::bind(config, init.bind_addr))?;
        let engine = Arc::new(TransferEngine::new(
            PathBuf::from(init.incoming_directory),
            init.auto_accept_from_paired,
        ));
        engine.set_require_explicit_fd_for_single_file(init.require_explicit_fd_for_single_file);
        let clipboard = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));

        let handle = runtime.spawn(accept_loop(
            node.clone(),
            Arc::clone(&engine),
            Arc::clone(&clipboard),
        ));

        Ok(Self {
            runtime,
            node,
            engine,
            clipboard,
            synced_peers: Mutex::new(Vec::new()),
            accept_task: Mutex::new(Some(handle)),
        })
    }
}

async fn accept_loop(
    node: OrionQuicNode,
    engine: Arc<TransferEngine>,
    clipboard: Arc<ClipboardEngine>,
) {
    loop {
        match node.accept_authenticated().await {
            Ok(Some((peer_id, conn))) => {
                let eng = Arc::clone(&engine);
                let clip = Arc::clone(&clipboard);
                tokio::spawn(async move {
                    while let Ok((send, mut recv)) = conn.accept_bi().await {
                        match read_stream_kind(&mut recv).await {
                            Ok(
                                kind @ (QuicStreamKind::RawFileChunk
                                | QuicStreamKind::StoredZipArchive),
                            ) => {
                                let _ = eng
                                    .handle_incoming_stream_with_kind(&peer_id, kind, send, recv)
                                    .await;
                            }
                            Ok(
                                kind @ (QuicStreamKind::Control | QuicStreamKind::ClipboardBlob),
                            ) => {
                                let _ = clip
                                    .handle_incoming_stream_with_kind(&peer_id, kind, send, recv)
                                    .await;
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Ok(None) => break,
            Err(_) => continue,
        }
    }
}

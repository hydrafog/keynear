use super::*;

pub(super) struct ReceiveContext<'a> {
    pub(super) registry: &'a TransferRegistry,
    pub(super) offer: &'a v1::TransferOffer,
    pub(super) recv: &'a mut RecvStream,
    pub(super) cancel_flag: &'a Arc<AtomicBool>,
}

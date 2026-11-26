use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use keynear_protocol::{
    encode_control_frame, envelope_to_control_frame, v1, ControlFrame, VisibilityMode,
    MAX_CONTROL_FRAME_BYTES,
};
use prost::Message;

use super::*;
use crate::node::OrionNodeInit;
use transfer::wrap_raw_fd;

mod bridge;
mod core;
mod core_handle;
mod node;

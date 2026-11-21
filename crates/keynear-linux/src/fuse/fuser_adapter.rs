use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request, FUSE_ROOT_ID,
};
use keynear_core::{read_plan, RemoteVfsNode, STREAM_CHUNK_BYTES};

type RemoteReader = Box<dyn Fn(u64, u64, usize) -> Vec<u8> + Send + Sync>;

pub struct KeynearFuseFs {
    nodes: BTreeMap<u64, RemoteVfsNode>,
    reader: RemoteReader,
}

impl KeynearFuseFs {
    pub fn new(root: RemoteVfsNode, children: Vec<RemoteVfsNode>, reader: RemoteReader) -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(FUSE_ROOT_ID, root);
        for (index, child) in children.into_iter().enumerate() {
            nodes.insert(FUSE_ROOT_ID + 1 + index as u64, child);
        }
        Self { nodes, reader }
    }

    fn attr(&self, ino: u64, node: &RemoteVfsNode) -> FileAttr {
        let now = UNIX_EPOCH + Duration::from_secs(1_763_000_000);
        FileAttr {
            ino,
            size: node.size_bytes,
            blocks: node.size_bytes.div_ceil(512),
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: if node.is_directory {
                FileType::Directory
            } else {
                FileType::RegularFile
            },
            perm: if node.is_directory { 0o555 } else { 0o444 },
            nlink: if node.is_directory { 2 } else { 1 },
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4_096,
            flags: 0,
        }
    }
}

impl Filesystem for KeynearFuseFs {
    fn getattr(&mut self, ino: u64, _req: Request, reply: ReplyAttr) {
        match self.nodes.get(&ino) {
            Some(node) => {
                let ttl = Duration::from_secs(1);
                reply.attr(&ttl, &self.attr(ino, node));
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn lookup(&mut self, _parent: u64, name: &OsStr, _req: Request, reply: ReplyEntry) {
        let name = name.to_string_lossy();
        let found = self
            .nodes
            .iter()
            .find(|(_, node)| node.name == name.as_ref());
        match found {
            Some((&ino, node)) => {
                let ttl = Duration::from_secs(1);
                reply.entry(&ttl, &self.attr(ino, node), 0);
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn readdir(&mut self, ino: u64, offset: i64, _req: Request, mut reply: ReplyDirectory) {
        if ino != FUSE_ROOT_ID {
            reply.error(libc::ENOENT);
            return;
        }
        for (index, (&child_ino, node)) in self.nodes.iter().enumerate().skip(offset as usize) {
            let kind = if node.is_directory {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            if reply.add(child_ino, index as i64 + 1, kind, &node.name) {
                break;
            }
        }
        reply.ok();
    }

    fn open(&mut self, _ino: u64, _flags: i32, _req: Request, reply: ReplyOpen) {
        reply.opened(0, 0);
    }

    fn read(
        &mut self,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _req: Request,
        reply: ReplyData,
    ) {
        if offset < 0 {
            reply.error(libc::EINVAL);
            return;
        }
        let Some(node) = self.nodes.get(&ino) else {
            reply.error(libc::ENOENT);
            return;
        };
        let requested = (size as usize).min(STREAM_CHUNK_BYTES);
        let Ok(plan) = read_plan(node.size_bytes, offset as u64, requested.max(1)) else {
            reply.error(libc::EIO);
            return;
        };
        if plan.length == 0 {
            reply.data(&[]);
            return;
        }
        let bytes = (self.reader)(ino, plan.offset, plan.length);
        reply.data(&bytes);
    }
}

pub fn mount_keynear_fuse(mountpoint: &Path, fs: KeynearFuseFs) -> std::io::Result<()> {
    fuser::Session::new(fs, mountpoint, &[] as &[&OsStr]).run()
}

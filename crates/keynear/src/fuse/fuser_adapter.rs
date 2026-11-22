#![allow(dead_code)]

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{
    Config, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    LockOwner, OpenFlags, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
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
        nodes.insert(INodeNo::ROOT.0, root);
        for (index, child) in children.into_iter().enumerate() {
            nodes.insert(INodeNo::ROOT.0 + 1 + index as u64, child);
        }
        Self { nodes, reader }
    }

    fn attr(&self, ino: INodeNo, node: &RemoteVfsNode) -> FileAttr {
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
    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match self.nodes.get(&ino.0) {
            Some(node) => {
                let ttl = Duration::from_secs(1);
                reply.attr(&ttl, &self.attr(ino, node));
            }
            None => reply.error(Errno::ENOENT),
        }
    }

    fn lookup(&self, _req: &Request, _parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let name = name.to_string_lossy();
        let found = self
            .nodes
            .iter()
            .find(|(_, node)| node.name == name.as_ref());
        match found {
            Some((&ino, node)) => {
                let ttl = Duration::from_secs(1);
                reply.entry(&ttl, &self.attr(INodeNo(ino), node), Generation(0));
            }
            None => reply.error(Errno::ENOENT),
        }
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        if ino != INodeNo::ROOT {
            reply.error(Errno::ENOENT);
            return;
        }
        for (index, (&child_ino, node)) in self.nodes.iter().enumerate().skip(offset as usize) {
            let kind = if node.is_directory {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            if reply.add(INodeNo(child_ino), index as u64 + 1, kind, &node.name) {
                break;
            }
        }
        reply.ok();
    }

    fn open(&self, _req: &Request, _ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        reply.opened(FileHandle(0), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let Some(node) = self.nodes.get(&ino.0) else {
            reply.error(Errno::ENOENT);
            return;
        };
        let requested = (size as usize).min(STREAM_CHUNK_BYTES);
        let Ok(plan) = read_plan(node.size_bytes, offset, requested.max(1)) else {
            reply.error(Errno::EIO);
            return;
        };
        if plan.length == 0 {
            reply.data(&[]);
            return;
        }
        let bytes = (self.reader)(ino.0, plan.offset, plan.length);
        reply.data(&bytes);
    }
}

pub fn mount_keynear_fuse(mountpoint: &Path, fs: KeynearFuseFs) -> std::io::Result<()> {
    fuser::Session::new(fs, mountpoint, &Config::default())?.run()
}

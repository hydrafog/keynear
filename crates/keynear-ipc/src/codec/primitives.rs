use crate::*;

pub(crate) fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn write_str(out: &mut Vec<u8>, value: &str) -> Result<(), KeynearError> {
    let len = u32::try_from(value.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: value.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    write_u32(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

pub(crate) fn write_opt_str(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), KeynearError> {
    match value {
        Some(inner) => {
            out.push(1);
            write_str(out, inner)?;
        }
        None => {
            out.push(0);
        }
    }
    Ok(())
}

pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(crate) fn read_exact(&mut self, len: usize) -> Result<&'a [u8], KeynearError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| KeynearError::Protocol("IPC frame offset overflow".to_string()))?;
        if end > self.bytes.len() {
            return Err(KeynearError::Protocol("truncated IPC frame".to_string()));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, KeynearError> {
        Ok(self.read_exact(1)?[0])
    }

    pub(crate) fn read_bool(&mut self) -> Result<bool, KeynearError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(KeynearError::Protocol(format!(
                "invalid IPC boolean byte {other}"
            ))),
        }
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, KeynearError> {
        let slice = self.read_exact(4)?;
        let mut arr = [0u8; 4];
        arr.copy_from_slice(slice);
        Ok(u32::from_be_bytes(arr))
    }

    pub(crate) fn read_u64(&mut self) -> Result<u64, KeynearError> {
        let slice = self.read_exact(8)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(slice);
        Ok(u64::from_be_bytes(arr))
    }

    pub(crate) fn read_string(&mut self) -> Result<String, KeynearError> {
        let len = self.read_u32()? as usize;
        if len > MAX_IPC_FRAME_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: len,
                max: MAX_IPC_FRAME_BYTES,
            });
        }
        let raw = self.read_exact(len)?;
        std::str::from_utf8(raw)
            .map(ToString::to_string)
            .map_err(|_| KeynearError::Protocol("invalid UTF-8 in IPC frame".to_string()))
    }

    pub(crate) fn read_opt_string(&mut self) -> Result<Option<String>, KeynearError> {
        if self.read_bool()? {
            Ok(Some(self.read_string()?))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn ensure_exhausted(&self) -> Result<(), KeynearError> {
        if self.offset != self.bytes.len() {
            return Err(KeynearError::Protocol(
                "unexpected trailing bytes in IPC frame".to_string(),
            ));
        }
        Ok(())
    }
}

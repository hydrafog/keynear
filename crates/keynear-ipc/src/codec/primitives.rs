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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_primitives_write_big_endian_boundaries() {
        let mut out = Vec::new();
        write_u32(&mut out, 0x0102_0304);
        write_u64(&mut out, u64::MAX);
        assert_eq!(&out[..4], &[0x01u8, 0x02, 0x03, 0x04][..]);

        let mut cur = Cursor::new(&out);
        assert_eq!(cur.read_u32().expect("read u32"), 0x0102_0304);
        assert_eq!(cur.read_u64().expect("read u64"), u64::MAX);
        cur.ensure_exhausted().expect("no trailing bytes");

        let zero = [0u8; 12];
        let mut zero_cur = Cursor::new(&zero);
        assert_eq!(zero_cur.read_u32().expect("zero u32"), 0);
        assert_eq!(zero_cur.read_u64().expect("zero u64"), 0);
    }

    #[test]
    fn string_primitives_roundtrip_empty_and_unicode() {
        let mut out = Vec::new();
        write_str(&mut out, "").expect("write empty string");
        write_str(&mut out, "héllo\n").expect("write unicode string");

        let mut cur = Cursor::new(&out);
        assert_eq!(cur.read_string().expect("read empty"), "");
        assert_eq!(cur.read_string().expect("read unicode"), "héllo\n");
        cur.ensure_exhausted().expect("no trailing bytes");
    }

    #[test]
    fn optional_string_primitives_roundtrip_none_and_some() {
        let mut out = Vec::new();
        write_opt_str(&mut out, None).expect("write none");
        assert_eq!(out, vec![0]);
        write_opt_str(&mut out, Some("x")).expect("write some");
        assert_eq!(&out[1..], &[1u8, 0, 0, 0, 1, b'x'][..]);

        let mut cur = Cursor::new(&out);
        assert_eq!(cur.read_opt_string().expect("read none"), None);
        assert_eq!(
            cur.read_opt_string().expect("read some"),
            Some("x".to_string())
        );
        cur.ensure_exhausted().expect("no trailing bytes");
    }

    #[test]
    fn read_u8_and_read_bool_reject_bad_and_truncated_bytes() {
        let bytes = [1u8, 0, 2];
        let mut cur = Cursor::new(&bytes);
        assert_eq!(cur.read_u8().expect("read u8"), 1);
        assert!(!cur.read_bool().expect("read false"));
        assert!(matches!(cur.read_bool(), Err(KeynearError::Protocol(_))));

        let empty: [u8; 0] = [];
        let mut empty_cur = Cursor::new(&empty);
        assert!(matches!(
            empty_cur.read_u8(),
            Err(KeynearError::Protocol(_))
        ));
        assert!(matches!(
            empty_cur.read_bool(),
            Err(KeynearError::Protocol(_))
        ));
    }

    #[test]
    fn read_string_rejects_oversized_length_and_invalid_utf8() {
        let mut oversized = Vec::new();
        write_u32(&mut oversized, (MAX_IPC_FRAME_BYTES + 1) as u32);
        assert!(matches!(
            Cursor::new(&oversized).read_string(),
            Err(KeynearError::FrameTooLarge { .. })
        ));

        let invalid_utf8 = [0u8, 0, 0, 1, 0xff];
        assert!(matches!(
            Cursor::new(&invalid_utf8).read_string(),
            Err(KeynearError::Protocol(_))
        ));
    }

    #[test]
    fn ensure_exhausted_rejects_trailing_bytes() {
        let bytes = [0u8, 0, 0, 1, 0x42, 0x99];
        let mut cur = Cursor::new(&bytes);
        assert_eq!(cur.read_u32().expect("read length"), 1);
        assert_eq!(cur.read_exact(1).expect("read payload")[0], 0x42);
        assert!(matches!(
            cur.ensure_exhausted(),
            Err(KeynearError::Protocol(_))
        ));
    }
}

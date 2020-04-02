use crate::utils::*;
use crate::*;

impl<'a> Bits<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Bits {
            path: bytes,
            range: 0..(bytes.len() as BitsLen * 8),
        }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        let u = std::mem::size_of::<BitsLen>();
        let start: BitsLen = bytes_to_int(&bytes[..u]);
        let end: BitsLen = bytes_to_int(&bytes[u..2 * u]);
        Self {
            path: &bytes[2 * u..],
            range: start..end,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok([
            &self.range.start.to_be_bytes(),
            &self.range.end.to_be_bytes(),
            &self.path[..],
        ]
        .concat())
    }

    pub fn first(&self) -> bool {
        bit(&self.path, self.range.start)
    }

    pub fn len(&self) -> BitsLen {
        self.range.end - self.range.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0 || self.path.len() == 0
    }

    pub fn shift(&self, n: BitsLen, tail: bool) -> Self {
        let (q, range) = offsets(&self.range, n, tail);
        if tail {
            Self {
                path: &self.path[..q as usize],
                range,
            }
        } else {
            Self {
                path: &self.path[q as usize..],
                range,
            }
        }
    }

    pub fn len_common_bits(a: &Self, b: &Self) -> BitsLen {
        len_lcp(&a.path, &a.range, &b.path, &b.range)
    }
}

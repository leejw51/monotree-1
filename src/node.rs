use crate::utils::*;
use crate::*;

impl<'a> Node<'a> {
    pub fn new(lc: Cell<'a>, rc: Cell<'a>) -> Self {
        match (&lc, &rc) {
            (&Some(_), &None) => Node::Soft(lc),
            (&None, &Some(_)) => Node::Soft(rc),
            (&Some(_), &Some(_)) => Node::Hard(lc, rc),
            _ => unreachable!("Node::new()"),
        }
    }

    pub fn cells_from_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, Cell<'a>)> {
        match Node::from_bytes(&bytes)? {
            Node::Soft(cell) => Ok((cell, None)),
            Node::Hard(lc, rc) => {
                if right {
                    Ok((rc, lc))
                } else {
                    Ok((lc, rc))
                }
            }
        }
    }

    fn parse_bytes(bytes: &'a [u8], right: bool) -> Result<(Cell<'a>, usize)> {
        let l = bytes.len();
        let i = if right { 0usize } else { HASH_LEN };
        let g = if right { l - HASH_LEN..l } else { 0..HASH_LEN };
        let start: u16 = bytes_to_int(&bytes[i..i + 2]);
        let end: u16 = bytes_to_int(&bytes[i + 2..i + 4]);
        let n = nbytes_across(start, end) as usize;
        Ok((
            Some(Unit {
                hash: &bytes[g],
                bits: Bits {
                    path: &bytes[i + 4..i + 4 + n],
                    range: start..end,
                },
            }),
            i + 4 + n,
        ))
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        match bytes.last() {
            Some(&0x00) => {
                let (cell, _) = Node::parse_bytes(&bytes[..bytes.len() - 1], false)?;
                Ok(Node::Soft(cell))
            }
            Some(&0x01) => {
                let (lc, size) = Node::parse_bytes(&bytes, false)?;
                let (rc, _) = Node::parse_bytes(&bytes[size..bytes.len() - 1], true)?;
                Ok(Node::Hard(lc, rc))
            }
            _ => unreachable!("Node::from_bytes()"),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Node::Soft(Some(unit)) => {
                Ok([&unit.hash[..], &unit.bits.to_bytes()?, &[0x00]].concat())
            }
            Node::Hard(Some(lu), Some(ru)) => {
                let (lu, ru) = if ru.bits.first() { (lu, ru) } else { (ru, lu) };
                Ok([
                    &lu.hash[..],
                    &lu.bits.to_bytes()?,
                    &ru.bits.to_bytes()?,
                    &ru.hash[..],
                    &[0x01],
                ]
                .concat())
            }
            _ => unreachable!("node.to_bytes()"),
        }
    }
}

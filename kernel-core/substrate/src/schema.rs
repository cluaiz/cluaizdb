use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};

/// Edge metadata stored in outbound_edges table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EdgeMetadata {
    pub weight: f32,
    pub created_at: u64,
    pub valid_until: u64,
}

impl EdgeMetadata {
    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        BigEndian::write_f32(&mut buf[0..4], self.weight);
        BigEndian::write_u64(&mut buf[4..12], self.created_at);
        BigEndian::write_u64(&mut buf[12..20], self.valid_until);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }
        Some(Self {
            weight: BigEndian::read_f32(&bytes[0..4]),
            created_at: BigEndian::read_u64(&bytes[4..12]),
            valid_until: BigEndian::read_u64(&bytes[12..20]),
        })
    }
}

/// Stats metadata stored in stats table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StatsMetadata {
    pub access_count: u64,
    pub last_access: u64,
    pub importance: f32,
}

impl StatsMetadata {
    pub fn to_bytes(&self) -> [u8; 20] {
        let mut buf = [0u8; 20];
        BigEndian::write_u64(&mut buf[0..8], self.access_count);
        BigEndian::write_u64(&mut buf[8..16], self.last_access);
        BigEndian::write_f32(&mut buf[16..20], self.importance);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }
        Some(Self {
            access_count: BigEndian::read_u64(&bytes[0..8]),
            last_access: BigEndian::read_u64(&bytes[8..16]),
            importance: BigEndian::read_f32(&bytes[16..20]),
        })
    }
}

/// Helper for encoding primary record key: `(col_id: u32, record_id: u128)` -> 20 bytes.
pub fn encode_record_key(col_id: u32, record_id: u128) -> [u8; 20] {
    let mut key = [0u8; 20];
    BigEndian::write_u32(&mut key[0..4], col_id);
    BigEndian::write_u128(&mut key[4..20], record_id);
    key
}

pub fn decode_record_key(bytes: &[u8]) -> Option<(u32, u128)> {
    if bytes.len() != 20 {
        return None;
    }
    let col_id = BigEndian::read_u32(&bytes[0..4]);
    let record_id = BigEndian::read_u128(&bytes[4..20]);
    Some((col_id, record_id))
}

/// Composite secondary index key encoding:
/// `<col_id: u32> + <term_bytes> + 0x00 + <record_id: u128>`
pub fn encode_index_key(col_id: u32, term: &[u8], record_id: u128) -> Vec<u8> {
    let mut key = Vec::with_capacity(4 + term.len() + 1 + 16);
    key.extend_from_slice(&col_id.to_be_bytes());
    key.extend_from_slice(term);
    key.push(0x00); // Null delimiter
    key.extend_from_slice(&record_id.to_be_bytes());
    key
}

/// Generates prefix for index seek: `<col_id: u32> + <term_bytes> + 0x00`
pub fn encode_index_prefix(col_id: u32, term: &[u8]) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(4 + term.len() + 1);
    prefix.extend_from_slice(&col_id.to_be_bytes());
    prefix.extend_from_slice(term);
    prefix.push(0x00);
    prefix
}

pub fn decode_index_key(bytes: &[u8]) -> Option<(u32, Vec<u8>, u128)> {
    if bytes.len() < 4 + 1 + 16 {
        return None;
    }
    let col_id = BigEndian::read_u32(&bytes[0..4]);
    let split_pos = bytes[4..bytes.len() - 16].iter().position(|&b| b == 0x00)?;
    let term = bytes[4..4 + split_pos].to_vec();
    let record_id = BigEndian::read_u128(&bytes[bytes.len() - 16..]);
    Some((col_id, term, record_id))
}

/// Edge key encoding: `(src_id: u128, type_id: u32, dst_id: u128)` -> 36 bytes.
pub fn encode_edge_key(first_id: u128, type_id: u32, second_id: u128) -> [u8; 36] {
    let mut key = [0u8; 36];
    BigEndian::write_u128(&mut key[0..16], first_id);
    BigEndian::write_u32(&mut key[16..20], type_id);
    BigEndian::write_u128(&mut key[20..36], second_id);
    key
}

pub fn decode_edge_key(bytes: &[u8]) -> Option<(u128, u32, u128)> {
    if bytes.len() != 36 {
        return None;
    }
    let first = BigEndian::read_u128(&bytes[0..16]);
    let type_id = BigEndian::read_u32(&bytes[16..20]);
    let second = BigEndian::read_u128(&bytes[20..36]);
    Some((first, type_id, second))
}

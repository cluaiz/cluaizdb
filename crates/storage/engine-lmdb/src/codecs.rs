use std::borrow::Cow;
use heed::{BytesDecode, BytesEncode};
use cluaizdb_types::UniversalNeuron;

/// A custom heed codec for NeuronId -> UniversalNeuron
pub struct UniversalNeuronCodec;

impl<'a> BytesEncode<'a> for UniversalNeuronCodec {
    type EItem = UniversalNeuron;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
        let encoded = serde_json::to_vec(item)?;
        if item.tier == cluaizdb_types::StorageTier::Cold {
            let compressed = zstd::stream::encode_all(std::io::Cursor::new(encoded), 3)?;
            Ok(Cow::Owned(compressed))
        } else {
            Ok(Cow::Owned(encoded))
        }
    }
}

impl<'a> BytesDecode<'a> for UniversalNeuronCodec {
    type DItem = UniversalNeuron;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, Box<dyn std::error::Error + Send + Sync>> {
        let is_compressed = bytes.len() >= 4 && bytes[0..4] == [0x28, 0xB5, 0x2F, 0xFD];
        if is_compressed {
            let decompressed = zstd::stream::decode_all(std::io::Cursor::new(bytes))?;
            let decoded = serde_json::from_slice(&decompressed)?;
            Ok(decoded)
        } else {
            let decoded = serde_json::from_slice(bytes)?;
            Ok(decoded)
        }
    }
}

/// Codec for storing raw byte arrays (like ZSTD blocks)
pub struct RawBytesCodec;

impl<'a> BytesEncode<'a> for RawBytesCodec {
    type EItem = [u8];

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Cow::Borrowed(item))
    }
}

impl<'a> BytesDecode<'a> for RawBytesCodec {
    type DItem = &'a [u8];

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, Box<dyn std::error::Error + Send + Sync>> {
        Ok(bytes)
    }
}

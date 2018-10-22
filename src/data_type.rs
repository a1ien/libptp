use super::{Error, Read};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

#[allow(non_snake_case)]
#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    UNDEF,
    INT8(i8),
    UINT8(u8),
    INT16(i16),
    UINT16(u16),
    INT32(i32),
    UINT32(u32),
    INT64(i64),
    UINT64(u64),
    INT128(i128),
    UINT128(u128),
    AINT8(Vec<i8>),
    AUINT8(Vec<u8>),
    AINT16(Vec<i16>),
    AUINT16(Vec<u16>),
    AINT32(Vec<i32>),
    AUINT32(Vec<u32>),
    AINT64(Vec<i64>),
    AUINT64(Vec<u64>),
    AINT128(Vec<i128>),
    AUINT128(Vec<u128>),
    STR(String),
}

impl DataType {
    pub fn encode(&self) -> Vec<u8> {
        use self::DataType::*;
        let mut out = vec![];
        match self {
            // UNDEF => {},
            INT8(val) => {
                out.write_i8(*val).ok();
            }
            UINT8(val) => {
                out.write_u8(*val).ok();
            }
            INT16(val) => {
                out.write_i16::<LittleEndian>(*val).ok();
            }
            UINT16(val) => {
                out.write_u16::<LittleEndian>(*val).ok();
            }
            INT32(val) => {
                out.write_i32::<LittleEndian>(*val).ok();
            }
            UINT32(val) => {
                out.write_u32::<LittleEndian>(*val).ok();
            }
            INT64(val) => {
                out.write_i64::<LittleEndian>(*val).ok();
            }
            UINT64(val) => {
                out.write_u64::<LittleEndian>(*val).ok();
            }
            INT128(val) => {
                out.write_i128::<LittleEndian>(*val).ok();
            }
            UINT128(val) => {
                out.write_u128::<LittleEndian>(*val).ok();
            }
            AINT8(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_i8(*item).ok();
                }
            }
            AUINT8(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_u8(*item).ok();
                }
            }
            AINT16(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_i16::<LittleEndian>(*item).ok();
                }
            }
            AUINT16(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_u16::<LittleEndian>(*item).ok();
                }
            }
            AINT32(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_i32::<LittleEndian>(*item).ok();
                }
            }
            AUINT32(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_u32::<LittleEndian>(*item).ok();
                }
            }
            AINT64(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_i64::<LittleEndian>(*item).ok();
                }
            }
            AUINT64(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_u64::<LittleEndian>(*item).ok();
                }
            }
            AINT128(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_i128::<LittleEndian>(*item).ok();
                }
            }
            AUINT128(val) => {
                out.write_u32::<LittleEndian>(val.len() as u32).ok();
                for item in val {
                    out.write_u128::<LittleEndian>(*item).ok();
                }
            }
            STR(val) => {
                out.write_u8(((val.len() as u8) * 2) + 1).ok();
                if !val.is_empty() {
                    for e in val.encode_utf16() {
                        out.write_u16::<LittleEndian>(e).ok();
                    }
                    out.write_all(b"\0\0").ok();
                }
            }
            _ => {}
        }
        out
    }

    pub fn read_type<T: Read>(kind: u16, reader: &mut T) -> Result<DataType, Error> {
        use self::DataType::*;
        Ok(match kind {
            // 0x0000 => UNDEF,
            0x0001 => INT8(reader.read_ptp_i8()?),
            0x0002 => UINT8(reader.read_ptp_u8()?),
            0x0003 => INT16(reader.read_ptp_i16()?),
            0x0004 => UINT16(reader.read_ptp_u16()?),
            0x0005 => INT32(reader.read_ptp_i32()?),
            0x0006 => UINT32(reader.read_ptp_u32()?),
            0x0007 => INT64(reader.read_ptp_i64()?),
            0x0008 => UINT64(reader.read_ptp_u64()?),
            0x0009 => INT128(reader.read_ptp_i128()?),
            0x000A => UINT128(reader.read_ptp_u128()?),
            0x4001 => AINT8(reader.read_ptp_i8_vec()?),
            0x4002 => AUINT8(reader.read_ptp_u8_vec()?),
            0x4003 => AINT16(reader.read_ptp_i16_vec()?),
            0x4004 => AUINT16(reader.read_ptp_u16_vec()?),
            0x4005 => AINT32(reader.read_ptp_i32_vec()?),
            0x4006 => AUINT32(reader.read_ptp_u32_vec()?),
            0x4007 => AINT64(reader.read_ptp_i64_vec()?),
            0x4008 => AUINT64(reader.read_ptp_u64_vec()?),
            0x4009 => AINT128(reader.read_ptp_i128_vec()?),
            0x400A => AUINT128(reader.read_ptp_u128_vec()?),
            0xFFFF => STR(reader.read_ptp_str()?),
            _ => UNDEF,
        })
    }
}

impl From<i8> for DataType {
    fn from(value: i8) -> Self {
        DataType::INT8(value)
    }
}

impl From<u8> for DataType {
    fn from(value: u8) -> Self {
        DataType::UINT8(value)
    }
}

impl From<i16> for DataType {
    fn from(value: i16) -> Self {
        DataType::INT16(value)
    }
}

impl From<u16> for DataType {
    fn from(value: u16) -> Self {
        DataType::UINT16(value)
    }
}

impl From<i32> for DataType {
    fn from(value: i32) -> Self {
        DataType::INT32(value)
    }
}

impl From<u32> for DataType {
    fn from(value: u32) -> Self {
        DataType::UINT32(value)
    }
}

impl From<i64> for DataType {
    fn from(value: i64) -> Self {
        DataType::INT64(value)
    }
}

impl From<u64> for DataType {
    fn from(value: u64) -> Self {
        DataType::UINT64(value)
    }
}

impl From<&str> for DataType {
    fn from(value: &str) -> Self {
        DataType::STR(value.to_owned())
    }
}

impl From<String> for DataType {
    fn from(value: String) -> Self {
        DataType::STR(value)
    }
}

#[derive(Debug)]
pub enum FormData {
    None,
    Range {
        min_value: DataType,
        max_value: DataType,
        step: DataType,
    },
    Enumeration {
        array: Vec<DataType>,
    },
}

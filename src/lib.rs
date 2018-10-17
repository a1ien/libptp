#![allow(non_snake_case)]
#[macro_use]
extern crate log;

use libusb;

use byteorder::{LittleEndian, ReadBytesExt};
use std::cmp::min;
use std::io::Cursor;

mod read;
mod data_type;
mod camera;
mod error;

pub use self::read::PtpRead;
pub use self::data_type::{PtpDataType, PtpFormData};
pub use self::camera::PtpCamera;
pub use self::error::Error;

#[derive(Debug, PartialEq)]
#[repr(u16)]
pub enum PtpContainerType {
    Command = 1,
    Data = 2,
    Response = 3,
    Event = 4,
}

impl PtpContainerType {
    fn from_u16(v: u16) -> Option<PtpContainerType> {
        use self::PtpContainerType::*;
        match v {
            1 => Some(Command),
            2 => Some(Data),
            3 => Some(Response),
            4 => Some(Event),
            _ => None,
        }
    }
}

pub type ResponseCode = u16;

#[allow(non_upper_case_globals)]
pub mod StandardResponseCode {
    use super::ResponseCode;

    pub const Undefined: ResponseCode = 0x2000;
    pub const Ok: ResponseCode = 0x2001;
    pub const GeneralError: ResponseCode = 0x2002;
    pub const SessionNotOpen: ResponseCode = 0x2003;
    pub const InvalidTransactionId: ResponseCode = 0x2004;
    pub const OperationNotSupported: ResponseCode = 0x2005;
    pub const ParameterNotSupported: ResponseCode = 0x2006;
    pub const IncompleteTransfer: ResponseCode = 0x2007;
    pub const InvalidStorageId: ResponseCode = 0x2008;
    pub const InvalidObjectHandle: ResponseCode = 0x2009;
    pub const DevicePropNotSupported: ResponseCode = 0x200A;
    pub const InvalidObjectFormatCode: ResponseCode = 0x200B;
    pub const StoreFull: ResponseCode = 0x200C;
    pub const ObjectWriteProtected: ResponseCode = 0x200D;
    pub const StoreReadOnly: ResponseCode = 0x200E;
    pub const AccessDenied: ResponseCode = 0x200F;
    pub const NoThumbnailPresent: ResponseCode = 0x2010;
    pub const SelfTestFailed: ResponseCode = 0x2011;
    pub const PartialDeletion: ResponseCode = 0x2012;
    pub const StoreNotAvailable: ResponseCode = 0x2013;
    pub const SpecificationByFormatUnsupported: ResponseCode = 0x2014;
    pub const NoValidObjectInfo: ResponseCode = 0x2015;
    pub const InvalidCodeFormat: ResponseCode = 0x2016;
    pub const UnknownVendorCode: ResponseCode = 0x2017;
    pub const CaptureAlreadyTerminated: ResponseCode = 0x2018;
    pub const DeviceBusy: ResponseCode = 0x2019;
    pub const InvalidParentObject: ResponseCode = 0x201A;
    pub const InvalidDevicePropFormat: ResponseCode = 0x201B;
    pub const InvalidDevicePropValue: ResponseCode = 0x201C;
    pub const InvalidParameter: ResponseCode = 0x201D;
    pub const SessionAlreadyOpen: ResponseCode = 0x201E;
    pub const TransactionCancelled: ResponseCode = 0x201F;
    pub const SpecificationOfDestinationUnsupported: ResponseCode = 0x2020;

    pub fn name(v: ResponseCode) -> Option<&'static str> {
        match v {
            Undefined => Some("Undefined"),
            Ok => Some("Ok"),
            GeneralError => Some("GeneralError"),
            SessionNotOpen => Some("SessionNotOpen"),
            InvalidTransactionId => Some("InvalidTransactionId"),
            OperationNotSupported => Some("OperationNotSupported"),
            ParameterNotSupported => Some("ParameterNotSupported"),
            IncompleteTransfer => Some("IncompleteTransfer"),
            InvalidStorageId => Some("InvalidStorageId"),
            InvalidObjectHandle => Some("InvalidObjectHandle"),
            DevicePropNotSupported => Some("DevicePropNotSupported"),
            InvalidObjectFormatCode => Some("InvalidObjectFormatCode"),
            StoreFull => Some("StoreFull"),
            ObjectWriteProtected => Some("ObjectWriteProtected"),
            StoreReadOnly => Some("StoreReadOnly"),
            AccessDenied => Some("AccessDenied"),
            NoThumbnailPresent => Some("NoThumbnailPresent"),
            SelfTestFailed => Some("SelfTestFailed"),
            PartialDeletion => Some("PartialDeletion"),
            StoreNotAvailable => Some("StoreNotAvailable"),
            SpecificationByFormatUnsupported => Some("SpecificationByFormatUnsupported"),
            NoValidObjectInfo => Some("NoValidObjectInfo"),
            InvalidCodeFormat => Some("InvalidCodeFormat"),
            UnknownVendorCode => Some("UnknownVendorCode"),
            CaptureAlreadyTerminated => Some("CaptureAlreadyTerminated"),
            DeviceBusy => Some("DeviceBusy"),
            InvalidParentObject => Some("InvalidParentObject"),
            InvalidDevicePropFormat => Some("InvalidDevicePropFormat"),
            InvalidDevicePropValue => Some("InvalidDevicePropValue"),
            InvalidParameter => Some("InvalidParameter"),
            SessionAlreadyOpen => Some("SessionAlreadyOpen"),
            TransactionCancelled => Some("TransactionCancelled"),
            SpecificationOfDestinationUnsupported => Some("SpecificationOfDestinationUnsupported"),
            _ => None,
        }
    }
}

pub type CommandCode = u16;

#[allow(non_upper_case_globals)]
pub mod StandardCommandCode {
    use super::CommandCode;

    pub const Undefined: CommandCode = 0x1000;
    pub const GetDeviceInfo: CommandCode = 0x1001;
    pub const OpenSession: CommandCode = 0x1002;
    pub const CloseSession: CommandCode = 0x1003;
    pub const GetStorageIDs: CommandCode = 0x1004;
    pub const GetStorageInfo: CommandCode = 0x1005;
    pub const GetNumObjects: CommandCode = 0x1006;
    pub const GetObjectHandles: CommandCode = 0x1007;
    pub const GetObjectInfo: CommandCode = 0x1008;
    pub const GetObject: CommandCode = 0x1009;
    pub const GetThumb: CommandCode = 0x100A;
    pub const DeleteObject: CommandCode = 0x100B;
    pub const SendObjectInfo: CommandCode = 0x100C;
    pub const SendObject: CommandCode = 0x100D;
    pub const InitiateCapture: CommandCode = 0x100E;
    pub const FormatStore: CommandCode = 0x100F;
    pub const ResetDevice: CommandCode = 0x1010;
    pub const SelfTest: CommandCode = 0x1011;
    pub const SetObjectProtection: CommandCode = 0x1012;
    pub const PowerDown: CommandCode = 0x1013;
    pub const GetDevicePropDesc: CommandCode = 0x1014;
    pub const GetDevicePropValue: CommandCode = 0x1015;
    pub const SetDevicePropValue: CommandCode = 0x1016;
    pub const ResetDevicePropValue: CommandCode = 0x1017;
    pub const TerminateOpenCapture: CommandCode = 0x1018;
    pub const MoveObject: CommandCode = 0x1019;
    pub const CopyObject: CommandCode = 0x101A;
    pub const GetPartialObject: CommandCode = 0x101B;
    pub const InitiateOpenCapture: CommandCode = 0x101C;

    pub fn name(v: CommandCode) -> Option<&'static str> {
        match v {
            Undefined => Some("Undefined"),
            GetDeviceInfo => Some("GetDeviceInfo"),
            OpenSession => Some("OpenSession"),
            CloseSession => Some("CloseSession"),
            GetStorageIDs => Some("GetStorageIDs"),
            GetStorageInfo => Some("GetStorageInfo"),
            GetNumObjects => Some("GetNumObjects"),
            GetObjectHandles => Some("GetObjectHandles"),
            GetObjectInfo => Some("GetObjectInfo"),
            GetObject => Some("GetObject"),
            GetThumb => Some("GetThumb"),
            DeleteObject => Some("DeleteObject"),
            SendObjectInfo => Some("SendObjectInfo"),
            SendObject => Some("SendObject"),
            InitiateCapture => Some("InitiateCapture"),
            FormatStore => Some("FormatStore"),
            ResetDevice => Some("ResetDevice"),
            SelfTest => Some("SelfTest"),
            SetObjectProtection => Some("SetObjectProtection"),
            PowerDown => Some("PowerDown"),
            GetDevicePropDesc => Some("GetDevicePropDesc"),
            GetDevicePropValue => Some("GetDevicePropValue"),
            SetDevicePropValue => Some("SetDevicePropValue"),
            ResetDevicePropValue => Some("ResetDevicePropValue"),
            TerminateOpenCapture => Some("TerminateOpenCapture"),
            MoveObject => Some("MoveObject"),
            CopyObject => Some("CopyObject"),
            GetPartialObject => Some("GetPartialObject"),
            InitiateOpenCapture => Some("InitiateOpenCapture"),
            _ => None,
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct PtpDeviceInfo {
    pub Version: u16,
    pub VendorExID: u32,
    pub VendorExVersion: u16,
    pub VendorExtensionDesc: String,
    pub FunctionalMode: u16,
    pub OperationsSupported: Vec<u16>,
    pub EventsSupported: Vec<u16>,
    pub DevicePropertiesSupported: Vec<u16>,
    pub CaptureFormats: Vec<u16>,
    pub ImageFormats: Vec<u16>,
    pub Manufacturer: String,
    pub Model: String,
    pub DeviceVersion: String,
    pub SerialNumber: String,
}

impl PtpDeviceInfo {
    pub fn decode(buf: &[u8]) -> Result<PtpDeviceInfo, Error> {
        let mut cur = Cursor::new(buf);

        Ok(PtpDeviceInfo {
            Version: cur.read_ptp_u16()?,
            VendorExID: cur.read_ptp_u32()?,
            VendorExVersion: cur.read_ptp_u16()?,
            VendorExtensionDesc: cur.read_ptp_str()?,
            FunctionalMode: cur.read_ptp_u16()?,
            OperationsSupported: cur.read_ptp_u16_vec()?,
            EventsSupported: cur.read_ptp_u16_vec()?,
            DevicePropertiesSupported: cur.read_ptp_u16_vec()?,
            CaptureFormats: cur.read_ptp_u16_vec()?,
            ImageFormats: cur.read_ptp_u16_vec()?,
            Manufacturer: cur.read_ptp_str()?,
            Model: cur.read_ptp_str()?,
            DeviceVersion: cur.read_ptp_str()?,
            SerialNumber: cur.read_ptp_str()?,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PtpObjectInfo {
    pub StorageID: u32,
    pub ObjectFormat: u16,
    pub ProtectionStatus: u16,
    pub ObjectCompressedSize: u32,
    pub ThumbFormat: u16,
    pub ThumbCompressedSize: u32,
    pub ThumbPixWidth: u32,
    pub ThumbPixHeight: u32,
    pub ImagePixWidth: u32,
    pub ImagePixHeight: u32,
    pub ImageBitDepth: u32,
    pub ParentObject: u32,
    pub AssociationType: u16,
    pub AssociationDesc: u32,
    pub SequenceNumber: u32,
    pub Filename: String,
    pub CaptureDate: String,
    pub ModificationDate: String,
    pub Keywords: String,
}

impl PtpObjectInfo {
    pub fn decode(buf: &[u8]) -> Result<PtpObjectInfo, Error> {
        let mut cur = Cursor::new(buf);

        Ok(PtpObjectInfo {
            StorageID: cur.read_ptp_u32()?,
            ObjectFormat: cur.read_ptp_u16()?,
            ProtectionStatus: cur.read_ptp_u16()?,
            ObjectCompressedSize: cur.read_ptp_u32()?,
            ThumbFormat: cur.read_ptp_u16()?,
            ThumbCompressedSize: cur.read_ptp_u32()?,
            ThumbPixWidth: cur.read_ptp_u32()?,
            ThumbPixHeight: cur.read_ptp_u32()?,
            ImagePixWidth: cur.read_ptp_u32()?,
            ImagePixHeight: cur.read_ptp_u32()?,
            ImageBitDepth: cur.read_ptp_u32()?,
            ParentObject: cur.read_ptp_u32()?,
            AssociationType: cur.read_ptp_u16()?,
            AssociationDesc: cur.read_ptp_u32()?,
            SequenceNumber: cur.read_ptp_u32()?,
            Filename: cur.read_ptp_str()?,
            CaptureDate: cur.read_ptp_str()?,
            ModificationDate: cur.read_ptp_str()?,
            Keywords: cur.read_ptp_str()?,
        })
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct PtpStorageInfo {
    pub StorageType: u16,
    pub FilesystemType: u16,
    pub AccessCapability: u16,
    pub MaxCapacity: u64,
    pub FreeSpaceInBytes: u64,
    pub FreeSpaceInImages: u32,
    pub StorageDescription: String,
    pub VolumeLabel: String,
}

impl PtpStorageInfo {
    pub fn decode<T: PtpRead>(cur: &mut T) -> Result<PtpStorageInfo, Error> {
        Ok(PtpStorageInfo {
            StorageType: cur.read_ptp_u16()?,
            FilesystemType: cur.read_ptp_u16()?,
            AccessCapability: cur.read_ptp_u16()?,
            MaxCapacity: cur.read_ptp_u64()?,
            FreeSpaceInBytes: cur.read_ptp_u64()?,
            FreeSpaceInImages: cur.read_ptp_u32()?,
            StorageDescription: cur.read_ptp_str()?,
            VolumeLabel: cur.read_ptp_str()?,
        })
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct PtpPropInfo {
    pub PropertyCode: u16,
    pub DataType: u16,
    pub GetSet: u8,
    pub IsEnable: u8,
    pub FactoryDefault: PtpDataType,
    pub Current: PtpDataType,
    pub Form: PtpFormData,
}

impl PtpPropInfo {
    pub fn decode<T: PtpRead>(cur: &mut T) -> Result<PtpPropInfo, Error> {
        let data_type;
        Ok(PtpPropInfo {
            PropertyCode: cur.read_u16::<LittleEndian>()?,
            DataType: {
                data_type = cur.read_u16::<LittleEndian>()?;
                data_type
            },
            GetSet: cur.read_u8()?,
            IsEnable: cur.read_u8()?,
            FactoryDefault: PtpDataType::read_type(data_type, cur)?,
            Current: PtpDataType::read_type(data_type, cur)?,
            Form: {
                match cur.read_u8()? {
                    // 0x00 => PtpFormData::None,
                    0x01 => PtpFormData::Range {
                        minValue: PtpDataType::read_type(data_type, cur)?,
                        maxValue: PtpDataType::read_type(data_type, cur)?,
                        step: PtpDataType::read_type(data_type, cur)?,
                    },
                    0x02 => PtpFormData::Enumeration {
                        array: {
                            let len = cur.read_u16::<LittleEndian>()? as usize;
                            let mut arr = Vec::with_capacity(len);
                            for _ in 0..len {
                                arr.push(PtpDataType::read_type(data_type, cur)?);
                            }
                            arr
                        },
                    },
                    _ => PtpFormData::None,
                }
            },
        })
    }
}

#[derive(Debug)]
struct PtpContainerInfo {
    /// payload len in bytes, usually relevant for data phases
    payload_len: usize,

    /// Container kind
    kind: PtpContainerType,

    /// StandardCommandCode or ResponseCode, depending on 'kind'
    code: u16,

    /// transaction ID that this container belongs to
    tid: u32,
}

const PTP_CONTAINER_INFO_SIZE: usize = 12;

impl PtpContainerInfo {
    pub fn parse<R: ReadBytesExt>(mut r: R) -> Result<PtpContainerInfo, Error> {
        let len = r.read_u32::<LittleEndian>()?;
        let kind_u16 = r.read_u16::<LittleEndian>()?;
        let kind = PtpContainerType::from_u16(kind_u16)
            .ok_or_else(|| Error::Malformed(format!("Invalid message type {:x}.", kind_u16)))?;
        let code = r.read_u16::<LittleEndian>()?;
        let tid = r.read_u32::<LittleEndian>()?;

        Ok(PtpContainerInfo {
            payload_len: len as usize - PTP_CONTAINER_INFO_SIZE,
            kind: kind,
            tid: tid,
            code: code,
        })
    }

    // does this container belong to the given transaction?
    pub fn belongs_to(&self, tid: u32) -> bool {
        self.tid == tid
    }
}

#[derive(Debug, Clone)]
pub struct PtpObjectTree {
    pub handle: u32,
    pub info: PtpObjectInfo,
    pub children: Option<Vec<PtpObjectTree>>,
}

impl PtpObjectTree {
    pub fn walk(&self) -> Vec<(String, PtpObjectTree)> {
        let mut input = vec![("".to_owned(), self.clone())];
        let mut output = vec![];

        while !input.is_empty() {
            for (prefix, item) in input.split_off(0) {
                let path = prefix.clone()
                    + (if prefix.is_empty() { "" } else { "/" })
                    + &item.info.Filename;

                output.push((path.clone(), item.clone()));

                if let Some(children) = item.children {
                    input.extend(children.into_iter().map(|x| (path.clone(), x)));
                }
            }
        }

        output
    }
}

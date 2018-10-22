#![allow(non_snake_case)]
#[macro_use]
extern crate log;

use byteorder::LittleEndian;
use std::io::Cursor;

mod camera;
mod data_type;
mod error;
mod read;

pub use self::camera::Camera;
pub use self::data_type::{DataType, FormData};
pub use self::error::Error;
pub use self::read::Read;

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
pub struct DeviceInfo {
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

impl DeviceInfo {
    pub fn decode(buf: &[u8]) -> Result<DeviceInfo, Error> {
        let mut cur = Cursor::new(buf);

        Ok(DeviceInfo {
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
pub struct ObjectInfo {
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

impl ObjectInfo {
    pub fn decode(buf: &[u8]) -> Result<ObjectInfo, Error> {
        let mut cur = Cursor::new(buf);

        Ok(ObjectInfo {
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
pub struct StorageInfo {
    pub StorageType: u16,
    pub FilesystemType: u16,
    pub AccessCapability: u16,
    pub MaxCapacity: u64,
    pub FreeSpaceInBytes: u64,
    pub FreeSpaceInImages: u32,
    pub StorageDescription: String,
    pub VolumeLabel: String,
}

impl StorageInfo {
    pub fn decode<T: Read>(cur: &mut T) -> Result<StorageInfo, Error> {
        Ok(StorageInfo {
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

#[derive(Debug)]
pub struct PropInfo {
    /// A specific property_code.
    pub property_code: u16,
    /// This field identifies the Datatype Code of the property.
    pub data_type: u16,
    /// This field indicates whether the property is read-only or read-write.
    pub get_set: u8,
    pub factory_default: DataType,
    pub current: DataType,
    pub form: FormData,
}

impl PropInfo {
    pub fn decode<T: Read>(cur: &mut T) -> Result<PropInfo, Error> {
        let data_type;
        Ok(PropInfo {
            property_code: cur.read_ptp_u16()?,
            data_type: {
                data_type = cur.read_ptp_u16()?;
                data_type
            },
            get_set: cur.read_u8()?,
            factory_default: DataType::read_type(data_type, cur)?,
            current: DataType::read_type(data_type, cur)?,
            form: {
                match cur.read_u8()? {
                    // 0x00 => FormData::None,
                    0x01 => FormData::Range {
                        min_value: DataType::read_type(data_type, cur)?,
                        max_value: DataType::read_type(data_type, cur)?,
                        step: DataType::read_type(data_type, cur)?,
                    },
                    0x02 => FormData::Enumeration {
                        array: {
                            let len = cur.read_u16::<LittleEndian>()? as usize;
                            let mut arr = Vec::with_capacity(len);
                            for _ in 0..len {
                                arr.push(DataType::read_type(data_type, cur)?);
                            }
                            arr
                        },
                    },
                    _ => FormData::None,
                }
            },
        })
    }
}

#[derive(Debug)]
pub struct PropInfoSony {
    /// A specific property_code.
    pub property_code: u16,
    /// This field identifies the Datatype Code of the property.
    pub data_type: u16,
    /// This field indicates whether the property is read-only or read-write.
    pub get_set: u8,
    /// This field indicates whether the property is valid, invalid or DispOnly.
    pub is_enable: u8,
    pub factory_default: DataType,
    pub current: DataType,
    pub form: FormData,
}

impl PropInfoSony {
    pub fn decode<T: Read>(cur: &mut T) -> Result<PropInfoSony, Error> {
        let data_type;
        Ok(PropInfoSony {
            property_code: cur.read_ptp_u16()?,
            data_type: {
                data_type = cur.read_ptp_u16()?;
                data_type
            },
            get_set: cur.read_u8()?,
            is_enable: cur.read_u8()?,
            factory_default: DataType::read_type(data_type, cur)?,
            current: DataType::read_type(data_type, cur)?,
            form: {
                match cur.read_u8()? {
                    // 0x00 => FormData::None,
                    0x01 => FormData::Range {
                        min_value: DataType::read_type(data_type, cur)?,
                        max_value: DataType::read_type(data_type, cur)?,
                        step: DataType::read_type(data_type, cur)?,
                    },
                    0x02 => FormData::Enumeration {
                        array: {
                            let len = cur.read_u16::<LittleEndian>()? as usize;
                            let mut arr = Vec::with_capacity(len);
                            for _ in 0..len {
                                arr.push(DataType::read_type(data_type, cur)?);
                            }
                            arr
                        },
                    },
                    _ => FormData::None,
                }
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct ObjectTree {
    pub handle: u32,
    pub info: ObjectInfo,
    pub children: Option<Vec<ObjectTree>>,
}

impl ObjectTree {
    pub fn walk(&self) -> Vec<(String, ObjectTree)> {
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

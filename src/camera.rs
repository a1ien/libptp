use super::{
    CommandCode, DeviceInfo, Error, ObjectInfo, Read, StandardCommandCode, StandardResponseCode,
    StorageInfo,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rusb::{constants, UsbContext};
use std::sync::{Arc, RwLock};
use std::{cmp::min, io::Cursor, slice, time::Duration};

pub struct Camera<T: UsbContext> {
    iface: u8,
    ep_in: u8,
    ep_out: u8,
    _ep_int: u8,
    current_tid: u32,
    handle: Arc<RwLock<rusb::DeviceHandle<T>>>,
}

impl<T: UsbContext> Camera<T> {
    pub fn new(device: &rusb::Device<T>) -> Result<Camera<T>, Error> {
        let config_desc = device.active_config_descriptor()?;

        let interface_desc = config_desc
            .interfaces()
            .flat_map(|i| i.descriptors())
            .find(|x| x.class_code() == constants::LIBUSB_CLASS_IMAGE)
            .ok_or(rusb::Error::NotFound)?;

        debug!("Found interface {}", interface_desc.interface_number());

        let mut handle = device.open()?;

        handle.claim_interface(interface_desc.interface_number())?;

        let find_endpoint = |direction, transfer_type| {
            interface_desc
                .endpoint_descriptors()
                .find(|ep| ep.direction() == direction && ep.transfer_type() == transfer_type)
                .map(|x| x.address())
                .ok_or(rusb::Error::NotFound)
        };

        Ok(Camera {
            iface: interface_desc.interface_number(),
            ep_in: find_endpoint(rusb::Direction::In, rusb::TransferType::Bulk)?,
            ep_out: find_endpoint(rusb::Direction::Out, rusb::TransferType::Bulk)?,
            _ep_int: find_endpoint(rusb::Direction::In, rusb::TransferType::Interrupt)?,
            current_tid: 0,
            handle: Arc::new(RwLock::new(handle)),
        })
    }

    /// execute a PTP transaction.
    /// consists of the following phases:
    ///  - command
    ///  - command data (optional, if `data` is Some)
    ///  - response data (optional, if response contains a payload)
    ///  - response status
    /// NB: each phase involves a separate USB transfer, and `timeout` is used for each phase,
    /// so the total time taken may be greater than `timeout`.
    pub fn command(
        &mut self,
        code: CommandCode,
        params: &[u32],
        data: Option<&[u8]>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, Error> {
        // timeout of 0 means unlimited timeout.
        let timeout = timeout.unwrap_or_else(Duration::default);

        let tid = self.current_tid;
        self.current_tid += 1;

        // Prepare payload of the request phase, containing the parameters
        let mut request_payload = Vec::with_capacity(params.len() * 4);
        for p in params {
            request_payload.write_u32::<LittleEndian>(*p).ok();
        }

        self.write_txn_phase(ContainerType::Command, code, tid, &request_payload, timeout)?;

        if let Some(data) = data {
            self.write_txn_phase(ContainerType::Data, code, tid, data, timeout)?;
        }

        // request phase is followed by data phase (optional) and response phase.
        // read both, check the status on the response, and return the data payload, if any.
        let mut data_phase_payload = vec![];
        loop {
            let (container, payload) = self.read_txn_phase(timeout)?;
            if !container.belongs_to(tid) {
                return Err(Error::Malformed(format!(
                    "mismatched txnid {}, expecting {}",
                    container.tid, tid
                )));
            }
            match container.kind {
                ContainerType::Data => {
                    data_phase_payload = payload;
                }
                ContainerType::Response => {
                    if container.code != StandardResponseCode::Ok {
                        return Err(Error::Response(container.code));
                    }
                    return Ok(data_phase_payload);
                }
                _ => {}
            }
        }
    }

    fn write_txn_phase(
        &mut self,
        kind: ContainerType,
        code: CommandCode,
        tid: u32,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<(), Error> {
        trace!(
            "Write {:?} - 0x{:04x} ({}), tid:{}",
            kind,
            code,
            StandardCommandCode::name(code).unwrap_or("unknown"),
            tid
        );

        const CHUNK_SIZE: usize = 1024 * 1024; // 1MB, must be a multiple of the endpoint packet size

        // The first chunk contains the header, and its payload must be copied into the temporary buffer
        let first_chunk_payload_bytes = min(payload.len(), CHUNK_SIZE - CONTAINER_INFO_SIZE);
        let mut buf = Vec::with_capacity(first_chunk_payload_bytes + CONTAINER_INFO_SIZE);
        buf.write_u32::<LittleEndian>((payload.len() + CONTAINER_INFO_SIZE) as u32)
            .ok();
        buf.write_u16::<LittleEndian>(kind as u16).ok();
        buf.write_u16::<LittleEndian>(code).ok();
        buf.write_u32::<LittleEndian>(tid).ok();
        buf.extend_from_slice(&payload[..first_chunk_payload_bytes]);
        self.handle
            .read()
            .unwrap()
            .write_bulk(self.ep_out, &buf, timeout)?;

        // Write any subsequent chunks, straight from the source slice
        for chunk in payload[first_chunk_payload_bytes..].chunks(CHUNK_SIZE) {
            self.handle
                .read()
                .unwrap()
                .write_bulk(self.ep_out, chunk, timeout)?;
        }

        Ok(())
    }

    // helper for command() above, retrieve container info and payload for the current phase
    fn read_txn_phase(&mut self, timeout: Duration) -> Result<(ContainerInfo, Vec<u8>), Error> {
        // buf is stack allocated and intended to be large enough to accomodate most
        // cmd/ctrl data (ie, not media) without allocating. payload handling below
        // deals with larger media responses. mark it as uninitalized to avoid paying
        // for zeroing out 8k of memory, since rust doesn't know what rusb does with this memory.
        let mut unintialized_buf: [u8; 8 * 1024];
        let buf = unsafe {
            unintialized_buf = ::std::mem::uninitialized();
            let n = self.handle.read().unwrap().read_bulk(
                self.ep_in,
                &mut unintialized_buf[..],
                timeout,
            )?;
            &unintialized_buf[..n]
        };

        let cinfo = ContainerInfo::parse(&buf[..])?;
        trace!("container {:?}", cinfo);

        // no payload? we're done
        if cinfo.payload_len == 0 {
            return Ok((cinfo, vec![]));
        }

        // allocate one extra to avoid a separate read for trailing short packet
        let mut payload = Vec::with_capacity(cinfo.payload_len + 1);
        payload.extend_from_slice(&buf[CONTAINER_INFO_SIZE..]);

        // response didn't fit into our original buf? read the rest
        // or if our original read were satisfied exactly, so there is still a ZLP to read
        if payload.len() < cinfo.payload_len || buf.len() == unintialized_buf.len() {
            unsafe {
                let p = payload.as_mut_ptr().add(payload.len());
                let pslice = slice::from_raw_parts_mut(p, payload.capacity() - payload.len());
                let n = self
                    .handle
                    .read()
                    .unwrap()
                    .read_bulk(self.ep_in, pslice, timeout)?;
                let sz = payload.len();
                payload.set_len(sz + n);
                trace!(
                    "  bulk rx {}, ({}/{})",
                    n,
                    payload.len(),
                    payload.capacity()
                );
            }
        }

        Ok((cinfo, payload))
    }

    pub fn get_objectinfo(
        &mut self,
        handle: u32,
        timeout: Option<Duration>,
    ) -> Result<ObjectInfo, Error> {
        let data = self.command(StandardCommandCode::GetObjectInfo, &[handle], None, timeout)?;
        Ok(ObjectInfo::decode(&data)?)
    }

    pub fn get_object(&mut self, handle: u32, timeout: Option<Duration>) -> Result<Vec<u8>, Error> {
        self.command(StandardCommandCode::GetObject, &[handle], None, timeout)
    }

    pub fn get_partialobject(
        &mut self,
        handle: u32,
        offset: u32,
        max: u32,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, Error> {
        self.command(
            StandardCommandCode::GetPartialObject,
            &[handle, offset, max],
            None,
            timeout,
        )
    }

    pub fn delete_object(&mut self, handle: u32, timeout: Option<Duration>) -> Result<(), Error> {
        self.command(StandardCommandCode::DeleteObject, &[handle], None, timeout)
            .map(|_| ())
    }

    pub fn power_down(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        self.command(StandardCommandCode::PowerDown, &[], None, timeout)
            .map(|_| ())
    }

    pub fn get_objecthandles(
        &mut self,
        storage_id: u32,
        handle_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u32>, Error> {
        let data = self.command(
            StandardCommandCode::GetObjectHandles,
            &[storage_id, filter.unwrap_or(0x0), handle_id],
            None,
            timeout,
        )?;
        // Parse ObjectHandleArrray
        let mut cur = Cursor::new(data);
        let value = cur.read_ptp_u32_vec()?;
        cur.expect_end()?;

        Ok(value)
    }

    pub fn get_objecthandles_root(
        &mut self,
        storage_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u32>, Error> {
        self.get_objecthandles(storage_id, 0xFFFF_FFFF, filter, timeout)
    }

    pub fn get_objecthandles_all(
        &mut self,
        storage_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u32>, Error> {
        self.get_objecthandles(storage_id, 0x0, filter, timeout)
    }

    // handle_id: None == root of store
    pub fn get_numobjects(
        &mut self,
        storage_id: u32,
        handle_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<u32, Error> {
        let data = self.command(
            StandardCommandCode::GetNumObjects,
            &[storage_id, filter.unwrap_or(0x0), handle_id],
            None,
            timeout,
        )?;

        // Parse ObjectHandleArrray
        let mut cur = Cursor::new(data);
        let value = cur.read_ptp_u32()?;
        cur.expect_end()?;

        Ok(value)
    }

    pub fn get_storage_info(
        &mut self,
        storage_id: u32,
        timeout: Option<Duration>,
    ) -> Result<StorageInfo, Error> {
        let data = self.command(
            StandardCommandCode::GetStorageInfo,
            &[storage_id],
            None,
            timeout,
        )?;

        // Parse ObjectHandleArrray
        let mut cur = Cursor::new(data);
        let res = StorageInfo::decode(&mut cur)?;
        cur.expect_end()?;

        Ok(res)
    }

    pub fn get_storageids(&mut self, timeout: Option<Duration>) -> Result<Vec<u32>, Error> {
        let data = self.command(StandardCommandCode::GetStorageIDs, &[], None, timeout)?;

        // Parse ObjectHandleArrray
        let mut cur = Cursor::new(data);
        let value = cur.read_ptp_u32_vec()?;
        cur.expect_end()?;

        Ok(value)
    }

    pub fn get_numobjects_roots(
        &mut self,
        storage_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<u32, Error> {
        self.get_numobjects(storage_id, 0xFFFF_FFFF, filter, timeout)
    }

    pub fn get_numobjects_all(
        &mut self,
        storage_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<u32, Error> {
        self.get_numobjects(storage_id, 0x0, filter, timeout)
    }

    pub fn get_device_info(&mut self, timeout: Option<Duration>) -> Result<DeviceInfo, Error> {
        let data = self.command(
            StandardCommandCode::GetDeviceInfo,
            &[0, 0, 0],
            None,
            timeout,
        )?;

        let device_info = DeviceInfo::decode(&data)?;
        debug!("device_info {:?}", device_info);
        Ok(device_info)
    }

    pub fn open_session(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        let session_id = 1;

        self.command(
            StandardCommandCode::OpenSession,
            &[session_id, 0, 0],
            None,
            timeout,
        )?;

        Ok(())
    }

    pub fn close_session(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        self.command(StandardCommandCode::CloseSession, &[], None, timeout)?;

        Ok(())
    }

    pub fn disconnect(&mut self, timeout: Option<Duration>) -> Result<(), Error> {
        self.close_session(timeout)?;
        self.handle.write().unwrap().release_interface(self.iface)?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.handle.write().unwrap().reset()?;
        Ok(())
    }

    pub fn clear_halt(&mut self) -> Result<(), Error> {
        self.handle.write().unwrap().clear_halt(self.ep_in)?;
        self.handle.write().unwrap().clear_halt(self.ep_out)?;
        self.handle.write().unwrap().clear_halt(self._ep_int)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
#[repr(u16)]
enum ContainerType {
    Command = 1,
    Data = 2,
    Response = 3,
    Event = 4,
}

impl ContainerType {
    fn from_u16(v: u16) -> Option<ContainerType> {
        use self::ContainerType::*;
        match v {
            1 => Some(Command),
            2 => Some(Data),
            3 => Some(Response),
            4 => Some(Event),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct ContainerInfo {
    /// payload len in bytes, usually relevant for data phases
    payload_len: usize,

    /// Container kind
    kind: ContainerType,

    /// StandardCommandCode or ResponseCode, depending on 'kind'
    code: u16,

    /// transaction ID that this container belongs to
    tid: u32,
}

const CONTAINER_INFO_SIZE: usize = 12;

impl ContainerInfo {
    pub fn parse<R: ReadBytesExt>(mut r: R) -> Result<ContainerInfo, Error> {
        let len = r.read_u32::<LittleEndian>()?;
        let kind_u16 = r.read_u16::<LittleEndian>()?;
        let kind = ContainerType::from_u16(kind_u16)
            .ok_or_else(|| Error::Malformed(format!("Invalid message type {:x}.", kind_u16)))?;
        let code = r.read_u16::<LittleEndian>()?;
        let tid = r.read_u32::<LittleEndian>()?;

        Ok(ContainerInfo {
            payload_len: len as usize - CONTAINER_INFO_SIZE,
            kind,
            tid,
            code,
        })
    }

    // does this container belong to the given transaction?
    pub fn belongs_to(&self, tid: u32) -> bool {
        self.tid == tid
    }
}

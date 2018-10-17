use crate::*;
use libusb::constants;
use std::{io::Cursor, slice, time::Duration};

pub struct PtpCamera<'a> {
    iface: u8,
    ep_in: u8,
    ep_out: u8,
    _ep_int: u8,
    current_tid: u32,
    handle: libusb::DeviceHandle<'a>,
}

impl<'a> PtpCamera<'a> {
    pub fn new(device: &libusb::Device<'a>) -> Result<PtpCamera<'a>, Error> {
        let config_desc = device.active_config_descriptor()?;

        let interface_desc = config_desc
            .interfaces()
            .flat_map(|i| i.descriptors())
            .find(|x| x.class_code() == constants::LIBUSB_CLASS_IMAGE)
            .ok_or(libusb::Error::NotFound)?;

        debug!("Found interface {}", interface_desc.interface_number());

        let mut handle = device.open()?;

        handle.claim_interface(interface_desc.interface_number())?;
        handle.set_alternate_setting(
            interface_desc.interface_number(),
            interface_desc.setting_number(),
        )?;

        let find_endpoint = |direction, transfer_type| {
            interface_desc
                .endpoint_descriptors()
                .find(|ep| ep.direction() == direction && ep.transfer_type() == transfer_type)
                .map(|x| x.address())
                .ok_or(libusb::Error::NotFound)
        };

        Ok(PtpCamera {
            iface: interface_desc.interface_number(),
            ep_in: find_endpoint(libusb::Direction::In, libusb::TransferType::Bulk)?,
            ep_out: find_endpoint(libusb::Direction::Out, libusb::TransferType::Bulk)?,
            _ep_int: find_endpoint(libusb::Direction::In, libusb::TransferType::Interrupt)?,
            current_tid: 0,
            handle: handle,
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
        let timeout = timeout.unwrap_or(Duration::new(0, 0));

        let tid = self.current_tid;
        self.current_tid += 1;

        // Prepare payload of the request phase, containing the parameters
        let mut request_payload = Vec::with_capacity(params.len() * 4);
        for p in params {
            request_payload.write_u32::<LittleEndian>(*p).ok();
        }

        self.write_txn_phase(
            PtpContainerType::Command,
            code,
            tid,
            &request_payload,
            timeout,
        )?;

        if let Some(data) = data {
            self.write_txn_phase(PtpContainerType::Data, code, tid, data, timeout)?;
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
                PtpContainerType::Data => {
                    data_phase_payload = payload;
                }
                PtpContainerType::Response => {
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
        kind: PtpContainerType,
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
        let first_chunk_payload_bytes = min(payload.len(), CHUNK_SIZE - PTP_CONTAINER_INFO_SIZE);
        let mut buf = Vec::with_capacity(first_chunk_payload_bytes + PTP_CONTAINER_INFO_SIZE);
        buf.write_u32::<LittleEndian>((payload.len() + PTP_CONTAINER_INFO_SIZE) as u32)
            .ok();
        buf.write_u16::<LittleEndian>(kind as u16).ok();
        buf.write_u16::<LittleEndian>(code).ok();
        buf.write_u32::<LittleEndian>(tid).ok();
        buf.extend_from_slice(&payload[..first_chunk_payload_bytes]);
        self.handle.write_bulk(self.ep_out, &buf, timeout)?;

        // Write any subsequent chunks, straight from the source slice
        for chunk in payload[first_chunk_payload_bytes..].chunks(CHUNK_SIZE) {
            self.handle.write_bulk(self.ep_out, chunk, timeout)?;
        }

        Ok(())
    }

    // helper for command() above, retrieve container info and payload for the current phase
    fn read_txn_phase(&mut self, timeout: Duration) -> Result<(PtpContainerInfo, Vec<u8>), Error> {
        // buf is stack allocated and intended to be large enough to accomodate most
        // cmd/ctrl data (ie, not media) without allocating. payload handling below
        // deals with larger media responses. mark it as uninitalized to avoid paying
        // for zeroing out 8k of memory, since rust doesn't know what libusb does with this memory.
        let mut unintialized_buf: [u8; 8 * 1024];
        let buf = unsafe {
            unintialized_buf = ::std::mem::uninitialized();
            let n = self
                .handle
                .read_bulk(self.ep_in, &mut unintialized_buf[..], timeout)?;
            &unintialized_buf[..n]
        };

        let cinfo = PtpContainerInfo::parse(&buf[..])?;
        trace!("container {:?}", cinfo);

        // no payload? we're done
        if cinfo.payload_len == 0 {
            return Ok((cinfo, vec![]));
        }

        // allocate one extra to avoid a separate read for trailing short packet
        let mut payload = Vec::with_capacity(cinfo.payload_len + 1);
        payload.extend_from_slice(&buf[PTP_CONTAINER_INFO_SIZE..]);

        // response didn't fit into our original buf? read the rest
        // or if our original read were satisfied exactly, so there is still a ZLP to read
        if payload.len() < cinfo.payload_len || buf.len() == unintialized_buf.len() {
            unsafe {
                let p = payload.as_mut_ptr().offset(payload.len() as isize);
                let pslice = slice::from_raw_parts_mut(p, payload.capacity() - payload.len());
                let n = self.handle.read_bulk(self.ep_in, pslice, timeout)?;
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
    ) -> Result<PtpObjectInfo, Error> {
        let data = self.command(StandardCommandCode::GetObjectInfo, &[handle], None, timeout)?;
        Ok(PtpObjectInfo::decode(&data)?)
    }

    pub fn get_object(&mut self, handle: u32, timeout: Option<Duration>) -> Result<Vec<u8>, Error> {
        self.command(StandardCommandCode::GetObject, &[handle], None, timeout)
    }

    pub fn delete_object(&mut self, handle: u32, timeout: Option<Duration>) -> Result<(), Error> {
        self.command(StandardCommandCode::DeleteObject, &[handle], None, timeout)
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
        self.get_objecthandles(storage_id, 0xFFFFFFFF, filter, timeout)
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
    ) -> Result<PtpStorageInfo, Error> {
        let data = self.command(
            StandardCommandCode::GetStorageInfo,
            &[storage_id],
            None,
            timeout,
        )?;

        // Parse ObjectHandleArrray
        let mut cur = Cursor::new(data);
        let res = PtpStorageInfo::decode(&mut cur)?;
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
        self.get_numobjects(storage_id, 0xFFFFFFFF, filter, timeout)
    }

    pub fn get_numobjects_all(
        &mut self,
        storage_id: u32,
        filter: Option<u32>,
        timeout: Option<Duration>,
    ) -> Result<u32, Error> {
        self.get_numobjects(storage_id, 0x0, filter, timeout)
    }

    pub fn get_device_info(&mut self, timeout: Option<Duration>) -> Result<PtpDeviceInfo, Error> {
        let data = self.command(
            StandardCommandCode::GetDeviceInfo,
            &[0, 0, 0],
            None,
            timeout,
        )?;

        let device_info = PtpDeviceInfo::decode(&data)?;
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
        self.handle.release_interface(self.iface)?;
        Ok(())
    }
}
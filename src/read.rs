use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;
use super::Error;

pub trait PtpRead: ReadBytesExt {
    fn read_ptp_u8(&mut self) -> Result<u8, Error> {
        Ok(self.read_u8()?)
    }

    fn read_ptp_i8(&mut self) -> Result<i8, Error> {
        Ok(self.read_i8()?)
    }

    fn read_ptp_u16(&mut self) -> Result<u16, Error> {
        Ok(self.read_u16::<LittleEndian>()?)
    }

    fn read_ptp_i16(&mut self) -> Result<i16, Error> {
        Ok(self.read_i16::<LittleEndian>()?)
    }

    fn read_ptp_u32(&mut self) -> Result<u32, Error> {
        Ok(self.read_u32::<LittleEndian>()?)
    }

    fn read_ptp_i32(&mut self) -> Result<i32, Error> {
        Ok(self.read_i32::<LittleEndian>()?)
    }

    fn read_ptp_u64(&mut self) -> Result<u64, Error> {
        Ok(self.read_u64::<LittleEndian>()?)
    }

    fn read_ptp_i64(&mut self) -> Result<i64, Error> {
        Ok(self.read_i64::<LittleEndian>()?)
    }

    fn read_ptp_u128(&mut self) -> Result<u128, Error> {
        Ok(self.read_u128::<LittleEndian>()?)
    }

    fn read_ptp_i128(&mut self) -> Result<i128, Error> {
        Ok(self.read_i128::<LittleEndian>()?)
    }

    #[inline(always)]
    fn read_ptp_vec<T: Sized, U: Fn(&mut Self) -> Result<T, Error>>(
        &mut self,
        func: U,
    ) -> Result<Vec<T>, Error> {
        let len = self.read_u32::<LittleEndian>()? as usize;
        (0..len).map(|_| func(self)).collect()
    }

    fn read_ptp_u8_vec(&mut self) -> Result<Vec<u8>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_u8())
    }

    fn read_ptp_i8_vec(&mut self) -> Result<Vec<i8>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_i8())
    }

    fn read_ptp_u16_vec(&mut self) -> Result<Vec<u16>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_u16())
    }

    fn read_ptp_i16_vec(&mut self) -> Result<Vec<i16>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_i16())
    }

    fn read_ptp_u32_vec(&mut self) -> Result<Vec<u32>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_u32())
    }

    fn read_ptp_i32_vec(&mut self) -> Result<Vec<i32>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_i32())
    }

    fn read_ptp_u64_vec(&mut self) -> Result<Vec<u64>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_u64())
    }

    fn read_ptp_i64_vec(&mut self) -> Result<Vec<i64>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_i64())
    }

    fn read_ptp_u128_vec(&mut self) -> Result<Vec<u128>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_u128())
    }

    fn read_ptp_i128_vec(&mut self) -> Result<Vec<i128>, Error> {
        self.read_ptp_vec(|cur| cur.read_ptp_i128())
    }

    fn read_ptp_str(&mut self) -> Result<String, Error> {
        let len = self.read_u8()?;
        if len > 0 {
            // len includes the trailing null u16
            let data: Vec<u16> = (0..(len - 1))
                .map(|_| self.read_u16::<LittleEndian>())
                .collect::<std::result::Result<_, _>>()?;
            self.read_u16::<LittleEndian>()?;
            String::from_utf16(&data)
                .map_err(|_| Error::Malformed(format!("Invalid UTF16 data: {:?}", data)))
        } else {
            Ok("".into())
        }
    }

    fn expect_end(&mut self) -> Result<(), Error>;
}

impl<T: AsRef<[u8]>> PtpRead for Cursor<T> {
    fn expect_end(&mut self) -> Result<(), Error> {
        let len = self.get_ref().as_ref().len();
        if len as u64 != self.position() {
            Err(Error::Malformed(format!(
                "Response {} bytes, expected {} bytes",
                len,
                self.position()
            )))
        } else {
            Ok(())
        }
    }
}
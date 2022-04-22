pub mod rmcp;
pub mod asf;
pub mod ipmi;
mod util;
pub mod cmd;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    OutBufferTooSmall,
    InvalidConfiguration,
    PayloadTooLarge,
    PayloadTooSmall,
    InvalidRmcpVersionNumber(u8),
    InvalidRmcpReservedByte(u8),
    UnsupportedProtocol,
    ExpectedSizeMismatch,
    InvalidChecksum,
    UndefinedAuthType(u8)
}

pub fn summon_from_bytes<'a, T: BytesDeserializable<'a>>(slice: &'a [u8], strict: bool) -> Result<T, Error>
{
    T::from_bytes(slice, strict)
}

pub trait BytesSerializationSized {
    fn size(&self) -> usize;
}

pub trait BytesSerializable: core::marker::Sized + BytesSerializationSized {
    fn write_to_slice(&self, _: &mut [u8], strict: bool) -> Result<(), Error>;
}

pub trait BytesDeserializable<'a>: core::marker::Sized + BytesSerializationSized {
    fn from_bytes(slice: &'a [u8], strict: bool) -> Result<Self, Error>;
}

impl<const N: usize> BytesSerializationSized for [u8; N] {
    fn size(&self) -> usize { 
        N
    }
}

impl<const N: usize> BytesSerializable for [u8; N] {

    fn write_to_slice(&self, slice: &mut[u8], _strict: bool) -> Result<(), Error>
    {
        if slice.len() < self.size() {
            return Err(Error::OutBufferTooSmall);
        }

        slice[0..N].copy_from_slice(self);
        Ok(())
    }
}
impl<const N: usize> BytesDeserializable<'_> for [u8; N] {
    fn from_bytes(slice: &'_ [u8], _strict: bool) -> Result<[u8;N], Error> {
        if slice.len() < N { return Err(Error::PayloadTooSmall) }

        let mut buf = [0u8;N];

        buf.copy_from_slice(&slice[..N]);

        Ok(buf)
    }
}

impl BytesSerializationSized for u32 {
    fn size(&self) -> usize { 4 }
}

impl BytesDeserializable<'_> for u8 {
    fn from_bytes(slice: &'_ [u8], _strict: bool) -> Result<u8, Error> {
        if slice.len() < 1 { return Err(Error::PayloadTooSmall) }
        Ok(slice[0])
    }
}

impl BytesSerializationSized for u8 {
    fn size(&self) -> usize { 1 }
}

impl BytesSerializable for u8 {
    fn write_to_slice(&self, slice: &mut[u8], _strict: bool) -> Result<(), Error>
    {
        if slice.len() < self.size() {
            return Err(Error::OutBufferTooSmall)
        }
        slice[0] = *self;
        Ok(())
    }
}

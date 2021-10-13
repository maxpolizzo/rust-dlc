#![no_std]

//! Bincode is a crate for encoding and decoding using a tiny binary
//! serialization strategy.  Using it, you can easily go from having
//! an object in memory, quickly serialize it to bytes, and then
//! deserialize it back just as fast!

#![doc(html_root_url = "https://docs.rs/bincode/2.0.0-dev")]
#![crate_name = "bincode"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(any(feature = "std", test))]
extern crate std;

mod features;

pub use features::*;

pub mod de;
pub mod enc;
pub mod error;

pub fn encode_into_slice<E: enc::Encodeable>(
    val: E,
    dst: &mut [u8],
) -> Result<usize, error::EncodeError> {
    let writer = enc::write::SliceWriter::new(dst);
    let mut encoder = enc::Encoder::<_>::new(writer);
    val.encode(&mut encoder)?;
    Ok(encoder.into_writer().bytes_written())
}

pub fn decode<'__de, D: de::BorrowDecodable<'__de>>(
    src: &'__de mut [u8],
) -> Result<D, error::DecodeError> {
    let reader = de::read::SliceReader::new(src);
    let mut decoder = de::Decoder::<_>::new(reader);
    D::borrow_decode(&mut decoder)
}

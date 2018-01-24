use bytes::{self, BufMut};
use flate2;
use std::io::{self, Read, Seek};
use super::{Result, ResultExt};

/// Create a buffer from the file
pub fn create_buffer<R: Read>(rdr: R, buflen: usize) -> Result<bytes::Bytes> {
    let mut fp = rdr.take(buflen as u64);

    let buffer = bytes::BytesMut::with_capacity(buflen);
    let mut buffer = buffer.writer();

    io::copy(&mut fp, &mut buffer)
        .chain_err(|| "failed to fill buffer with contents of input file")?;
    Ok(buffer.into_inner().freeze())
}

/// Read out the possible header bytes from the file
pub fn header_bytes<R: Read + Seek>(fp: R, buflen: usize) -> Result<bytes::Bytes> {
    // Assume that it is GZip-encoded
    let fp = flate2::read::GzDecoder::new(fp);
    // If it was in fact GZip-encoded
    if fp.header().is_some() {
        // Create a buffer out of the uncompressed bytes
        create_buffer(fp, buflen)
    } else {
        // Otherwise, we need to get back the original file
        let mut fp = fp.into_inner();
        // Rewind to the beginning of it
        fp.seek(io::SeekFrom::Start(0))
            .chain_err(|| "failed to seek back to beginning of file")?;
        // And create a buffer from the uncompressed bytes
        create_buffer(fp, buflen)
    }
}

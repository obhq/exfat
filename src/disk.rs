use std::error::Error;
use std::fmt::Display;

/// Encapsulate a disk partition.
pub trait DiskPartition {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<u64, Box<dyn Error + Send + Sync>>;

    fn read_exact(
        &self,
        mut offset: u64,
        mut buf: &mut [u8],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        while !buf.is_empty() {
            let n = self.read(offset, buf)?;

            if n == 0 {
                return Err(Box::new(UnexpectedEop));
            }

            offset += n;
            buf = &mut buf[n.try_into().unwrap()..];
        }

        Ok(())
    }
}

/// An error for unexpected end of partition.
#[derive(Debug)]
struct UnexpectedEop;

impl Display for UnexpectedEop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("end of partition has been reached")
    }
}

impl Error for UnexpectedEop {}

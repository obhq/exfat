use core::fmt::Display;

/// Encapsulate a disk partition.
pub trait DiskPartition {
    #[cfg(not(feature = "std"))]
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<u64, Box<dyn Display + Send + Sync>>;

    #[cfg(feature = "std")]
    fn read(
        &self,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    #[cfg(not(feature = "std"))]
    fn read_exact(
        &self,
        mut offset: u64,
        mut buf: &mut [u8],
    ) -> Result<(), Box<dyn Display + Send + Sync>> {
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

    #[cfg(feature = "std")]
    fn read_exact(
        &self,
        mut offset: u64,
        mut buf: &mut [u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("end of partition has been reached")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UnexpectedEop {}

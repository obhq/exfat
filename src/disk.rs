use alloc::sync::Arc;
use core::error::Error;
use core::ops::Deref;

/// Encapsulate a disk partition.
pub trait DiskPartition {
    type Err: PartitionError + 'static;

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Self::Err>;

    fn read_exact(&self, mut offset: u64, mut buf: &mut [u8]) -> Result<(), Self::Err> {
        while !buf.is_empty() {
            let n = self.read(offset, buf)?;

            if n == 0 {
                return Err(PartitionError::unexpected_eop());
            }

            offset = n
                .try_into()
                .ok()
                .and_then(|n| offset.checked_add(n))
                .unwrap();

            buf = &mut buf[n..];
        }

        Ok(())
    }
}

/// Represents an error when an operation on [`DiskPartition`] fails.
pub trait PartitionError: Error + Send + Sync {
    fn unexpected_eop() -> Self;
}

impl<T: DiskPartition> DiskPartition for &T {
    type Err = T::Err;

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Self::Err> {
        (*self).read(offset, buf)
    }
}

impl<T: DiskPartition> DiskPartition for Arc<T> {
    type Err = T::Err;

    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Self::Err> {
        self.deref().read(offset, buf)
    }
}

#[cfg(feature = "std")]
impl DiskPartition for std::fs::File {
    type Err = std::io::Error;

    #[cfg(unix)]
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Self::Err> {
        std::os::unix::fs::FileExt::read_at(self, buf, offset)
    }

    #[cfg(windows)]
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, Self::Err> {
        std::os::windows::fs::FileExt::seek_read(self, buf, offset)
    }
}

#[cfg(feature = "std")]
impl PartitionError for std::io::Error {
    fn unexpected_eop() -> Self {
        std::io::Error::from(std::io::ErrorKind::UnexpectedEof)
    }
}

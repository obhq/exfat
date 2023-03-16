use crate::disk::DiskPartition;
use std::error::Error;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;
use thiserror::Error;

/// An implementation of [`DiskPartition`] backed by an exFAT image.
pub struct Image<F: Read + Seek> {
    file: Mutex<(F, u64)>,
}

impl<F: Read + Seek> Image<F> {
    pub fn open(mut file: F) -> Result<Self, OpenError> {
        let offset = match file.stream_position() {
            Ok(v) => v,
            Err(e) => return Err(OpenError::GetStreamPositionFailed(e)),
        };

        Ok(Self {
            file: Mutex::new((file, offset)),
        })
    }
}

impl<F: Read + Seek> DiskPartition for Image<F> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let mut file = self
            .file
            .lock()
            .expect("the mutex that protect the inner file is poisoned");

        // Seek the file.
        if offset != file.1 {
            match file.0.seek(SeekFrom::Start(offset)) {
                Ok(v) => {
                    // The specified offset is out of range.
                    if v != offset {
                        return Ok(0);
                    }
                }
                Err(e) => return Err(ReadError::SeekFailed(e).into()),
            }

            file.1 = offset;
        }

        // Read the file.
        let read = match file.0.read(buf) {
            Ok(v) => v.try_into().unwrap(),
            Err(e) => return Err(ReadError::ReadFailed(e).into()),
        };

        file.1 += read;

        Ok(read)
    }
}

/// Represents an error for [`Image::open()`].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot get the current seek position of the file")]
    GetStreamPositionFailed(#[source] std::io::Error),
}

/// Represents an error for [`Image::read()`].
#[derive(Debug, Error)]
enum ReadError {
    #[error("cannot seek the image to the target offset")]
    SeekFailed(#[source] std::io::Error),

    #[error("cannot read the image")]
    ReadFailed(#[source] std::io::Error),
}

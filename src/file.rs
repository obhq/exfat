use crate::cluster::ClustersReader;
use crate::disk::DiskPartition;
use crate::entries::StreamEntry;
use crate::fat::Fat;
use crate::param::Params;
use crate::timestamp::Timestamps;
use alloc::sync::Arc;
use core::cmp::min;
use thiserror::Error;

/// Represents a file in an exFAT filesystem.
pub struct File<D> {
    name: String,
    len: u64,
    reader: Option<ClustersReader<Arc<D>, Arc<Params>>>,
    timestamps: Timestamps,
}

impl<D> File<D> {
    pub(crate) fn new(
        disk: &Arc<D>,
        params: &Arc<Params>,
        fat: &Fat,
        name: String,
        stream: StreamEntry,
        timestamps: Timestamps,
    ) -> Result<Self, NewError> {
        // Create a cluster reader.
        let alloc = stream.allocation();
        let first_cluster = alloc.first_cluster();
        let len = stream.valid_data_length();
        let reader = if first_cluster == 0 {
            None
        } else {
            match ClustersReader::new(
                disk.clone(),
                params.clone(),
                fat,
                first_cluster,
                Some(len),
                Some(stream.no_fat_chain()),
            ) {
                Ok(v) => Some(v),
                Err(e) => return Err(NewError::CreateClustersReaderFailed(first_cluster, len, e)),
            }
        };

        Ok(Self {
            name,
            len,
            reader,
            timestamps,
        })
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn timestamps(&self) -> &Timestamps {
        &self.timestamps
    }
}

#[cfg(feature = "std")]
impl<D> std::io::Seek for File<D> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind, SeekFrom};

        // Check if empty file.
        let r = match &mut self.reader {
            Some(v) => v,
            None => return std::io::empty().seek(pos),
        };

        // Get absolute offset.
        let o = match pos {
            SeekFrom::Start(v) => min(v, r.data_length()),
            SeekFrom::End(v) => {
                if v >= 0 {
                    r.data_length()
                } else if let Some(v) = r.data_length().checked_sub(v.unsigned_abs()) {
                    v
                } else {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
            }
            SeekFrom::Current(v) => v.try_into().map_or_else(
                |_| {
                    r.stream_position()
                        .checked_sub(v.unsigned_abs())
                        .ok_or_else(|| Error::from(ErrorKind::InvalidInput))
                },
                |v| Ok(min(r.stream_position().saturating_add(v), r.data_length())),
            )?,
        };

        assert!(r.seek(o));

        Ok(o)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        let r = match &mut self.reader {
            Some(v) => v,
            None => return Ok(()),
        };

        r.rewind();

        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        let r = match &mut self.reader {
            Some(v) => v,
            None => return Ok(0),
        };

        Ok(r.stream_position())
    }
}

#[cfg(feature = "std")]
impl<D: DiskPartition> std::io::Read for File<D> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.reader {
            Some(v) => v.read(buf),
            None => Ok(0),
        }
    }
}

/// Represents an error for [`File::new()`].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("cannot create a clusters reader for allocation {0}:{1}")]
    CreateClustersReaderFailed(usize, u64, #[source] crate::cluster::NewError),
}

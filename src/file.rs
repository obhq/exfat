use crate::cluster::ClustersReader;
use crate::disk::DiskPartition;
use crate::entries::StreamEntry;
use crate::timestamp::Timestamps;
use crate::ExFat;
use core::cmp::min;
use std::io::{empty, Empty};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use thiserror::Error;

/// Represents a file in the exFAT.
pub struct File<P: DiskPartition> {
    name: String,
    len: u64,
    reader: Reader<P>, // FIXME: Use trait object once https://github.com/rust-lang/rfcs/issues/2035 is resolved.
    timestamps: Timestamps,
}

impl<P: DiskPartition> File<P> {
    pub(crate) fn new(
        exfat: Arc<ExFat<P>>,
        name: String,
        stream: StreamEntry,
        timestamps: Timestamps,
    ) -> Result<Self, NewError> {
        // Create a cluster reader.
        let alloc = stream.allocation();
        let first_cluster = alloc.first_cluster();
        let len = stream.valid_data_length();
        let reader = if first_cluster == 0 {
            Reader::Empty(empty())
        } else {
            let reader = match ClustersReader::new(
                exfat,
                first_cluster,
                Some(len),
                Some(stream.no_fat_chain()),
            ) {
                Ok(v) => v,
                Err(e) => return Err(NewError::CreateClustersReaderFailed(first_cluster, len, e)),
            };

            Reader::Cluster(reader)
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

impl<P: DiskPartition> Seek for File<P> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind};

        // Check if empty file.
        let r = match &mut self.reader {
            Reader::Cluster(r) => r,
            Reader::Empty(r) => return r.seek(pos),
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
            Reader::Cluster(r) => r,
            Reader::Empty(r) => return r.rewind(),
        };

        Ok(r.rewind())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        let r = match &mut self.reader {
            Reader::Cluster(r) => r,
            Reader::Empty(r) => return r.stream_position(),
        };

        Ok(r.stream_position())
    }
}

impl<P: DiskPartition> Read for File<P> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read(buf),
            Reader::Empty(r) => r.read(buf),
        }
    }
}

/// Encapsulate the either [`ClustersReader`] or [`Empty`].
enum Reader<P: DiskPartition> {
    Cluster(ClustersReader<P>),
    Empty(Empty),
}

/// Represents an error for [`File::new()`].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("cannot create a clusters reader for allocation {0}:{1}")]
    CreateClustersReaderFailed(usize, u64, #[source] crate::cluster::NewError),
}

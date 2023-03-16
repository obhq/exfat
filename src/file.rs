use crate::cluster::ClustersReader;
use crate::disk::DiskPartition;
use crate::entries::StreamEntry;
use crate::ExFat;
use std::io::{empty, Empty};
use std::io::{IoSliceMut, Read, Seek, SeekFrom};
use std::sync::Arc;
use thiserror::Error;

/// Represents a file in the exFAT.
pub struct File<P: DiskPartition> {
    name: String,
    len: u64,
    reader: Reader<P>, // FIXME: Use trait object once https://github.com/rust-lang/rfcs/issues/2035 is resolved.
}

impl<P: DiskPartition> File<P> {
    pub(crate) fn new(
        exfat: Arc<ExFat<P>>,
        name: String,
        stream: StreamEntry,
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

        Ok(Self { name, len, reader })
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
}

impl<P: DiskPartition> Seek for File<P> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match &mut self.reader {
            Reader::Cluster(r) => r.seek(pos),
            Reader::Empty(r) => r.seek(pos),
        }
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        match &mut self.reader {
            Reader::Cluster(r) => r.rewind(),
            Reader::Empty(r) => r.rewind(),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        match &mut self.reader {
            Reader::Cluster(r) => r.stream_position(),
            Reader::Empty(r) => r.stream_position(),
        }
    }
}

impl<P: DiskPartition> Read for File<P> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read(buf),
            Reader::Empty(r) => r.read(buf),
        }
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read_vectored(bufs),
            Reader::Empty(r) => r.read_vectored(bufs),
        }
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read_to_end(buf),
            Reader::Empty(r) => r.read_to_end(buf),
        }
    }

    fn read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read_to_string(buf),
            Reader::Empty(r) => r.read_to_string(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        match &mut self.reader {
            Reader::Cluster(r) => r.read_exact(buf),
            Reader::Empty(r) => r.read_exact(buf),
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

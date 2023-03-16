use crate::disk::DiskPartition;
use crate::ExFat;
use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use thiserror::Error;

/// A cluster reader to read all data in a cluster chain.
pub(crate) struct ClustersReader<P: DiskPartition> {
    exfat: Arc<ExFat<P>>,
    chain: Vec<usize>,
    data_length: u64,
    offset: u64,
}

impl<P: DiskPartition> ClustersReader<P> {
    pub fn new(
        exfat: Arc<ExFat<P>>,
        first_cluster: usize,
        data_length: Option<u64>,
        no_fat_chain: Option<bool>,
    ) -> Result<Self, NewError> {
        if first_cluster < 2 {
            return Err(NewError::InvalidFirstCluster);
        }

        // Get cluster chain.
        let params = &exfat.params;
        let fat = &exfat.fat;
        let cluster_size = params.cluster_size();
        let (chain, data_length) = if no_fat_chain.unwrap_or(false) {
            // If the NoFatChain bit is 1 then DataLength must not be zero.
            let data_length = match data_length {
                Some(v) if v > 0 => v,
                _ => return Err(NewError::InvalidDataLength),
            };

            // FIXME: Use div_ceil once https://github.com/rust-lang/rust/issues/88581 stabilized.
            let count = (data_length + cluster_size - 1) / cluster_size;
            let chain: Vec<usize> = (first_cluster..(first_cluster + count as usize)).collect();

            (chain, data_length)
        } else {
            let chain: Vec<usize> = fat.get_cluster_chain(first_cluster).collect();

            if chain.is_empty() {
                return Err(NewError::InvalidFirstCluster);
            }

            let data_length = match data_length {
                Some(v) => {
                    if v > cluster_size * chain.len() as u64 {
                        return Err(NewError::InvalidDataLength);
                    } else {
                        v
                    }
                }
                None => params.bytes_per_sector * (params.sectors_per_cluster * chain.len() as u64),
            };

            (chain, data_length)
        };

        Ok(Self {
            exfat,
            chain,
            data_length,
            offset: 0,
        })
    }

    pub fn cluster(&self) -> usize {
        self.chain[(self.offset / self.exfat.params.cluster_size()) as usize]
    }
}

impl<P: DiskPartition> Seek for ClustersReader<P> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        use std::io::{Error, ErrorKind};

        self.offset = match pos {
            SeekFrom::Start(v) => min(v, self.data_length),
            SeekFrom::End(v) => {
                if v >= 0 {
                    self.data_length
                } else if let Some(v) = self.data_length.checked_sub(v.unsigned_abs()) {
                    v
                } else {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
            }
            SeekFrom::Current(v) => {
                if v >= 0 {
                    min(self.offset + (v as u64), self.data_length)
                } else if let Some(v) = self.offset.checked_sub(v.unsigned_abs()) {
                    v
                } else {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
            }
        };

        Ok(self.offset)
    }

    fn rewind(&mut self) -> std::io::Result<()> {
        self.offset = 0;
        Ok(())
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.offset)
    }
}

impl<P: DiskPartition> Read for ClustersReader<P> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::io::{Error, ErrorKind};

        // Check if the actual read is required.
        if buf.is_empty() || self.offset == self.data_length {
            return Ok(0);
        }

        // Get remaining data in the current cluster.
        let cluster_size = self.exfat.params.cluster_size();
        let cluster_remaining = cluster_size - self.offset % cluster_size;
        let remaining = min(cluster_remaining, self.data_length - self.offset);

        // Get the offset in the partition.
        let cluster = self.chain[(self.offset / cluster_size) as usize];
        let offset = match self.exfat.params.cluster_offset(cluster) {
            Some(v) => v + self.offset % cluster_size,
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("cluster #{cluster} is not available"),
                ));
            }
        };

        // Read image.
        let amount = min(buf.len(), remaining as usize);

        if let Err(e) = self.exfat.partition.read_exact(offset, &mut buf[..amount]) {
            return Err(Error::new(ErrorKind::Other, e));
        }

        self.offset += amount as u64;

        Ok(amount)
    }
}

/// Represents an error for [`new()`][ClustersReader::new()].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("first cluster is not valid")]
    InvalidFirstCluster,

    #[error("data length is not valid")]
    InvalidDataLength,
}

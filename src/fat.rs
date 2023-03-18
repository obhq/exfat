use crate::disk::DiskPartition;
use crate::param::Params;
use byteorder::{ByteOrder, LE};
use std::error::Error;
use thiserror::Error;

pub(super) struct Fat {
    entries: Vec<u32>,
}

impl Fat {
    pub fn load<P: DiskPartition>(
        params: &Params,
        partition: &P,
        index: usize,
    ) -> Result<Self, LoadError> {
        // Get FAT region offset.
        let sector = match params.fat_length.checked_mul(index as u64) {
            Some(v) => match params.fat_offset.checked_add(v) {
                Some(v) => v,
                None => return Err(LoadError::InvalidFatOffset),
            },
            None => return Err(LoadError::InvalidFatLength),
        };

        let offset = match sector.checked_mul(params.bytes_per_sector) {
            Some(v) => v,
            None => return Err(LoadError::InvalidFatOffset),
        };

        // Load entries.
        let count = params.cluster_count + 2;
        let mut data = vec![0u8; count * 4];

        if let Err(e) = partition.read_exact(offset, &mut data) {
            return Err(LoadError::ReadFailed(offset, e));
        }

        // Convert each entry from little endian to native endian.
        let mut entries = vec![0u32; count];

        LE::read_u32_into(&data, &mut entries);

        Ok(Self { entries })
    }

    pub fn get_cluster_chain(&self, first: usize) -> ClusterChain<'_> {
        ClusterChain {
            entries: &self.entries,
            next: first,
        }
    }
}

pub(crate) struct ClusterChain<'fat> {
    entries: &'fat [u32],
    next: usize,
}

impl<'fat> Iterator for ClusterChain<'fat> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // Check next entry.
        let entries = self.entries;
        let next = self.next;

        if next < 2 || next >= entries.len() || entries[next] == 0xfffffff7 {
            return None;
        }

        // Move to next entry.
        self.next = entries[next] as usize;

        Some(next)
    }
}

/// Represents an error for [`Fat::load()`].
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("invalid FatLength")]
    InvalidFatLength,

    #[error("invalid FatOffset")]
    InvalidFatOffset,

    #[error("cannot read data at {0:#018x}")]
    ReadFailed(u64, #[source] Box<dyn Error + Send + Sync>),
}

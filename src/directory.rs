use crate::cluster::ClustersReader;
use crate::disk::DiskPartition;
use crate::entries::{ClusterAllocation, EntriesReader, EntryType, FileEntry, StreamEntry};
use crate::file::File;
use crate::ExFat;
use std::sync::Arc;
use thiserror::Error;

/// Represents a directory in the exFAT.
pub struct Directory<P: DiskPartition> {
    exfat: Arc<ExFat<P>>,
    name: String,
    stream: StreamEntry,
}

impl<P: DiskPartition> Directory<P> {
    pub(crate) fn new(exfat: Arc<ExFat<P>>, name: String, stream: StreamEntry) -> Self {
        Self {
            exfat,
            name,
            stream,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn open(&self) -> Result<Vec<Item<P>>, OpenError> {
        // Create an entries reader.
        let alloc = self.stream.allocation();
        let mut reader = match ClustersReader::new(
            self.exfat.clone(),
            alloc.first_cluster(),
            Some(alloc.data_length()),
            Some(self.stream.no_fat_chain()),
        ) {
            Ok(v) => EntriesReader::new(v),
            Err(e) => return Err(OpenError::CreateClustersReaderFailed(alloc.clone(), e)),
        };

        // Read file entries.
        let mut items: Vec<Item<P>> = Vec::new();

        loop {
            // Read primary entry.
            let entry = match reader.read() {
                Ok(v) => v,
                Err(e) => return Err(OpenError::ReadEntryFailed(e)),
            };

            // Check entry type.
            let ty = entry.ty();

            if !ty.is_regular() {
                break;
            } else if ty.type_category() != EntryType::PRIMARY {
                return Err(OpenError::NotPrimaryEntry(entry.index(), entry.cluster()));
            } else if ty.type_importance() != EntryType::CRITICAL || ty.type_code() != 5 {
                return Err(OpenError::NotFileEntry(entry.index(), entry.cluster()));
            }

            // Parse file entry.
            let file = match FileEntry::load(&entry, &mut reader) {
                Ok(v) => v,
                Err(e) => return Err(OpenError::LoadFileEntryFailed(e)),
            };

            // Construct item.
            let name = file.name;
            let attrs = file.attributes;
            let stream = file.stream;

            items.push(if attrs.is_directory() {
                Item::Directory(Directory::new(self.exfat.clone(), name, stream))
            } else {
                match File::new(self.exfat.clone(), name, stream) {
                    Ok(v) => Item::File(v),
                    Err(e) => {
                        return Err(OpenError::CreateFileObjectFailed(
                            entry.index(),
                            entry.cluster(),
                            e,
                        ));
                    }
                }
            });
        }

        Ok(items)
    }
}

/// Represents an item in the directory.
pub enum Item<P: DiskPartition> {
    Directory(Directory<P>),
    File(File<P>),
}

/// Represents an error for [`open()`][Directory::open].
#[derive(Debug, Error)]
pub enum OpenError {
    #[error("cannot create a clusters reader for allocation {0}")]
    CreateClustersReaderFailed(ClusterAllocation, #[source] crate::cluster::NewError),

    #[error("cannot read an entry")]
    ReadEntryFailed(#[source] crate::entries::ReaderError),

    #[error("entry #{0} on cluster #{1} is not a primary entry")]
    NotPrimaryEntry(usize, usize),

    #[error("entry #{0} on cluster #{1} is not a file entry")]
    NotFileEntry(usize, usize),

    #[error("cannot load file entry")]
    LoadFileEntryFailed(#[source] crate::entries::FileEntryError),

    #[error("cannot create a file object for directory entry #{0} on cluster #{1}")]
    CreateFileObjectFailed(usize, usize, #[source] crate::file::NewError),
}

use crate::cluster::ClustersReader;
use crate::disk::DiskPartition;
use crate::entries::{ClusterAllocation, EntriesReader, EntryType, FileEntry, StreamEntry};
use crate::fat::Fat;
use crate::file::File;
use crate::param::Params;
use crate::timestamp::Timestamps;
use alloc::sync::Arc;
use thiserror::Error;

/// Represents a directory in an exFAT filesystem.
pub struct Directory<D> {
    disk: Arc<D>,
    params: Arc<Params>,
    fat: Arc<Fat>,
    name: String,
    stream: StreamEntry,
    timestamps: Timestamps,
}

impl<D> Directory<D> {
    pub(crate) fn new(
        disk: Arc<D>,
        params: Arc<Params>,
        fat: Arc<Fat>,
        name: String,
        stream: StreamEntry,
        timestamps: Timestamps,
    ) -> Self {
        Self {
            disk,
            params,
            fat,
            name,
            stream,
            timestamps,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn timestamps(&self) -> &Timestamps {
        &self.timestamps
    }
}

impl<D: DiskPartition> Directory<D> {
    pub fn open(&self) -> Result<Vec<Item<D>>, DirectoryError> {
        // Create an entries reader.
        let alloc = self.stream.allocation();
        let mut reader = match ClustersReader::new(
            &self.disk,
            &self.params,
            &self.fat,
            alloc.first_cluster(),
            Some(alloc.data_length()),
            Some(self.stream.no_fat_chain()),
        ) {
            Ok(v) => EntriesReader::new(v),
            Err(e) => return Err(DirectoryError::CreateClustersReaderFailed(alloc.clone(), e)),
        };

        // Read file entries.
        let mut items: Vec<Item<D>> = Vec::new();

        loop {
            // Read primary entry.
            let entry = match reader.read() {
                Ok(v) => v,
                Err(e) => return Err(DirectoryError::ReadEntryFailed(e)),
            };

            // Check entry type.
            let ty = entry.ty();

            if !ty.is_regular() {
                break;
            } else if ty.type_category() != EntryType::PRIMARY {
                return Err(DirectoryError::NotPrimaryEntry(
                    entry.index(),
                    entry.cluster(),
                ));
            } else if ty.type_importance() != EntryType::CRITICAL || ty.type_code() != 5 {
                return Err(DirectoryError::NotFileEntry(entry.index(), entry.cluster()));
            }

            // Parse file entry.
            let file = match FileEntry::load(&entry, &mut reader) {
                Ok(v) => v,
                Err(e) => return Err(DirectoryError::LoadFileEntryFailed(e)),
            };

            // Construct item.
            let name = file.name;
            let attrs = file.attributes;
            let stream = file.stream;
            let timestamps = file.timestamps;

            items.push(if attrs.is_directory() {
                Item::Directory(Self {
                    disk: self.disk.clone(),
                    params: self.params.clone(),
                    fat: self.fat.clone(),
                    name,
                    stream,
                    timestamps,
                })
            } else {
                match File::new(
                    &self.disk,
                    &self.params,
                    &self.fat,
                    name,
                    stream,
                    timestamps,
                ) {
                    Ok(v) => Item::File(v),
                    Err(e) => {
                        return Err(DirectoryError::CreateFileObjectFailed(
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
pub enum Item<D> {
    Directory(Directory<D>),
    File(File<D>),
}

/// Represents an error when [`Directory::open()`] fails.
#[derive(Debug, Error)]
pub enum DirectoryError {
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

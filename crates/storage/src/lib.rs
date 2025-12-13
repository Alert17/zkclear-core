mod storage_trait;
mod in_memory;

#[cfg(feature = "rocksdb")]
mod rocksdb_impl;

pub use storage_trait::{Storage, StorageError};
pub use in_memory::InMemoryStorage;

#[cfg(feature = "rocksdb")]
pub use rocksdb_impl::RocksDBStorage;


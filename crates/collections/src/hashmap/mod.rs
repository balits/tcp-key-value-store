use thiserror::Error;

mod hash_table;
mod dict;

#[derive(Error, Debug)]
pub enum PartialRehashError {
    #[error("Unexpected size of {table_name} name, got: {size_got}, expected: {size_expected}")]
    InvalidTableSize {
        table_name: String,
        size_got: usize,
        size_expected: usize,
    },
    #[error("Rehash index invalid got: {rehash_idx}, valid range: -1..{table_size}")]
    InvalidRehashIndex {
        rehash_idx: isize,
        table_size: usize,
    },
    #[error("Couldnt rehash table while load load factor is not high enough")]
    IncorrectLoadFactor {
        rehash_idx: isize,
        load_factor: f32,
    }
}

pub use dict::Dict;
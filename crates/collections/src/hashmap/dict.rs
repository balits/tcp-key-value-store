use std::fmt::format;
use std::primitive;

use crate::Node;
use crate::{hashmap::PartialRehashError, node};

use super::hash_table::DEFAULT_BUCKET_SIZE;
use super::hash_table::HashTable;

#[derive(Debug)]
pub struct Dict {
    primary: Box<HashTable>,
    secondary: Box<HashTable>,
    rehash_idx: isize,
}

impl Default for Dict {
    fn default() -> Self {
        let primary = Box::new(HashTable::default());
        let secondary = Box::new(HashTable::new_with_buckets(2 * primary.bucket_count()));
        Self {
            primary,
            secondary,
            rehash_idx: -1,
        }
    }
}

impl Dict {
    pub fn insert(&mut self, key: &str, value: &str) -> Option<String> {
        if self.primary.load_factor_f32() >= 1f32 || self.rehash_idx != -1 {
            self.try_partial_rehash().unwrap();
        }

        if self.rehash_idx == -1 {
            self.primary.insert_without_resize(node!(key, value))
        } else {
            self.secondary.insert_without_resize(node!(key, value))
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&Node> {
        if self.primary.load_factor_f32() >= 1f32 || self.rehash_idx != -1 {
            self.try_partial_rehash().unwrap();
        }

        self.primary
            .iter()
            .find(|node| node.key == key)
            .or_else(|| self.secondary.iter().find(|node| node.key == key))
    }

    pub fn delete(&mut self) -> Option<String> {
        if self.primary.load_factor_f32() >= 1f32 || self.rehash_idx != -1 {
            self.try_partial_rehash().unwrap();
        }

        todo!()
    }

    // [private]

    /// move all the hashes from the active tables `rehash_idx`-th bucket
    /// into the other table
    fn try_partial_rehash(&mut self) -> Result<(), PartialRehashError> {
        match self.rehash_idx {
            -1 => {
                // we can only rehash if the load factor has exceeded a certain threshhold
                if self.primary.load_factor_f32() < 1f32 {
                    return Err(PartialRehashError::IncorrectLoadFactor {
                        rehash_idx: self.rehash_idx,
                        load_factor: self.primary.load_factor_f32(),
                    });
                }

                self.rehash_idx = 0;
            }
            // safe cast since bucket_count() cannot return more than isize::MAX due to Vec's allocations laws
            i if i < -1 || (i as usize) > self.primary.bucket_count() => {
                return Err(PartialRehashError::InvalidRehashIndex {
                    rehash_idx: self.rehash_idx,
                    table_size: self.primary.bucket_count(),
                });
            }
            i if (i as usize) == self.primary.bucket_count() => {
                // finished move all the buckets
                println!("finished move all the buckets");
                assert_eq!(
                    self.primary.used(),
                    0,
                    "Moved all buckets from primary, to secondary, but primary still has keys"
                );

                // double the secondary (which will now be the primary) table size
                let new_secondary = Box::new(HashTable::new_with_buckets(
                    2 * self.secondary.bucket_count(),
                ));
                self.primary = std::mem::replace(&mut self.secondary, new_secondary);

                self.rehash_idx = -1;
                return Ok(());
            }
            _ => {
                // All good here
            }
        }

        // primary bucket was not allocated properly
        if self.primary.bucket_count() <= self.rehash_idx as usize {
            return Err(PartialRehashError::InvalidTableSize {
                table_name: "primary".to_string(),
                size_got: self.primary.bucket_count(),
                size_expected: DEFAULT_BUCKET_SIZE,
            });
        }

        // secondary bucket was not allocated properly
        if self.secondary.bucket_count() <= self.rehash_idx as usize {
            return Err(PartialRehashError::InvalidTableSize {
                table_name: "secondary".to_string(),
                size_got: self.secondary.bucket_count(),
                size_expected: self.primary.bucket_count() * 2,
            });
        }

        // move the whole bucket from primary to secondary,
        // replacing it with LinkedList::Default(), e.g. an empty list
        //
        // and update the secondary buckets size  accordingly
        let mut bucket_to_move =
            std::mem::take(&mut self.primary.buckets[self.rehash_idx as usize]);
        let target_bucket = self
            .secondary
            .buckets
            .get_mut(self.rehash_idx as usize)
            .unwrap();

        let moved_items = bucket_to_move.len();
        if target_bucket.is_empty() {
            let _ = std::mem::replace(target_bucket, bucket_to_move);
        } else {
            target_bucket.append(&mut bucket_to_move);
        }

        self.secondary.items += moved_items;
        self.primary.items -= moved_items;

        // done, move to next bucket to rehash
        self.rehash_idx += 1;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt::format;

    use crate::hashmap::Dict;

    #[test]
    fn it_works() {
        let mut d = Dict::default();

        let pairs: Vec<String> = (0..10).map(|i| format!("{i}")).collect();

        for x in pairs {
            d.insert(&x, &x);
            if d.primary.bucket_count() as isize == d.rehash_idx {
                println!("SHOULD HAVE SWAPPED NOW");
            }

            // println!(
            //     "rehash_idx: {}\n{:?}\n{:?}\nprimary load factor: {}\n",
            //     d.rehash_idx,
            //     &d.primary,
            //     &d.secondary,
            //     d.primary.load_factor_f32(),
            // );
            dbg!(&d);
        }

        for i in 0..9 {
            let key = format!("{}", i);
            d.get(&key);
            dbg!(&d);
        }
    }
}

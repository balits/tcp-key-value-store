use super::hash_table::HashTable;
use crate::Entry;

#[derive(Debug)]
pub struct Dict {
    primary: Box<HashTable>,
    secondary: Box<HashTable>,
    migrate_pos: isize,
}

impl Default for Dict {
    fn default() -> Self {
        let primary = Box::new(HashTable::default());
        Self {
            primary: primary,
            secondary: Box::new(HashTable::EMPTY_TABLE),
            migrate_pos: -1,
        }
    }
}

impl Dict {
    /// Constant to figure out how many items could be stored in a bucket at max
    pub const MAX_ENTRIES_PER_BUCKET: usize = 2;

    /// Max amount of items migrated from one table to another
    /// during one migration. 
    pub const MAX_REHASH_OPS: usize = 2;

    pub fn size(&self) -> usize {
        self.primary.items + self.secondary.items
    }

    pub fn insert(&mut self, key: &str, value: &str) -> Option<String> {
        assert_ne!(self.primary.bucket_count(), 0, "inserting into empty dict");

        let old = self.primary.insert(Entry {
            key: key.into(),
            value: value.into(),
        });
        // trigger the rehash only if the load factor is exceeded
        // AND we are not finished with the previous migration
        if self.primary.bucket_count() * Self::MAX_ENTRIES_PER_BUCKET < self.primary.items
            && self.migrate_pos == -1
        {
            self.trigger_migration();
        }
        self.migrate();
        old
    }

    pub fn get(&mut self, key: &str) -> Option<&Entry> {
        self.migrate();
        self.primary.get(key).or_else(|| self.secondary.get(key))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.migrate();
        self.primary
            .get_mut(key)
            .or_else(|| self.secondary.get_mut(key))
    }

    pub fn remove(&mut self, key: &str) -> Option<Entry> {
        self.migrate();
        self.primary
            .remove(key)
            .or_else(|| self.secondary.remove(key))
    }

    // [private]

    fn trigger_migration(&mut self) {
        assert!(
            self.secondary.is_empty(),
            "triggered rehash on non-empty secondary table"
        );
        let new_primary = Box::new(HashTable::new_with_buckets(self.primary.bucket_count() * 2));
        self.secondary = std::mem::replace(&mut self.primary, new_primary);
        self.migrate_pos = 0;
    }

    /// Move `MAX_REHASH_OPS` number of items from primary hash table to secondary hash table
    /// keeping migrate_pos as an index into which bucket we need to move from
    fn migrate(&mut self) {
        if self.secondary.is_empty() && self.migrate_pos < 0 {
            return;
        }
        // println!("migrate begin");
        let mut ops = 0;

        while self.valid_migrate_pos() {
            let bucket = &mut self.secondary.buckets[self.migrate_pos as usize];

            if bucket.is_empty() {
                self.migrate_pos += 1;
                continue;
            }

            while ops < Self::MAX_REHASH_OPS {
                if let Some(node) = bucket.pop_front() {
                    self.primary.insert(node);
                    self.secondary.items -= 1;
                    ops += 1;
                } else {
                    break;
                }
            }

            if ops == Self::MAX_REHASH_OPS {
                if bucket.is_empty() {
                    self.migrate_pos += 1;
                }
                break;
            } else {
                self.migrate_pos += 1;
                continue;
            }
        }

        if self.secondary.is_empty() || (self.migrate_pos as usize) == self.secondary.bucket_count()
        {
            self.secondary = Box::new(HashTable::EMPTY_TABLE);
            self.migrate_pos = -1;

            // println!("migration finished, empty out secondary table")
        }

        // println!("migrate end, {:#?}", self);
    }

    fn valid_migrate_pos(&self) -> bool {
        self.migrate_pos > -1 && (self.migrate_pos as usize) < self.secondary.bucket_count()
    }
}

#[cfg(test)]
mod test {
    use crate::Dict;

    #[test]
    fn insert() {
        let mut d = Dict::default();
        let old = d.insert("hi", "baby");
        assert!(old.is_none());
        assert_eq!(d.size(), 1);

        let old = d.insert("hi", "something else");
        assert_eq!(old.unwrap(), "baby");
        assert_eq!(d.size(), 1);

        let old = d.insert("hello", "yellow");
        assert!(old.is_none());
        assert_eq!(d.size(), 2);
    }

    #[test]
    fn it_works_big_time() {
        let mut d = Dict::default();

        let strs: Vec<String> = (0..18).map(|i| format!("{i}")).collect();

        for x in strs {
            d.insert(&x, &x);
        }
        dbg!(&d.primary.load_factor(), &d);

        for i in 0..6 {
            let key = format!("{}", i);
            let x = d.get(&key);
            assert_eq!(x.unwrap().key, key);
        }
        assert!(!d.secondary.is_empty());

        let e = d.get("???");
        assert!(e.is_none());
        assert!(d.secondary.is_empty());

        dbg!(&d.primary.load_factor(), &d);
    }
}

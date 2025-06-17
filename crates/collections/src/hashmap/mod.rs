use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{List, Node};

const INIT_BUCKETS: usize = 4;

#[derive(Debug)]
pub(crate) struct HashTable {
    buckets: Vec<List>,
    items: usize,
    mask: usize,
}

impl HashTable {
    pub fn new() -> Self {
        Self {
            buckets: Vec::new(),
            items: 0,
            mask: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.items
    }

    pub fn insert(&mut self, key: &str, value: &str) -> Option<String> {
        if self.items == 0 || self.items > self.buckets.len() * 3 / 4 {
            self.resize();
        }

        let hash = Self::hash(key);
        let i = hash as usize & self.mask;

        match self.buckets[i].iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => {
                let old = std::mem::replace(v, value.to_string());

                Some(old)
            }
            None => {
                self.buckets[i].push_from_parts(key, value);
                self.items += 1;

                None
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&Node> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        self.buckets[i].iter().find(|n| n.key == key)
    }

    // [private]

    fn hash(key: &str) -> u64 {
        let mut h = DefaultHasher::default();
        h.write(key.as_bytes());
        h.finish()
    }

    fn idx(&self, key: &str) -> usize {
        Self::hash(key) as usize & self.mask
    }

    fn resize(&mut self) {
        let new_cap = match self.buckets.len() {
            0 => INIT_BUCKETS,
            n => n * 2,
        };
        self.mask = new_cap - 1;

        let mut new_buckets: Vec<List> = (0..new_cap).map(|_| List::new()).collect();

        for bucks in self.buckets.drain(..) {
            for elem in bucks {
                let i = Self::hash(&elem.key) as usize & self.mask;
                new_buckets[i].push(elem);
            }
        }

        let _ = std::mem::replace(&mut self.buckets, new_buckets);
        // dropping old buckets
    }
}

#[cfg(test)]
mod test {
    

    use crate::{hashmap::HashTable, node};

    #[test]
    fn insert() {
        let mut t = HashTable::new();

        let old = t.insert("foo", "bar");
        assert_eq!(old, None);
        assert_eq!(t.len(), 1);

        let old = t.insert("foo", "baz");
        assert_eq!(old, Some("bar".into()));
        assert_eq!(t.len(), 1);

        t.insert("peti", "is a baby");
        t.insert("sina", "is a tiny baby");

        assert_eq!(t.len(), 3);
        dbg!(t);
    }
    #[test]
    fn get() {
        let mut t = HashTable::new();

        t.insert("peti", "is a baby");
        t.insert("sina", "is a tiny baby");

        assert_eq!(t.get("peti"), Some(&node!("peti", "is a baby")));
        assert_eq!(t.get("sina"), Some(&node!("sina", "is a tiny baby")));
        dbg!(t);
    }

    #[test]
    fn dbg() {
        let mut t = HashTable::new();

        let pairs: Vec<(String, String)> = (0..32)
            .map(|i| {
                let k = format!("{i}");
                let v = format!("{i}");
                (k, v)
            })
            .collect();

        for (k,v) in pairs {
            t.insert(&k, &v);
        } 

        dbg!(t);
    }
}

use std::{
    collections::{LinkedList, linked_list::Cursor},
    hash::{DefaultHasher, Hasher},
};

use crate::{Node, node};

pub const DEFAULT_BUCKET_SIZE: usize = 2;

#[derive(Debug)]
pub(crate) struct HashTable {
    pub(crate) buckets: Vec<LinkedList<Node>>,
    pub(crate) items: usize,
    pub(crate) mask: usize,
}

#[derive(Debug)]
pub struct Iter<'a> {
    ht: &'a HashTable,
    cursor: Cursor<'a, Node>,
    bucket_idx: usize,
}

impl Default for HashTable {
    fn default() -> Self {
        Self::new_with_buckets(DEFAULT_BUCKET_SIZE)
    }
}

impl HashTable {
    /// Creates a new, empty hashmap
    ///
    /// # Note
    ///
    /// This is a `const` function since it does not allocate,
    pub const fn new_empty() -> Self {
        Self {
            buckets: Vec::new(),
            items: 0,
            mask: 0,
        }
    }

    /// Creates a new `HashTable` with `cap` many buckets
    pub fn new_with_buckets(cap: usize) -> Self {
        Self {
            buckets: (0..cap).map(|_| LinkedList::new()).collect(),
            items: 0,
            mask: cap - 1,
        }
    }

    /// Returns the number of items in the hashmap
    pub fn used(&self) -> usize {
        self.items
    }

    /// Shorthand for `self.len() == 0`
    pub fn is_empty(&self) -> bool {
        self.used() == 0
    }

    /// Returns the number of buckets, or "slots" of the hashmap
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    pub fn load_factor(&self) -> usize {
        self.items.checked_div(self.bucket_count()).unwrap_or(0)
    }

    pub fn load_factor_f32(&self) -> f32 {
        if self.bucket_count() == 0 {
            0f32
        } else {
            (self.items as f32) / self.bucket_count() as f32
        }
    }

    /// Shorthand for `self.insert(node!(key, value))`
    fn insert_kv(&mut self, key: &str, value: &str) -> Option<String> {
        self.insert(node!(key, value))
    }

    /// Inserts node into the map (even if its empty)
    /// but does not resize exponentially,
    /// leading to long chains
    pub fn insert_without_resize(&mut self, node: Node) -> Option<String> {
        // Of course we cannot insert to an empty map
        if self.bucket_count() == 0 {
            self.resize();
        }
        self._insert(node)
    }

    /// Insert a key-value pair into the hashmap,
    /// returning the previous value (if there was any)
    pub fn insert(&mut self, node: Node) -> Option<String> {
        if self.is_empty() || self.items > self.bucket_count() * 3 / 4 {
            self.resize();
        }

        self._insert(node)
    }

    fn _insert(&mut self, node: Node) -> Option<String> {
        let hash = Self::hash(&node.key.as_str());
        let i = hash as usize & self.mask;

        match self.buckets[i]
            .iter_mut()
            .find(|n| n.key.as_str() == &node.key)
        {
            Some(n) => {
                let old = std::mem::replace(&mut n.value, node.value);

                Some(old)
            }
            None => {
                self.buckets[i].push_back(node);
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

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Node> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        self.buckets[i].iter_mut().find(|n| n.key == key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn remove(&mut self, key: &str) -> Option<Node> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        let mut cursor_mut = self.buckets[i].cursor_front_mut();
        loop {
            match cursor_mut.current() {
                Some(node) => {
                    if node.key == key {
                        return cursor_mut.remove_current();
                    }
                    cursor_mut.move_next();
                }
                None => return None,
            }
        }
    }

    // [adapters]

    pub fn iter(&self) -> Iter<'_> {
        let cursor = if self.is_empty() {
            /// FIXME: stupid variable that nobody wants here!
            static EMPTY_LIST: LinkedList<Node> = LinkedList::new();
            EMPTY_LIST.cursor_front()
        } else {
            self.buckets[0].cursor_front()
        };

        Iter {
            ht: &self,
            cursor,
            bucket_idx: 0,
        }
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

    // [private]

    /// Resizes the hashmap
    ///
    /// # Panics
    ///
    /// This will not allocate more than `isize::MAX`
    /// and will panic if it ever tries to
    fn resize(&mut self) {
        let new_cap = self.next_capacity();
        // NOTE: allocating more than `isize::MAX`
        // panics when a `Vec` resize internally
        let mut new_buckets: Vec<_> = (0..new_cap).map(|_| LinkedList::new()).collect();

        for bucks in self.buckets.drain(..) {
            for elem in bucks {
                let i = Self::hash(&elem.key) as usize & self.mask;
                new_buckets[i].push_back(elem);
            }
        }

        let _ = std::mem::replace(&mut self.buckets, new_buckets);
        // dropping old buckets
    }

    /// Returns the new capacity,
    /// and sets the `mask` accordingly
    ///
    /// # Note
    ///
    /// Growing the hashmap should only rely on this funcion
    /// as this ensures capacity is always a power of two
    fn next_capacity(&mut self) -> usize {
        let cap = match self.buckets.len() {
            0 => DEFAULT_BUCKET_SIZE,
            n => n * 2,
        };
        self.mask = cap - 1;

        cap
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.cursor.current() {
                Some(node) => {
                    self.cursor.move_next();
                    return Some(node);
                }
                None => {
                    if self.bucket_idx == self.ht.bucket_count() - 1 {
                        return None;
                    }
                    self.bucket_idx += 1;
                    self.cursor = self.ht.buckets[self.bucket_idx].cursor_front();
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::HashTable;
    use crate::{hashmap::hash_table::DEFAULT_BUCKET_SIZE, node};

    #[test]
    fn insert() {
        let mut t = HashTable::new_empty();

        let old = t.insert_kv("foo", "bar");
        assert_eq!(old, None);
        assert_eq!(t.used(), 1);

        let old = t.insert_kv("foo", "baz");
        assert_eq!(old, Some("bar".into()));
        assert_eq!(t.used(), 1);

        t.insert_kv("peti", "is a baby");
        t.insert_kv("sina", "is a tiny baby");

        assert_eq!(t.used(), 3);
        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }
    #[test]
    fn get() {
        let mut t = HashTable::new_empty();

        t.insert_kv("peti", "is a baby");
        t.insert_kv("sina", "is a tiny baby");

        assert_eq!(t.get("peti"), Some(&node!("peti", "is a baby")));
        assert_eq!(t.get("sina"), Some(&node!("sina", "is a tiny baby")));
        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }

    #[test]
    fn dbg() {
        let mut t = HashTable::new_with_buckets(DEFAULT_BUCKET_SIZE);

        let pairs: Vec<(String, String)> = (0..25)
            .map(|i| {
                let k = format!("{i}");
                let v = format!("{i}");
                (k, v)
            })
            .collect();

        for (k, v) in pairs {
            t.insert(node!(&k, &v));
        }

        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }
    #[test]
    fn dbg_long_chains() {
        let mut t = HashTable::new_empty();

        let pairs: Vec<(String, String)> = (0..25)
            .map(|i| {
                let k = format!("{i}");
                let v = format!("{i}");
                (k, v)
            })
            .collect();

        for (k, v) in pairs {
            t.insert_without_resize(node!(k, v));
        }

        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }

    #[test]
    fn iter() {
        let mut h = HashTable::new_empty();

        for i in 0..32 {
            h.insert_kv(format!("{}", i).as_str(), "");
        }

        for node in h.iter() {
            println!("{node:?}");
        }

        dbg!(h);
    }

    #[test]
    fn rust_doc_example() {
        let mut book_reviews = HashTable::new_empty();

        // Review some books.
        book_reviews.insert_kv("Adventures of Huckleberry Finn", "My favorite book.");
        book_reviews.insert_kv("Grimms' Fairy Tales", "Masterpiece.");
        book_reviews.insert_kv("Pride and Prejudice", "Very enjoyable.");
        book_reviews.insert_kv("The Adventures of Sherlock Holmes", "Eye lyked it alot.");

        if !book_reviews.contains_key("Les Misérables") {
            println!(
                "We've got {} reviews, but Les Misérables ain't one.",
                book_reviews.used()
            );
        }

        // oops, this review has a lot of spelling mistakes, let's delete it.
        book_reviews.remove("The Adventures of Sherlock Holmes");

        // Look up the values associated with some keys.
        let to_find = ["Pride and Prejudice", "Alice's Adventure in Wonderland"];
        for &book in &to_find {
            match book_reviews.get(book) {
                Some(review) => println!("{book}: {review:?}"),
                None => println!("{book} is unreviewed."),
            }
        }

        // Look up the value for a key (will panic if the key is not found).
        // println!("Review for Jane: {}", book_reviews["Pride and Prejudice"]);

        dbg!(&book_reviews);
        // Iterate over everything.
        for node in book_reviews.iter() {
            println!("{}: \"{}\"", node.key, node.value);
        }
    }
}

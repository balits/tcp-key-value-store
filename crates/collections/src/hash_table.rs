use std::{
    collections::{LinkedList, linked_list::Cursor},
    hash::{DefaultHasher, Hasher},
};

use super::Entry;

#[derive(Debug)]
pub(crate) struct HashTable {
    pub(crate) buckets: Vec<LinkedList<Entry>>,
    pub(crate) items: usize,
    pub(crate) mask: usize,
}

#[derive(Debug)]
pub struct Iter<'a> {
    ht: &'a HashTable,
    cursor: Cursor<'a, Entry>,
    bucket_idx: usize,
}

impl Default for HashTable {
    fn default() -> Self {
        Self::new_with_buckets(Self::DEFAULT_BUCKET_SIZE)
    }
}

impl HashTable {
    pub const DEFAULT_BUCKET_SIZE: usize = 4;
    pub const EMPTY_TABLE: HashTable = HashTable {
        buckets: Vec::new(),
        mask: 0,
        items: 0,
    };

    /// Creates a new `HashTable` with `cap` many buckets
    pub fn new_with_buckets(size: usize) -> Self {
        let mut buckets = Vec::with_capacity(size);
        for _ in 0..size {
            buckets.push(LinkedList::new());
        }

        let mask = if size == 0 { 0 } else { size - 1 };

        Self {
            buckets,
            items: 0,
            mask,
        }
    }

    /// Shorthand for `self.items == 0`
    pub fn is_empty(&self) -> bool {
        self.items == 0
    }

    /// Returns the number of buckets, or "slots" of the hash table
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// Returns the loadfactor of the hash table
    /// computed as num of items / num of buckets
    pub fn load_factor(&self) -> usize {
        self.items.checked_div(self.bucket_count()).unwrap_or(0)
    }

    /// Inserts an item into the hash table.
    /// This does not resize the table, so if
    /// the tables size is 0, then this function return early with `None`
    pub fn insert(&mut self, node: Entry) -> Option<String> {
        let hash = Self::hash(&node.key.as_str());
        let i = hash as usize & self.mask;

        let slot: Option<&mut Entry> = self
            .buckets
            .get_mut(i)?
            .iter_mut()
            .find(|n| n.key.as_str() == &node.key);

        match slot {
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

    pub fn get(&self, key: &str) -> Option<&Entry> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        self.buckets.get(i)?.iter().find(|n| n.key == key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        self.buckets.get_mut(i)?.iter_mut().find(|n| n.key == key)
    }

    pub fn remove(&mut self, key: &str) -> Option<Entry> {
        let hash = Self::hash(key);
        let i = hash as usize & self.mask;
        let mut cursor_mut = self.buckets.get_mut(i)?.cursor_front_mut();
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

    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    // [adapters]

    pub fn iter(&self) -> Iter<'_> {
        let cursor = if self.is_empty() {
            /// FIXME: stupid variable that nobody wants here!
            static EMPTY_LIST: LinkedList<Entry> = LinkedList::new();
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
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ht.bucket_count() == 0 {
            return None;
        }

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
    macro_rules! node {
        ( $key: expr, $value: expr ) => {
            $crate::Entry {
                key: $key.into(),
                value: $value.into(),
            }
        };
    }

    #[test]
    fn insert() {
        let mut t = HashTable::default();
        dbg!(&t);

        let old = t.insert(node!("foo", "bar"));
        assert_eq!(old, None);
        assert_eq!(t.items, 1);

        // let old = t.insert(node!("foo", "baz"));
        // assert_eq!(old, Some("bar".into()));
        // assert_eq!(t.items(), 1);

        // t.insert(node!("peti", "is a baby"));
        // t.insert(node!("sina", "is a tiny baby"));

        // assert_eq!(t.items(), 3);
        // assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }
    #[test]
    fn get() {
        let mut t = HashTable::default();

        t.insert(node!("peti", "is a baby"));
        t.insert(node!("sina", "is a tiny baby"));

        assert_eq!(t.get("peti"), Some(&node!("peti", "is a baby")));
        assert_eq!(t.get("sina"), Some(&node!("sina", "is a tiny baby")));
        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t);
    }

    #[test]
    fn dbg() {
        let mut t = HashTable::new_with_buckets(12);

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
        dbg!(t.load_factor(), t);
    }
    #[test]
    fn dbg_long_chains() {
        let mut t = HashTable::new_with_buckets(2);

        let pairs: Vec<(String, String)> = (0..25)
            .map(|i| {
                let k = format!("{i}");
                let v = format!("{i}");
                (k, v)
            })
            .collect();

        for (k, v) in pairs {
            t.insert(node!(k, v));
        }

        assert_eq!(t.mask + 1, t.bucket_count());
        dbg!(t.load_factor(), t);
    }

    #[test]
    fn iter() {
        let mut h = HashTable::new_with_buckets(0);

        for i in 0..32 {
            h.insert(node!(format!("{}", i).as_str(), ""));
        }

        for node in h.iter() {
            println!("{node:?}");
        }

        dbg!(h);
    }

    #[test]
    fn rust_doc_example() {
        let mut book_reviews = HashTable::new_with_buckets(0);

        // Review some books.
        book_reviews.insert(node!("Adventures of Huckleberry Finn", "My favorite book."));
        book_reviews.insert(node!("Grimms' Fairy Tales", "Masterpiece."));
        book_reviews.insert(node!("Pride and Prejudice", "Very enjoyable."));
        book_reviews.insert(node!(
            "The Adventures of Sherlock Holmes",
            "Eye lyked it alot."
        ));

        if !book_reviews.contains_key("Les Misérables") {
            println!(
                "We've got {} reviews, but Les Misérables ain't one.",
                book_reviews.items
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

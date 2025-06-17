use crate::boxnode;

pub struct List {
    head: Option<Box<Node>>,
    len: usize,
}

impl List {
    pub fn new() -> Self {
        Self { head: None, len: 0 }
    }
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push<S: Into<String>>(&mut self, key: S, value: S) {
        self.push_boxed(boxnode!(key, value));
    }

    fn push_boxed(&mut self, mut boxed: Box<Node>) {
        boxed.next = self.head.take();
        let next_node = Some(boxed);
        self.head = next_node;
        self.len += 1;
    }

    #[inline]
    pub fn pop(&mut self) -> Option<Node> {
        match self.head.take() {
            None => None,
            Some(mut node) => {
                self.head = node.next.take();
                self.len -= 1;
                Some(*node)
            }
        }
    }

    pub fn peek(&self) -> Option<&Node> {
        match self.head {
            Some(ref head) => Some(head),
            None => None,
        }
    }

    pub fn peek_mut(&mut self) -> Option<&mut Node> {
        match self.head {
            Some(ref mut head) => Some(head),
            None => None,
        }
    }
    // [adapters]

    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
}

impl Drop for List {
    fn drop(&mut self) {
        let mut curr = self.head.take();
        while let Some(mut node) = curr {
            curr = node.next.take();
            // node goes out of scope here, calling drop
        }
    }
}

impl std::fmt::Debug for List {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl IntoIterator for List {
    type Item = <IterOwn as Iterator>::Item;
    type IntoIter = IterOwn;

    fn into_iter(self) -> Self::IntoIter {
        IterOwn::new(self)
    }
}

pub struct Node {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) next: Option<Box<Node>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}
impl Eq for Node {}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}, {}>", self.key, self.value)
    }
}

// [iterators]

pub struct Iter<'a> {
    current: Option<&'a Node>,
    len: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Node;
    fn next(&mut self) -> Option<Self::Item> {
        match self.current.take() {
            None => None,
            Some(node) => {
                self.current = node.next.as_deref();
                Some(node)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> Iter<'a> {
    pub fn new(list: &'a List) -> Self {
        let current = list.head.as_ref().map(|h| &**h);

        Self {
            current,
            len: list.len,
        }
    }
}

pub struct IterOwn(List);

impl Iterator for IterOwn {
    type Item = Node;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len, Some(self.0.len))
    }
}

impl IterOwn {
    pub fn new(list: List) -> Self {
        Self(list)
    }
}

#[cfg(test)]
mod tests {
    use super::List;
    use crate::node;

    #[test]
    pub fn push() {
        let mut list = List::new();

        for i in 0..10 {
            let k = format!("key{i}");
            let v = format!("value{i}");
            list.push(k, v);
        }

        assert_eq!(10, list.len());
    }

    #[test]
    fn pop() {
        let mut list = List::new();

        // Check empty list behaves right
        assert!(list.pop().is_none());

        // Populate list
        list.push("k1", "v1");
        list.push("k2", "v2");
        list.push("k3", "v3");

        // Check normal removal
        let p = list.pop().unwrap();
        assert_eq!(p.key, "k3");
        assert_eq!(p.value, "v3");

        let p = list.pop().unwrap();
        assert_eq!(p.key, "k2");
        assert_eq!(p.value, "v2");

        // Push some more just to make sure nothing's corrupted
        list.push("k5", "v5");
        list.push("k6", "v6");

        // Check normal removal
        let p = list.pop().unwrap();
        assert_eq!(p.key, "k6");
        assert_eq!(p.value, "v6");
        let p = list.pop().unwrap();
        assert_eq!(p.key, "k5");
        assert_eq!(p.value, "v5");

        // Check exhaustion
        let p = list.pop().unwrap();
        assert_eq!(p.key, "k1");
        assert_eq!(p.value, "v1");
        assert!(list.pop().is_none());
    }
    #[test]
    fn peek() {
        let mut list = List::new();
        assert_eq!(list.peek(), None);
        assert_eq!(list.peek_mut(), None);

        list.push("k1", "v1");
        list.push("k2", "v2");
        list.push("k3", "v3");

        assert_eq!(list.peek(), Some(&node!("k3", "v3")));
        assert_eq!(list.peek_mut(), Some(&mut node!("k3", "v3")));
        list.pop();
        assert_eq!(list.peek(), Some(&node!("k2", "v2")));
        assert_eq!(list.peek_mut(), Some(&mut node!("k2", "v2")));
        list.pop();
        assert_eq!(list.peek(), Some(&node!("k1", "v1")));
        assert_eq!(list.peek_mut(), Some(&mut node!("k1", "v1")));
        list.pop();
        assert_eq!(list.peek(), None);
        assert_eq!(list.peek_mut(), None);
    }

    #[test]
    fn iter() {
        let mut list = List::new();

        for i in 0..10 {
            let k = format!("key{i}");
            let v = format!("value{i}");
            list.push(k, v);
        }

        for (i, e) in list.iter().enumerate() {
            let k = format!("key{}", 10 - (i + 1));
            let v = format!("value{}", 10 - (i + 1));
            assert_eq!(e, &node!(k, v));
        }

        assert_eq!(list.len(), 10);

        for (i, e) in list.into_iter().enumerate() {
            let k = format!("key{}", 10 - (i + 1));
            let v = format!("value{}", 10 - (i + 1));
            assert_eq!(e, node!(k, v));
        }
    }
}

#[macro_export]
macro_rules! boxnode {
    ( $key: expr, $value: expr) => {
        Box::new($crate::linked_list::Node {
            key: $key.into(),
            value: $value.into(),
            next: None,
        })
    };
}

#[macro_export]
macro_rules! node {
    ( $key: expr, $value: expr) => {
        $crate::linked_list::Node {
            key: $key.into(),
            value: $value.into(),
            next: None,
        }
    };
}

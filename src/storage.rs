use std::{
    collections::HashMap, hash::{BuildHasherDefault, DefaultHasher}, sync::Mutex
};

pub static MAP: Mutex<HashMap<String, String, BuildHasherDefault<DefaultHasher>>> =
    Mutex::new(HashMap::with_hasher(BuildHasherDefault::new()));
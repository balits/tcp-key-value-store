use std::{
    collections::HashMap, hash::{BuildHasherDefault, DefaultHasher}, sync::Mutex
};

use collections::Dict;
use std::sync::OnceLock;

pub static MAP: Mutex<HashMap<String, String, BuildHasherDefault<DefaultHasher>>> =
    Mutex::new(HashMap::with_hasher(BuildHasherDefault::new()));

pub static mut MAP2: OnceLock<Dict> = OnceLock::new();

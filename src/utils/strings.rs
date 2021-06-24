use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
lazy_static! {
    pub static ref STRING_POOL: Mutex<HashMap<String, &'static str>> = Default::default();
}

// don't make this function From<XXX>. We'd better call this explicitly
// prevent any unintentional mem leak
pub fn intern_string(s: &str) -> &'static str {
    *STRING_POOL
        .lock()
        .unwrap()
        .entry(s.to_owned())
        .or_insert_with(|| Box::leak(s.to_string().into_boxed_str()))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InternedString(&'static str);

impl From<&'static str> for InternedString {
    fn from(str: &'static str) -> Self {
        InternedString(str)
    }
}

impl From<InternedString> for &'static str {
    fn from(str: InternedString) -> Self {
        str.0
    }
}

impl std::ops::Deref for InternedString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl serde::ser::Serialize for InternedString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0)
    }
}

impl<'de> serde::de::Deserialize<'de> for InternedString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(intern_string(&s).into())
    }
}

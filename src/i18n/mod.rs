use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

type JsonMap = serde_json::Map<String, serde_json::Value>;

static INSTANCE: once_cell::sync::OnceCell<Arc<RwLock<I18n>>> = once_cell::sync::OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18n {
    locale: String,
    strings: JsonMap,
}

impl I18n {
    pub fn new(locale: &str) -> Self {
        let strings = Self::load_locale(locale);
        Self {
            locale: locale.to_string(),
            strings,
        }
    }

    fn load_locale(locale: &str) -> JsonMap {
        let path = Path::new("locales").join(format!("{}.json", locale));
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = value.as_object() {
                    return obj.clone();
                }
            }
        }
        JsonMap::new()
    }

    pub fn t(&self, key: &str) -> String {
        let keys: Vec<&str> = key.split('.').collect();
        self.get_nested(&self.strings, &keys)
    }

    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut result = self.t(key);
        for (k, v) in args {
            result = result.replace(&format!("{{{}}}", k), v);
        }
        result
    }

    fn get_nested(&self, map: &JsonMap, keys: &[&str]) -> String {
        if keys.is_empty() {
            return String::new();
        }

        if let Some(value) = map.get(keys[0]) {
            if keys.len() == 1 {
                return value.as_str().unwrap_or("").to_string();
            }
            if let Some(obj) = value.as_object() {
                return self.get_nested(obj, &keys[1..]);
            }
        }

        keys.join(".")
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.strings = Self::load_locale(locale);
        self.locale = locale.to_string();
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn available_locales() -> Vec<&'static str> {
        vec!["en", "zh"]
    }
}

pub fn init(locale: &str) {
    let i18n = Arc::new(RwLock::new(I18n::new(locale)));
    INSTANCE.set(i18n).ok();
}

pub fn get() -> Arc<RwLock<I18n>> {
    INSTANCE
        .get()
        .cloned()
        .expect("I18n not initialized")
        .clone()
}

pub fn t(key: &str) -> String {
    get().read().t(key)
}

pub fn t_with_args(key: &str, args: &[(&str, &str)]) -> String {
    get().read().t_with_args(key, args)
}

pub fn set_locale(locale: &str) {
    get().write().set_locale(locale);
}

pub fn locale() -> String {
    get().read().locale().to_string()
}

pub fn available_locales() -> Vec<&'static str> {
    I18n::available_locales()
}

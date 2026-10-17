//! Preferencias guardadas en `~/.arena-cero` (como el `localStorage` de la web): calidad,
//! volumen de la música y sensibilidad del mouse. Formato `clave=valor`, una por línea.

use std::collections::BTreeMap;

fn path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".arena-cero"))
}

fn read() -> BTreeMap<String, String> {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect())
        .unwrap_or_default()
}

pub fn get(key: &str) -> Option<String> {
    read().get(key).cloned()
}

pub fn get_f32(key: &str) -> Option<f32> {
    get(key).and_then(|v| v.parse().ok())
}

pub fn set(key: &str, value: impl ToString) {
    let mut m = read();
    m.insert(key.to_string(), value.to_string());
    if let Some(p) = path() {
        let s: String = m.iter().map(|(k, v)| format!("{k}={v}\n")).collect();
        let _ = std::fs::write(p, s);
    }
}

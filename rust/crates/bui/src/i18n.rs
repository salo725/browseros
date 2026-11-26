//! i18n: load UI translations from /usr/share/bravos/i18n/<code>.json,
//! with English fallback. Supports RTL languages.

use std::collections::HashMap;

#[derive(Clone)]
pub struct Tr {
    pub code: String,
    pub name: String,
    pub rtl: bool,
    strings: HashMap<String, String>,
    en: HashMap<String, String>,
}

impl Tr {
    fn dir_candidates() -> Vec<String> {
        let mut v = Vec::new();
        if let Ok(d) = std::env::var("BRAVOS_I18N_DIR") {
            v.push(d);
        }
        v.push("/usr/share/bravos/i18n".into());
        v.push("i18n".into());
        v
    }

    fn parse(code: &str) -> Option<(String, String, bool, HashMap<String, String>)> {
        let mut path = None;
        for d in Self::dir_candidates() {
            let p = format!("{}/{}.json", d, code);
            if std::path::Path::new(&p).exists() {
                path = Some(p);
                break;
            }
        }
        let path = path?;
        let s = std::fs::read_to_string(path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&s).ok()?;
        let mut map = HashMap::new();
        if let Some(obj) = v["strings"].as_object() {
            for (k, val) in obj {
                if let Some(sv) = val.as_str() {
                    map.insert(k.clone(), sv.to_string());
                }
            }
        }
        Some((
            v["code"].as_str().unwrap_or(code).to_string(),
            v["name"].as_str().unwrap_or(code).to_string(),
            v["rtl"].as_bool().unwrap_or(false),
            map,
        ))
    }

    /// Load translations for a language code ("auto" detects from $LANG).
    pub fn load(code: &str) -> Tr {
        let code = if code == "auto" {
            let lang = std::env::var("LANG").unwrap_or_default();
            let base = lang.split('.').next().unwrap_or("en_US");
            // "zh_CN.UTF-8" -> "zh-CN"
            let norm = base.replace('_', "-");
            match norm.rsplit_once('-') {
                Some((p, s)) if s.len() == 2 && s.chars().all(|c| c.is_ascii_digit()) => p.to_string(),
                _ => norm,
            }
        } else {
            code.to_string()
        };
        let en = Self::parse("en")
            .map(|(_, _, _, m)| m)
            .unwrap_or_default();
        let mut cur = Self::parse(&code);
        if cur.is_none() {
            // try base language (zh-Hant -> zh, pt-BR -> pt)
            if let Some((p, _)) = code.split_once('-') {
                cur = Self::parse(p);
            }
        }
        match cur {
            Some((c, name, rtl, strings)) => Tr { code: c, name, rtl, strings, en },
            None => Tr {
                code: "en".into(),
                name: "English".into(),
                rtl: false,
                strings: HashMap::new(),
                en,
            },
        }
    }

    /// Translate a key. Falls back to English, then the key itself.
    pub fn t(&self, key: &str) -> String {
        if let Some(s) = self.strings.get(key) {
            return s.clone();
        }
        if let Some(s) = self.en.get(key) {
            return s.clone();
        }
        key.to_string()
    }

    /// List available languages from the i18n directory.
    pub fn languages() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for d in Self::dir_candidates() {
            if let Ok(rd) = std::fs::read_dir(&d) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if let Some(code) = name.strip_suffix(".json") {
                        if let Some((_, n, _, _)) = Self::parse(code) {
                            out.push((code.to_string(), n));
                        }
                    }
                }
                break;
            }
        }
        out.sort();
        out
    }
}

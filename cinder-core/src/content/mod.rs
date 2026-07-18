pub mod loader;
mod loader_validation;
mod system_text_defs;
pub mod text_defs;
pub mod types;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    fn collect_string_values(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(arr) => {
                for item in arr {
                    collect_string_values(item, out);
                }
            }
            serde_json::Value::Object(map) => {
                for val in map.values() {
                    collect_string_values(val, out);
                }
            }
            _ => {}
        }
    }

    fn find_placeholders_with_spaces(s: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut chars = s.char_indices().peekable();
        while let Some((i, ch)) = chars.next() {
            if ch != '{' {
                continue;
            }
            let start = i + 1;
            let mut end = start;
            while let Some(&(_, c)) = chars.peek() {
                if c == '}' {
                    chars.next();
                    break;
                }
                end += 1;
                chars.next();
            }
            let inner = &s[start..end];
            if !inner.is_empty() && inner.contains(' ') {
                results.push(format!("{{{inner}}}"));
            }
        }
        results
    }

    #[test]
    fn template_placeholders_have_no_spaces() {
        let content_dir =
            std::path::PathBuf::from(env!("CINDER_PROJECT_DIR")).join("content");
        let mut violations: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for entry in std::fs::read_dir(&content_dir).unwrap().flatten() {
            let pack_dir = entry.path();
            if !pack_dir.is_dir() {
                continue;
            }
            let locales_dir = pack_dir.join("locales");
            if !locales_dir.is_dir() {
                continue;
            }
            for locale_entry in std::fs::read_dir(&locales_dir).unwrap().flatten() {
                let locale_dir = locale_entry.path();
                if !locale_dir.is_dir() {
                    continue;
                }
                for json_entry in std::fs::read_dir(&locale_dir).unwrap().flatten() {
                    let path = json_entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    let text = match std::fs::read_to_string(&path) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let mut strings = Vec::new();
                    collect_string_values(&value, &mut strings);
                    for s in &strings {
                        let bad = find_placeholders_with_spaces(s);
                        if !bad.is_empty() {
                            let rel = path
                                .strip_prefix(&content_dir)
                                .unwrap_or(&path)
                                .display()
                                .to_string();
                            violations.entry(rel).or_default().extend(bad);
                        }
                    }
                }
            }
        }

        if !violations.is_empty() {
            let mut msg = String::from("Template placeholders contain spaces (always a typo):\n");
            for (file, placeholders) in &violations {
                for p in placeholders {
                    msg.push_str(&format!("  {file}: {p}\n"));
                }
            }
            panic!("{msg}");
        }
    }
}

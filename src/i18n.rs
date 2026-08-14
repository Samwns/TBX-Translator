pub fn t<'a>(key: &'a str, lang: &str) -> String {
    let data = crate::locales_gen::get_i18n_data();

    let base_lang = lang.split('_').next().unwrap_or(lang);

    if let Some(lang_map) = data.get(lang).or_else(|| data.get(base_lang)) {
        if let Some(val) = lang_map.get(key) {
            return val.clone();
        }
    }

    if let Some(en_map) = data.get("en") {
        if let Some(val) = en_map.get(key) {
            return val.clone();
        }
    }

    if let Some(pt_map) = data.get("pt") {
        if let Some(val) = pt_map.get(key) {
            return val.clone();
        }
    }

    key.to_string()
}

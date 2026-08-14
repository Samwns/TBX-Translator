use std::collections::HashMap;
use std::sync::OnceLock;

pub fn get_i18n_data() -> &'static HashMap<String, HashMap<String, String>> {
    static DATA: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut map = HashMap::new();
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/af.json")) {
            map.insert("af".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sq.json")) {
            map.insert("sq".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/bn.json")) {
            map.insert("bn".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/hy.json")) {
            map.insert("hy".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/eu.json")) {
            map.insert("eu".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/bs.json")) {
            map.insert("bs".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/be.json")) {
            map.insert("be".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ar.json")) {
            map.insert("ar".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/az.json")) {
            map.insert("az".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/am.json")) {
            map.insert("am".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ca.json")) {
            map.insert("ca".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/zh-CN.json")) {
            map.insert("zh-CN".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ceb.json")) {
            map.insert("ceb".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/bg.json")) {
            map.insert("bg".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/en.json")) {
            map.insert("en".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ny.json")) {
            map.insert("ny".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/co.json")) {
            map.insert("co".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/da.json")) {
            map.insert("da".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/hr.json")) {
            map.insert("hr".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/cs.json")) {
            map.insert("cs".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/eo.json")) {
            map.insert("eo".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/nl.json")) {
            map.insert("nl".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/et.json")) {
            map.insert("et".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/tl.json")) {
            map.insert("tl".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/fi.json")) {
            map.insert("fi".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/zh-TW.json")) {
            map.insert("zh-TW".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ka.json")) {
            map.insert("ka".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/el.json")) {
            map.insert("el".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/gu.json")) {
            map.insert("gu".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/fy.json")) {
            map.insert("fy".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/fr.json")) {
            map.insert("fr".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/gl.json")) {
            map.insert("gl".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ht.json")) {
            map.insert("ht".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ha.json")) {
            map.insert("ha".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/de.json")) {
            map.insert("de".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/iw.json")) {
            map.insert("iw".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/hi.json")) {
            map.insert("hi".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/hu.json")) {
            map.insert("hu".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/haw.json")) {
            map.insert("haw".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/hmn.json")) {
            map.insert("hmn".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/is.json")) {
            map.insert("is".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/id.json")) {
            map.insert("id".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ig.json")) {
            map.insert("ig".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/kn.json")) {
            map.insert("kn".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ku.json")) {
            map.insert("ku".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/it.json")) {
            map.insert("it".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ga.json")) {
            map.insert("ga".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/kk.json")) {
            map.insert("kk".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/jw.json")) {
            map.insert("jw".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ja.json")) {
            map.insert("ja".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/km.json")) {
            map.insert("km".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ky.json")) {
            map.insert("ky".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ko.json")) {
            map.insert("ko".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/lo.json")) {
            map.insert("lo".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mk.json")) {
            map.insert("mk".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ml.json")) {
            map.insert("ml".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/lv.json")) {
            map.insert("lv".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/la.json")) {
            map.insert("la".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mg.json")) {
            map.insert("mg".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/lt.json")) {
            map.insert("lt".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/lb.json")) {
            map.insert("lb".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mr.json")) {
            map.insert("mr".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ms.json")) {
            map.insert("ms".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/pt.json")) {
            map.insert("pt".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ne.json")) {
            map.insert("ne".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mt.json")) {
            map.insert("mt".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ps.json")) {
            map.insert("ps".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mi.json")) {
            map.insert("mi".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/pa.json")) {
            map.insert("pa".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/fa.json")) {
            map.insert("fa".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/mn.json")) {
            map.insert("mn".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/my.json")) {
            map.insert("my".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/no.json")) {
            map.insert("no".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/pl.json")) {
            map.insert("pl".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ru.json")) {
            map.insert("ru".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ro.json")) {
            map.insert("ro".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sd.json")) {
            map.insert("sd".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sm.json")) {
            map.insert("sm".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/si.json")) {
            map.insert("si".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/gd.json")) {
            map.insert("gd".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sr.json")) {
            map.insert("sr".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/st.json")) {
            map.insert("st".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sn.json")) {
            map.insert("sn".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ta.json")) {
            map.insert("ta".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sk.json")) {
            map.insert("sk".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/te.json")) {
            map.insert("te".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sl.json")) {
            map.insert("sl".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/es.json")) {
            map.insert("es".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/so.json")) {
            map.insert("so".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/su.json")) {
            map.insert("su".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sw.json")) {
            map.insert("sw".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/sv.json")) {
            map.insert("sv".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/tr.json")) {
            map.insert("tr".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/tg.json")) {
            map.insert("tg".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/ur.json")) {
            map.insert("ur".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/th.json")) {
            map.insert("th".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/uz.json")) {
            map.insert("uz".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/uk.json")) {
            map.insert("uk".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/vi.json")) {
            map.insert("vi".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/xh.json")) {
            map.insert("xh".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/cy.json")) {
            map.insert("cy".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/zu.json")) {
            map.insert("zu".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/yi.json")) {
            map.insert("yi".to_string(), parsed);
        }
        if let Ok(parsed) = serde_json::from_str(include_str!("../locales/yo.json")) {
            map.insert("yo".to_string(), parsed);
        }
        map
    })
}

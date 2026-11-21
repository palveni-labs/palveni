//! The format-1 pure resource package accepted by the workbench. The manifest
//! and all CSS-facing data are checked before any files become publicly visible.

mod color;
mod tokens;

use std::collections::BTreeMap;

use semver::Version;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::{check_compat, manifest::is_valid_plugin_id};
use tokens::*;

pub const MAX_SKIN_JSON_BYTES: usize = 512 * 1024;
pub const MAX_PLUGIN_TOML_BYTES: usize = 64 * 1024;
pub const MAX_ASSET_BYTES: usize = 2 * 1024 * 1024;

type Result<T> = std::result::Result<T, String>;
type Dict = Map<String, Value>;

#[derive(Clone, Debug)]
pub struct ValidatedSkin {
    pub id: String,
    pub plugin_id: String,
    pub version: String,
    pub name: BTreeMap<String, String>,
    pub description: BTreeMap<String, String>,
    pub fonts: BTreeMap<String, Option<u64>>,
    pub preview: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    plugin: Plugin,
    compat: Compat,
    provides: Provides,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plugin {
    id: String,
    version: String,
    runtime: String,
    name: String,
    description: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    license: Option<String>,
    homepage: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Compat {
    api: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provides {
    skin: String,
    preview: Option<String>,
}

fn check(ok: bool, field: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(format!("invalid skin field: {field}"))
    }
}

fn plain(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.encode_utf16().count() <= max
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '<' | '>'))
}

impl Package {
    fn validate(&self) -> Result<()> {
        let p = &self.plugin;
        check(p.runtime == "none", "plugin.runtime")?;
        check(is_valid_plugin_id(&p.id), "plugin.id")?;
        check(Version::parse(&p.version).is_ok(), "plugin.version")?;
        check(plain(&p.name, 64), "plugin.name")?;
        check(
            p.description.as_deref().is_none_or(|v| plain(v, 1024)),
            "plugin.description",
        )?;
        check(p.authors.iter().all(|v| plain(v, 120)), "plugin.authors")?;
        check(
            p.license.as_deref().is_none_or(|v| plain(v, 128)),
            "plugin.license",
        )?;
        check(
            p.homepage.as_deref().is_none_or(|v| {
                plain(v, 1024) && (v.starts_with("https://") || v.starts_with("http://"))
            }),
            "plugin.homepage",
        )?;
        check_compat(&self.compat.api).map_err(|e| e.to_string())?;
        check(self.provides.skin == "skin.json", "provides.skin")?;
        check(
            self.provides.preview.as_deref().is_none_or(|v| {
                matches!(
                    v,
                    "preview.png" | "preview.webp" | "preview.avif" | "preview.jpg"
                )
            }),
            "provides.preview",
        )
    }
}

pub fn is_valid_skin_id(id: &str) -> bool {
    (2..=63).contains(&id.len())
        && id != "workcolor"
        && !id.contains("..")
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-'))
}

/// Validates a package manifest and its skin document. The installer also
/// checks that declared assets exist, have the expected size and type, and
/// never resolve outside the package.
pub fn validate_skin(plugin_toml: &[u8], skin_json: &[u8]) -> Result<ValidatedSkin> {
    check(
        plugin_toml.len() <= MAX_PLUGIN_TOML_BYTES,
        "plugin.toml size",
    )?;
    check(skin_json.len() <= MAX_SKIN_JSON_BYTES, "skin.json size")?;
    let package: Package =
        toml::from_str(std::str::from_utf8(plugin_toml).map_err(|_| "plugin.toml is not UTF-8")?)
            .map_err(|_| "invalid pure-resource plugin.toml")?;
    package.validate()?;
    let json: Value = serde_json::from_slice(skin_json).map_err(|_| "invalid skin.json")?;
    let root = fields(
        &json,
        &[
            "format",
            "id",
            "name",
            "slots",
            "colors",
            "elevation",
            "work",
            "type",
            "shape",
            "motion",
        ],
        &["fonts", "description", "author"],
        "skin",
    )?;
    check(root["format"].as_u64() == Some(1), "format")?;
    let id = root["id"]
        .as_str()
        .filter(|id| is_valid_skin_id(id))
        .ok_or("invalid skin id")?;
    let name = localized(&root["name"], "name")?;
    let description = root
        .get("description")
        .map(|v| localized(v, "description"))
        .transpose()?
        .unwrap_or_default();
    if let Some(author) = root.get("author") {
        check(author.as_str().is_some_and(|v| plain(v, 80)), "author")?;
    }

    let slots = map(&root["slots"], "slots")?;
    check(slots.len() == SLOTS.len(), "slots")?;
    for &(key, choices) in SLOTS {
        check(
            slots
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| choices.contains(&s)),
            key,
        )?;
    }
    for field in ["colors", "elevation", "work"] {
        let modes = fields(&root[field], &["light", "dark"], &[], field)?;
        for mode in ["light", "dark"] {
            match field {
                "colors" => palette(&modes[mode])?,
                "elevation" => {
                    for (key, shadow) in fields(&modes[mode], ELEVATION_TOKENS, &[], "elevation")? {
                        check(shadow.as_str().is_some_and(color::shadow), key)?;
                    }
                }
                _ => work_colors(&modes[mode])?,
            }
        }
    }
    typography(&root["type"])?;
    let shape = map(&root["shape"], "shape")?;
    check(shape.len() == SHAPE_LIMITS.len(), "shape")?;
    for &(key, min, max) in SHAPE_LIMITS {
        bounded(shape.get(key), key, min, max)?;
    }
    let motion = fields(&root["motion"], &["fast", "base", "ease"], &[], "motion")?;
    bounded(motion.get("fast"), "motion.fast", 0., 600.)?;
    bounded(motion.get("base"), "motion.base", 0., 900.)?;
    let ease = motion["ease"]
        .as_array()
        .filter(|a| a.len() == 4)
        .ok_or("invalid motion.ease")?;
    for (i, v) in ease.iter().enumerate() {
        let (min, max) = if i % 2 == 0 { (0., 1.) } else { (-2., 3.) };
        bounded(Some(v), "motion.ease", min, max)?;
    }

    Ok(ValidatedSkin {
        id: id.to_owned(),
        plugin_id: package.plugin.id,
        version: package.plugin.version,
        preview: package.provides.preview,
        name,
        description,
        fonts: fonts(root.get("fonts"))?,
    })
}

fn map<'a>(v: &'a Value, field: &str) -> Result<&'a Dict> {
    v.as_object()
        .ok_or_else(|| format!("{field} must be an object"))
}

fn fields<'a>(v: &'a Value, required: &[&str], optional: &[&str], field: &str) -> Result<&'a Dict> {
    let obj = map(v, field)?;
    for key in obj.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("unknown {field} field"));
        }
    }
    for key in required {
        if !obj.contains_key(*key) {
            return Err(format!("missing {field}.{key}"));
        }
    }
    Ok(obj)
}

fn bounded(v: Option<&Value>, field: &str, min: f64, max: f64) -> Result<()> {
    check(
        v.and_then(Value::as_f64)
            .is_some_and(|n| (min..=max).contains(&n)),
        field,
    )
}

fn localized(v: &Value, field: &str) -> Result<BTreeMap<String, String>> {
    let values = fields(v, &[], &["zh-CN", "en"], field)?;
    check(!values.is_empty(), field)?;
    values
        .iter()
        .map(|(lang, txt)| {
            let text = txt
                .as_str()
                .filter(|s| plain(s, 120))
                .ok_or_else(|| format!("invalid {field}.{lang}"))?;
            Ok((lang.clone(), text.to_owned()))
        })
        .collect()
}

fn palette(v: &Value) -> Result<()> {
    let vals = fields(v, COLOR_TOKENS, &[], "colors")?;
    let mut colors = BTreeMap::new();
    for (key, c) in vals {
        let color = c
            .as_str()
            .and_then(color::parse)
            .ok_or_else(|| format!("invalid colour for {key}"))?;
        check(!OPAQUE.contains(&key.as_str()) || color.alpha == 1.0, key)?;
        colors.insert(key.as_str(), color);
    }
    for &(front, back, min) in CONTRAST_PAIRS {
        if colors[front].contrast_on(colors[back]) + 1e-6 < min {
            return Err(format!("insufficient contrast for {front} on {back}"));
        }
    }
    Ok(())
}

fn work_colors(v: &Value) -> Result<()> {
    let obj = fields(
        v,
        &[
            "fieldL", "fieldK", "subL", "washL", "washK", "barL", "barK", "onField",
        ],
        &[],
        "work",
    )?;
    for (key, val) in obj {
        if key == "onField" {
            check(val.as_str().and_then(color::parse).is_some(), key)?;
        } else {
            bounded(
                Some(val),
                key,
                0.,
                if key.ends_with('L') { 1.0 } else { 2.0 },
            )?;
        }
    }
    Ok(())
}

fn family(value: &str) -> bool {
    value == "-apple-system"
        || ((1..=63).contains(&value.len())
            && value.as_bytes()[0].is_ascii_alphanumeric()
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'-')))
}

fn typography(v: &Value) -> Result<()> {
    let obj = fields(
        v,
        &[
            "heading",
            "ui",
            "mono",
            "size",
            "weight",
            "leading",
            "headingTracking",
        ],
        &[],
        "type",
    )?;
    for key in ["heading", "ui", "mono"] {
        let families = obj[key].as_array().ok_or("invalid type family list")?;
        check(
            (1..=16).contains(&families.len())
                && families.iter().all(|v| v.as_str().is_some_and(family))
                && families
                    .last()
                    .and_then(Value::as_str)
                    .is_some_and(|s| GENERIC_FAMILIES.contains(&s)),
            key,
        )?;
    }
    for (key, val) in fields(&obj["size"], TYPE_SIZE_TOKENS, &[], "type.size")? {
        bounded(Some(val), key, 9., 48.)?;
    }
    for (key, val) in fields(
        &obj["weight"],
        &["regular", "medium", "strong", "heading"],
        &[],
        "type.weight",
    )? {
        bounded(Some(val), key, 100., 900.)?;
    }
    bounded(obj.get("leading"), "type.leading", 1., 2.2)?;
    bounded(
        obj.get("headingTracking"),
        "type.headingTracking",
        -0.06,
        0.1,
    )
}

fn font_path(src: &str) -> bool {
    src.strip_prefix("fonts/")
        .and_then(|name| name.strip_suffix(".woff2"))
        .is_some_and(|stem| {
            (1..=121).contains(&stem.len())
                && !src.contains("..")
                && stem.as_bytes()[0].is_ascii_alphanumeric()
                && stem
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        })
}

fn font_weight(s: &str) -> bool {
    let valid = |v: &str| {
        v.len() == 3 && v.bytes().all(|b| b.is_ascii_digit()) && ("100"..="900").contains(&v)
    };
    match s.split_once(' ') {
        Some((start, end)) => valid(start) && valid(end) && start <= end,
        None => valid(s),
    }
}

fn unicode_range(s: &str) -> bool {
    fn scalar(text: &str) -> Option<u32> {
        if text.is_empty() || text.len() > 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(text, 16)
            .ok()
            .filter(|n| *n <= 0x10ffff)
    }
    s.len() <= 6000
        && s.split(',').enumerate().all(|(i, elem)| {
            let elem = if i == 0 { elem } else { elem.trim_start() };
            let Some(code) = elem.strip_prefix("U+") else {
                return false;
            };
            if let Some((start, end)) = code.split_once('-') {
                matches!((scalar(start),scalar(end)), (Some(a),Some(b)) if a<=b)
            } else if code.contains('?') {
                let digits = code.trim_end_matches('?');
                code.len() <= 6
                    && digits.bytes().all(|b| b.is_ascii_hexdigit())
                    && scalar(&code.replace('?', "0")).is_some()
            } else {
                scalar(code).is_some()
            }
        })
}

fn fonts(v: Option<&Value>) -> Result<BTreeMap<String, Option<u64>>> {
    let mut paths = BTreeMap::new();
    let Some(v) = v else {
        return Ok(paths);
    };
    let entries = v
        .as_array()
        .filter(|a| a.len() <= 400)
        .ok_or("invalid fonts list")?;
    for entry in entries {
        let f = fields(
            entry,
            &["family", "src", "weight", "style"],
            &[
                "unicodeRange",
                "sizeAdjust",
                "ascentOverride",
                "descentOverride",
                "bytes",
            ],
            "font",
        )?;
        check(f["family"].as_str().is_some_and(family), "font.family")?;
        let path = f["src"]
            .as_str()
            .filter(|s| font_path(s))
            .ok_or("invalid font.src")?;
        check(f["weight"].as_str().is_some_and(font_weight), "font.weight")?;
        check(
            matches!(f["style"].as_str(), Some("normal" | "italic")),
            "font.style",
        )?;
        if let Some(v) = f.get("unicodeRange") {
            check(v.as_str().is_some_and(unicode_range), "font.unicodeRange")?;
        }
        for key in ["sizeAdjust", "ascentOverride", "descentOverride"] {
            if f.contains_key(key) {
                bounded(
                    f.get(key),
                    key,
                    if key == "sizeAdjust" { 50. } else { 0. },
                    200.,
                )?;
            }
        }
        let bytes = f
            .get("bytes")
            .map(|v| {
                v.as_u64()
                    .filter(|n| (1..=MAX_ASSET_BYTES as u64).contains(n))
                    .ok_or("invalid font.bytes")
            })
            .transpose()?;
        let size = paths.entry(path.to_owned()).or_insert(None);
        if let Some(declared) = bytes {
            check(size.is_none_or(|n| n == declared), "conflicting font sizes")?;
            *size = Some(declared);
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NATIVE_TOML: &[u8] = include_bytes!("../../../plugins/skin-native/plugin.toml");
    const NATIVE_JSON: &[u8] = include_bytes!("../../../plugins/skin-native/skin.json");

    #[test]
    fn official_manifests_declare_the_shipped_files() {
        let fixtures: &[(&str, &[u8], &[u8])] = &[
            ("native", NATIVE_TOML, NATIVE_JSON),
            (
                "gear",
                include_bytes!("../../../plugins/skin-gear/plugin.toml"),
                include_bytes!("../../../plugins/skin-gear/skin.json"),
            ),
            (
                "ink",
                include_bytes!("../../../plugins/skin-ink/plugin.toml"),
                include_bytes!("../../../plugins/skin-ink/skin.json"),
            ),
        ];
        for &(id, plugin, json) in fixtures {
            let skin = validate_skin(plugin, json).expect("shipped skin");
            assert_eq!(skin.id, id);
            assert_eq!(skin.plugin_id, format!("com.palveni.skin.{id}"));
            assert!(skin.name.contains_key("zh-CN") && skin.name.contains_key("en"));
            assert_eq!(skin.preview.as_deref(), Some("preview.png"));
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../plugins/skin-{id}"));
            for (file, declared) in skin.fonts {
                let len = dir.join(file).metadata().expect("shipped font").len();
                assert_eq!(declared, Some(len));
            }
        }
    }

    #[test]
    fn pure_resource_manifest_allows_only_skin_capabilities() {
        let valid = std::str::from_utf8(NATIVE_TOML).expect("fixture UTF-8");
        for (before, after) in [
            ("runtime = \"none\"", "runtime = \"wasm\""),
            ("[compat]", "entry = \"plugin.wasm\"\n[compat]"),
            (
                "[compat]",
                "[permissions]\nnetwork = [\"example.org\"]\n[compat]",
            ),
            ("^0.1", "^2"),
            ("skin = \"skin.json\"", "skin = \"../skin.json\""),
            ("preview.png", "preview.svg"),
            ("com.palveni.skin.native", "/tmp/native"),
        ] {
            let plugin = valid.replace(before, after);
            assert!(
                validate_skin(plugin.as_bytes(), NATIVE_JSON).is_err(),
                "{after}"
            );
        }
        assert!(validate_skin(&vec![b' '; MAX_PLUGIN_TOML_BYTES + 1], NATIVE_JSON).is_err());
        assert!(validate_skin(NATIVE_TOML, &vec![b' '; MAX_SKIN_JSON_BYTES + 1]).is_err());
    }

    fn changed(pointer: &str, value: Value) -> Result<ValidatedSkin> {
        let mut raw: Value = serde_json::from_slice(NATIVE_JSON).expect("fixture");
        *raw.pointer_mut(pointer).expect("field") = value;
        validate_skin(
            NATIVE_TOML,
            &serde_json::to_vec(&raw).expect("fixture JSON"),
        )
    }

    #[test]
    fn validates_safety_of_tokens_assets_and_ids() {
        for (pointer, value) in [
            ("/id", json!("workcolor")),
            ("/id", json!("a..b")),
            ("/slots/nav", json!("menu")),
            ("/shape/row-h", json!(2)),
            ("/colors/light/bg", json!("url(asset)")),
            ("/colors/light/bg", json!("#fff0")),
            ("/colors/light/fg", json!("#fff")),
            ("/elevation/dark/panel", json!("var(--custom)")),
            ("/type/heading", json!(["serif; cursor: none"])),
            ("/type/size/base", json!(-1)),
            ("/motion/ease", json!([2, 1, 0.2, 1])),
            ("/work/dark/barK", json!(200)),
            ("/fonts/0/src", json!("fonts/../private.woff2")),
            ("/fonts/0/src", json!("https://example.org/x.woff2")),
            ("/fonts/0/bytes", json!(MAX_ASSET_BYTES + 1)),
            ("/fonts/0/weight", json!("900 100")),
            ("/fonts/0/unicodeRange", json!("U+005B,url()")),
            ("/name", json!({"zh-CN":"<name>"})),
            ("/colors/dark", json!({})),
            ("/name", json!({"zh-CN":"name","other":"text"})),
        ] {
            assert!(changed(pointer, value).is_err(), "{pointer}");
        }
        assert!(changed("/colors/dark/chart-1", json!("oklch(0.5 0.15 300)")).is_ok());
    }
}

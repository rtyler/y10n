/**
 * The y10n module contains the bulk of the functionality that this library provides
 */
#[macro_use]
extern crate lazy_static;

use glob::glob;
use log::*;
use std::collections::HashMap;
use std::fs::File;

lazy_static! {
    static ref LANG_REGEX: regex::Regex =
        regex::Regex::new(r"(?P<code>\w+)-?(?P<region>\w+)?(;q=(?P<quality>([0-9]*[.])?[0-9]+)?)?")
            .unwrap();
}

/**
 * Y10n is a stateful struct that can be loaded with localization files
 */
pub struct Y10n {
    translations: HashMap<String, serde_yaml::Value>,
}

impl Y10n {
    fn new() -> Self {
        Self {
            translations: HashMap::default(),
        }
    }

    /**
     * Create and load a Y10n instance from the yml files in the given glob
     *
     * For example `"l10n/**/*.yml"` will load all the yml files in the `l10n` directory using each
     * file's name (e.g. `en.yml`) to derive it's language key (`en`).
     */
    fn from_glob(pattern: &str) -> Self {
        let mut this = Self::new();
        trace!(
            "Attempting to load translations from glob pattern: {:?}",
            pattern
        );

        for entry in glob(pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    trace!("Loading translations from: {}", path.display());

                    if let Some(stem) = path.file_stem() {
                        let key = stem.to_string_lossy();
                        // TODO: Make this error handling more robust
                        let value = serde_yaml::from_reader(
                            File::open(&path).expect("Failed to load file"),
                        )
                        .expect("Failed to deserialize YAML");

                        this.translations.insert(key.to_string(), value);
                    }
                }
                Err(e) => warn!("{:?}", e),
            }
        }
        this
    }

    /**
     * Return a Vec of all the names of languages that have been loaded
     * These are conventionally just the file stems of the yml files loaded
     */
    fn languages(&self) -> Vec<&String> {
        self.translations.keys().collect()
    }

    /**
     * Returns the merged serde_yaml::Value for the given sets of languages.
     *
     * THis function is useful for managing language fallbacks to account for partial translations.
     * FOr example if the German `de` translation file only has one string in it, but the English
     * `en` file has 10, then this function could be called with a Vec of `Language` instances of
     * `[de, en]` and the result would contain the one German string and 9 English strings.
     */
    fn localize(&self, languages: &[Language]) -> serde_yaml::Value {
        use serde_yaml::{Mapping, Value};

        let mut values = vec![];

        for lang in languages {
            if let Some(value) = self.translations.get(&lang.code) {
                values.push(value.clone());
            }
        }

        let mut map = Value::Mapping(Mapping::new());

        for value in values.into_iter().rev() {
            merge_yaml(&mut map, value);
        }
        map
    }
}

/**
 * Parse a string containing the value of an Accept-Language header
 *
 * For example: 'en,de;q=0.5`
 */
pub fn parse_accept_language(header: &str) -> Vec<Language> {
    trace!("Parsing languages from: {}", header);
    let mut results = vec![];

    for part in header.split(",") {
        if let Ok(language) = Language::from(part) {
            results.push(language);
        }
    }
    results
}

/**
 * Language
 */
#[derive(Clone, Debug)]
pub struct Language {
    pub code: String,
    region: Option<String>,
    quality: f64,
}

impl Language {
    /**
     * Create a `Language` instance from a segment of an `Accepts-Language` header
     *
     * For example `en` or `de;q=0.5`.
     */
    fn from(segment: &str) -> Result<Language, Error> {
        if let Some(captures) = LANG_REGEX.captures(segment) {
            Ok(Language {
                code: captures
                    .name("code")
                    .map_or("unknown".to_string(), |c| c.as_str().to_string()),
                region: captures
                    .name("region")
                    .map_or(None, |c| Some(c.as_str().to_string())),
                quality: captures
                    .name("quality")
                    .map_or(1.0, |c| c.as_str().parse().unwrap_or(0.0)),
            })
        } else {
            Err(Error::Generic)
        }
    }
}

#[derive(Clone, Debug)]
enum Error {
    Generic,
}

/**
 * Merge a couple of serde_yaml together
 *
 * THis code courtesy of https://stackoverflow.com/a/67743348
 */
fn merge_yaml(a: &mut serde_yaml::Value, b: serde_yaml::Value) {
    match (a, b) {
        (a @ &mut serde_yaml::Value::Mapping(_), serde_yaml::Value::Mapping(b)) => {
            let a = a.as_mapping_mut().unwrap();
            for (k, v) in b {
                if v.is_sequence() && a.contains_key(&k) && a[&k].is_sequence() {
                    let mut _b = a.get(&k).unwrap().as_sequence().unwrap().to_owned();
                    _b.append(&mut v.as_sequence().unwrap().to_owned());
                    a[&k] = serde_yaml::Value::from(_b);
                    continue;
                }
                if !a.contains_key(&k) {
                    a.insert(k.to_owned(), v.to_owned());
                } else {
                    merge_yaml(&mut a[&k], v);
                }
            }
        }
        (a, b) => *a = b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn y10n_from_valid_glob() {
        let y10n = Y10n::from_glob("l10n/*.yml");
        assert_eq!(y10n.languages().len(), 2);
    }

    #[test]
    fn y10n_localize() {
        use serde_yaml::Value;

        let y10n = Y10n::from_glob("l10n/*.yml");
        let en = Language::from("en").expect("Failed to parse!");
        let de = Language::from("de").expect("Failed to parse!");
        let value = y10n.localize(&[de, en]);
        if let Some(map) = value.as_mapping() {
            let key = "greeting".into();
            let greeting = map.get(&key).expect("Failed to find a greeting");
            assert_eq!(&Value::String("moin moin".to_string()), greeting);

            let secret = map.get(&"secret".into()).expect("Failed to find a secret");
            assert_eq!(&Value::String("pancakes".to_string()), secret);
        } else {
            assert!(false, "The value wasn't a map like I expected");
        }
    }

    #[test]
    fn language_from_segment() {
        let lang = Language::from("en-US");
        assert!(lang.is_ok());
        let lang = lang.unwrap();
        assert_eq!("en", lang.code);
        assert_eq!(Some("US".to_string()), lang.region);
        assert_eq!(1.0, lang.quality);
    }

    #[test]
    fn parse_langs_simple() {
        let header = "en-US,en;q=0.5";
        let langs = parse_accept_language(&header);
        assert_eq!(langs.len(), 2);
    }

    #[test]
    fn parse_langs_multi() {
        let header = "en-US,en;q=0.7,de-DE;q=0.3";
        let langs = parse_accept_language(&header);
        assert_eq!(langs.len(), 3);
        let de = langs.get(2).unwrap();
        assert_eq!("de", de.code);
        assert_eq!(0.3, de.quality);
    }
}

/**
 * The y10n module contains the bulk of the functionality that this library provides
 */
#[macro_use]
extern crate lazy_static;

use log::*;

lazy_static! {
    static ref LANG_REGEX: regex::Regex = regex::Regex::new(r"(?P<code>\w+)-?(?P<region>\w+)?(;q=(?P<quality>([0-9]*[.])?[0-9]+)?)?").unwrap();
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
    fn from(segment: &str) -> Result<Language, Error> {
        if let Some(captures) = LANG_REGEX.captures(segment) {
            println!("caps: {:?}", captures);
            Ok(Language {
                code: captures.name("code").map_or("unknown".to_string(), |c| c.as_str().to_string()),
                region: captures.name("region").map_or(None, |c| Some(c.as_str().to_string())),
                quality: captures.name("quality").map_or(1.0, |c| c.as_str().parse().unwrap_or(0.0)),
            })
        }
        else {
            Err(Error::Generic)
        }
    }
}

#[derive(Clone, Debug)]
enum Error {
    Generic,
}


#[cfg(test)]
mod tests {
    use super::*;

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


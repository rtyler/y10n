
use handlebars::{Context, Helper, HelperDef, HelperResult, Output, RenderContext};
/// The handlebars module has the optional Handlebars support for Y10n which can
/// be enabled with the `hb` feature
use log::*;
use std::collections::HashMap;

pub use crate::{Language, Y10n};
pub use handlebars::Handlebars;

///  This helper ensures that the `t` helper inside handlebars can be used
///  properly for resolving Y10n values
///
///  The way the helper should be used is with the first parameter being the path
///  to the right localization string, and they keyword arguments for any variables
///  that the localization string requires.
///
///  Suppose that the `en.yml` file contains:
///
///  ```yaml
///  ---
///  greeting: "Hello there {{who}}"
///  ```
///
///  And then the handlebars template of: `This is a string, {{t "greeting" who=user}}`
///
///  This will look up the `greeting` string and interpolate the current context's
///  `user` value into the `who placeholder.
///
///  The helper can be registered with:
///
///  ```rust
///  use y10n::handlebars::*;
///  let y10n = Y10n::from_glob("l10n/*.yml");
///  let languages: Vec<Language> = vec!["en".into()];
///  let mut hb = Handlebars::new();
///  hb.register_helper("t", Box::new(HandlebarsHelper::new(&y10n, languages)));
///  ```
#[derive(Clone, Debug)]
pub struct HandlebarsHelper<'a> {
    y10n: &'a Y10n,
    languages: Vec<Language>,
}

impl<'a> HandlebarsHelper<'a> {
    ///
    /// Instantiation of the HandlebarsHelper should come with a pre-existing
    /// Y10n struct and an array of preferred languages for rendering the localization
    /// strings in the Handlebars templates
    pub fn new(y10n: &'a Y10n, languages: Vec<Language>) -> Self {
        Self { y10n, languages }
    }
}

impl HelperDef for HandlebarsHelper<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        hb: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let param = h.param(0).unwrap().render();
        trace!("Looking up localization string: {}", param);

        if let Some(buf) = self.y10n.lookup(&param, &self.languages) {
            let mut data: HashMap<String, String> = HashMap::new();
            for (key, value) in h.hash() {
                data.insert(key.to_string(), value.render());
            }
            out.write(&hb.render_template(buf, &data)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handlebars_helper() {
        use handlebars::Handlebars;
        let y10n = crate::Y10n::from_glob("l10n/*.yml");
        let languages: Vec<crate::Language> = vec!["en".into()];
        let template = r#"Well that's it. {{t "thankyou" team=team}}"#;

        let mut hb = Handlebars::new();
        hb.register_helper("t", Box::new(HandlebarsHelper::new(&y10n, languages)));
        let mut data: HashMap<String, String> = HashMap::new();
        data.insert("team".into(), "Foo".into());
        let rendered = hb
            .render_template(template, &data)
            .expect("Failed to render");

        assert_eq!(rendered, "Well that's it. Thanks for playing Foo!");
    }
}

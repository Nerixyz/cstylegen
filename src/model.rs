use ahash::AHashMap;
use cssparser::{CowRcStr, RGBA};

use crate::combinator::combine_path;

#[derive(Debug, Clone)]
pub struct ChatterinoMeta<'i> {
    pub author: CowRcStr<'i>,
    pub icon_set: CowRcStr<'i>,
}

#[derive(Debug)]
pub enum RuleValue<'i> {
    ColorRef(CowRcStr<'i>),
    Color(cssparser::RGBA),
}

pub type RuleMap<'i> = AHashMap<CowRcStr<'i>, Rule<'i>>;

#[derive(Debug)]
pub enum Rule<'i> {
    Value(RuleValue<'i>),
    Nested(RuleMap<'i>),
}

#[derive(Debug)]
pub struct Theme<'i> {
    pub meta: ChatterinoMeta<'i>,
    pub colors: CustomColors<'i>,
    pub rules: RuleMap<'i>,
}

pub type CustomColors<'i> = AHashMap<CowRcStr<'i>, cssparser::RGBA>;

#[derive(Debug)]
pub struct FlatTheme<'i> {
    pub meta: ChatterinoMeta<'i>,
    pub rules: AHashMap<String, RGBA>,
}

#[derive(Debug, thiserror::Error)]
pub enum FlattenError<'i> {
    #[error("'{0}' was used in {1} but never defined anywhere.")]
    MissingColor(CowRcStr<'i>, String),
}

impl<'i> Theme<'i> {
    pub fn flatten(&self) -> Result<FlatTheme, FlattenError<'i>> {
        let mut flat = FlatTheme {
            meta: self.meta.clone(),
            rules: Default::default(),
        };
        inner_flatten(&mut flat.rules, "", &self.rules, &self.colors)?;
        Ok(flat)
    }
}

fn inner_flatten<'i>(
    map: &mut AHashMap<String, RGBA>,
    prefix: &str,
    rules: &RuleMap<'i>,
    colors: &CustomColors,
) -> Result<(), FlattenError<'i>> {
    for (name, rule) in rules {
        match rule {
            Rule::Value(value) => {
                let path = combine_path(prefix, name);
                let value = match value {
                    RuleValue::ColorRef(name) => {
                        let Some(color) = colors.get(name) else {
                            return Err(FlattenError::MissingColor(name.clone(), path));
                        };
                        *color
                    }
                    RuleValue::Color(c) => *c,
                };
                map.insert(path, value);
            }
            Rule::Nested(nested) => {
                inner_flatten(
                    map,
                    &combine_path(prefix, name),
                    nested,
                    colors,
                )?;
            }
        }
    }
    Ok(())
}

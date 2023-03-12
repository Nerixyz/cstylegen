use std::collections::hash_map;

use cssparser::{
    AtRuleParser, BasicParseError, Color, CowRcStr, DeclarationListParser,
    DeclarationParser, QualifiedRuleParser, RuleListParser,
    _cssparser_internal_to_lowercase, RGBA,
};
use tracing::warn;

use crate::model::{
    ChatterinoMeta, CustomColors, Rule, RuleMap, RuleValue, Theme,
};

#[derive(thiserror::Error, Debug)]
pub enum ParseError<'a> {
    #[error("Unexpected {0}")]
    UnexpectedMeta(CowRcStr<'a>),
    #[error("Missing '{0}' in meta")]
    MissingMetaItem(&'static str),
    #[error("'currentColor' isn't supported")]
    CurrentColorFound,
    #[error("Expected a color or var(..)")]
    ExpectedColorOrVar,
    #[error("Expected a @chatterino metadata block")]
    MissingMetaBlock,
    #[error("Found duplicate @chatterino metadata block")]
    DuplicateMetaBlock,
    #[error("Found duplicate :root block")]
    DuplicateRootBlock,
    #[error("Found duplicate block ('{0}')")]
    DuplicateBlock(CowRcStr<'a>),
}

type SingleRule<'i> = (CowRcStr<'i>, Rule<'i>);

enum TopLevelItem<'i> {
    Meta(ChatterinoMeta<'i>),
    Root(CustomColors<'i>),
    Regular(SingleRule<'i>),
}

struct RegularRuleParser;

impl<'i> DeclarationParser<'i> for RegularRuleParser {
    type Declaration = (CowRcStr<'i>, Rule<'i>);

    type Error = ParseError<'i>;

    fn parse_value<'t>(
        &mut self,
        name: cssparser::CowRcStr<'i>,
        p: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Declaration, cssparser::ParseError<'i, Self::Error>> {
        let var: Result<CowRcStr, cssparser::ParseError<ParseError<'i>>> = p
            .try_parse(|p| {
                p.expect_function_matching("var")?;
                p.parse_nested_block(|p| {
                    let name = p.expect_ident_cloned()?;
                    // TODO: support fallback
                    Ok(name)
                })
            });
        let value = match var {
            Ok(var) => Ok(RuleValue::ColorRef(var)),
            Err(_) => parse_color(p).map(RuleValue::Color),
        }?;

        Ok((name, Rule::Value(value)))
    }
}

impl<'i> AtRuleParser<'i> for RegularRuleParser {
    type Prelude = CowRcStr<'i>;
    type AtRule = (CowRcStr<'i>, Rule<'i>);
    type Error = ParseError<'i>;

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        if !name.eq_ignore_ascii_case("nest") {
            return Err(input.new_error(
                cssparser::BasicParseErrorKind::AtRuleInvalid(name),
            ));
        }

        input.skip_whitespace();
        let ident = input.expect_ident_cloned()?;
        Ok(ident)
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &cssparser::ParserState,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        let rules = DeclarationListParser::new(input, RegularRuleParser)
            .filter_map(warn_about_invalid)
            .collect();
        Ok((prelude, Rule::Nested(rules)))
    }
}

struct TopLevelParser;

enum QualifiedType<'i> {
    Root,
    Regular(CowRcStr<'i>),
}

impl<'i> QualifiedRuleParser<'i> for TopLevelParser {
    type Prelude = QualifiedType<'i>;

    type QualifiedRule = TopLevelItem<'i>;

    type Error = ParseError<'i>;

    fn parse_prelude<'t>(
        &mut self,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        let parse_root = |input: &mut cssparser::Parser<'i, 't>| {
            input.expect_colon()?;
            input.expect_ident_matching("root")?;
            Ok(())
        };
        if input.try_parse::<_, _, BasicParseError>(parse_root).is_ok() {
            return Ok(QualifiedType::Root);
        }

        let ident = input.expect_ident_cloned()?;
        Ok(QualifiedType::Regular(ident))
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &cssparser::ParserState,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, cssparser::ParseError<'i, Self::Error>>
    {
        match prelude {
            QualifiedType::Root => {
                let color_map =
                    DeclarationListParser::new(input, RootBlockParser)
                        .filter_map(warn_about_invalid)
                        .collect();
                Ok(TopLevelItem::Root(color_map))
            }
            QualifiedType::Regular(name) => {
                let rules =
                    DeclarationListParser::new(input, RegularRuleParser)
                        .filter_map(warn_about_invalid)
                        .collect();
                Ok(TopLevelItem::Regular((name, Rule::Nested(rules))))
            }
        }
    }
}

impl<'i> AtRuleParser<'i> for TopLevelParser {
    type Prelude = ();

    type AtRule = TopLevelItem<'i>;

    type Error = ParseError<'i>;

    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Prelude, cssparser::ParseError<'i, Self::Error>> {
        if !name.eq_ignore_ascii_case("chatterino") {
            return Err(input.new_error(
                cssparser::BasicParseErrorKind::AtRuleInvalid(name),
            ));
        }
        Ok(())
    }

    fn parse_block<'t>(
        &mut self,
        _prelude: Self::Prelude,
        _start: &cssparser::ParserState,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::AtRule, cssparser::ParseError<'i, Self::Error>> {
        let mut author = None;
        let mut icon_set = None;
        for item in DeclarationListParser::new(input, ChatterinoMetaParser)
            .filter_map(warn_about_invalid)
        {
            match item {
                ChatterinoMetaItem::Author(v) => author = Some(v),
                ChatterinoMetaItem::IconSet(v) => icon_set = Some(v),
            }
        }

        Ok(TopLevelItem::Meta(ChatterinoMeta {
            author: author.ok_or_else(|| {
                input.new_custom_error(ParseError::MissingMetaItem("author"))
            })?,
            icon_set: icon_set.ok_or_else(|| {
                input.new_custom_error(ParseError::MissingMetaItem("icon-set"))
            })?,
        }))
    }
}

struct RootBlockParser;
impl<'i> DeclarationParser<'i> for RootBlockParser {
    type Declaration = (CowRcStr<'i>, RGBA);

    type Error = ParseError<'i>;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Declaration, cssparser::ParseError<'i, Self::Error>> {
        Ok((name, parse_color(input)?))
    }
}

impl<'i> AtRuleParser<'i> for RootBlockParser {
    type Prelude = ();

    type AtRule = (CowRcStr<'i>, RGBA);

    type Error = ParseError<'i>;
}

struct ChatterinoMetaParser;
enum ChatterinoMetaItem<'i> {
    Author(CowRcStr<'i>),
    IconSet(CowRcStr<'i>),
}
impl<'i> DeclarationParser<'i> for ChatterinoMetaParser {
    type Declaration = ChatterinoMetaItem<'i>;

    type Error = ParseError<'i>;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        p: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self::Declaration, cssparser::ParseError<'i, Self::Error>> {
        cssparser::match_ignore_ascii_case! { &name,
            "author" => {
                Ok(ChatterinoMetaItem::Author(p.expect_string_cloned()?))
            },
            "icon-set" => {
                Ok(ChatterinoMetaItem::IconSet(p.expect_string_cloned()?))
            },
            _ => {
                Err(p.new_custom_error(ParseError::UnexpectedMeta(name)))
            }
        }
    }
}
impl<'i> AtRuleParser<'i> for ChatterinoMetaParser {
    type Prelude = ();
    type AtRule = ChatterinoMetaItem<'i>;
    type Error = ParseError<'i>;
}

fn parse_color<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<cssparser::RGBA, cssparser::ParseError<'i, ParseError<'i>>> {
    match Color::parse(input) {
        Ok(Color::RGBA(color)) => Ok(color),
        Ok(Color::CurrentColor) => {
            Err(input.new_custom_error(ParseError::CurrentColorFound))
        }
        Err(e) => {
            dbg!(e);
            Err(input.new_custom_error(ParseError::ExpectedColorOrVar))
        }
    }
}

fn warn_about_invalid<Rule, Error>(
    rule: Result<Rule, (cssparser::ParseError<Error>, &str)>,
) -> Option<Rule>
where
    Error: std::fmt::Debug,
{
    match rule {
        Ok(rule) => Some(rule),
        Err((error, source)) => {
            warn!(error = ?error, "Error parsing '{source}'");
            None
        }
    }
}

#[derive(Default)]
struct ThemeParserState<'i> {
    meta: Option<ChatterinoMeta<'i>>,
    colors: Option<CustomColors<'i>>,
    rules: RuleMap<'i>,
}

pub fn parse<'i>(
    input: &mut cssparser::Parser<'i, '_>,
) -> Result<Theme<'i>, cssparser::ParseError<'i, ParseError<'i>>> {
    let mut state = ThemeParserState::default();

    for item in RuleListParser::new_for_stylesheet(input, TopLevelParser)
        .filter_map(warn_about_invalid)
    {
        match item {
            TopLevelItem::Meta(meta) if state.meta.is_none() => {
                state.meta = Some(meta);
            }
            TopLevelItem::Meta(_) => {
                return Err(
                    input.new_custom_error(ParseError::DuplicateMetaBlock)
                );
            }
            TopLevelItem::Root(root) if state.colors.is_none() => {
                state.colors = Some(root);
            }
            TopLevelItem::Root(_) => {
                return Err(
                    input.new_custom_error(ParseError::DuplicateRootBlock)
                );
            }
            TopLevelItem::Regular((name, rule)) => {
                match state.rules.entry(name) {
                    hash_map::Entry::Vacant(e) => {
                        e.insert(rule);
                    }
                    hash_map::Entry::Occupied(e) => {
                        return Err(input.new_custom_error(
                            ParseError::DuplicateBlock(e.key().clone()),
                        ));
                    }
                }
            }
        };
    }

    Ok(Theme {
        meta: state.meta.ok_or_else(|| {
            input.new_custom_error(ParseError::MissingMetaBlock)
        })?,
        colors: state.colors.unwrap_or_default(),
        rules: state.rules,
    })
}

use std::collections::BTreeMap;

use ahash::AHashMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum YamlFields<'a> {
    #[serde(borrow)]
    Nested(BTreeMap<&'a str, Option<YamlStruct<'a>>>),
    #[serde(borrow)]
    Sequence(Vec<&'a str>),
}

#[derive(Debug, Deserialize)]
struct YamlStruct<'a> {
    fields: Option<YamlFields<'a>>,
    #[serde(borrow)]
    r#ref: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct YamlRootFile<'a> {
    #[serde(borrow)]
    definitions: AHashMap<&'a str, YamlStruct<'a>>,
    #[serde(borrow)]
    layout: AHashMap<&'a str, YamlStruct<'a>>,
}

pub enum LayoutItem<'a> {
    Ref {
        field_name: &'a str,
        referenced: &'a str,
        item_count: usize,
    },
    Field {
        name: &'a str,
    },
    Struct {
        field_name: &'a str,
        fields: Vec<LayoutItem<'a>>,
        item_count: usize,
    },
}

impl<'a> LayoutItem<'a> {
    pub fn item_count(&self) -> usize {
        match self {
            LayoutItem::Ref { item_count, .. } => *item_count,
            LayoutItem::Field { .. } => 1,
            LayoutItem::Struct { item_count, .. } => *item_count,
        }
    }
}

pub struct LayoutDefinition<'a> {
    pub fields: Vec<LayoutItem<'a>>,
    pub item_count: usize,
}

// we're using a BTreeMap here to keep the ouput sorted
// (avoids recompilations at the cost of speed)
pub struct Layout<'a> {
    pub definitions: BTreeMap<&'a str, LayoutDefinition<'a>>,
    pub items: BTreeMap<&'a str, Vec<LayoutItem<'a>>>,
}

pub enum FlatLayoutItem<'a> {
    Field {
        name: &'a str,
        id: usize,
    },
    Struct {
        name: &'a str,
        fields: Vec<FlatLayoutItem<'a>>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError<'a> {
    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_yaml::Error),
    #[error("Couldn't find definition for '{0}'")]
    RefNotFound(&'a str),
    #[error("Found struct with both 'ref' and 'fields' in {0}")]
    RefAndFields(&'a str),
    #[error("Definition of {0} isn't a struct")]
    DefinitionNotStruct(&'a str),
    #[error("Layout of {0} isn't a struct")]
    LayoutNotStruct(&'a str),
}

impl<'a> Layout<'a> {
    pub fn parse(source: &'a str) -> Result<Self, ParseError<'a>> {
        let yaml: YamlRootFile = serde_yaml::from_str(source)?;

        let mut layout = Self {
            definitions: Default::default(),
            items: Default::default(),
        };

        for (key, value) in yaml.definitions {
            let LayoutItem::Struct {fields, item_count, ..} =
                convert_struct(&layout, key, &value)? else {
                    return Err(ParseError::DefinitionNotStruct(key));
                };

            layout
                .definitions
                .insert(key, LayoutDefinition { fields, item_count });
        }

        for (key, value) in yaml.layout {
            let LayoutItem::Struct {fields, ..} =
                convert_struct(&layout, key, &value)? else {
                    return Err(ParseError::LayoutNotStruct(key));
                };

            layout.items.insert(key, fields);
        }

        Ok(layout)
    }

    pub fn count_items(&self) -> usize {
        self.items
            .values()
            .flat_map(|s| s.iter().map(|i| i.item_count()))
            .sum()
    }

    pub fn flatten(&self) -> Vec<FlatLayoutItem<'a>> {
        fn convert_items<'a>(
            item_id: &mut usize,
            layout: &Layout<'a>,
            name: &'a str,
            items: &[LayoutItem<'a>],
        ) -> FlatLayoutItem<'a> {
            let mut converted = vec![];
            for item in items {
                match item {
                    LayoutItem::Ref {
                        field_name,
                        referenced,
                        ..
                    } => {
                        let Some(referenced) = layout.definitions.get(referenced) else {
                            panic!("referenced struct not found ({referenced})");
                        };
                        converted.push(convert_items(
                            item_id,
                            layout,
                            field_name,
                            &referenced.fields,
                        ))
                    }
                    LayoutItem::Field { name } => {
                        converted
                            .push(FlatLayoutItem::Field { name, id: *item_id });
                        *item_id += 1;
                    }
                    LayoutItem::Struct {
                        field_name, fields, ..
                    } => {
                        converted.push(convert_items(
                            item_id, layout, field_name, fields,
                        ));
                    }
                }
            }
            FlatLayoutItem::Struct {
                name,
                fields: converted,
            }
        }

        let mut item_id = 0;
        let mut items = vec![];
        for (name, s) in self.items.iter() {
            items.push(convert_items(&mut item_id, self, name, s));
        }

        items
    }
}

fn convert_struct<'a>(
    current: &Layout<'a>,
    name: &'a str,
    s: &YamlStruct<'a>,
) -> Result<LayoutItem<'a>, ParseError<'a>> {
    match (&s.r#ref, &s.fields) {
        (Some(r), None) => {
            let Some(d) = current.definitions.get(r) else {
                return Err(ParseError::RefNotFound(r));
            };
            Ok(LayoutItem::Ref {
                field_name: name,
                item_count: d.item_count,
                referenced: r,
            })
        }
        (None, Some(fields)) => {
            let mut items = Vec::new();
            let mut item_count = 0;
            match fields {
                YamlFields::Nested(n) => {
                    for (name, inner) in n {
                        match inner {
                            Some(ref inner) => {
                                let converted =
                                    convert_struct(current, name, inner)?;
                                item_count += converted.item_count();
                                items.push(converted);
                            }
                            None => {
                                items.push(LayoutItem::Field { name });
                                item_count += 1;
                            }
                        }
                    }
                }
                YamlFields::Sequence(s) => {
                    for name in s {
                        items.push(LayoutItem::Field { name });
                    }
                    item_count += s.len();
                }
            }

            Ok(LayoutItem::Struct {
                field_name: name,
                fields: items,
                item_count,
            })
        }
        _ => Err(ParseError::RefAndFields(name)),
    }
}

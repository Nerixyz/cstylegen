use std::io;

use crate::{
    combinator::combine_path,
    helper::Fork,
    layout::{FlatLayoutItem, Layout},
    model::FlatTheme,
};

use super::{key_matcher, Printer};

pub fn generate_impl(
    p: &mut Printer<impl io::Write>,
    layout: &Layout,
    theme: &FlatTheme,
) -> io::Result<()> {
    // TODO: should this be a template?
    p.write_line("#include \"GeneratedTheme.hpp\"")?;
    p.write_line("#include <QColor>")?;
    p.write_line("#include <QString>")?;
    p.write_line("#include <cstring>")?;
    p.write_line("")?;

    p.write_line("namespace {")?;
    p.indent();
    p.write_line("int getDataIndex(const QLatin1String &name);")?;
    p.dedent();
    p.write_line("} //  namespace")?;

    p.write_line("namespace chatterino::theme {")?;

    p.write_line("GeneratedTheme::GeneratedTheme() {")?;
    p.indent();

    p.write_line("this->reset();")?;
    p.write_line("this->applyChanges();")?;

    p.dedent();
    p.write_line("}")?;

    p.write_line("void GeneratedTheme::applyChanges() {")?;
    p.indent();
    p.write_line("const auto d = [this](size_t i) -> const QColor& { return this->colors_[i]; };")?;

    let flattened_layout = layout.flatten();
    for item in flattened_layout.iter() {
        let FlatLayoutItem::Struct { name, fields } = item else {
            panic!("Top level item not struct");
        };

        writeln!(p, "this->{name} = {{")?;
        p.indent();
        for field in fields {
            print_field(p, field)?;
        }
        p.dedent();
        writeln!(p, "}};")?;
    }
    p.write_line("this->reset();")?;

    p.dedent();
    p.write_line("}")?;

    p.write_line("void GeneratedTheme::reset() {")?;
    p.indent();

    let mut paths = vec![];
    for item in flattened_layout.iter() {
        let FlatLayoutItem::Struct { name, fields } = item else {
            panic!("Top level item not struct");
        };
        for field in fields {
            reset_field(p, &mut paths, name, theme, field)?;
        }
    }

    let mut fork = Fork::new();
    for (key, value) in paths.iter() {
        fork.insert(key.as_bytes(), *value);
    }

    p.dedent();
    p.write_line("}")?;

    p.write_line("bool GeneratedTheme::setColor(const QLatin1String &name, QColor color) {")?;
    p.indent();

    p.write_line("auto idx = getDataIndex(name);")?;
    p.write_line("if (idx < 0) return false;")?;
    p.write_line("this->colors_[idx] = color;")?;
    p.write_line("return true;")?;

    p.dedent();
    p.write_line("}")?;

    p.write_line("} //  namespace chatterino::theme")?;

    p.write_line("namespace {")?;
    key_matcher::print_key_matcher(p, &fork)?;
    p.write_line("} //  namespace")?;

    Ok(())
}

fn print_field(
    p: &mut Printer<impl io::Write>,
    item: &FlatLayoutItem,
) -> io::Result<()> {
    match item {
        FlatLayoutItem::Field { id, .. } => writeln!(p, "d({id}),"),
        FlatLayoutItem::Struct { fields, .. } => {
            writeln!(p, "{{")?;
            p.indent();
            for field in fields {
                print_field(p, field)?;
            }
            p.dedent();
            writeln!(p, "}},")
        }
    }
}

fn reset_field(
    p: &mut Printer<impl io::Write>,
    paths: &mut Vec<(String, usize)>,
    prefix: &str,
    theme: &FlatTheme,
    item: &FlatLayoutItem,
) -> io::Result<()> {
    match item {
        FlatLayoutItem::Field { id, name } => {
            let path = combine_path(prefix, name);
            let Some(color) =  theme.rules.get(&path) else {
                panic!("no rule for: {path}");
            };
            writeln!(
                p,
                "this->colors_[{id}] = {{{}, {}, {}, {}}};",
                color.red, color.green, color.blue, color.alpha
            )?;
            paths.push((path, *id));
        }
        FlatLayoutItem::Struct { name, fields } => {
            let prefix = combine_path(prefix, name);
            for field in fields {
                reset_field(p, paths, &prefix, theme, field)?;
            }
        }
    }
    Ok(())
}

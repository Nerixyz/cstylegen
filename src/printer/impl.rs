use std::io;

use crate::{
    combinator::combine_path,
    layout::{FlatLayoutItem, Layout},
    model::FlatTheme,
};

use super::Printer;

pub fn generate_impl(
    p: &mut Printer<impl io::Write>,
    layout: &Layout,
    theme: &FlatTheme,
) -> io::Result<()> {
    // TODO: should this be a template?
    p.write_line("#include \"GeneratedTheme.hpp\"")?;
    p.write_line("#include <QColor>")?;
    p.write_line("#include <QString>")?;
    p.write_line("#include <QByteArray>")?;
    p.write_line("#include <QMap>")?;
    p.write_line("#include <cstring>")?;
    p.write_line("")?;

    p.write_line("namespace {")?;
    p.indent();
    p.write_line("int getDataIndex(const QByteArray &name);")?;
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

    p.dedent();
    p.write_line("}")?;

    p.write_line(
        "bool GeneratedTheme::setColor(const QByteArray &name, QColor color) {",
    )?;
    p.indent();

    p.write_line("auto idx = getDataIndex(name);")?;
    p.write_line("if (idx < 0) return false;")?;
    p.write_line("this->colors_[idx] = color;")?;
    p.write_line("return true;")?;

    p.dedent();
    p.write_line("}")?;

    p.write_line("} //  namespace chatterino::theme")?;

    p.write_line("namespace {")?;
    p.write_line("int getDataIndex(const QByteArray &name) {")?;
    p.indent();
    p.write_line("static const QMap<QByteArray, size_t> dataMap = {")?;
    p.indent();
    for (path, value) in paths {
        writeln!(p, "{{\"{path}\", {value}}},")?;
    }
    p.dedent();
    p.write_line("};")?;
    p.write_line("return dataMap.value(name, -1);")?;
    p.dedent();
    p.write_line("}")?;
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

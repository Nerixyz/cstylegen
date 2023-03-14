use std::io;

use crate::layout::{Layout, LayoutItem};

use super::Printer;

pub fn generate_header(
    p: &mut Printer<impl io::Write>,
    layout: &Layout,
) -> io::Result<()> {
    p.write_line("#include <QColor>")?;
    p.write_line("#include <QByteArray>")?;
    p.write_line("")?;

    p.write_line("namespace chatterino::theme {")?;

    p.write_line("class GeneratedTheme {")?;
    p.write_line("public:")?;
    p.indent();

    for (name, def) in layout.definitions.iter() {
        writeln!(p, "struct {name} {{")?;
        p.indent();
        for item in def.fields.iter() {
            write_struct_field(p, item)?;
        }
        p.dedent();
        writeln!(p, "}};")?;
    }

    for (name, fields) in layout.items.iter() {
        write_struct(p, name, fields)?;
    }

    writeln!(p, "GeneratedTheme();")?;
    p.dedent();
    writeln!(p)?;
    writeln!(p, "protected:")?;
    p.indent();
    writeln!(p, "bool setColor(const QByteArray &name, QColor color);")?;
    writeln!(p, "void reset();")?;
    writeln!(p, "void applyChanges();")?;
    p.dedent();
    writeln!(p)?;
    writeln!(p, "private:")?;
    p.indent();
    writeln!(p, "QColor colors_[{}];", layout.count_items())?;
    p.dedent();

    p.write_line("};")?;
    p.write_line("}  // namespace chatterino::theme")?;

    Ok(())
}

fn write_struct_field(
    p: &mut Printer<impl io::Write>,
    field: &LayoutItem,
) -> io::Result<()> {
    match field {
        LayoutItem::Ref {
            field_name,
            referenced,
            ..
        } => {
            writeln!(p, "{referenced} {field_name};")
        }
        LayoutItem::Field { name } => {
            writeln!(p, "QColor {name};")
        }
        LayoutItem::Struct {
            field_name, fields, ..
        } => write_struct(p, field_name, fields),
    }
}

fn write_struct(
    p: &mut Printer<impl io::Write>,
    struct_name: &str,
    fields: &[LayoutItem],
) -> io::Result<()> {
    writeln!(p)?;
    writeln!(p, "struct {{")?;
    p.indent();
    for item in fields {
        write_struct_field(p, item)?;
    }
    p.dedent();
    writeln!(p, "}} {struct_name};")?;
    Ok(())
}

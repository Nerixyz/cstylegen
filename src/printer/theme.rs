use std::io;

use crate::model::FlatTheme;

use super::Printer;

pub fn generate(
    p: &mut Printer<impl io::Write>,
    theme: &FlatTheme,
) -> io::Result<()> {
    p.write_line("@meta")?;
    writeln!(p, "author={}", theme.meta.author)?;
    writeln!(p, "iconset={}", theme.meta.icon_set)?;
    p.write_line("@colors")?;
    for (color, value) in theme.rules.iter() {
        writeln!(
            p,
            "{color}=#{:02x}{:02x}{:02x}{:02x}",
            value.alpha, value.red, value.green, value.blue,
        )?;
    }
    Ok(())
}

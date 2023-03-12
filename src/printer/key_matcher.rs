use std::io;

use crate::helper::Fork;

use super::Printer;

pub fn print_key_matcher(
    p: &mut Printer<impl io::Write>,
    f: &Fork<usize>,
) -> io::Result<()> {
    p.write_line("int getDataIndex(const QLatin1String &name) {")?;
    p.indent();
    p.write_line("auto size = name.size();")?;
    p.write_line("auto data = name.data();")?;

    print_fork(p, f, 0, 0)?;

    p.write_line("return -1;")?;

    p.dedent();
    p.write_line("}")?;
    Ok(())
}

fn print_fork(
    p: &mut Printer<impl io::Write>,
    f: &Fork<usize>,
    position: usize,
    mut known_length: usize,
) -> io::Result<()> {
    let length_check = known_length < f.min_size();
    if length_check {
        writeln!(p, "if (size >= {}) {{", f.min_size())?;
        known_length = f.min_size();
        p.indent();
    }

    match f {
        Fork::Run {
            prefix, remaining, ..
        } => {
            writeln!(
                p,
                "if (std::memcmp(data + {position}, \"{}\", {}) == 0) {{",
                std::str::from_utf8(prefix).expect("valid utf8"),
                prefix.len()
            )?;
            p.indent();

            print_fork(p, remaining, position + prefix.len(), known_length)?;

            p.dedent();
            p.write_line("}")?;
        }
        Fork::Chaotic { items, .. } => {
            writeln!(p, "switch (data[{position}]) {{",)?;

            for (case, subfork) in items {
                writeln!(
                    p,
                    "case '{}': {{",
                    char::from_u32(*case as u32).expect("valid utf8")
                )?;
                p.indent();
                print_fork(p, subfork, position + 1, known_length)?;
                p.dedent();
                p.write_line("}")?;
                p.write_line("break;")?;
            }

            p.write_line("}")?;
        }
        Fork::Empty {
            total_prefix,
            value,
            remaining,
        } => {
            writeln!(
                p,
                "if (size == {}) return {};",
                total_prefix.len(),
                value
            )?;
            print_fork(p, remaining, position, known_length)?;
        }
        Fork::End {
            prefix,
            total_prefix,
            value,
        } => match prefix.len() {
            0 => {
                writeln!(
                    p,
                    "if (size == {}) return {};",
                    total_prefix.len(),
                    *value
                )?;
            }
            1 => {
                writeln!(
                    p,
                    "if (size == {} && data[{}] == '{}') return {};",
                    total_prefix.len(),
                    position,
                    char::from_u32(prefix[0] as u32).expect("valid utf8"),
                    *value
                )?;
            }
            _ => {
                writeln!(
                    p,
                    "if (size == {} && std::memcmp(data + {position}, \"{}\", {}) == 0) return {};",
                    total_prefix.len(),
                    std::str::from_utf8(prefix).expect("valid utf8"),
                    prefix.len(),
                    *value
                )?;
            }
        },
        Fork::None => (),
    }

    if length_check {
        p.dedent();
        p.write_line("}")?;
    }
    Ok(())
}

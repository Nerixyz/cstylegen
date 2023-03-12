mod combinator;
mod errors;
mod helper;
mod layout;
mod model;
mod parse;
mod printer;

use std::{
    ffi::{OsStr, OsString},
    fs::{self},
    path::{Path, PathBuf},
};

use clap::Parser;
use cssparser::ParserInput;
use printer::{header::generate_header, r#impl::generate_impl, Printer};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
enum Args {
    /// Generate code to manage a theme.
    Code {
        #[clap(short, default_value = "layout.yml")]
        /// Path to a layout.yml file that contains the theme layout.
        layout: OsString,
        /// The default style that gets loaded when the theme is initially loaded (or when reset() is called).
        default_style: OsString,
        #[clap(short, default_value = ".")]
        /// Output directory for all generated files.
        output_dir: OsString,
        #[clap(short, default_value_t = false)]
        /// Whether to generate an additional 'GeneratedTheme.timestamp' file.
        timestamp: bool,
    },
    /// Generates a 'c2theme' from a style-sheet.
    Theme {
        /// Path to an input style-sheet, for example Dark.css.
        input: OsString,
        #[clap(short, default_value = ".")]
        /// Output directory for all generated files.
        output_dir: OsString,
        #[clap(short, default_value_t = false)]
        /// Whether to generate an additional .timestamp file.
        timestamp: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args {
        Args::Code {
            layout,
            default_style,
            output_dir,
            timestamp,
        } => generate_code(&layout, &default_style, &output_dir, timestamp),
        Args::Theme {
            input,
            output_dir,
            timestamp,
        } => generate_theme(&input, &output_dir, timestamp),
    }
}

fn generate_theme(
    input_file: &OsStr,
    output_dir: &OsStr,
    timestamp: bool,
) -> anyhow::Result<()> {
    let input = fs::read_to_string(input_file)?;
    let mut parser_input = ParserInput::new(&input);
    let mut parser = cssparser::Parser::new(&mut parser_input);

    let parsed = match parse::parse(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            errors::print_error_with_source(
                input_file,
                &input,
                &errors::format_css_parse_error(&e),
                &e.location,
            );
            std::process::exit(1)
        }
    };
    let flat = match parsed.flatten() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to resolve values: {e}");
            std::process::exit(1)
        }
    };

    let mut output_path = PathBuf::from(output_dir);
    match Path::new(input_file).file_stem() {
        Some(s) => output_path.push(s),
        None => output_path.push("ChatterinoTheme"),
    }
    output_path.set_extension("c2theme");

    let mut imp = std::fs::File::create(&output_path)?;
    let mut printer = Printer::new(&mut imp);
    printer::theme::generate(&mut printer, &flat)?;

    if timestamp {
        generate_timestamp(&mut output_path)?;
    }

    Ok(())
}

fn generate_code(
    layout: &OsStr,
    default_style: &OsStr,
    output_dir: &OsString,
    timestamp: bool,
) -> anyhow::Result<()> {
    let layout = fs::read_to_string(layout)?;
    let default_style = fs::read_to_string(default_style)?;
    let mut parser_input = ParserInput::new(&default_style);
    let mut parser = cssparser::Parser::new(&mut parser_input);

    let parsed = parse::parse(&mut parser).unwrap();
    let flat = parsed.flatten().unwrap();

    let mut output_path = PathBuf::from(output_dir);
    output_path.push("GeneratedTheme");

    output_path.set_extension("cpp");
    let mut imp = std::fs::File::create(&output_path)?;
    let mut printer = Printer::new(&mut imp);
    let layout = layout::Layout::parse(&layout).unwrap();
    generate_impl(&mut printer, &layout, &flat)?;

    output_path.set_extension("hpp");
    let mut header = std::fs::File::create(&output_path)?;
    let mut printer = Printer::new(&mut header);
    generate_header(&mut printer, &layout)?;

    if timestamp {
        generate_timestamp(&mut output_path)?;
    }

    Ok(())
}

fn generate_timestamp(output_file: &mut PathBuf) -> anyhow::Result<()> {
    output_file.set_extension("timestamp");
    std::fs::File::create(output_file)?;
    Ok(())
}

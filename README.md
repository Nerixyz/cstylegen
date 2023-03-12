# cstylegen

A CLI to generate `c2theme` and C++ files.

There are two modes - `code` and `theme`. `code` generates a `GeneratedHeader.{cpp,hpp}` that stores theme data.
`theme` generates a `c2theme` from CSS.

## Installing

You can download prebuilt binaries [from the releases](https://github.com/Nerixyz/cstylegen/releases).

### Manual Build

- Clone the repo.
- Run `cargo instal --path .`.

## `code`

```text
Usage: cstylegen code [OPTIONS] <DEFAULT_STYLE>

Arguments:
  <DEFAULT_STYLE>  The default style that gets loaded when the theme is initially loaded (or when reset() is called)

Options:
  -l <LAYOUT>          Path to a layout.yml file that contains the theme layout [default: layout.yml]
  -o <OUTPUT_DIR>      Output directory for all generated files [default: .]
  -t                   Whether to generate an additional 'GeneratedTheme.timestamp' file
```

## `theme`

```text
Usage: cstylegen theme [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Path to an input style-sheet, for example Dark.css

Options:
  -o <OUTPUT_DIR>      Output directory for all generated files [default: .]
  -t                   Whether to generate an additional .timestamp file
```

## Style-Sheets

The CSS files read by this CLI have some restrictions.

- `var` is supported, but only variables created in `:root` are accepted. Furthermore, the variables have to be colors (something like `rgba(var(--my-color), 10%))` isn't possible).
- Since [`cssparser`](https://github.com/servo/rust-cssparser) doesn't yet support the [CSS nesting spec](https://www.w3.org/TR/css-nesting-1/), nesting is achieved through `@nest <name> { .. }`.

## `layout.yml`

This file contains the layout of the generated structs inside `GeneratedTheme.hpp`.

In the top-level `definitions`, you can define named structs.

The actual layout (structs and fields) are defined under the top-level `layout`. Structs can be nested and contain references to definitions:

### Example

The following `layout.yml` will generate the following structs:

```yaml
# layout.yml
definitions:
  TabColors:
    fields:
      text:
      backgrounds:
        fields:
          - regular
          - hover

layout:
  colors:
    fields:
      - accentColor
  tabs:
    fields:
      border:
      dividerLine:
      regular:
        ref: TabColors
      newMessage:
        ref: TabColors
```

```cpp
// GeneratedTheme.hpp
// ...
class GeneratedTheme {
public:
    struct TabColors {
        QColor text;
        struct {
            QColor regular;
            QColor hover;
        } backgrounds;
    };

    struct {
        QColor accentColor;
    } colors;

    struct {
        QColor border;
        QColor dividerLine;
        TabColors regular;
        TabColors newMessage;
    } tabs;
    // ...
};
```

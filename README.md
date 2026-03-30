# rushdown-definition-list
rushdown-definition-list is an extension for the rushdown that allows you to use definition lists in your markdown documents. 

## Installation
Add dependency to your `Cargo.toml`:

```toml
[dependencies]
rushdown-definition-list = "x.y.z"
```

rushdown-definition-list can also be used in `no_std` environments. To enable this feature, add the following line to your `Cargo.toml`:

```toml
rushdown-definition-list = { version = "x.y.z", default-features = false, features = ["no-std"] }
```

## Syntax

Please refer to the [Definition List](https://michelf.ca/projects/php-markdown/extra/#def-list:~:text=Destroy%20your%20computer!**%20%20%20%20%20%7C-,Definition%20Lists,-Markdown%20Extra%20implements) section of the PHP Markdown Extra documentation for the syntax of definition lists.

```markdown
Apple
:   Pomaceous fruit of plants of the genus Malus in 
    the family Rosaceae.

Orange
:   The fruit of an evergreen tree of the genus Citrus.
```

## Usage
### Example

```rust
use core::fmt::Write;
use rushdown::{
    new_markdown_to_html,
    parser::{self, ParserExtension},
    renderer::html::{self, RendererExtension},
    Result,
};
use rushdown_definition_list::{
    definition_list_html_renderer_extension, definition_list_parser_extension, 
};

let markdown_to_html = new_markdown_to_html(
    parser::Options::default(),
    html::Options::default(),
    definition_list_parser_extension(),
    definition_list_html_renderer_extension(),
);
let mut output = String::new();
let input = r#"
Apple
:   Pomaceous fruit of plants of the genus Malus in 
    the family Rosaceae.

Orange
:   The fruit of an evergreen tree of the genus Citrus.
"#;
match markdown_to_html(&mut output, input) {
    Ok(_) => {
        println!("HTML output:\n{}", output);
    }
    Err(e) => {
        println!("Error: {:?}", e);
    }
}
```

`definition_list_html_renderer_extension` overrides a paragraph renderer.
If you want to use custom paragraph renderer, you can override it after calling `definition_list_html_renderer_extension`:

```rust
use core::fmt::Write;
use rushdown::{
    new_markdown_to_html,
    parser::{self, ParserExtension},
    renderer::html::{self, RendererExtension},
    Result,
};
use rushdown_definition_list::{
    definition_list_html_renderer_extension, definition_list_parser_extension, is_in_tight_list,
};

let markdown_to_html = new_markdown_to_html(
    parser::Options::default(),
    html::Options::default(),
    definition_list_parser_extension(),
    definition_list_html_renderer_extension()
      .and(html::paragraph_renderer(html::ParagraphRendererOptions {
        is_in_tight_block: Some(is_in_tight_list), // must use `is_in_tight_list` to avoid wrapping paragraphs in tight definition lists with <p> tags.
        ..Default::default() // you can set other options.
      }))

);
let mut output = String::new();
let input = r#"
Apple
:   Pomaceous fruit of plants of the genus Malus in 
    the family Rosaceae.

Orange
:   The fruit of an evergreen tree of the genus Citrus.
"#;
match markdown_to_html(&mut output, input) {
    Ok(_) => {
        println!("HTML output:\n{}", output);
    }
    Err(e) => {
        println!("Error: {:?}", e);
    }
}
```

## Donation
BTC: 1NEDSyUmo4SMTDP83JJQSWi1MvQUGGNMZB

Github sponsors also welcome.

## License
MIT

## Author
Yusuke Inuzuka

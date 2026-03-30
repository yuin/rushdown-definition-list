use std::{fs, path::PathBuf};

use rushdown::{new_markdown_to_html, parser, renderer::html, test::MarkdownTestSuite};
use rushdown_definition_list::{
    definition_list_html_renderer_extension, definition_list_parser_extension,
};

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn test_definition_list() {
    let path = data_path("definition_list.txt");
    let s = fs::read_to_string(&path).expect("failed to read definition_list.txt");
    let suite = MarkdownTestSuite::with_str(s.as_str()).unwrap();
    let markdown_to_html = new_markdown_to_html(
        parser::Options::default(),
        html::Options {
            allows_unsafe: true,
            xhtml: false,
            ..html::Options::default()
        },
        definition_list_parser_extension(),
        definition_list_html_renderer_extension(),
    );
    suite.execute(&markdown_to_html)
}

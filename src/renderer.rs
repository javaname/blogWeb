use ammonia::Builder;
use pulldown_cmark::{html, Options, Parser};

use crate::error::Result;

pub fn render_safe_html(markdown: &str) -> Result<(String, String)> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut rendered = String::new();
    html::push_html(&mut rendered, parser);
    let safe = Builder::default()
        .link_rel(None)
        .add_tags(["table", "thead", "tbody", "tr", "th", "td"])
        .clean(&rendered)
        .to_string();
    Ok((safe, build_excerpt(markdown, 200)))
}

fn build_excerpt(content: &str, limit: usize) -> String {
    let mut plain = String::with_capacity(content.len());
    for ch in content.trim().chars() {
        match ch {
            '#' | '*' | '`' | '>' | '[' | ']' | '(' | ')' | '\n' | '\r' => plain.push(' '),
            _ => plain.push(ch),
        }
    }
    let collapsed = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = String::new();
    for (index, ch) in collapsed.chars().enumerate() {
        if index >= limit {
            result.push_str("...");
            return result;
        }
        result.push(ch);
    }
    result
}

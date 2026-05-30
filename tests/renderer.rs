use blogweb::renderer;

#[test]
fn render_safe_html_removes_script_and_builds_go_compatible_excerpt() {
    let (html, excerpt) =
        renderer::render_safe_html("# Baseline\n\n<script>alert(1)</script>\n\nStable text.")
            .unwrap();

    assert_eq!(html, "<h1>Baseline</h1>\n\n<p>Stable text.</p>\n");
    assert_eq!(excerpt, "Baseline <script alert 1 </script Stable text.");
}

#[test]
fn render_safe_html_keeps_gfm_table_and_safe_link() {
    let (html, excerpt) = renderer::render_safe_html(
        "| A | B |\n| - | - |\n| [site](https://example.com) | `code` |\n",
    )
    .unwrap();

    assert!(html.contains("<table>"), "{html}");
    assert!(html.contains("<a href=\"https://example.com\""), "{html}");
    assert!(html.contains("<code>code</code>"), "{html}");
    assert!(excerpt.contains("| A | B | | - | - | | site https://example.com | code |"));
}

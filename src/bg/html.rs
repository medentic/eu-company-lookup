use scraper::{ElementRef, Html, Node};

/// Decode HTML with a real parser. Inline nodes remain adjacent; block and br
/// boundaries become newlines. No script, style or hidden element text is exposed.
pub(super) fn text(html: &str) -> String {
    element_text(Html::parse_fragment(html).root_element())
}

pub(super) fn element_text(element: ElementRef<'_>) -> String {
    fn walk(el: ElementRef<'_>, out: &mut String, depth: usize) {
        if depth > 64 {
            return;
        }
        let name = el.value().name();
        let style = el
            .value()
            .attr("style")
            .unwrap_or("")
            .replace(' ', "")
            .to_lowercase();
        if matches!(name, "script" | "style" | "noscript")
            || el.value().attr("hidden").is_some()
            || style.contains("display:none")
            || style.contains("visibility:hidden")
        {
            return;
        }
        let block = matches!(
            name,
            "div" | "p" | "br" | "tr" | "td" | "th" | "li" | "h1" | "h2"
        );
        if block {
            out.push('\n');
        }
        for child in el.children() {
            match child.value() {
                Node::Text(t) => out.push_str(t),
                Node::Element(_) => walk(ElementRef::wrap(child).unwrap(), out, depth + 1),
                _ => (),
            }
        }
        if block {
            out.push('\n');
        }
    }
    let mut value = String::new();
    walk(element, &mut value, 0);
    value
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

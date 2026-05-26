use pulldown_cmark::{Options, Parser, html as cmark_html};

pub struct Rendered {
    pub html: String,
    pub excerpt: String,
}

pub fn render(md: &str) -> Rendered {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(md, opts);
    let mut raw = String::with_capacity(md.len() * 2);
    cmark_html::push_html(&mut raw, parser);

    let safe = sanitizer().clean(&raw).to_string();
    let excerpt = make_excerpt(&safe, 160);

    Rendered {
        html: safe,
        excerpt,
    }
}

fn sanitizer() -> ammonia::Builder<'static> {
    let mut b = ammonia::Builder::default();
    b.link_rel(Some("noopener noreferrer nofollow"));
    b.url_relative(ammonia::UrlRelative::Deny);
    b
}

fn make_excerpt(html: &str, max_chars: usize) -> String {
    let mut buf = String::with_capacity(max_chars * 4 + 8);
    let mut count = 0usize;
    let mut in_tag = false;
    let mut prev_space = false;
    for ch in html.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch.is_whitespace() {
            if !prev_space && count > 0 {
                buf.push(' ');
                count += 1;
                prev_space = true;
                if count >= max_chars {
                    buf.push('…');
                    break;
                }
            }
            continue;
        }
        prev_space = false;
        buf.push(ch);
        count += 1;
        if count >= max_chars {
            buf.push('…');
            break;
        }
    }
    buf.trim().to_string()
}

pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = true;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "note".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_and_lists() {
        let r = render("# Hi\n\n- one\n- two\n");
        assert!(r.html.contains("<h1>Hi</h1>"));
        assert!(r.html.contains("<li>one</li>"));
    }

    #[test]
    fn strips_script_tags() {
        let r = render("<script>alert('xss')</script>hello");
        assert!(!r.html.contains("<script"));
        assert!(!r.html.to_lowercase().contains("alert"));
        assert!(r.html.contains("hello"));
    }

    #[test]
    fn strips_event_handlers() {
        let r = render("<a href=\"/x\" onclick=\"steal()\">x</a>");
        assert!(!r.html.contains("onclick"));
    }

    #[test]
    fn strips_javascript_urls() {
        let r = render("[bad](javascript:alert(1))");
        assert!(!r.html.contains("javascript:"));
    }

    #[test]
    fn adds_noopener_to_links() {
        let r = render("<a href=\"https://example.com\">x</a>");
        assert!(r.html.contains("rel=\"noopener noreferrer nofollow\""));
    }

    #[test]
    fn excerpt_strips_tags() {
        let r = render("# Title\n\nHello **world** with [link](https://x.test).");
        assert!(!r.excerpt.contains('<'));
        assert!(r.excerpt.contains("Hello world"));
    }

    #[test]
    fn excerpt_truncates() {
        let long_md = "a ".repeat(500);
        let r = render(&long_md);
        assert!(r.excerpt.chars().count() <= 161);
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    #[test]
    fn slugify_collapses_dashes() {
        assert_eq!(slugify("--Foo   Bar--"), "foo-bar");
    }

    #[test]
    fn slugify_fallback_when_empty() {
        assert_eq!(slugify("!!!"), "note");
    }
}

//! Unit and integration tests.

#[cfg(test)]
mod tree_tests {
    use crate::tree::Tree;

    #[test]
    fn seed_pop() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 2;
        t.push(None, "http://a/".into());
        let m = t.pop().unwrap();
        assert_eq!(m.get(&None), Some(&"http://a/".to_string()));
    }

    #[test]
    fn depth1_child() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 2;
        t.push(None, "http://a/".into());
        t.pop(); // consume seed
        t.push(Some("http://a/".into()), "http://a/b".into());
        let m = t.pop().unwrap();
        assert_eq!(m.get(&Some("http://a/".into())), Some(&"http://a/b".into()));
    }

    #[test]
    fn depth0_blocks_children() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 0;
        t.push(None, "http://a/".into());
        t.pop();
        t.push(Some("http://a/".into()), "http://a/b".into());
        // nothing should be pushed at depth 0
        assert!(t.pop().is_none());
    }

    #[test]
    fn mailto_bypasses_depth() {
        let mut t = Tree::new(None, String::new(), 0, false);
        t.max_depth = 1;
        t.push(None, "http://a/".into());
        t.pop();
        // add a child at depth 1
        t.push(Some("http://a/".into()), "http://a/b".into());
        t.pop(); // consume depth-1 child
                 // now try to push a mailto from depth-1 child — should be accepted
        t.push(Some("http://a/b".into()), "mailto:foo@bar.com".into());
        let m = t.pop();
        assert!(m.is_some());
    }
}

#[cfg(test)]
mod words_tests {
    use crate::words::{count_words, normalize_words};
    use indexmap::IndexMap;

    #[test]
    fn basic_split_and_count() {
        let mut wh = IndexMap::new();
        let mut gh = IndexMap::new();
        count_words("hello world hello", 3, None, &mut wh, -1, &mut gh);
        assert_eq!(*wh.get("hello").unwrap(), 2);
        assert_eq!(*wh.get("world").unwrap(), 1);
    }

    #[test]
    fn min_length_filter() {
        let mut wh = IndexMap::new();
        let mut gh = IndexMap::new();
        count_words("hi hello ok", 3, None, &mut wh, -1, &mut gh);
        assert!(!wh.contains_key("hi"));
        assert!(!wh.contains_key("ok"));
        assert!(wh.contains_key("hello"));
    }

    #[test]
    fn lowercase_flag() {
        let n = normalize_words("Hello WORLD", true, false, false);
        assert!(n.contains("hello"));
        assert!(n.contains("world"));
    }

    #[test]
    fn umlaut_conversion() {
        let n = normalize_words("äöüß", false, false, true);
        assert!(n.contains("ae"));
        assert!(n.contains("oe"));
        assert!(n.contains("ue"));
        assert!(n.contains("ss"));
    }

    #[test]
    fn groups_of_2() {
        let mut wh = IndexMap::new();
        let mut gh = IndexMap::new();
        count_words("the quick brown", 1, None, &mut wh, 2, &mut gh);
        assert!(gh.contains_key("the quick"));
        assert!(gh.contains_key("quick brown"));
    }
}

#[cfg(test)]
mod html_tests {
    use crate::html;

    #[test]
    fn base_url_path_empty() {
        let b = html::base_url_for_page("http://example.com");
        assert_eq!(b, "http://example.com");
    }

    #[test]
    fn base_url_path_chop() {
        let b = html::base_url_for_page("http://example.com/path/page.html");
        assert_eq!(b, "http://example.com/path");
    }

    #[test]
    fn extract_hrefs_relative() {
        let html = r#"<a href="/about">About</a>"#;
        let hrefs = html::extract_hrefs(html, "http://example.com/");
        assert!(hrefs.iter().any(|u| u.contains("/about")));
    }

    #[test]
    fn strip_script() {
        let html = "<p>Hello</p><script>alert(1)</script><p>World</p>";
        let stripped = html::strip_scripts_styles_simple(html, false, true);
        assert!(!stripped.contains("alert"));
        assert!(stripped.contains("Hello"));
    }

    #[test]
    fn js_redirect_extract() {
        let body = r#"location.href = "/new-page";"#;
        let v = html::extract_js_redirects(body);
        assert_eq!(v, vec!["/new-page"]);
    }

    #[test]
    fn entity_decode() {
        let decoded = html::decode_html_entities("&amp;hello&lt;world&gt;");
        assert_eq!(decoded, "&hello<world>");
    }
}

#[cfg(test)]
mod capture_tests {
    use crate::capture;

    #[test]
    fn path_components() {
        let c = capture::extract_path_components("http://example.com/blog/post-one.html");
        assert!(c.contains(&"blog".to_string()));
        assert!(c.contains(&"post-one".to_string()));
    }

    #[test]
    fn subdomain_components() {
        let s = capture::extract_subdomain_components("http://sub.example.com/");
        assert!(s.contains(&"sub".to_string()));
    }

    #[test]
    fn registrable_domain() {
        let d = capture::extract_registrable_domain("http://sub.example.com/");
        assert_eq!(d, Some("example.com".to_string()));
    }
}

#[cfg(test)]
mod metadata_tests {
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn pdf_byte_regex() {
        let mut f = NamedTempFile::with_suffix(".pdf").unwrap();
        writeln!(f, "pdf:Author='Jane Doe'").unwrap();
        writeln!(f, "dc:creator='Bob'").unwrap();
        let results = crate::metadata::get_pdf_data(f.path(), false);
        assert!(results.contains(&"Jane Doe".to_string()), "{results:?}");
        assert!(results.contains(&"Bob".to_string()), "{results:?}");
    }

    /// Requires exiftool installed; skipped when missing.
    #[test]
    #[ignore]
    fn exiftool_doc() {
        // tested via integration fixture with a real .doc file
    }
}

// ----- HTTP integration test (static fetcher) --------------------------------
#[cfg(test)]
mod http_tests {
    use crate::fetcher::static_fetcher::{StaticConfig, StaticFetcher};
    use crate::fetcher::FetchOutcome;
    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    async fn hello_handler(
        _req: Request<hyper::body::Incoming>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        Ok(Response::new(Full::new(Bytes::from(
            "<html><body><a href='/page2'>link</a><p>hello world</p></body></html>",
        ))))
    }

    #[tokio::test]
    async fn static_fetch_html() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = TcpListener::bind(addr).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, service_fn(hello_handler))
                        .await
                        .ok();
                });
            }
        });

        let fetcher = StaticFetcher::new(StaticConfig::default()).unwrap();
        let outcome = fetcher.fetch(&format!("http://127.0.0.1:{port}/")).await;

        match outcome {
            FetchOutcome::Response { body, .. } => {
                let text = String::from_utf8_lossy(&body);
                assert!(text.contains("hello world"));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }
}

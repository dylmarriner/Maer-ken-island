//! The dashboard's pages and their assets, embedded in the binary so the
//! server runs from one file with nothing to install beside it.

use warp::{Filter, Reply};

const OVERVIEW: &str = include_str!("../../static/index.html");
const PEOPLE: &str = include_str!("../../static/people.html");
const CREATOR: &str = include_str!("../../static/creator.html");
const NOT_FOUND: &str = include_str!("../../static/not-found.html");

const HTML: &str = "text/html; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";

/// The files under `/static/`. Keeping them in one table means a new asset is
/// one line here rather than a new route.
pub const ASSETS: [(&str, &str, &str); 5] = [
    ("style.css", include_str!("../../static/style.css"), CSS),
    ("app.js", include_str!("../../static/app.js"), JS),
    ("overview.js", include_str!("../../static/overview.js"), JS),
    ("people.js", include_str!("../../static/people.js"), JS),
    ("creator.js", include_str!("../../static/creator.js"), JS),
];

/// Pages change whenever the binary does, so they are revalidated every time;
/// assets are embedded in that same binary, so a short cache is safe and keeps
/// a reload from refetching the stylesheet.
const PAGE_CACHE: &str = "no-cache";
const ASSET_CACHE: &str = "public, max-age=600";

fn served(body: &'static str, content_type: &'static str, cache: &'static str) -> impl warp::Reply {
    warp::reply::with_header(
        warp::reply::with_header(body, "content-type", content_type),
        "cache-control",
        cache,
    )
}

fn page(body: &'static str) -> impl warp::Reply {
    served(body, HTML, PAGE_CACHE)
}

pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = std::convert::Infallible> + Clone {
    let overview = warp::path::end().and(warp::get()).map(|| page(OVERVIEW));
    let people = warp::path!("people").and(warp::get()).map(|| page(PEOPLE));
    let creator = warp::path!("creator")
        .and(warp::get())
        .map(|| page(CREATOR));
    let assets =
        warp::path!("static" / String)
            .and(warp::get())
            .and_then(|name: String| async move {
                match ASSETS.iter().find(|(asset, _, _)| *asset == name) {
                    Some((_, body, content_type)) => {
                        Ok(served(body, content_type, ASSET_CACHE).into_response())
                    }
                    None => Err(warp::reject::not_found()),
                }
            });
    // Anything else is a mistyped address, and a person is reading it, so it
    // gets the page that says so rather than warp's bare 404.
    let missing = warp::any().map(|| {
        warp::reply::with_status(
            warp::reply::html(NOT_FOUND),
            warp::http::StatusCode::NOT_FOUND,
        )
    });

    overview
        .or(people)
        .or(creator)
        .or(assets)
        .or(missing)
        .map(|reply| Box::new(reply) as Box<dyn warp::Reply>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pages_are_served_with_their_content_types() {
        let filter = routes();
        for (path, kind, marker) in [
            ("/", "text/html", "id=\"activity\""),
            ("/people", "text/html", "id=\"roster\""),
            ("/creator", "text/html", "id=\"creator\""),
            ("/static/app.js", "text/javascript", "export"),
            ("/static/overview.js", "text/javascript", "/api/activity"),
            ("/static/people.js", "text/javascript", "/api/humans"),
            (
                "/static/creator.js",
                "text/javascript",
                "/api/creator/options",
            ),
            ("/static/style.css", "text/css", "prefers-color-scheme"),
        ] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 200, "{path}");
            let content_type = response.headers()["content-type"].to_str().unwrap();
            assert!(content_type.starts_with(kind), "{path}: {content_type}");
            assert!(response.headers().contains_key("cache-control"), "{path}");
            let body = std::str::from_utf8(response.body()).unwrap();
            assert!(body.contains(marker), "{path} is missing {marker}");
        }
    }

    #[tokio::test]
    async fn an_unknown_page_says_so_in_words() {
        let filter = routes();
        for path in ["/nowhere", "/static/missing.js", "/people/extra"] {
            let response = warp::test::request().path(path).reply(&filter).await;
            assert_eq!(response.status(), 404, "{path}");
            let body = std::str::from_utf8(response.body()).unwrap();
            assert!(body.contains("</html>"), "{path} did not get the page");
        }
    }

    #[test]
    fn scripts_never_insert_markup() {
        // Names and attributes are user-supplied; text must go in as text.
        for (name, script, _) in ASSETS.iter().filter(|(name, _, _)| name.ends_with(".js")) {
            for banned in [
                "innerHTML",
                "outerHTML",
                "insertAdjacentHTML",
                "document.write",
            ] {
                assert!(!script.contains(banned), "{name} uses {banned}");
            }
        }
    }

    #[test]
    fn every_page_is_a_complete_document_in_plain_english() {
        for (name, html) in [
            ("index.html", OVERVIEW),
            ("people.html", PEOPLE),
            ("creator.html", CREATOR),
            ("not-found.html", NOT_FOUND),
        ] {
            assert!(html.starts_with("<!doctype html>"), "{name}");
            assert!(html.contains("<html lang=\"en\">"), "{name}");
            assert!(html.contains("name=\"viewport\""), "{name}: not responsive");
            assert!(html.contains("<title>"), "{name}: no title");
            assert!(html.contains("rel=\"stylesheet\""), "{name}");
            for placeholder in ["Lorem ipsum", "TODO", "FIXME", "TBD", "Coming soon"] {
                assert!(
                    !html.contains(placeholder),
                    "{name} still says {placeholder}"
                );
            }
        }
    }
}

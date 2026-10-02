//! The dashboard's pages, embedded in the binary.

use warp::Filter;

const INDEX: &str = include_str!("../../static/index.html");
const CREATOR: &str = include_str!("../../static/creator.html");
const DASHBOARD_JS: &str = include_str!("../../static/dashboard.js");
const CREATOR_JS: &str = include_str!("../../static/creator.js");
const STYLE: &str = include_str!("../../static/style.css");

fn page(body: &'static str, content_type: &'static str) -> impl warp::Reply {
    warp::reply::with_header(body, "content-type", content_type)
}

pub fn routes() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let html = "text/html; charset=utf-8";
    let js = "text/javascript; charset=utf-8";
    let index = warp::path::end()
        .and(warp::get())
        .map(move || page(INDEX, html));
    let creator = warp::path!("creator")
        .and(warp::get())
        .map(move || page(CREATOR, html));
    let dashboard_js = warp::path!("static" / "dashboard.js")
        .and(warp::get())
        .map(move || page(DASHBOARD_JS, js));
    let creator_js = warp::path!("static" / "creator.js")
        .and(warp::get())
        .map(move || page(CREATOR_JS, js));
    let style = warp::path!("static" / "style.css")
        .and(warp::get())
        .map(|| page(STYLE, "text/css; charset=utf-8"));
    index.or(creator).or(dashboard_js).or(creator_js).or(style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pages_are_served_with_their_content_types() {
        let filter = routes();
        for (path, kind, marker) in [
            ("/", "text/html", "id=\"roster\""),
            ("/creator", "text/html", "id=\"creator\""),
            ("/static/dashboard.js", "text/javascript", "/api/humans"),
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
            let body = std::str::from_utf8(response.body()).unwrap();
            assert!(body.contains(marker), "{path} is missing {marker}");
        }
    }

    #[test]
    fn scripts_never_insert_markup() {
        // Names and attributes are user-supplied; text must go in as text.
        for (name, script) in [("dashboard.js", DASHBOARD_JS), ("creator.js", CREATOR_JS)] {
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
}

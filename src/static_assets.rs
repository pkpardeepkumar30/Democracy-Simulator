use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
pub async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

pub async fn app_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../web/app.js"),
    )
        .into_response()
}

pub async fn city_bundle_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../web/dist/city.bundle.js"),
    )
        .into_response()
}

pub async fn styles_css() -> Response {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../web/styles.css"),
    )
        .into_response()
}

pub async fn manifest() -> Response {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        include_str!("../web/manifest.webmanifest"),
    )
        .into_response()
}

pub async fn service_worker() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        include_str!("../web/sw.js"),
    )
        .into_response()
}

pub async fn robots_txt() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "User-agent: *\nAllow: /\nSitemap: https://the-republic.pages.dev/sitemap.xml\n",
    )
        .into_response()
}

pub async fn sitemap() -> Response {
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://the-republic.pages.dev/</loc>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
"#,
    )
        .into_response()
}

pub async fn favicon() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

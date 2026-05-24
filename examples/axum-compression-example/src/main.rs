use anycms_spa::spa;
use axum::routing::get;
use axum::Router;
use tracing::info;

spa!(Spa, "assets", {
    .with_default_security_headers()
    .with_error_page(404, "404.html")
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("Server running at http://127.0.0.1:8080");
    info!("");
    info!("Test commands:");
    info!("  # Brotli compression (preferred)");
    info!("  curl -v -H 'Accept-Encoding: br,gzip' http://127.0.0.1:8080/css/style.css");
    info!("");
    info!("  # Gzip fallback");
    info!("  curl -v -H 'Accept-Encoding: gzip' http://127.0.0.1:8080/css/style.css");
    info!("");
    info!("  # No compression (identity)");
    info!("  curl -v http://127.0.0.1:8080/css/style.css");
    info!("");
    info!("  # Security headers + charset + Vary");
    info!("  curl -vI http://127.0.0.1:8080/");
    info!("");
    info!("  # Custom 404 page");
    info!("  curl -v http://127.0.0.1:8080/nonexistent");
    info!("");
    info!("  # ETag / 304");
    info!("  curl -v -H 'If-None-Match: <etag>' http://127.0.0.1:8080/");
    info!("");
    info!("  # Range request (partial content)");
    info!("  curl -v -H 'Range: bytes=0-49' http://127.0.0.1:8080/css/style.css");
    info!("");
    info!("  # Range request with If-Range (ETag mismatch = full content)");
    info!("  curl -v -H 'Range: bytes=0-49' -H 'If-Range: \"wrong-etag\"' http://127.0.0.1:8080/css/style.css");

    let app = Router::new()
        .route("/api/health", get(health))
        .merge(Spa::spa_router());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

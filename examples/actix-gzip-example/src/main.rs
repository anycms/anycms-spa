use actix_web::{App, HttpServer};
use anycms_spa::spa;
use tracing::info;

spa!(Spa, "assets", "/", ["index.html"]);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Server running at http://127.0.0.1:8080");
    info!("Test commands:");
    info!("  curl -v http://127.0.0.1:8080/css/style.css");
    info!("  curl -v -H 'Accept-Encoding: gzip' http://127.0.0.1:8080/css/style.css");
    info!("  curl -v -H 'If-None-Match: <etag>' http://127.0.0.1:8080/css/style.css");
    HttpServer::new(|| App::new().service(Spa::spa_service()))
        .bind("127.0.0.1:8080")?
        .run()
        .await?;
    Ok(())
}

use anycms_spa::spa;
use axum::Router;
use tracing::info;

spa!(Spa, "assets", {
    .with_default_security_headers()
});

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    info!("Server running at http://127.0.0.1:8080");
    info!("");
    info!("Override: place files in ./assets/ to replace embedded assets (same path as spa! config)");
    info!("Example: echo '<h1>Patched!</h1>' > ./assets/index.html && restart");
    info!("");
    info!("Test commands:");
    info!("  curl http://127.0.0.1:8080/");
    info!("  curl http://127.0.0.1:8080/css/style.css");
    info!("  curl http://127.0.0.1:8080/js/app.js");

    let app = Router::new().merge(Spa::spa_router());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use anycms_spa::spa;
use tracing::info;

spa!(Spa, "assets", "/", ["index.html"]);
spa!(Dashboard, "dashboard", "/dashboard", ["index.html"]);


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Hello, world!");
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(Dashboard::spa_service())
            .service(Spa::spa_service()) 
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await?;
    Ok(())
}

#[get("/home")]
pub async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

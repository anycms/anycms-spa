use anycms_spa::spa;
use salvo::prelude::*;

spa!(Spa, "assets");
spa!(Dashboard, "dashboard", "/dashboard", ["index.html"]);

#[handler]
async fn home(res: &mut Response) {
    res.render(Text::Plain("Hello, World!"));
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let router = Router::new()
        .push(Router::with_path("/home").get(home))
        .push(Dashboard::spa_router())
        .push(Spa::spa_router());

    let listener = TcpListener::new("0.0.0.0:3000").bind().await;
    Server::new(listener).serve(router).await;
}

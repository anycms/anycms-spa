# Web 框架的 SPA 功能集成

## 使用方法
### 添加依赖
```toml
# actix-web 框架
anycms-spa = {version ="*",features = ["actix"]}    
# axum 框架
anycms-spa = {version ="*",features = ["axum"]}   

rust-embed = "8.7.2"
paste = "1.0.15"
```

### `spa!` 宏使用
> 宏需要用在 main.rs 或者 lib.rs 文件中
```rust
/// spa!(名称,资源路径, 路由前缀, [index文件名称数组])
/// 路由前缀和 index 文件名数组 可选
///spa!(name, assets_path, route_prefix, [index])
spa!(Spa, "assets");
/// 等价于
spa!(Spa, "assets", "/", ["index.html"]);

```

## 示例代码
### Actix 示例代码
```rust 

use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use anycms_spa::spa;
use tracing::info;

// spa!(name,assets_path, route_prefix, index)]
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

```


### Axum 示例代码
```rust
```
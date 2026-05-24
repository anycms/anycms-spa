# Web 框架的 SPA 功能集成

支持 Actix-web、Axum、Salvo 三大框架，通过统一的 `spa!` 宏将前端静态资源嵌入 Rust 二进制文件并提供 SPA 路由服务。

## 功能特性

- **SPA 路由**：未匹配路径自动 fallback 到 `index.html`
- **路径遍历防护**：自动拒绝包含 `..` 的恶意路径
- **ETag 缓存**：基于 `SHA256` 生成 ETag，支持 `If-None-Match` 返回 `304 Not Modified`
- **Gzip 预压缩**（可选）：启动时预压缩文本资源，自动响应 `Accept-Encoding: gzip`

## 使用方法

### 添加依赖

```toml
# actix-web 框架
anycms-spa = { version = "*", features = ["actix"] }
# axum 框架
anycms-spa = { version = "*", features = ["axum"] }
# salvo 框架
anycms-spa = { version = "*", features = ["salvo"] }

# 启用 gzip 预压缩（可选，与框架 feature 组合使用）
anycms-spa = { version = "*", features = ["actix", "gzip"] }

# 注意：框架 features 互斥，只能启用一个

# 由于 `rust-embed` 是过程宏，需要手动添加以下依赖
rust-embed = "8.9.0"
paste = "1.0.15"
```

### `spa!` 宏使用

> 宏需要用在 main.rs 或者 lib.rs 文件中

```rust
/// spa!(名称, 资源路径, 路由前缀, [index 文件名称数组])
/// 路由前缀和 index 文件名数组可选
spa!(Spa, "assets");
/// 等价于
spa!(Spa, "assets", "/", ["index.html"]);

/// 带路由前缀的 Dashboard
spa!(Dashboard, "dashboard", "/dashboard", ["index.html"]);
```

## 示例代码

### Actix-web 示例

```rust
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
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
```

### Axum 示例

```rust
use anycms_spa::spa;
use axum::{routing::get, Router};

spa!(Spa, "assets");
spa!(Dashboard, "dashboard", "/dashboard", ["index.html"]);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/home", get(root))
        .merge(Dashboard::spa_router())
        .merge(Spa::spa_router());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}
```

### Salvo 示例

```rust
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
```

## API 对比

| 框架 | 宏生成的方法 | 返回类型 |
|------|-------------|---------|
| Actix-web | `Struct::spa_service()` | `actix_web::Resource` |
| Axum | `Struct::spa_router()` | `axum::Router` |
| Salvo | `Struct::spa_router()` | `salvo::Router` |

## HTTP 缓存策略

| 资源类型 | Cache-Control | ETag | Gzip |
|---------|---------------|------|------|
| 静态资源（JS/CSS/图片等） | `public, max-age=31536000` | SHA256 | 可选 |
| HTML 页面 | `no-cache` | SHA256 | 可选 |
| 未匹配路径（SPA fallback） | `no-cache` | SHA256 | 可选 |

### ETag

所有响应自动携带 `ETag` header（基于 `rust_embed` 内置的 SHA256 哈希，零额外计算）。客户端发送 `If-None-Match` 时返回 `304 Not Modified`，避免重复传输。

### Gzip 预压缩

启用 `gzip` feature 后，`SpaHandler` 在初始化时预压缩所有文本类文件（`text/*`、`application/javascript`、`application/json`、`application/xml`、`application/wasm`、`image/svg+xml`）。仅当压缩后体积更小时才缓存压缩版本。

```bash
# 不压缩：1937 bytes
curl http://127.0.0.1:8080/css/style.css

# gzip 压缩：857 bytes（-56%）
curl -H "Accept-Encoding: gzip" http://127.0.0.1:8080/css/style.css

# 条件请求：304 Not Modified
curl -H "If-None-Match: \"20b3442cdd9d14fb0dabb3a38966c226\"" http://127.0.0.1:8080/css/style.css
```

## 完整示例

查看 [examples/](examples/) 目录获取可运行的完整项目。

# Web 框架的 SPA 功能集成

支持 Actix-web、Axum、Salvo 三大框架，通过统一的 `spa!` 宏将前端静态资源嵌入 Rust 二进制文件并提供 SPA 路由服务。

## 功能特性

- **SPA 路由**：未匹配路径自动 fallback 到 `index.html`
- **路径遍历防护**：自动拒绝包含 `..` 的恶意路径
- **ETag 缓存**：基于 `SHA256` 生成 ETag，支持 `If-None-Match` 返回 `304 Not Modified`
- **Gzip 预压缩**（可选）：启动时预压缩文本资源，自动响应 `Accept-Encoding: gzip`
- **Brotli 预压缩**（可选）：比 Gzip 压缩率高 15-20%，自动响应 `Accept-Encoding: br`
- **内容协商**：br > gzip > identity 自动选择最优编码
- **安全头注入**（可选）：一键注入 X-Content-Type-Options、X-Frame-Options 等安全头
- **自定义错误页面**（可选）：配置自定义 404 等错误页面
- **Vary 头**：有压缩变体时自动添加 `Vary: Accept-Encoding`
- **charset**：text/* 类资源自动追加 `; charset=utf-8`
- **Range 下载**（断点续传）：支持 `Range` / `If-Range` 请求，返回 `206 Partial Content`

## 使用方法

### 添加依赖

```toml
# actix-web 框架
anycms-spa = { version = "*", features = ["actix"] }
# axum 框架
anycms-spa = { version = "*", features = ["axum"] }
# salvo 框架
anycms-spa = { version = "*", features = ["salvo"] }

# 启用压缩（可选，与框架 feature 组合使用）
anycms-spa = { version = "*", features = ["actix", "gzip"] }
anycms-spa = { version = "*", features = ["axum", "gzip", "brotli"] }

# 注意：框架 features 互斥，只能启用一个；gzip 和 brotli 可同时启用

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

/// 带扩展配置：用 { } 配置块，在任意简写形式后追加
spa!(Spa, "assets", {
    .with_default_security_headers()
    .with_error_page(404, "404.html")
});

spa!(Dashboard, "dashboard", "/dashboard", {
    .with_security_header("Content-Security-Policy", "default-src 'self'")
});

/// 完整形式也支持 { } 配置块
spa!(Spa, "assets", "/", ["index.html"], {
    .with_default_security_headers()
});
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

### Axum 示例（带 Brotli + 安全头）

```rust
use anycms_spa::spa;
use axum::{routing::get, Router};

spa!(Spa, "assets", {
    .with_default_security_headers()
    .with_error_page(404, "404.html")
});

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/home", get(root))
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

| 资源类型 | Cache-Control | ETag | Gzip | Brotli |
|---------|---------------|------|------|--------|
| 静态资源（JS/CSS/图片等） | `public, max-age=31536000` | SHA256 | 可选 | 可选 |
| HTML 页面 | `no-cache` | SHA256 | 可选 | 可选 |
| 未匹配路径（SPA fallback） | `no-cache` | SHA256 | 可选 | 可选 |

### ETag

所有响应自动携带 `ETag` header（基于 `rust_embed` 内置的 SHA256 哈希，零额外计算）。客户端发送 `If-None-Match` 时返回 `304 Not Modified`，避免重复传输。

### 压缩

启用 `gzip` 和/或 `brotli` feature 后，`SpaHandler` 在初始化时预压缩所有文本类文件（`text/*`、`application/javascript`、`application/json`、`application/xml`、`application/wasm`、`image/svg+xml`）。仅当压缩后体积更小时才缓存压缩版本。

当两者同时启用时，自动协商最优编码：**br > gzip > identity**。

```bash
# 不压缩：824 bytes
curl http://127.0.0.1:8080/css/style.css

# Brotli 压缩：296 bytes（-64%）
curl -H "Accept-Encoding: br" http://127.0.0.1:8080/css/style.css

# Gzip 压缩：429 bytes（-48%）
curl -H "Accept-Encoding: gzip" http://127.0.0.1:8080/css/style.css

# 条件请求：304 Not Modified
curl -H 'If-None-Match: "<etag>"' http://127.0.0.1:8080/css/style.css
```

### 安全头

通过 `SpaConfig` 配置，支持自定义和预设两种方式：

```rust
// 使用预设安全头
spa!(Spa, "assets", {
    .with_default_security_headers()
});
// 注入以下头：
// X-Content-Type-Options: nosniff
// X-Frame-Options: SAMEORIGIN
// X-XSS-Protection: 1; mode=block
// Referrer-Policy: strict-origin-when-cross-origin

// 自定义安全头
spa!(Spa, "assets", {
    .with_security_header("Content-Security-Policy", "default-src 'self'")
    .with_security_header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
});
```

安全头会应用到所有响应，包括 304 Not Modified 和错误响应。

### 自定义错误页面

```rust
spa!(Spa, "assets", {
    .with_error_page(404, "404.html")
});
```

当 SPA fallback 也找不到 index 文件时，返回配置的自定义错误页面（支持压缩协商和安全头）。

### Range 下载（断点续传）

所有响应自动携带 `Accept-Ranges: bytes` 头，客户端可通过 `Range` 请求部分内容：

```bash
# 请求前 50 字节
curl -H "Range: bytes=0-49" http://127.0.0.1:8080/css/style.css
# → 206 Partial Content
# → Content-Range: bytes 0-49/824

# 从第 100 字节到末尾
curl -H "Range: bytes=100-" http://127.0.0.1:8080/css/style.css

# 请求最后 10 字节
curl -H "Range: bytes=-10" http://127.0.0.1:8080/css/style.css

# If-Range：ETag 匹配时返回 206，不匹配时返回完整内容 200
curl -H 'Range: bytes=0-49' -H 'If-Range: "<etag>"' http://127.0.0.1:8080/css/style.css
```

支持三种 `Range` 格式：`bytes=start-end`、`bytes=start-`（开放结尾）、`bytes=-suffix`（末尾 N 字节）。`If-Range` 支持 ETag 匹配。

## 完整示例

查看 [examples/](examples/) 目录获取可运行的完整项目：

- [actix-spa-example](examples/actix-spa-example/) — Actix-web 基础用法
- [actix-gzip-example](examples/actix-gzip-example/) — Actix-web + Gzip 压缩
- [axum-spa-example](examples/axum-spa-example/) — Axum 基础用法
- [axum-compression-example](examples/axum-compression-example/) — Axum + Brotli + Gzip + 安全头 + 自定义 404 + Range 下载
- [salvo-spa-example](examples/salvo-spa-example/) — Salvo 基础用法

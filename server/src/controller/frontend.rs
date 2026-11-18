use actix_web::{HttpRequest, HttpResponse};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct WebAssets;

/// 返回前端入口 index.html。
pub async fn index() -> HttpResponse {
    asset("index.html")
}

///SPA 路由回退；API、网关和 MCP 的未匹配路径保持 404。
pub async fn fallback(request: HttpRequest) -> HttpResponse {
    let path = request.path();
    if path.starts_with("/api/") || path.starts_with("/v1/") || path.starts_with("/mcp") {
        return HttpResponse::NotFound().json(serde_json::json!({
            "code": 4,
            "message": "not found",
            "data": null
        }));
    }

    if path.starts_with("/assets/") {
        let name = &path[1..];
        return asset(name);
    }

    asset("index.html")
}

fn asset(name: &str) -> HttpResponse {
    match WebAssets::get(name) {
        Some(file) => {
            let mime = match name.rsplit('.').next() {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };
            HttpResponse::Ok().content_type(mime).body(file.data)
        }
        None => HttpResponse::NotFound().body("frontend asset is missing"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn embeds_frontend_entry_and_assets() {
        assert!(WebAssets::get("index.html").is_some());
        let asset = WebAssets::iter()
            .find(|asset| asset.ends_with(".js"))
            .unwrap();
        assert!(WebAssets::get(&asset).is_some());

        let request =
            actix_web::test::TestRequest::with_uri(&format!("/{asset}")).to_http_request();
        let response = fallback(request).await;
        assert!(response.status().is_success());
        assert!(WebAssets::iter().any(|asset| asset.starts_with("assets/")));
    }
}

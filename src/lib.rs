use console_error_panic_hook::set_once as set_panic_hook;
use identicon_rs::Identicon;
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    set_panic_hook();

    if !matches!(req.method(), Method::Get | Method::Head | Method::Options) {
        return Response::error("Method not allowed", 405);
    }

    let path = req.path();

    if path == "/" {
        return landing_page();
    }

    let identicon = Identicon::new(&path);
    let png_data = identicon
        .export_png_data()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    let headers = Headers::from_iter([
        ("Content-Type", "image/png"),
        ("Cache-Control", "public, max-age=31536000, immutable"),
        ("ETag", &format!("\"{}\"", path)),
    ]);

    let cors = Cors::default()
        .with_methods([Method::Get, Method::Head, Method::Options])
        .with_origins(["*"]);

    Response::builder()
        .with_headers(headers)
        .fixed(png_data)
        .with_cors(&cors)
}

fn log_request(req: &Request) {
    let cf = req.cf().expect("CF object should be present");
    console_log!(
        "{} {} {}, located at: {:?}, within: {}",
        req.headers()
            .get("CF-Connecting-IP")
            .unwrap_or_default()
            .unwrap_or_default(),
        req.method().to_string(),
        req.path(),
        cf.coordinates().unwrap_or_default(),
        cf.region().unwrap_or("unknown region".into())
    );
}

fn landing_page() -> Result<Response> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Identicon Generator</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 600px;
            margin: 50px auto;
            padding: 20px;
            line-height: 1.6;
        }
        h1 { color: #333; }
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 4px;
        }
        .example {
            display: flex;
            align-items: center;
            gap: 16px;
            margin: 20px 0;
        }
        .example img {
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
    </style>
</head>
<body>
    <h1>Identicon Generator</h1>
    <p>Generate unique identicons from any string. Simply append your desired string to the URL path.</p>

    <h2>Usage</h2>
    <p>Request: <code>GET /{your-string}</code></p>
    <p>Returns a PNG image of the identicon.</p>

    <h2>Examples</h2>
    <div class="example">
        <img src="/hello" alt="hello identicon" width="64" height="64">
        <code>/hello</code>
    </div>
    <div class="example">
        <img src="/user@example.com" alt="email identicon" width="64" height="64">
        <code>/user@example.com</code>
    </div>
    <div class="example">
        <img src="/12345" alt="number identicon" width="64" height="64">
        <code>/12345</code>
    </div>
</body>
</html>"#;

    Response::from_html(html)
}

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
        "{} {}, located at: {:?}, within: {}",
        req.method().to_string(),
        req.path(),
        cf.coordinates().unwrap_or_default(),
        cf.region().unwrap_or("unknown region".into())
    );
}

use console_error_panic_hook::set_once as set_panic_hook;
use identicon_rs::Identicon;
use image::{ImageReader, RgbaImage};
use std::io::Cursor;
use worker::*;

// Embed the sprite sheet at compile time
static ROBO_SPRITES: &[u8] = include_bytes!("robo.png");

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

    if let Some(input) = path.strip_prefix("/robo/") {
        if input.is_empty() {
            return Response::error("Missing input parameter", 400);
        }
        return generate_robo(input);
    }

    let identicon = Identicon::new(&path);
    let png_data = identicon
        .export_png_data()
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    png_response(png_data, &path)
}

fn png_response(png_data: Vec<u8>, etag: &str) -> Result<Response> {
    let headers = Headers::from_iter([
        ("Content-Type", "image/png"),
        ("Cache-Control", "public, max-age=31536000, immutable"),
        ("ETag", &format!("\"{}\"", etag)),
    ]);

    let cors = Cors::default()
        .with_methods([Method::Get, Method::Head, Method::Options])
        .with_origins(["*"]);

    Response::builder()
        .with_headers(headers)
        .fixed(png_data)
        .with_cors(&cors)
}

/// Generate robot avatar based on input string
fn generate_robo(input: &str) -> Result<Response> {
    let hash = md5::compute(input);
    let buckets = get_buckets(&hash.0);

    let body_style = buckets[0];
    let head_style = buckets[1];
    let eye_style = buckets[2];
    let mouth_style = buckets[3];
    let acc_style = buckets[4];
    let bh_color = buckets[5];
    let em_color = buckets[6];
    let acc_color = buckets[7];

    let sprite_sheet = ImageReader::new(Cursor::new(ROBO_SPRITES))
        .with_guessed_format()
        .map_err(|e| worker::Error::RustError(e.to_string()))?
        .decode()
        .map_err(|e| worker::Error::RustError(e.to_string()))?
        .into_rgba8();

    let mut output = RgbaImage::new(300, 300);

    // Composite layers (order matters: body, head, mouth, eyes, accessories)
    // Body: y = bh_color * 1500 + 900
    composite_sprite(
        &mut output,
        &sprite_sheet,
        body_style as u32 * 300,
        bh_color as u32 * 1500 + 900,
    );

    // Head: y = bh_color * 1500 + 1200
    composite_sprite(
        &mut output,
        &sprite_sheet,
        head_style as u32 * 300,
        bh_color as u32 * 1500 + 1200,
    );

    // Mouth: y = em_color * 1500
    composite_sprite(
        &mut output,
        &sprite_sheet,
        mouth_style as u32 * 300,
        em_color as u32 * 1500,
    );

    // Eyes: y = em_color * 1500 + 300
    composite_sprite(
        &mut output,
        &sprite_sheet,
        eye_style as u32 * 300,
        em_color as u32 * 1500 + 300,
    );

    // Accessories: y = acc_color * 1500 + 600
    composite_sprite(
        &mut output,
        &sprite_sheet,
        acc_style as u32 * 300,
        acc_color as u32 * 1500 + 600,
    );

    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    output
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    png_response(png_data, &format!("robo-{}", input))
}

/// Convert MD5 hash bytes into 8 buckets (0-9 each)
fn get_buckets(hash: &[u8; 16]) -> [u8; 8] {
    let mut buckets = [0u8; 8];
    for i in 0..8 {
        let pair_value = ((hash[i * 2] as u16) << 8) + hash[i * 2 + 1] as u16;
        buckets[i] = (pair_value % 10) as u8;
    }
    buckets
}

/// Composite a 300x300 sprite from the sprite sheet onto the output image
fn composite_sprite(output: &mut RgbaImage, sprite_sheet: &RgbaImage, src_x: u32, src_y: u32) {
    for y in 0..300 {
        for x in 0..300 {
            let src_pixel = sprite_sheet.get_pixel(src_x + x, src_y + y);
            // Alpha blending
            if src_pixel[3] > 0 {
                let dst_pixel = output.get_pixel(x, y);
                let alpha = src_pixel[3] as f32 / 255.0;
                let inv_alpha = 1.0 - alpha;

                let r = (src_pixel[0] as f32 * alpha + dst_pixel[0] as f32 * inv_alpha) as u8;
                let g = (src_pixel[1] as f32 * alpha + dst_pixel[1] as f32 * inv_alpha) as u8;
                let b = (src_pixel[2] as f32 * alpha + dst_pixel[2] as f32 * inv_alpha) as u8;
                let a = ((src_pixel[3] as f32 + dst_pixel[3] as f32 * inv_alpha).min(255.0)) as u8;

                output.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }
    }
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
        h2 { margin-top: 32px; }
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
        hr {
            margin: 40px 0;
            border: none;
            border-top: 1px solid #eee;
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

    <hr>

    <h1>Robot Avatars</h1>
    <p>Generate unique robot avatars from any string using the <code>/robo/</code> path.</p>

    <h2>Usage</h2>
    <p>Request: <code>GET /robo/{your-string}</code></p>
    <p>Returns a 300x300 PNG image of a robot avatar.</p>

    <h2>Examples</h2>
    <div class="example">
        <img src="/robo/hello" alt="hello robot" width="100" height="100">
        <code>/robo/hello</code>
    </div>
    <div class="example">
        <img src="/robo/user@example.com" alt="email robot" width="100" height="100">
        <code>/robo/user@example.com</code>
    </div>
    <div class="example">
        <img src="/robo/12345" alt="number robot" width="100" height="100">
        <code>/robo/12345</code>
    </div>
</body>
</html>"#;

    Response::from_html(html)
}

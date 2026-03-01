//! Build script for EventSleuth.
//!
//! Generates the application icon programmatically and embeds it along with
//! the Windows application manifest into the final executable.

use std::path::Path;

fn main() {
    // Only run resource embedding on Windows MSVC targets.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    generate_icon();

    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set_manifest_file("assets/app.manifest");
    res.set("ProductName", "EventSleuth");
    res.set("FileDescription", "Fast Windows Event Log Viewer");

    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=Failed to compile Windows resources: {e}");
    }
}

/// Generates a simple magnifying-glass-over-log icon programmatically.
/// Produces a multi-resolution .ico file at `assets/icon.ico`.
fn generate_icon() {
    let icon_path = Path::new("assets/icon.ico");
    if icon_path.exists() {
        return; // Don't regenerate if it already exists.
    }

    // Generate 256x256, 48x48, 32x32, 16x16 sizes.
    let sizes: &[u32] = &[256, 48, 32, 16];
    let mut ico_data: Vec<u8> = Vec::new();

    // ICO header: reserved(2) + type(2) + count(2)
    ico_data.extend_from_slice(&[0, 0]); // reserved
    ico_data.extend_from_slice(&1u16.to_le_bytes()); // type = 1 (icon)
    ico_data.extend_from_slice(&(sizes.len() as u16).to_le_bytes()); // image count

    // We'll build each PNG image, then write directory entries + data.
    let mut png_blobs: Vec<Vec<u8>> = Vec::new();
    for &size in sizes {
        let img = render_icon(size);
        let mut png_data = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        image::ImageEncoder::write_image(encoder, &img, size, size, image::ColorType::Rgba8.into())
            .expect("PNG encoding failed");
        png_blobs.push(png_data);
    }

    // Calculate offsets: header(6) + directory_entries(16 each) + data
    let dir_size = 6 + sizes.len() * 16;
    let mut offset = dir_size;

    // Write directory entries
    for (i, &size) in sizes.iter().enumerate() {
        let w = if size >= 256 { 0u8 } else { size as u8 };
        let h = w;
        ico_data.push(w); // width
        ico_data.push(h); // height
        ico_data.push(0); // colour palette
        ico_data.push(0); // reserved
        ico_data.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        ico_data.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico_data.extend_from_slice(&(png_blobs[i].len() as u32).to_le_bytes()); // data size
        ico_data.extend_from_slice(&(offset as u32).to_le_bytes()); // data offset
        offset += png_blobs[i].len();
    }

    // Write image data
    for blob in &png_blobs {
        ico_data.extend_from_slice(blob);
    }

    std::fs::create_dir_all("assets").ok();
    std::fs::write(icon_path, &ico_data).expect("Failed to write icon.ico");
}

/// Render the EventSleuth icon at the given size: a magnifying glass
/// over a document/log symbol, using the brand blue/teal palette.
fn render_icon(size: u32) -> Vec<u8> {
    let s = size as f64;
    let mut pixels = vec![0u8; (size * size * 4) as usize];

    // Background: rounded rectangle with gradient feel (dark blue-grey)
    let bg_r = 30u8;
    let bg_g = 35u8;
    let bg_b = 50u8;
    let corner_radius = s * 0.18;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f64;
            let fy = y as f64;

            // Rounded rect check
            let inside = is_in_rounded_rect(fx, fy, s, s, corner_radius);
            if !inside {
                continue;
            }

            // Slight vertical gradient
            let t = fy / s;
            let r = lerp_u8(bg_r, bg_r.saturating_add(15), t);
            let g = lerp_u8(bg_g, bg_g.saturating_add(10), t);
            let b = lerp_u8(bg_b, bg_b.saturating_add(20), t);

            set_pixel(&mut pixels, size, x, y, r, g, b, 255);
        }
    }

    // Draw document/log lines (left side, representing log entries)
    let doc_left = s * 0.15;
    let doc_right = s * 0.60;
    let line_positions = [0.25, 0.37, 0.49, 0.61, 0.73];
    let line_colors: [(u8, u8, u8); 5] = [
        (224, 108, 96),  // red (error)
        (224, 168, 64),  // amber (warning)
        (122, 162, 212), // blue (info)
        (122, 162, 212), // blue (info)
        (136, 136, 136), // grey (verbose)
    ];

    for (i, &ypos) in line_positions.iter().enumerate() {
        let ly = (s * ypos) as u32;
        let thickness = (s * 0.035).max(1.0) as u32;
        let (cr, cg, cb) = line_colors[i];
        // Small severity dot
        let dot_cx = doc_left + s * 0.03;
        let dot_cy = s * ypos;
        let dot_r = s * 0.02;
        draw_filled_circle(&mut pixels, size, dot_cx, dot_cy, dot_r, cr, cg, cb, 255);
        // Line bar
        for dy in 0..thickness {
            for lx in ((doc_left + s * 0.07) as u32)..((doc_right) as u32) {
                if ly + dy < size && lx < size {
                    set_pixel(&mut pixels, size, lx, ly + dy, 180, 190, 210, 180);
                }
            }
        }
    }

    // Draw magnifying glass (right side, overlapping)
    let glass_cx = s * 0.62;
    let glass_cy = s * 0.52;
    let glass_r = s * 0.22;
    let glass_thickness = s * 0.04;

    // Glass fill (semi-transparent teal)
    draw_filled_circle(
        &mut pixels,
        size,
        glass_cx,
        glass_cy,
        glass_r - glass_thickness,
        60,
        180,
        200,
        80,
    );

    // Glass ring
    draw_ring(
        &mut pixels,
        size,
        glass_cx,
        glass_cy,
        glass_r,
        glass_thickness,
        80,
        220,
        240,
        255,
    );

    // Glass handle
    let handle_angle = std::f64::consts::FRAC_PI_4; // 45 degrees
    let handle_start_x = glass_cx + (glass_r - glass_thickness * 0.5) * handle_angle.cos();
    let handle_start_y = glass_cy + (glass_r - glass_thickness * 0.5) * handle_angle.sin();
    let handle_len = s * 0.18;
    let handle_end_x = handle_start_x + handle_len * handle_angle.cos();
    let handle_end_y = handle_start_y + handle_len * handle_angle.sin();
    draw_thick_line(
        &mut pixels,
        size,
        handle_start_x,
        handle_start_y,
        handle_end_x,
        handle_end_y,
        glass_thickness * 1.2,
        80,
        220,
        240,
        255,
    );

    pixels
}

fn is_in_rounded_rect(x: f64, y: f64, w: f64, h: f64, r: f64) -> bool {
    if x < 0.0 || x >= w || y < 0.0 || y >= h {
        return false;
    }
    // Check corners
    let corners = [(r, r), (w - r, r), (r, h - r), (w - r, h - r)];
    for &(cx, cy) in &corners {
        if (x < r || x > w - r) && (y < r || y > h - r) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy > r * r {
                return false;
            }
        }
    }
    true
}

fn set_pixel(pixels: &mut [u8], stride: u32, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    let idx = ((y * stride + x) * 4) as usize;
    if idx + 3 < pixels.len() {
        // Alpha blend
        let src_a = a as f64 / 255.0;
        let dst_a = pixels[idx + 3] as f64 / 255.0;
        let out_a = src_a + dst_a * (1.0 - src_a);
        if out_a > 0.0 {
            pixels[idx] =
                ((r as f64 * src_a + pixels[idx] as f64 * dst_a * (1.0 - src_a)) / out_a) as u8;
            pixels[idx + 1] =
                ((g as f64 * src_a + pixels[idx + 1] as f64 * dst_a * (1.0 - src_a)) / out_a) as u8;
            pixels[idx + 2] =
                ((b as f64 * src_a + pixels[idx + 2] as f64 * dst_a * (1.0 - src_a)) / out_a) as u8;
            pixels[idx + 3] = (out_a * 255.0) as u8;
        }
    }
}

fn draw_filled_circle(
    pixels: &mut [u8],
    stride: u32,
    cx: f64,
    cy: f64,
    r: f64,
    cr: u8,
    cg: u8,
    cb: u8,
    ca: u8,
) {
    let x0 = (cx - r - 1.0).max(0.0) as u32;
    let y0 = (cy - r - 1.0).max(0.0) as u32;
    let x1 = (cx + r + 1.0).min(stride as f64 - 1.0) as u32;
    let y1 = (cy + r + 1.0).min(stride as f64 - 1.0) as u32;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f64 - cx;
            let dy = py as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= r {
                let edge_alpha = ((r - dist).min(1.0) * ca as f64) as u8;
                set_pixel(pixels, stride, px, py, cr, cg, cb, edge_alpha);
            }
        }
    }
}

fn draw_ring(
    pixels: &mut [u8],
    stride: u32,
    cx: f64,
    cy: f64,
    r: f64,
    thickness: f64,
    cr: u8,
    cg: u8,
    cb: u8,
    ca: u8,
) {
    let outer = r;
    let inner = r - thickness;
    let x0 = (cx - outer - 1.0).max(0.0) as u32;
    let y0 = (cy - outer - 1.0).max(0.0) as u32;
    let x1 = (cx + outer + 1.0).min(stride as f64 - 1.0) as u32;
    let y1 = (cy + outer + 1.0).min(stride as f64 - 1.0) as u32;
    for py in y0..=y1 {
        for px in x0..=x1 {
            let dx = px as f64 - cx;
            let dy = py as f64 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist >= inner && dist <= outer {
                let edge_out = (outer - dist).min(1.0).max(0.0);
                let edge_in = (dist - inner).min(1.0).max(0.0);
                let alpha = (edge_out.min(edge_in) * ca as f64) as u8;
                set_pixel(pixels, stride, px, py, cr, cg, cb, alpha);
            }
        }
    }
}

fn draw_thick_line(
    pixels: &mut [u8],
    stride: u32,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    thickness: f64,
    cr: u8,
    cg: u8,
    cb: u8,
    ca: u8,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return;
    }
    let half_t = thickness / 2.0;
    let px_min = (x0.min(x1) - half_t - 1.0).max(0.0) as u32;
    let py_min = (y0.min(y1) - half_t - 1.0).max(0.0) as u32;
    let px_max = (x0.max(x1) + half_t + 1.0).min(stride as f64 - 1.0) as u32;
    let py_max = (y0.max(y1) + half_t + 1.0).min(stride as f64 - 1.0) as u32;

    for py in py_min..=py_max {
        for px in px_min..=px_max {
            let fx = px as f64;
            let fy = py as f64;
            // Distance from point to line segment
            let t = ((fx - x0) * dx + (fy - y0) * dy) / (len * len);
            let t = t.clamp(0.0, 1.0);
            let closest_x = x0 + t * dx;
            let closest_y = y0 + t * dy;
            let dist = ((fx - closest_x).powi(2) + (fy - closest_y).powi(2)).sqrt();
            if dist <= half_t {
                let alpha = ((half_t - dist).min(1.0) * ca as f64) as u8;
                set_pixel(pixels, stride, px, py, cr, cg, cb, alpha);
            }
        }
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t.clamp(0.0, 1.0)) as u8
}

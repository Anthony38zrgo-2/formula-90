//! Diagnostic tool for BG3-009: skybox gradient rendering as visible square.
//!
//! Computes expected pixel_size and viewport coverage for a given camera/viewport
//! configuration, then reports whether the sprite would cover the full viewport.
//!
//! Usage:
//! ```
//! cargo run --bin bg_diagnose -- [--fov 70] [--viewport 1280x720] [--distance 800] [--tex 1x1]
//! ```

use skybox_engine::{pixel_size_from_camera, CameraState, KeepAspect, COVERAGE_FACTOR};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut fov = 70.0_f64;
    let mut viewport = (1280.0_f64, 720.0_f64);
    let mut distance = 800.0_f64;
    let mut tex_size = (1_i32, 1_i32);
    let mut keep_aspect = KeepAspect::Height;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--fov" => {
                i += 1;
                fov = args[i].parse().unwrap_or(70.0);
            }
            "--viewport" => {
                i += 1;
                let parts: Vec<&str> = args[i].split('x').collect();
                if parts.len() == 2 {
                    viewport.0 = parts[0].parse().unwrap_or(1280.0);
                    viewport.1 = parts[1].parse().unwrap_or(720.0);
                }
            }
            "--distance" => {
                i += 1;
                distance = args[i].parse().unwrap_or(800.0);
            }
            "--tex" => {
                i += 1;
                let parts: Vec<&str> = args[i].split('x').collect();
                if parts.len() == 2 {
                    tex_size.0 = parts[0].parse().unwrap_or(1);
                    tex_size.1 = parts[1].parse().unwrap_or(1);
                }
            }
            "--keep-width" => {
                keep_aspect = KeepAspect::Width;
            }
            _ => {}
        }
        i += 1;
    }

    let camera = CameraState {
        fov_deg: fov,
        keep_aspect,
        viewport_size: viewport,
        yaw_rad: 0.0,
        pitch_rad: 0.0,
    };

    let pixel_size = pixel_size_from_camera(&camera, distance, tex_size, COVERAGE_FACTOR);

    // Compute visible frustum dimensions at the sprite distance
    let fov_rad = fov.to_radians();
    let aspect = viewport.0 / viewport.1.max(0.001);
    let visible_fov = 2.0 * distance.abs() * (fov_rad * 0.5).tan();
    let (visible_h, visible_w) = match keep_aspect {
        KeepAspect::Height => (visible_fov, visible_fov * aspect),
        KeepAspect::Width => (visible_fov / aspect.max(0.001), visible_fov),
    };

    // Compute sprite world-space size
    let sprite_w = tex_size.0 as f64 * pixel_size;
    let sprite_h = tex_size.1 as f64 * pixel_size;

    // Coverage check
    let covers_w = sprite_w >= visible_w;
    let covers_h = sprite_h >= visible_h;
    let covers_full = covers_w && covers_h;

    println!("=== BG3-009 Skybox Diagnostic ===");
    println!();
    println!("Camera:");
    println!(
        "  FOV:          {:.1} deg ({})",
        fov,
        match keep_aspect {
            KeepAspect::Height => "KEEP_HEIGHT (vertical)",
            KeepAspect::Width => "KEEP_WIDTH (horizontal)",
        }
    );
    println!("  Viewport:     {:.0} x {:.0}", viewport.0, viewport.1);
    println!("  Aspect:       {:.4}", aspect);
    println!();
    println!("Skybox:");
    println!("  Distance:     {:.1}", distance);
    println!("  Texture:      {}x{}", tex_size.0, tex_size.1);
    println!("  pixel_size:   {:.4}", pixel_size);
    println!();
    println!("Frustum at distance {:.0}:", distance);
    println!("  Visible H:    {:.2}", visible_h);
    println!("  Visible W:    {:.2}", visible_w);
    println!();
    println!("Sprite world-space size:");
    println!("  Width:        {:.2}", sprite_w);
    println!("  Height:       {:.2}", sprite_h);
    println!();
    println!("Coverage:");
    println!(
        "  Width:        {} ({:.2} >= {:.2})",
        if covers_w { "OK" } else { "FAIL" },
        sprite_w,
        visible_w
    );
    println!(
        "  Height:       {} ({:.2} >= {:.2})",
        if covers_h { "OK" } else { "FAIL" },
        sprite_h,
        visible_h
    );
    println!(
        "  Full screen:  {}",
        if covers_full {
            "YES"
        } else {
            "NO - EDGES VISIBLE"
        }
    );

    if !covers_full {
        println!();
        println!("[BG3-009] The sprite does NOT cover the full viewport.");
        println!("  This would cause the gradient to appear as a visible square.");
        println!(
            "  Required pixel_size for full coverage: {:.4}",
            pixel_size_from_camera(&camera, distance, tex_size, 1.0)
        );
    } else {
        println!();
        println!("[OK] The sprite should cover the full viewport.");
        println!("  If the gradient still appears as a square, the issue is NOT pixel_size.");
        println!("  Check: render_priority, depth test, BG_CANVAS background mode,");
        println!("  or shader SCREEN_UV behavior on Sprite3D.");
    }
}

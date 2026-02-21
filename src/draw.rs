use ab_glyph::{FontRef, PxScale};
use image::{DynamicImage, GenericImage, Pixel, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use libblur::{
    ConvolutionMode, EdgeMode, EdgeMode2D, GaussianBlurParams, ThreadingPolicy, gaussian_blur_image,
};
use resvg::{
    tiny_skia::{self, Pixmap},
    usvg::{self, Options},
};

const TITLEBAR_H: f32 = 52.0;
const R: f32 = 10.0;

const BG_SCALE_H: f32 = 1.185;
const BG_SCALE_W: f32 = 1.115;

pub fn frame(img: &RgbaImage, scale: f32, title: Option<String>) -> RgbaImage {
    let mut canvas = img.clone();

    canvas = draw_titlebar(&mut canvas, scale, title);
    draw_rounded_rect(&mut canvas, R, scale);

    canvas = draw_bg(&canvas);
    canvas
}

fn draw_titlebar(img: &RgbaImage, scale: f32, title: Option<String>) -> RgbaImage {
    let titlebar_h = (TITLEBAR_H * scale).round() as u32;

    let (w, h) = img.dimensions();
    let mut expanded = RgbaImage::new(w, h + titlebar_h);

    for y in 0..titlebar_h {
        for x in 0..w {
            expanded.put_pixel(x, y, Rgba([0, 0, 0, 255]));
        }
    }

    let btns_w = (92.0 * scale).round() as u32;
    // let btns_w = 173.0;
    let btns_h = (12.0 * scale).round() as u32;
    // let btns_h = 22.0;

    let btns = render_svg_to_rgba(btns_w, btns_h);

    let bx = 0.0f32.round() as u32;
    let by = (titlebar_h - btns_h) / 2;

    for y in 0..btns_h {
        for x in 0..btns_w {
            let btn_px = btns.get_pixel(x, y);
            match btn_px.alpha() {
                255 => *expanded.get_pixel_mut(x + bx, y + by) = *btn_px,
                0 => {}
                _ => {
                    let bg_px = expanded.get_pixel(x + bx, y + by);
                    let out_px = blend_over(*bg_px, *btn_px);
                    *expanded.get_pixel_mut(x + bx, y + by) = out_px
                }
            }
            // if btn_px.alpha() != 0 {
            //     expanded.put_pixel(bx + x, by + y, *btn_px);
            // }
        }
    }

    if let Some(title) = title {
        let font_bytes = include_bytes!("../assets/SFPRODISPLAYREGULAR.OTF");
        let font = FontRef::try_from_slice(font_bytes).unwrap();

        let font_size = 20.0 * scale;
        let font_scale = PxScale::from(font_size);

        draw_text_mut(
            &mut expanded,
            Rgba([255, 255, 255, 255]),
            (w / 2 - (font_size * 3.5).round() as u32) as i32,
            (titlebar_h / 2 - (font_size.round() as u32) / 2) as i32,
            font_scale,
            &font,
            &title,
        );
    }

    expanded.copy_from(img, 0, titlebar_h).unwrap();
    expanded
}

fn draw_bg(src: &RgbaImage) -> RgbaImage {
    let (w, h) = src.dimensions();

    let bg_w = (w as f32 * BG_SCALE_W).floor() as u32;
    let bg_h = (h as f32 * BG_SCALE_H).floor() as u32;
    let mut bg = RgbaImage::new(bg_w, bg_h);

    for y in 0..bg_h {
        let t = y as f32 / bg_h as f32;
        // let v = (30.0 + t * 40.0) as u8;
        let v = (220.0 + t * 250.0) as u8;

        for x in 0..bg_w {
            let px = bg.get_pixel_mut(x, y);
            *px = Rgba([v, v, v, 255]);
        }
    }

    let shadow = shadow(src, 20.0);
    let (sw, sh) = shadow.dimensions();

    let off_x = (bg_w - w) / 2;
    let off_y = (bg_h - h) / 2;

    let shadow_off_x = off_x as i32 - (sw as i32 - w as i32) / 2 + 2;
    let shadow_off_y = off_y as i32 - (sh as i32 - h as i32) / 2 + 2;

    for y in 0..sh {
        for x in 0..sw {
            let px = shadow.get_pixel(x, y);
            let a = px.alpha();
            if a == 0 {
                continue;
            }

            let dx = shadow_off_x + x as i32;
            let dy = shadow_off_y + y as i32;
            if dx < 0 || dy < 0 || dx >= bg_w as i32 || dy >= bg_h as i32 {
                continue;
            }

            let bg_px = bg.get_pixel(dx as u32, dy as u32);
            let out_px = blend_over(*bg_px, *px);
            *bg.get_pixel_mut(dx as u32, dy as u32) = out_px;
        }
    }

    for y in 0..h {
        for x in 0..w {
            let px = src.get_pixel(x, y);
            match px.alpha() {
                255 => *bg.get_pixel_mut(x + off_x, y + off_y) = *px,
                0 => {}
                _ => {
                    let bg_px = bg.get_pixel(x + off_x, y + off_y);
                    let out_px = blend_over(*bg_px, *px);
                    *bg.get_pixel_mut(x + off_x, y + off_y) = out_px
                }
            }
        }
    }

    bg
}

fn draw_rounded_rect(canvas: &mut RgbaImage, radius_float: f32, scale: f32) {
    let left = 0;
    let top = 0;
    let right = canvas.width();
    let bottom = canvas.height();

    let radius = (radius_float * scale).round() as u32;

    for y in top..bottom {
        for x in left..right {
            let mut draw = true;

            // top-left
            if x < left + radius && y < top + radius {
                let cx = left + radius;
                let cy = top + radius;

                if !in_corner_circle(x, y, radius, cx, cy) {
                    draw = false;
                }
            }

            // top-right
            if x >= right - radius && y < top + radius {
                let cx = right - radius;
                let cy = top + radius;

                if !in_corner_circle(x, y, radius, cx, cy) {
                    draw = false;
                }
            }

            // bottom-left
            if x < left + radius && y >= bottom - radius {
                let cx = left + radius;
                let cy = bottom - radius;

                if !in_corner_circle(x, y, radius, cx, cy) {
                    draw = false;
                }
            }

            // bottom-right
            if x >= right - radius && y >= bottom - radius {
                let cx = right - radius;
                let cy = bottom - radius;

                if !in_corner_circle(x, y, radius, cx, cy) {
                    draw = false;
                }
            }

            if !draw {
                let px = canvas.get_pixel_mut(x, y);
                *px = Rgba([0, 0, 0, 0]);
            }
        }
    }
}

fn render_svg_to_rgba(w: u32, h: u32) -> RgbaImage {
    let svg_data = include_assets();

    let opt = Options::default();
    let tree = usvg::Tree::from_str(&String::from_utf8_lossy(&svg_data), &opt).unwrap();

    let size = tree.size();
    let sx = w as f32 / size.width();
    let sy = h as f32 / size.height();
    let transform = tiny_skia::Transform::from_scale(sx, sy);

    let mut pixmap = Pixmap::new(w, h).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut img = RgbaImage::new(w, h);
    let data = pixmap.data();

    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            img.put_pixel(x, y, Rgba([data[i], data[i + 1], data[i + 2], data[i + 3]]));
        }
    }

    img
}

fn shadow(img: &RgbaImage, sigma: f32) -> RgbaImage {
    let (w, h) = img.dimensions();

    let margin = (sigma * 3.0).ceil() as u32;
    let sw = w + margin * 2;
    let sh = h + margin * 2;

    let mut shadow = RgbaImage::new(sw, sh);

    for y in 0..h {
        for x in 0..w {
            let a = img.get_pixel(x, y).alpha();
            if a == 0 {
                continue;
            }
            shadow.put_pixel(x + margin, y + margin, Rgba([0, 0, 0, a]));
        }
    }

    let dyn_img = DynamicImage::ImageRgba8(shadow);
    // let mut blurred = dyn_img.blur(sigma).to_rgba8();
    // let blurred = gaussian_blur(shadow, dst, params, edge_modes, threading_policy, hint)
    // let mut dst_img = BlurImageMut::borrow(
    //     &mut shado2w.as_mut(),
    //     w,
    //     h,
    //     libblur::FastBlurChannels::Channels4,
    // );
    // libblur::fast_gaussian(
    //     &mut dst_img,
    //     AnisotropicRadius::new(sigma as u32),
    //     libblur::ThreadingPolicy::Single,
    //     EdgeMode2D::new(EdgeMode::Wrap),
    // )
    // .unwrap();
    //
    let blurred = gaussian_blur_image(
        dyn_img,
        GaussianBlurParams::new(31, sigma as f64),
        EdgeMode2D::new(EdgeMode::Clamp),
        ConvolutionMode::FixedPoint,
        ThreadingPolicy::Adaptive,
    )
    .unwrap();

    // for p in blurred.pixels_mut() {
    //     if p[3] < 8 {
    //         p[3] = 0;
    //     }
    // }

    blurred.as_rgba8().unwrap().clone()
}

fn blend_over(bg: Rgba<u8>, fg: Rgba<u8>) -> Rgba<u8> {
    let a = fg.alpha() as f32 / 255.0;

    let r = (fg[0] as f32 * a + bg[0] as f32 * (1.0 - a)) as u8;
    let g = (fg[1] as f32 * a + bg[1] as f32 * (1.0 - a)) as u8;
    let b = (fg[2] as f32 * a + bg[2] as f32 * (1.0 - a)) as u8;

    Rgba([r, g, b, 255])
}

fn in_corner_circle(x: u32, y: u32, r: u32, cx: u32, cy: u32) -> bool {
    let dx = x as i32 - cx as i32;
    let dy = y as i32 - cy as i32;
    (dx * dx + dy * dy) <= (r * r) as i32
}

fn include_assets() -> [u8; 640] {
    include_bytes!("../assets/WindowControls.svg").clone()
}

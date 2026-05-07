use prism::canvas::{Image, ShapeType, Color};
use image::{RgbaImage, Rgba, imageops};
use std::io::Cursor;

pub mod animated_sprite;
pub use animated_sprite::{AnimatedSprite, RotationOptions, RotationDirection};


pub fn solid_circle(size: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::RoundedRectangle(0.0, (size, size), 0.0, size * 0.5),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn solid_ellipse(w: f32, h: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::Ellipse(0.0, (w, h), 0.0),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn planet_image(radius: u32, r: u8, g: u8, b: u8, size: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: generate_planet_rgba(radius, r, g, b, 1.0).into(),
        color: None,
    }
}

pub fn planet_grayscale(radius: u32, size: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: generate_planet_rgba(radius, 255, 255, 255, 1.0).into(),
        color: None,
    }
}

pub fn with_tint(image: &Image, color: Color) -> Image {
    Image {
        shape: image.shape.clone(),
        image: image.image.clone(),
        color: Some(color),
    }
}

pub fn planet_atmosphere(radius: u32, r: u8, g: u8, b: u8, atmosphere: f32, size: f32) -> Image {
    let rf = radius as f32;
    let atm_px = rf * atmosphere.clamp(0.0, 1.0);
    let outer_r = rf + atm_px;
    let diameter = (outer_r * 2.0).ceil().max(1.0) as u32;
    let mut img = RgbaImage::new(diameter, diameter);
    let cx = outer_r;

    for py in 0..diameter {
        for px in 0..diameter {
            let dx = px as f32 - cx + 0.5;
            let dy = py as f32 - cx + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            let (alpha, brightness) = if dist <= rf {
                let rim = ((rf - dist) / rf).min(1.0);
                (255u8, 0.7 + 0.3 * rim)
            } else if atm_px > 0.0 && dist <= rf + atm_px {
                let t = (dist - rf) / atm_px;
                let alpha = ((1.0 - t) * 180.0) as u8;
                (alpha, 0.6 + 0.15 * (1.0 - t))
            } else {
                continue;
            };

            img.put_pixel(px, py, Rgba([
                (r as f32 * brightness).min(255.0) as u8,
                (g as f32 * brightness).min(255.0) as u8,
                (b as f32 * brightness).min(255.0) as u8,
                alpha,
            ]));
        }
    }

    Image {
        shape: ShapeType::Rectangle(0.0, (size, size), 0.0),
        image: img.into(),
        color: None,
    }
}

pub fn glow_ring(w: f32, h: f32, ring_width: f32, corner_radius: f32, color: Color) -> Image {
    let total_w = w + 2.0 * ring_width;
    let total_h = h + 2.0 * ring_width;
    Image {
        shape: ShapeType::RoundedRectangle(
            ring_width,
            (total_w, total_h),
            0.0,
            corner_radius + ring_width * 0.5,
        ),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub fn tint_overlay(w: f32, h: f32, color: Color) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255])).into(),
        color: Some(color),
    }
}

pub(crate) fn generate_planet_rgba(radius: u32, r: u8, g: u8, b: u8, brightness_scale: f32) -> RgbaImage {
    let diameter = radius * 2;
    let mut img = RgbaImage::new(diameter, diameter);
    let cx = radius as f32;
    let rf = radius as f32;

    for py in 0..diameter {
        for px in 0..diameter {
            let dx = px as f32 - cx + 0.5;
            let dy = py as f32 - cx + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > rf { continue; }

            let rim = ((rf - dist) / rf).min(1.0);
            let brightness = (0.7 + 0.3 * rim) * brightness_scale;

            img.put_pixel(px, py, Rgba([
                (r as f32 * brightness).min(255.0) as u8,
                (g as f32 * brightness).min(255.0) as u8,
                (b as f32 * brightness).min(255.0) as u8,
                255,
            ]));
        }
    }

    img
}

pub fn load_image(bytes: &[u8]) -> Image {
    let rgba = image::load_from_memory(bytes)
        .expect("quartz: cannot decode image from bytes")
        .into_rgba8();
    let (w, h) = (rgba.width() as f32, rgba.height() as f32);
    make_image(rgba, w, h)
}

pub fn load_image_sized(bytes: &[u8], w: f32, h: f32) -> Image {
    let rgba = image::load_from_memory(bytes)
        .expect("quartz: cannot decode image from bytes")
        .into_rgba8();
    make_image(rgba, w, h)
}

pub fn load_animation(bytes: &[u8], size: (f32, f32), fps: f32) -> AnimatedSprite {
    AnimatedSprite::decode_vec(bytes.to_vec(), size, fps)
        .expect("quartz: failed to decode animation from bytes")
}

pub fn flip_horizontal(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let flipped = imageops::flip_horizontal(&pixels);
    make_image(flipped, w, h)
}

pub fn flip_vertical(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let flipped = imageops::flip_vertical(&pixels);
    make_image(flipped, w, h)
}

pub fn rotate_cw(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate270(&pixels);
    make_image(rotated, h, w)
}

pub fn rotate_ccw(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate90(&pixels);
    make_image(rotated, h, w)
}

pub fn rotate_180(img: Image) -> Image {
    let (pixels, w, h) = extract(img);
    let rotated = imageops::rotate180(&pixels);
    make_image(rotated, w, h)
}

fn extract(img: Image) -> (RgbaImage, f32, f32) {
    let (w, h) = match img.shape {
        ShapeType::Rectangle(_, size, _) => size,
        _ => panic!("image transform: expected a Rectangle shape"),
    };
    let pixels: RgbaImage = (*img.image).clone();
    (pixels, w, h)
}

pub(crate) fn make_image(pixels: RgbaImage, w: f32, h: f32) -> Image {
    Image {
        shape: ShapeType::Rectangle(0.0, (w, h), 0.0),
        image: pixels.into(),
        color: None,
    }
}

pub fn star_field(width: u32, height: u32, star_count: u32, seed: u64) -> Image {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([5, 5, 15, 255]));

    let mut state = seed.max(1);
    let mut next = || -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..star_count {
        let x = (next() % width as u64) as u32;
        let y = (next() % height as u64) as u32;
        let brightness = 100 + (next() % 156) as u8;
        let size_roll = next() % 100;
        let radius = if size_roll < 70 { 0 } else if size_roll < 92 { 1 } else { 2 };

        for dy in 0..=radius * 2 {
            for dx in 0..=radius * 2 {
                let px = x as i32 + dx as i32 - radius as i32;
                let py = y as i32 + dy as i32 - radius as i32;
                if px >= 0 && py >= 0 && (px as u32) < width && (py as u32) < height {
                    let dist = ((dx as f32 - radius as f32).powi(2)
                              + (dy as f32 - radius as f32).powi(2)).sqrt();
                    if dist <= radius as f32 + 0.5 {
                        let falloff = 1.0 - (dist / (radius as f32 + 1.0));
                        let b = (brightness as f32 * falloff).min(255.0) as u8;
                        img.put_pixel(px as u32, py as u32, Rgba([b, b, b.saturating_add(20), 255]));
                    }
                }
            }
        }
    }

    Image {
        shape: ShapeType::Rectangle(0.0, (width as f32, height as f32), 0.0),
        image: img.into(),
        color: None,
    }
}
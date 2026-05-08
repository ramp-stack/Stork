use prism::canvas::{Image, ShapeType};
use image::{RgbaImage, AnimationDecoder, imageops};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Copy)]
pub struct RotationOptions {
    pub degrees:   f32,
    pub direction: RotationDirection,
}

impl RotationOptions {
    pub fn clockwise(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::Clockwise }
    }
    pub fn counter_clockwise(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::CounterClockwise }
    }
    pub fn degrees(degrees: f32) -> Self {
        Self { degrees, direction: RotationDirection::Clockwise }
    }
    pub(crate) fn to_radians(self) -> f32 {
        let r = self.degrees.to_radians();
        match self.direction {
            RotationDirection::Clockwise        =>  r,
            RotationDirection::CounterClockwise => -r,
        }
    }
}

impl Default for RotationOptions {
    fn default() -> Self {
        Self { degrees: 0.0, direction: RotationDirection::Clockwise }
    }
}

#[derive(Clone)]
pub struct AnimatedSprite {
    frames:                Vec<RgbaImage>,
    current_frame:         usize,
    frame_duration:        f32,
    time_since_last_frame: f32,
    size:                  (f32, f32),
    mirrored_h:            bool,
    mirrored_v:            bool,
    rotation:              RotationOptions,
    /// When true, `get_current_image` and `update_animation` use
    /// ShapeType::Ellipse instead of Rectangle. Set for circular objects
    /// (gravity wells, black holes) so their glow is round, not square.
    pub use_ellipse:       bool,
}

impl AnimatedSprite {
    pub fn new(gif_bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        Self::decode_slice(gif_bytes, size, fps)
    }

    pub(crate) fn decode_vec(bytes: Vec<u8>, size: (f32, f32), fps: f32) -> Result<Self, String> {
        Self::decode_slice(&bytes, size, fps)
    }

    fn decode_slice(bytes: &[u8], size: (f32, f32), fps: f32) -> Result<Self, String> {
        let cursor  = Cursor::new(bytes);
        let decoder = image::codecs::gif::GifDecoder::new(cursor)
            .map_err(|e| format!("Failed to decode GIF: {}", e))?;
        let mut frames = Vec::new();
        for frame_result in decoder.into_frames() {
            let frame = frame_result
                .map_err(|e| format!("Failed to decode frame: {}", e))?;
            frames.push(frame.into_buffer());
        }
        if frames.is_empty() {
            return Err("GIF has no frames".to_string());
        }

        let tw = size.0.round().max(1.0) as u32;
        let th = size.1.round().max(1.0) as u32;
        frames = frames.into_iter().map(|f| {
            let fw = f.width();
            let fh = f.height();
            if fw == tw && fh == th { return f; }

            let scale = (tw as f32 / fw as f32).min(th as f32 / fh as f32);
            let rw = (fw as f32 * scale).round().max(1.0) as u32;
            let rh = (fh as f32 * scale).round().max(1.0) as u32;
            let resized = imageops::resize(&f, rw, rh, imageops::FilterType::Nearest);

            let mut canvas = RgbaImage::from_pixel(tw, th, image::Rgba([0, 0, 0, 0]));
            let ox = tw.saturating_sub(rw) / 2;
            let oy = th.saturating_sub(rh) / 2;
            imageops::overlay(&mut canvas, &resized, ox as i64, oy as i64);
            canvas
        }).collect();

        Ok(Self::from_frames(frames, size, fps))
    }

    pub fn from_frames(frames: Vec<RgbaImage>, size: (f32, f32), fps: f32) -> Self {
        assert!(!frames.is_empty(), "AnimatedSprite::from_frames requires at least one frame");
        Self {
            frames,
            current_frame:         0,
            frame_duration:        1.0 / fps,
            time_since_last_frame: 0.0,
            size,
            mirrored_h:            false,
            mirrored_v:            false,
            rotation:              RotationOptions::default(),
            use_ellipse:           false,
        }
    }

    /// When `true`, this animation renders with a round (Ellipse) shape
    /// instead of a rectangle.  Set this before calling `set_glow` so
    /// that the glow outline matches the circular appearance.
    pub fn set_ellipse_shape(&mut self, v: bool) { self.use_ellipse = v; }

    pub fn fps(&self) -> f32 { 1.0 / self.frame_duration }

    pub fn update(&mut self, delta_time: f32) {
        self.time_since_last_frame += delta_time;
        while self.time_since_last_frame >= self.frame_duration {
            self.time_since_last_frame -= self.frame_duration;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    pub fn get_current_image(&self) -> Image {
        let mut pixels = self.frames[self.current_frame].clone();
        if self.mirrored_h { pixels = imageops::flip_horizontal(&pixels); }
        if self.mirrored_v { pixels = imageops::flip_vertical(&pixels); }
        let shape = if self.use_ellipse {
            ShapeType::Ellipse(0.0, self.size, self.rotation.to_radians())
        } else {
            ShapeType::Rectangle(0.0, self.size, self.rotation.to_radians())
        };
        Image {
            shape,
            image: pixels.into(),
            color: None,
        }
    }

    pub fn set_fps(&mut self, fps: f32) { self.frame_duration = 1.0 / fps; }

    pub fn reset(&mut self) {
        self.current_frame         = 0;
        self.time_since_last_frame = 0.0;
    }

    pub fn frame_count(&self) -> usize { self.frames.len() }

    pub fn set_frame(&mut self, frame: usize) {
        if frame < self.frames.len() {
            self.current_frame         = frame;
            self.time_since_last_frame = 0.0;
        }
    }

    pub fn mirror(&mut self)                         { self.mirrored_h = !self.mirrored_h; }
    pub fn set_mirrored(&mut self, v: bool)          { self.mirrored_h = v; }
    pub fn is_mirrored(&self) -> bool                { self.mirrored_h }
    pub fn mirror_vertical(&mut self)                { self.mirrored_v = !self.mirrored_v; }
    pub fn set_mirrored_vertical(&mut self, v: bool) { self.mirrored_v = v; }
    pub fn is_mirrored_vertical(&self) -> bool       { self.mirrored_v }

    pub fn set_rotation(&mut self, options: RotationOptions) { self.rotation = options; }

    pub fn rotate_by(&mut self, options: RotationOptions) {
        let new_rad = self.rotation.to_radians() + options.to_radians();
        self.rotation = RotationOptions {
            degrees:   new_rad.to_degrees(),
            direction: RotationDirection::Clockwise,
        };
    }

    pub fn clear_rotation(&mut self)      { self.rotation = RotationOptions::default(); }
    pub fn rotation_degrees(&self) -> f32 { self.rotation.to_radians().to_degrees() }

    pub fn rotate_90_cw(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate270(f)).collect();
        self.size = (self.size.1, self.size.0);
    }

    pub fn rotate_90_ccw(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate90(f)).collect();
        self.size = (self.size.1, self.size.0);
    }

    pub fn rotate_180(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::rotate180(f)).collect();
    }

    /// Bake a vertical pixel-flip into every frame permanently.
    /// Call once at bootstrap to pre-compute a flipped variant; zero per-frame cost.
    pub fn flip_vertical_frames(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::flip_vertical(f)).collect();
    }

    /// Bake a horizontal pixel-flip into every frame permanently.
    /// Call once at bootstrap to pre-compute a mirrored variant; zero per-frame cost.
    pub fn flip_horizontal_frames(&mut self) {
        self.frames = self.frames.iter().map(|f| imageops::flip_horizontal(f)).collect();
    }
}

impl std::fmt::Debug for AnimatedSprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimatedSprite")
            .field("frame_count",    &self.frames.len())
            .field("current_frame",  &self.current_frame)
            .field("frame_duration", &self.frame_duration)
            .field("size",           &self.size)
            .field("mirrored_h",     &self.mirrored_h)
            .field("mirrored_v",     &self.mirrored_v)
            .field("rotation",       &self.rotation)
            .finish()
    }
}

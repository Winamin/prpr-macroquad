//! Loading and rendering textures. Also render textures, per-pixel image manipulations.

use crate::{
    color::Color,
    file::{load_file, FileError},
    get_context, get_quad_context,
    math::Rect,
};

use crate::quad_gl::{DrawMode, Vertex};
use glam::{vec2, Vec2};
use std::collections::HashMap;

pub use crate::quad_gl::FilterMode;

/// Image, data stored in CPU memory
#[derive(Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy)]
struct SpriteDrawData {
    positions: [Vec2; 4],
    uv: [(f32, f32); 4],
    color: Color,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes.len()", &self.bytes.len())
            .finish()
    }
}

pub(crate) struct SpriteBatcher { batches: HashMap<u32, (miniquad::Texture, Vec<SpriteDrawData>)>, }

impl SpriteBatcher {
    pub fn new() -> Self {
        Self {
            batches: HashMap::new(),
        }
    }

    pub fn add_sprite(&mut self, texture: Texture2D, data: SpriteDrawData) {
        let raw = texture.raw_miniquad_texture_handle();
        let key = raw.gl_internal_id();
        self.batches
            .entry(key)
            .or_insert_with(|| (raw, Vec::new()))
            .1
            .push(data);
    }

    pub fn flush(&mut self, gl: &mut crate::quad_gl::QuadGl) {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();

        for (_key, (texture, sprites)) in self.batches.drain() {
            all_vertices.clear();
            all_indices.clear();

            for sprite in &sprites {
                let base_index = all_vertices.len() as u16;

                all_vertices.push(Vertex::new(
                    sprite.positions[0].x, sprite.positions[0].y, 0.,
                    sprite.uv[0].0, sprite.uv[0].1, sprite.color
                ));
                all_vertices.push(Vertex::new(
                    sprite.positions[1].x, sprite.positions[1].y, 0.,
                    sprite.uv[1].0, sprite.uv[1].1, sprite.color
                ));
                all_vertices.push(Vertex::new(
                    sprite.positions[2].x, sprite.positions[2].y, 0.,
                    sprite.uv[2].0, sprite.uv[2].1, sprite.color
                ));
                all_vertices.push(Vertex::new(
                    sprite.positions[3].x, sprite.positions[3].y, 0.,
                    sprite.uv[3].0, sprite.uv[3].1, sprite.color
                ));
                all_indices.extend_from_slice(&[
                    base_index, base_index + 1, base_index + 2,
                    base_index, base_index + 2, base_index + 3,
                ]);
            }
            if !all_vertices.is_empty() {
                gl.texture(Some(Texture2D::from_miniquad_texture(texture)));
                gl.draw_mode(DrawMode::Triangles);
                gl.geometry(&all_vertices, &all_indices);
            }
        }
    }
}

impl Image {
    /// Creates an empty Image.
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// let image = Image::empty();
    /// ```
    pub fn empty() -> Image {
        Image {
            width: 0,
            height: 0,
            bytes: vec![],
        }
    }

    /// Creates an Image from a slice of bytes that contains an encoded image.
    ///
    /// If `format` is None, it will make an educated guess on the
    /// [ImageFormat][image::ImageFormat].
    ///
    /// # Example
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// let icon = Image::from_file_with_format(
    ///     include_bytes!("../examples/rust.png"),
    ///     Some(ImageFormat::Png),
    ///     );
    /// ```
    pub fn from_file_with_format(bytes: &[u8], format: Option<image::ImageFormat>) -> Image {
        let img = if let Some(fmt) = format {
            image::load_from_memory_with_format(bytes, fmt)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        } else {
            image::load_from_memory(bytes)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        };
        let width = img.width() as u16;
        let height = img.height() as u16;
        let bytes = img.into_raw();

        Image {
            width,
            height,
            bytes,
        }
    }

    /// Creates an Image filled with the provided [Color].
    pub fn gen_image_color(width: u16, height: u16, color: Color) -> Image {
        let pixel_count = width as usize * height as usize;
        let mut bytes = Vec::with_capacity(pixel_count * 4);
        let color_bytes = [
            (color.r * 255.) as u8,
            (color.g * 255.) as u8,
            (color.b * 255.) as u8,
            (color.a * 255.) as u8,
        ];
        bytes.extend_from_slice(&color_bytes);
        while bytes.len() < bytes.capacity() {
            let remaining = bytes.capacity() - bytes.len();
            let copy_len = remaining.min(bytes.len());
            bytes.extend_from_within(..copy_len);
        }

        Image { width, height, bytes }
    }

    /// Updates this image from a slice of [Color]s.
    pub fn update(&mut self, colors: &[Color]) {
        assert!(self.width as usize * self.height as usize == colors.len());
        let target_len = colors.len() * 4;
        if self.bytes.len() != target_len {
            self.bytes.resize(target_len, 0);
        }

        // 使用迭代器处理，利于自动向量化
        for (chunk, color) in self.bytes.chunks_exact_mut(4).zip(colors.iter()) {
            chunk[0] = (color.r * 255.) as u8;
            chunk[1] = (color.g * 255.) as u8;
            chunk[2] = (color.b * 255.) as u8;
            chunk[3] = (color.a * 255.) as u8;
        }
    }

    /// Returns the width of this image.
    pub fn width(&self) -> usize {
        self.width as usize
    }

    /// Returns the height of this image.
    pub fn height(&self) -> usize {
        self.height as usize
    }

    /// Returns this image's data as a slice of 4-byte arrays.
    pub fn get_image_data(&self) -> &[[u8; 4]] {
        use std::slice;

        unsafe {
            slice::from_raw_parts(
                self.bytes.as_ptr() as *const [u8; 4],
                self.width as usize * self.height as usize,
            )
        }
    }

    /// Returns this image's data as a mutable slice of 4-byte arrays.
    pub fn get_image_data_mut(&mut self) -> &mut [[u8; 4]] {
        use std::slice;

        unsafe {
            slice::from_raw_parts_mut(
                self.bytes.as_mut_ptr() as *mut [u8; 4],
                self.width as usize * self.height as usize,
            )
        }
    }

    /// Modifies a pixel [Color] in this image.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        let width = self.width;

        self.get_image_data_mut()[(y * width as u32 + x) as usize] = color.into();
    }

    /// Returns a pixel [Color] from this image.
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        self.get_image_data()[(y * self.width as u32 + x) as usize].into()
    }

    /// Returns an Image from a rect inside this image.
    pub fn sub_image(&self, rect: Rect) -> Image {
        let width = rect.w as usize;
        let height = rect.h as usize;
        let mut bytes = vec![0; width * height * 4];

        let x_start = rect.x as usize;
        let y_start = rect.y as usize;
        let src_stride = self.width as usize * 4;
        let dst_stride = width * 4;
        unsafe {
            let src_ptr = self.bytes.as_ptr();
            let dst_ptr = bytes.as_mut_ptr();

            for y in 0..height {
                let src_offset = (y_start + y) * src_stride + x_start * 4;
                let dst_offset = y * dst_stride;

                std::ptr::copy_nonoverlapping(
                    src_ptr.add(src_offset),
                    dst_ptr.add(dst_offset),
                    dst_stride,
                );
            }
        }
        Image {
            width: width as u16,
            height: height as u16,
            bytes,
        }
    }

    /// Saves this image as a PNG file.
    pub fn export_png(&self, path: &str) {
        let width = self.width as usize;
        let height = self.height as usize;
        let mut bytes = vec![0; width * height * 4];
        let row_len = width * 4;

        unsafe {
            let src_ptr = self.bytes.as_ptr();
            let dst_ptr = bytes.as_mut_ptr();

            for y in 0..height {
                let src_row = (height - y - 1) * row_len;
                let dst_row = y * row_len;

                std::ptr::copy_nonoverlapping(
                    src_ptr.add(src_row),
                    dst_ptr.add(dst_row),
                    row_len,
                );
            }
        }

        image::save_buffer(
            path,
            &bytes,
            self.width as _,
            self.height as _,
            image::ColorType::Rgba8,
        ).unwrap();
    }
}

/// Loads an [Image] from a file into CPU memory.
pub async fn load_image(path: &str) -> Result<Image, FileError> {
    let bytes = load_file(path).await?;

    Ok(Image::from_file_with_format(&bytes, None))
}

/// Loads a [Texture2D] from a file into GPU memory.
pub async fn load_texture(path: &str) -> Result<Texture2D, FileError> {
    let bytes = load_file(path).await?;

    Ok(Texture2D::from_file_with_format(&bytes[..], None))
}

#[derive(Clone, Copy, Debug)]
pub struct RenderTarget {
    pub texture: Texture2D,
    pub render_pass: miniquad::RenderPass,
}

impl RenderTarget {
    pub fn delete(&self) {
        self.texture.delete();

        let context = get_quad_context();
        self.render_pass.delete(context);
    }
}

pub fn render_target(width: u32, height: u32) -> RenderTarget {
    let context = get_quad_context();

    let texture = miniquad::Texture::new_render_texture(
        context,
        miniquad::TextureParams {
            width,
            height,
            ..Default::default()
        },
    );

    let render_pass = miniquad::RenderPass::new(context, texture, None);

    let texture = Texture2D::from_miniquad_texture(texture);

    RenderTarget {
        texture,
        render_pass,
    }
}

#[derive(Debug, Clone)]
pub struct DrawTextureParams {
    pub dest_size: Option<Vec2>,

    /// Part of texture to draw. If None - draw the whole texture.
    /// Good use example: drawing an image from texture atlas.
    /// Is None by default
    pub source: Option<Rect>,

    /// Rotation in radians
    pub rotation: f32,

    /// Mirror on the X axis
    pub flip_x: bool,

    /// Mirror on the Y axis
    pub flip_y: bool,

    /// Rotate around this point.
    /// When `None`, rotate around the texture's center.
    /// When `Some`, the coordinates are in screen-space.
    /// E.g. pivot (0,0) rotates around the top left corner of the screen, not of the
    /// texture.
    pub pivot: Option<Vec2>,
}

impl Default for DrawTextureParams {
    fn default() -> DrawTextureParams {
        DrawTextureParams {
            dest_size: None,
            source: None,
            rotation: 0.,
            pivot: None,
            flip_x: false,
            flip_y: false,
        }
    }
}

pub fn draw_texture(texture: Texture2D, x: f32, y: f32, color: Color) {
    draw_texture_ex(texture, x, y, color, Default::default());
}

pub fn draw_texture_ex(
    texture: Texture2D,
    x: f32,
    y: f32,
    color: Color,
    params: DrawTextureParams,
) {
    let context = get_context();
    let batcher = &mut context.sprite_batcher;

    let tex_w = texture.width();
    let tex_h = texture.height();

    let Rect { x: mut sx, y: mut sy, w: mut sw, h: mut sh } =
        params.source.unwrap_or(Rect { x: 0., y: 0., w: tex_w, h: tex_h });

    let mut texture = texture;

    if let Some((batched_texture, uv)) = context.texture_batcher.get(texture) {
        let batch_w = batched_texture.width();
        let batch_h = batched_texture.height();

        sx = ((sx / tex_w) * uv.w + uv.x) * batch_w;
        sy = ((sy / tex_h) * uv.h + uv.y) * batch_h;
        sw = (sw / tex_w) * uv.w * batch_w;
        sh = (sh / tex_h) * uv.h * batch_h;

        texture = batched_texture;
    }

    let (sin_r, cos_r) = params.rotation.sin_cos();

    let (mut w, mut h) = params
        .dest_size
        .map(|dst| (dst.x, dst.y))
        .unwrap_or((sw, sh));

    let mut draw_x = x;
    let mut draw_y = y;
    if params.flip_x {
        draw_x += w;
        w = -w;
    }
    if params.flip_y {
        draw_y += h;
        h = -h;
    }

    let pivot = params
        .pivot
        .unwrap_or_else(|| vec2(draw_x + w / 2., draw_y + h / 2.));

    let rotate = |v: Vec2| -> Vec2 {
        let rel = v - pivot;
        vec2(
            cos_r * rel.x - sin_r * rel.y + pivot.x,
            sin_r * rel.x + cos_r * rel.y + pivot.y,
        )
    };

    let positions = [
        rotate(vec2(draw_x, draw_y)),
        rotate(vec2(draw_x + w, draw_y)),
        rotate(vec2(draw_x + w, draw_y + h)),
        rotate(vec2(draw_x, draw_y + h)),
    ];

    let tex_w = texture.width();
    let tex_h = texture.height();
    let uv = [
        (sx / tex_w, sy / tex_h),
        ((sx + sw) / tex_w, sy / tex_h),
        ((sx + sw) / tex_w, (sy + sh) / tex_h),
        (sx / tex_w, (sy + sh) / tex_h),
    ];

    batcher.add_sprite(texture, SpriteDrawData {
        positions,
        uv,
        color,
    });
}

#[deprecated(since = "0.3.0", note = "Use draw_texture_ex instead")]
pub fn draw_texture_rec(
    texture: Texture2D,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    sx: f32,
    sy: f32,
    sw: f32,
    sh: f32,
    color: Color,
) {
    draw_texture_ex(
        texture,
        x,
        y,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            source: Some(Rect {
                x: sx,
                y: sy,
                w: sw,
                h: sh,
            }),
            ..Default::default()
        },
    );
}

/// Get pixel data from screen buffer and return an Image (screenshot)
pub fn get_screen_data() -> Image {
    unsafe {
        crate::window::get_internal_gl().flush();
    }

    let context = get_context();

    let texture = Texture2D::from_miniquad_texture(miniquad::Texture::new_render_texture(
        get_quad_context(),
        miniquad::TextureParams {
            width: context.screen_width as _,
            height: context.screen_height as _,
            ..Default::default()
        },
    ));

    texture.grab_screen();

    texture.get_texture_data()
}

/// Texture, data stored in GPU memory
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texture2D {
    pub(crate) texture: miniquad::Texture,
}

impl Texture2D {
    /// Creates an empty Texture2D.
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::empty();
    /// # }
    /// ```
    pub fn empty() -> Texture2D {
        Texture2D {
            texture: miniquad::Texture::empty(),
        }
    }

    /// Creates a Texture2D from a slice of bytes that contains an encoded image.
    ///
    /// If `format` is None, it will make an educated guess on the
    /// [ImageFormat][image::ImageFormat].
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::from_file_with_format(
    ///     include_bytes!("../examples/rust.png"),
    ///     None,
    ///     );
    /// # }
    /// ```
    pub fn from_file_with_format<'a>(
        bytes: &[u8],
        format: Option<image::ImageFormat>,
    ) -> Texture2D {
        let img = if let Some(fmt) = format {
            image::load_from_memory_with_format(bytes, fmt)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        } else {
            image::load_from_memory(bytes)
                .unwrap_or_else(|e| panic!("{}", e))
                .to_rgba8()
        };
        let width = img.width() as u16;
        let height = img.height() as u16;
        let bytes = img.into_raw();

        Self::from_rgba8(width, height, &bytes)
    }

    /// Creates a Texture2D from an [Image].
    pub fn from_image(image: &Image) -> Texture2D {
        Texture2D::from_rgba8(image.width, image.height, &image.bytes)
    }

    /// Creates a Texture2D from a miniquad
    /// [Texture](https://docs.rs/miniquad/0.3.0-alpha/miniquad/graphics/struct.Texture.html)
    pub fn from_miniquad_texture(texture: miniquad::Texture) -> Texture2D {
        Texture2D { texture }
    }

    /// Creates a Texture2D from a slice of bytes in an R,G,B,A sequence,
    /// with the given width and height.
    ///
    /// # Example
    ///
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// // Create a 2x2 texture from a byte slice with 4 rgba pixels
    /// let bytes: Vec<u8> = vec![255, 0, 0, 192, 0, 255, 0, 192, 0, 0, 255, 192, 255, 255, 255, 192];
    /// let texture = Texture2D::from_rgba8(2, 2, &bytes);
    /// # }
    /// ```
    pub fn from_rgba8(width: u16, height: u16, bytes: &[u8]) -> Texture2D {
        let ctx = get_context();

        let texture = miniquad::Texture::from_rgba8(get_quad_context(), width, height, bytes);
        let texture = Texture2D { texture };

        ctx.texture_batcher.add_unbatched(texture);

        texture
    }

    /// Uploads [Image] data to this texture.
    pub fn update(&self, image: &Image) {
        assert_eq!(self.texture.width, image.width as u32);
        assert_eq!(self.texture.height, image.height as u32);

        let ctx = get_quad_context();

        self.texture.update(ctx, &image.bytes);
    }

    /// Uploads [Image] data to part of this texture.
    pub fn update_part(
        &self,
        image: &Image,
        x_offset: i32,
        y_offset: i32,
        width: i32,
        height: i32,
    ) {
        let ctx = get_quad_context();

        self.texture
            .update_texture_part(ctx, x_offset, y_offset, width, height, &image.bytes)
    }

    /// Returns the width of this texture.
    pub fn width(&self) -> f32 {
        self.texture.width as f32
    }

    /// Returns the height of this texture.
    pub fn height(&self) -> f32 {
        self.texture.height as f32
    }

    /// Sets the [FilterMode] of this texture.
    ///
    /// Use Nearest if you need integer-ratio scaling for pixel art, for example.
    ///
    /// # Example
    /// ```
    /// # use macroquad::prelude::*;
    /// # #[macroquad::main("test")]
    /// # async fn main() {
    /// let texture = Texture2D::empty();
    /// texture.set_filter(FilterMode::Linear);
    /// # }
    /// ```
    pub fn set_filter(&self, filter_mode: FilterMode) {
        let ctx = get_quad_context();

        self.texture.set_filter(ctx, filter_mode);
    }

    /// Returns the handle for this texture.
    pub fn raw_miniquad_texture_handle(&self) -> miniquad::Texture {
        self.texture
    }

    /// Updates this texture from the screen.
    pub fn grab_screen(&self) {
        use miniquad::*;

        let (internal_format, _, _) = self.texture.format.into_gl_params(false);
        unsafe {
            gl::glBindTexture(gl::GL_TEXTURE_2D, self.texture.gl_internal_id());
            gl::glCopyTexImage2D(
                gl::GL_TEXTURE_2D,
                0,
                internal_format,
                0,
                0,
                self.texture.width as _,
                self.texture.height as _,
                0,
            );
        }
    }

    /// Returns an [Image] from the pixel data in this texture.
    ///
    /// This operation can be expensive.
    pub fn get_texture_data(&self) -> Image {
        let mut image = Image {
            width: self.texture.width as _,
            height: self.texture.height as _,
            bytes: vec![0; self.texture.width as usize * self.texture.height as usize * 4],
        };

        self.texture.read_pixels(&mut image.bytes);

        image
    }

    /// Unloads texture from GPU memory.
    ///
    /// Using a deleted texture could give different results on different
    /// platforms and is not recommended.
    pub fn delete(&self) {
        self.raw_miniquad_texture_handle().delete()
    }
}

pub(crate) struct Batcher {
    unbatched: Vec<Texture2D>,
    atlas: crate::text::atlas::Atlas,
    uv_cache: HashMap<u32, Rect>,
}

impl Batcher {
    pub fn new(ctx: &mut miniquad::Context) -> Batcher {
        Batcher {
            unbatched: vec![],
            atlas: crate::text::atlas::Atlas::new(ctx, miniquad::FilterMode::Linear),
            uv_cache: HashMap::new(),
        }
    }

    pub fn add_unbatched(&mut self, texture: Texture2D) {
        self.unbatched.push(texture);
    }

    pub fn get(&mut self, texture: Texture2D) -> Option<(Texture2D, Rect)> {
        let id = texture.raw_miniquad_texture_handle().gl_internal_id();
        let uv_rect = self.uv_cache.get(&(id as u32))?;
        Some((self.atlas.texture(), *uv_rect))
    }
}

/// Build an atlas out of all currently loaded texture
/// Later on all draw_texture calls with texture available in the atlas will use
/// the one from the atlas
/// NOTE: the GPU memory and texture itself in Texture2D will still be allocated
/// and Texture->Image conversions will work with Texture2D content, not the atlas
pub fn build_textures_atlas() {
    let context = get_context();

    for texture in context.texture_batcher.unbatched.drain(0..) {
        let sprite: Image = texture.get_texture_data();
        let id = texture.raw_miniquad_texture_handle().gl_internal_id() as u32;

        context.texture_batcher.atlas.cache_sprite(id as _, sprite);
        let uv_rect = context.texture_batcher.atlas.get_uv_rect(id as _).unwrap();
        context.texture_batcher.uv_cache.insert(id, uv_rect);
    }

    let texture = context.texture_batcher.atlas.texture();
    crate::telemetry::log_string(&format!("Atlas: {} {}", texture.width(), texture.height()));
}
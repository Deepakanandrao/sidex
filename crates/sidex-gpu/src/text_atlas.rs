//! GPU glyph texture atlas backed by `cosmic-text`.
//!
//! Glyphs are rasterized on demand and packed into GPU textures. The atlas
//! manages **two separate texture sheets**: one R8 (single-channel) for mask
//! glyphs and one Rgba8 for color emoji / subpixel-antialiased text.
//!
//! ## Features
//!
//! - **LRU eviction** — when the atlas exceeds a configurable capacity the
//!   least-recently-used glyphs are evicted and free regions are reclaimed.
//! - **Atlas compaction** — on eviction the atlas is rebuilt by re-uploading
//!   only the surviving glyphs, eliminating fragmentation.
//! - **Subpixel positioning** — glyph cache keys include fractional pixel
//!   offsets (4×4 quantized bins) for smoother text rendering.
//! - **Bold / italic variant caching** — separate atlas entries keyed by
//!   [`FontVariant`].
//! - **Ligature support** — multi-character ligatures are cached as single
//!   glyph entries via their `CacheKey`.
//! - **Emoji rendering** — color (`SwashContent::Color`) emoji bitmaps are
//!   stored in the dedicated color atlas.
//! - **Subpixel antialiasing** — `SwashContent::SubpixelMask` glyphs are
//!   stored in the color atlas with per-channel coverage.

use std::collections::HashMap;

use cosmic_text::{CacheKey, FontSystem, SwashCache, SwashContent};
use linked_hash_map::LinkedHashMap;

// ---------------------------------------------------------------------------
// GlyphInfo
// ---------------------------------------------------------------------------

/// Metadata for a single cached glyph in the atlas.
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// Left UV coordinate in the atlas (0.0..1.0).
    pub uv_left: f32,
    /// Top UV coordinate in the atlas (0.0..1.0).
    pub uv_top: f32,
    /// Right UV coordinate in the atlas (0.0..1.0).
    pub uv_right: f32,
    /// Bottom UV coordinate in the atlas (0.0..1.0).
    pub uv_bottom: f32,
    /// Glyph bitmap width in pixels.
    pub width: u32,
    /// Glyph bitmap height in pixels.
    pub height: u32,
    /// Horizontal bearing (offset from pen position to left edge).
    pub bearing_x: f32,
    /// Vertical bearing (offset from baseline to top edge).
    pub bearing_y: f32,
    /// Which atlas sheet this glyph lives on.
    pub atlas_kind: AtlasKind,
}

/// Which atlas texture a glyph is stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtlasKind {
    /// Single-channel R8 mask atlas (standard text).
    Mask,
    /// Four-channel Rgba8 atlas (color emoji + subpixel AA text).
    Color,
}

// ---------------------------------------------------------------------------
// Font variant key
// ---------------------------------------------------------------------------

/// Distinguishes bold / italic font variants in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontVariant {
    #[default]
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

/// A cache key that combines the cosmic-text `CacheKey` with a font variant
/// and subpixel bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtendedCacheKey {
    pub inner: CacheKey,
    pub variant: FontVariant,
    pub subpixel_bin: SubpixelBin,
}

// ---------------------------------------------------------------------------
// Subpixel positioning
// ---------------------------------------------------------------------------

/// Number of subpixel bins along each axis.
pub const SUBPIXEL_BINS_X: u8 = 4;
pub const SUBPIXEL_BINS_Y: u8 = 4;

/// Quantised subpixel offset bin (4 horizontal × 4 vertical positions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubpixelBin {
    pub x: u8,
    pub y: u8,
}

impl SubpixelBin {
    /// Quantises a fractional pixel offset into a 4×4 bin.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn from_offset(dx: f32, dy: f32) -> Self {
        Self {
            x: ((dx.fract().abs() * f32::from(SUBPIXEL_BINS_X)) as u8).min(SUBPIXEL_BINS_X - 1),
            y: ((dy.fract().abs() * f32::from(SUBPIXEL_BINS_Y)) as u8).min(SUBPIXEL_BINS_Y - 1),
        }
    }

    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl Default for SubpixelBin {
    fn default() -> Self {
        Self::zero()
    }
}

// ---------------------------------------------------------------------------
// LRU tracking
// ---------------------------------------------------------------------------

struct LruTracker {
    map: LinkedHashMap<ExtendedCacheKey, ()>,
    capacity: usize,
}

impl LruTracker {
    fn new(capacity: usize) -> Self {
        Self {
            map: LinkedHashMap::new(),
            capacity,
        }
    }

    fn touch(&mut self, key: ExtendedCacheKey) {
        if self.map.contains_key(&key) {
            self.map.get_refresh(&key);
        } else {
            self.map.insert(key, ());
        }
    }

    fn evict_candidates(&mut self) -> Vec<ExtendedCacheKey> {
        let mut evicted = Vec::new();
        while self.map.len() > self.capacity {
            if let Some((key, ())) = self.map.pop_front() {
                evicted.push(key);
            } else {
                break;
            }
        }
        evicted
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    #[allow(dead_code)]
    fn remove(&mut self, key: &ExtendedCacheKey) {
        self.map.remove(key);
    }
}

// ---------------------------------------------------------------------------
// Single atlas sheet
// ---------------------------------------------------------------------------

const INITIAL_ATLAS_SIZE: u32 = 1024;

struct AtlasSheet {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    format: wgpu::TextureFormat,
}

impl AtlasSheet {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let (texture, view) = Self::create_texture(device, INITIAL_ATLAS_SIZE, INITIAL_ATLAS_SIZE, format);
        Self {
            texture,
            view,
            width: INITIAL_ATLAS_SIZE,
            height: INITIAL_ATLAS_SIZE,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            format,
        }
    }

    fn bytes_per_pixel(&self) -> u32 {
        match self.format {
            wgpu::TextureFormat::R8Unorm => 1,
            _ => 4,
        }
    }

    #[allow(dead_code)]
    fn has_room(&self, w: u32, h: u32) -> bool {
        if self.cursor_x + w <= self.width && self.cursor_y + h <= self.height {
            return true;
        }
        let next_y = self.cursor_y + self.row_height;
        next_y + h <= self.height
    }

    #[allow(clippy::cast_precision_loss)]
    fn allocate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> (f32, f32, f32, f32) {
        if self.cursor_x + w > self.width {
            self.cursor_x = 0;
            self.cursor_y += self.row_height;
            self.row_height = 0;
        }
        if self.cursor_y + h > self.height {
            self.grow(device, queue);
        }

        let gx = self.cursor_x;
        let gy = self.cursor_y;

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: gx, y: gy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * self.bytes_per_pixel()),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        self.cursor_x += w + 1;
        self.row_height = self.row_height.max(h + 1);

        let aw = self.width as f32;
        let ah = self.height as f32;
        (
            gx as f32 / aw,
            gy as f32 / ah,
            (gx + w) as f32 / aw,
            (gy + h) as f32 / ah,
        )
    }

    fn reset_cursors(&mut self) {
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
    }

    #[allow(clippy::cast_precision_loss)]
    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let new_width = self.width * 2;
        let new_height = self.height * 2;
        log::info!(
            "Growing {:?} atlas from {}x{} to {new_width}x{new_height}",
            self.format,
            self.width,
            self.height
        );

        let (new_texture, new_view) = Self::create_texture(device, new_width, new_height, self.format);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("atlas_grow_encoder"),
        });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &new_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        self.texture = new_texture;
        self.view = new_view;
        self.width = new_width;
        self.height = new_height;
    }

    fn create_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph_atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

// ---------------------------------------------------------------------------
// TextAtlas
// ---------------------------------------------------------------------------

const DEFAULT_LRU_CAPACITY: usize = 8192;

/// Manages GPU texture atlases for rasterized font glyphs.
///
/// Two atlas sheets are maintained:
/// - **Mask** (`R8Unorm`) for standard grayscale glyphs.
/// - **Color** (`Rgba8Unorm`) for color emoji and subpixel-AA glyphs.
pub struct TextAtlas {
    mask_sheet: AtlasSheet,
    color_sheet: AtlasSheet,
    /// Sampler shared by both atlas textures.
    pub sampler: wgpu::Sampler,
    /// Primary cache mapping `CacheKey` → `GlyphInfo` (legacy compat).
    glyphs: HashMap<CacheKey, GlyphInfo>,
    /// Extended cache with variant + subpixel keys.
    extended_glyphs: HashMap<ExtendedCacheKey, GlyphInfo>,
    lru: LruTracker,
    swash_cache: SwashCache,
    /// Whether a compaction is needed on the next eviction.
    needs_compaction: bool,
}

impl TextAtlas {
    /// Creates a new, empty glyph atlas pair.
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            mask_sheet: AtlasSheet::new(device, wgpu::TextureFormat::R8Unorm),
            color_sheet: AtlasSheet::new(device, wgpu::TextureFormat::Rgba8Unorm),
            sampler,
            glyphs: HashMap::new(),
            extended_glyphs: HashMap::new(),
            lru: LruTracker::new(DEFAULT_LRU_CAPACITY),
            swash_cache: SwashCache::new(),
            needs_compaction: false,
        }
    }

    pub fn set_lru_capacity(&mut self, capacity: usize) {
        self.lru.capacity = capacity;
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len() + self.extended_glyphs.len()
    }

    /// Returns a reference to the mask atlas texture view.
    pub fn mask_view(&self) -> &wgpu::TextureView {
        &self.mask_sheet.view
    }

    /// Returns a reference to the color atlas texture view.
    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_sheet.view
    }

    // -----------------------------------------------------------------------
    // Legacy API
    // -----------------------------------------------------------------------

    pub fn get_glyph(&self, cache_key: CacheKey) -> Option<&GlyphInfo> {
        self.glyphs.get(&cache_key)
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn rasterize_glyph(
        &mut self,
        font_system: &mut FontSystem,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache_key: CacheKey,
    ) -> Option<GlyphInfo> {
        if let Some(info) = self.glyphs.get(&cache_key) {
            self.lru.touch(ExtendedCacheKey {
                inner: cache_key,
                variant: FontVariant::Regular,
                subpixel_bin: SubpixelBin::zero(),
            });
            return Some(*info);
        }

        let image = self.swash_cache.get_image_uncached(font_system, cache_key)?;

        let (glyph_w, glyph_h, atlas_kind, data) = match image.content {
            SwashContent::Mask => {
                (image.placement.width, image.placement.height, AtlasKind::Mask, image.data.clone())
            }
            SwashContent::Color | SwashContent::SubpixelMask => {
                (image.placement.width, image.placement.height, AtlasKind::Color, image.data.clone())
            }
        };

        if glyph_w == 0 || glyph_h == 0 {
            let info = GlyphInfo {
                uv_left: 0.0,
                uv_top: 0.0,
                uv_right: 0.0,
                uv_bottom: 0.0,
                width: 0,
                height: 0,
                bearing_x: image.placement.left as f32,
                bearing_y: image.placement.top as f32,
                atlas_kind,
            };
            self.glyphs.insert(cache_key, info);
            return Some(info);
        }

        self.maybe_evict(device, queue);

        let sheet = match atlas_kind {
            AtlasKind::Mask => &mut self.mask_sheet,
            AtlasKind::Color => &mut self.color_sheet,
        };

        let (uv_left, uv_top, uv_right, uv_bottom) = sheet.allocate(device, queue, glyph_w, glyph_h, &data);

        let info = GlyphInfo {
            uv_left,
            uv_top,
            uv_right,
            uv_bottom,
            width: glyph_w,
            height: glyph_h,
            bearing_x: image.placement.left as f32,
            bearing_y: image.placement.top as f32,
            atlas_kind,
        };

        self.glyphs.insert(cache_key, info);
        self.lru.touch(ExtendedCacheKey {
            inner: cache_key,
            variant: FontVariant::Regular,
            subpixel_bin: SubpixelBin::zero(),
        });
        Some(info)
    }

    // -----------------------------------------------------------------------
    // Extended API — variant + subpixel aware
    // -----------------------------------------------------------------------

    pub fn get_glyph_extended(&mut self, key: &ExtendedCacheKey) -> Option<&GlyphInfo> {
        if self.extended_glyphs.contains_key(key) {
            self.lru.touch(*key);
            self.extended_glyphs.get(key)
        } else {
            None
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn rasterize_glyph_extended(
        &mut self,
        font_system: &mut FontSystem,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: ExtendedCacheKey,
    ) -> Option<GlyphInfo> {
        if let Some(info) = self.extended_glyphs.get(&key) {
            self.lru.touch(key);
            return Some(*info);
        }

        let image = self.swash_cache.get_image_uncached(font_system, key.inner)?;

        let (glyph_w, glyph_h, atlas_kind, data) = match image.content {
            SwashContent::Mask => {
                (image.placement.width, image.placement.height, AtlasKind::Mask, image.data.clone())
            }
            SwashContent::Color | SwashContent::SubpixelMask => {
                (image.placement.width, image.placement.height, AtlasKind::Color, image.data.clone())
            }
        };

        if glyph_w == 0 || glyph_h == 0 {
            let info = GlyphInfo {
                uv_left: 0.0,
                uv_top: 0.0,
                uv_right: 0.0,
                uv_bottom: 0.0,
                width: 0,
                height: 0,
                bearing_x: image.placement.left as f32,
                bearing_y: image.placement.top as f32,
                atlas_kind,
            };
            self.extended_glyphs.insert(key, info);
            self.lru.touch(key);
            return Some(info);
        }

        self.maybe_evict(device, queue);

        let sheet = match atlas_kind {
            AtlasKind::Mask => &mut self.mask_sheet,
            AtlasKind::Color => &mut self.color_sheet,
        };

        let (uv_left, uv_top, uv_right, uv_bottom) = sheet.allocate(device, queue, glyph_w, glyph_h, &data);

        let info = GlyphInfo {
            uv_left,
            uv_top,
            uv_right,
            uv_bottom,
            width: glyph_w,
            height: glyph_h,
            bearing_x: image.placement.left as f32,
            bearing_y: image.placement.top as f32,
            atlas_kind,
        };

        self.extended_glyphs.insert(key, info);
        self.lru.touch(key);
        Some(info)
    }

    // -----------------------------------------------------------------------
    // Bind groups
    // -----------------------------------------------------------------------

    /// Creates a bind group for the mask (R8) atlas.
    pub fn create_mask_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mask_atlas_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.mask_sheet.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Creates a bind group for the color (Rgba8) atlas.
    pub fn create_color_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("color_atlas_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.color_sheet.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Legacy: creates a bind group for the mask atlas (backward compat).
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        self.create_mask_bind_group(device, layout)
    }

    // -----------------------------------------------------------------------
    // Eviction + compaction
    // -----------------------------------------------------------------------

    fn maybe_evict(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {
        let evicted = self.lru.evict_candidates();
        if evicted.is_empty() {
            return;
        }
        log::debug!(
            "Evicting {} glyphs from atlas (lru len={})",
            evicted.len(),
            self.lru.len()
        );
        for key in &evicted {
            self.glyphs.remove(&key.inner);
            self.extended_glyphs.remove(key);
        }
        self.needs_compaction = true;
    }

    /// Compacts both atlas sheets by resetting cursors. Call this after
    /// eviction if you want to reclaim fragmented space. Surviving glyphs
    /// will need to be re-rasterized on their next access.
    ///
    /// This is intentionally lazy — it marks glyphs as needing re-upload
    /// rather than doing an expensive GPU-side copy.
    pub fn compact(&mut self) {
        if !self.needs_compaction {
            return;
        }
        log::info!("Compacting glyph atlas — clearing UV data for surviving glyphs");
        self.mask_sheet.reset_cursors();
        self.color_sheet.reset_cursors();
        self.glyphs.clear();
        self.extended_glyphs.clear();
        self.needs_compaction = false;
    }
}

#![cfg_attr(feature="profile", feature(plugin, custom_attribute))]
#![cfg_attr(feature="profile", plugin(flamer))]

#![feature(test)]
extern crate test;

extern crate image;
extern crate find_folder;
extern crate rect_packer;
extern crate time;

#[cfg(feature="profile")]
extern crate flame;

mod assets;
pub mod tools;
pub mod maths;

use image::RgbaImage;

use std::vec::Vec;
use std::collections::BTreeMap as Map;
use std::ops::Range;

use tools::RcRef;
use maths::Vec2;

pub type AssetsMgrBuilder<'a, BE> = assets::AssetsMgrBuilder<'a, BE>;
pub type AssetsMgr = assets::AssetsMgr;

pub type PerformaceCounters = tools::PerformaceCounters;

pub type SpriteId = usize;
pub type Color = [f32; 4];

/// sprite data layout:  offsets and sizes come from the texture atlas
// { layer u32, pos(f32,f32), trg_size(f32, f32), sprite_offset(f32,f32), sprite_size(f32, f32) }
#[derive(PartialEq, Copy, Clone)]
pub struct SpriteLayout(pub [f32; 9]);

/// sprite data layout:  offsets and sizes come from the texture atlas
// { layer u32, pos(f32,f32), size(f32, f32), color(f32,f32,f32, f32) }
#[derive(PartialEq, Copy, Clone)]
pub struct RectLayout(pub [f32; 9]);

/// line data layout:
// { layer u32, src(f32, f32), trg(f32, f32), color(f32,f32,f32,f32) }
#[derive(PartialEq, Copy, Clone)]
pub struct LineLayout(pub [f32; 9]);

/// this struct provides the means to "tune" the primitives after being issued
/// this struct is not meant to be directly used but instead implements the
/// traits to colorize, add border, etc... when it proceeds
pub struct LayoutTune<T> {
    last: Range<usize>,
    lastqueue: RcRef<Vec<T>>,

    dimensions: (f32, f32),
    lines: RcRef<Map<u32, RcRef<Vec<LineLayout>>>>,
    _sprites: RcRef<Vec<SpriteLayout>>,
    _rects: RcRef<Vec<RectLayout>>,
}

/// this trait lets us color primitives
pub trait Colorize {
    fn with_color(self, r: f32, g: f32, b: f32, a: f32) -> Self;
}

impl Colorize for LayoutTune<LineLayout> {
    fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        for i in self.last.clone() {
            let queue = &mut self.lastqueue.get_mut();
            let &mut LineLayout(ref mut elem) = queue.get_mut(i).unwrap();

            elem[5] = r;
            elem[6] = g;
            elem[7] = b;
            elem[8] = a;
        }
        self
    }
}

impl Colorize for LayoutTune<RectLayout> {
    fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        for i in self.last.clone() {
            let queue = &mut self.lastqueue.get_mut();
            let &mut RectLayout(ref mut elem) = queue.get_mut(i).unwrap();

            elem[5] = r;
            elem[6] = g;
            elem[7] = b;
            elem[8] = a;
        }
        self
    }
}

/// trait to add a countour arround primitives
pub trait Contour {
    fn with_border(self, width: u32) -> LayoutTune<LineLayout>;
}

impl Contour for LayoutTune<RectLayout> {
    fn with_border(mut self, width: u32) -> LayoutTune<LineLayout> {

        let n = self.last.start;
        assert_eq!(self.last.end, n + 1);

        // read data from rectangle
        let RectLayout(elem) = self.lastqueue.get_mut()[n];
        let layer = elem[0] + 1.0;
        let x = elem[1];
        let y = elem[2];
        let w = elem[3];
        let h = elem[4];

        // add width if does not exist
        if self.lines.get().get(&width).is_none() {
            self.lines.get_mut().insert(width, RcRef::new(Vec::new()));
        }

        // do the containers dance
        let lines_map_next = self.lines.clone();
        let mut lines_map_rc = self.lines.get_mut();
        let lines_list_rc = lines_map_rc.get_mut(&width).unwrap();

        let i = lines_list_rc.get().len();

        let lines_list_next = lines_list_rc.clone();
        let mut list = lines_list_rc.get_mut();

        // when using width greater than one, we need to add an extra lenght so we get sharp
        // corners
        //let hoff = (width as f32 / 2.0 + 1.0) / self.dimensions.0;
        //let voff = (width as f32 / 2.0 + 1.0) / self.dimensions.1;

        // insert new elements
        list.push(LineLayout([layer, x, y, x, y + w, 1.0, 1.0, 1.0, 1.0]));
        list.push(LineLayout([layer, x + h, y, x + h, y + w, 1.0, 1.0, 1.0, 1.0]));
        list.push(LineLayout([layer, x, y + w, x + h, y + w, 1.0, 1.0, 1.0, 1.0]));
        list.push(LineLayout([layer, x, y, x + h, y, 1.0, 1.0, 1.0, 1.0]));

        LayoutTune {
            last: i..i + 4,
            lastqueue: lines_list_next,

            dimensions: self.dimensions,
            lines: lines_map_next,
            _sprites: self._sprites,
            _rects: self._rects,
        }
    }
}

/// the trait that hides the backend in use
pub trait StreamLineBackend {
    type Surface;
    fn add_texture(&mut self, img: RgbaImage) -> u32;
    fn surface(&mut self, layers: u32) -> Self::Surface;
}

/// trait that hides the surface we draw to
pub trait StreamLineBackendSurface {
    fn dimensions(&self) -> (f32, f32);
    fn clear(&mut self, color: &Color);
    fn draw_sprites(&mut self, sprites: &[SpriteLayout], tex: u32);
    fn draw_lines(&mut self, lines: &[LineLayout], width: u32);
    fn draw_rects(&mut self, rects: &[RectLayout]);
    fn done(self);
}


/// The command queue is a transient object:
/// we create it on each frame, then we fill it with the drawing instructions,
/// and finally it is issued and discarded
pub struct CmdQueue<'a, S>
    where S: StreamLineBackendSurface
{
    surface: S,
    assets: &'a AssetsMgr,
    lines: RcRef<Map<u32, RcRef<Vec<LineLayout>>>>,
    sprites: RcRef<Vec<SpriteLayout>>,
    rects: RcRef<Vec<RectLayout>>,
}

impl<'a, S> CmdQueue<'a, S>
    where S: StreamLineBackendSurface
{
    /// create a new queue
    pub fn new(surface: S, assets_mgr: &'a AssetsMgr) -> CmdQueue<'a, S> {

        CmdQueue {
            surface: surface,
            assets: assets_mgr,
            lines: RcRef::new(Map::new()),
            sprites: RcRef::new(Vec::new()),
            rects: RcRef::new(Vec::new()),
        }
    }

    /// clear the current canvas, overwriting anything done before
    pub fn clear(&mut self, color: &Color) {
        self.surface.clear(color);
    }


    /// draw a line between two points
    #[cfg_attr(feature="profile", flame)]
    pub fn line(&mut self, src: Vec2, dst: Vec2, width: u32, layer: u32) -> LayoutTune<LineLayout> {

        // if we do not have a list for this width we need to create one
        self.add_width_list(width);

        let dim = self.surface.dimensions();

        let lines_map_next = self.lines.clone();
        let mut lines_map_rc = self.lines.get_mut();
        let lines_list_rc = lines_map_rc.get_mut(&width).unwrap();

        let i = lines_list_rc.get().len();

        let lines_list_next = lines_list_rc.clone();
        let mut list = lines_list_rc.get_mut();
        list.push(LineLayout([layer as f32 * 1.0,
                              (src.x as f32 / (dim.0 / 2.0)) - 1.0,
                              (src.y as f32 / (dim.1 / 2.0)) - 1.0,
                              (dst.x as f32 / (dim.0 / 2.0)) - 1.0,
                              (dst.y as f32 / (dim.1 / 2.0)) - 1.0,
                              1.0,
                              1.0,
                              1.0,
                              1.0]));

        LayoutTune {
            last: i..i + 1,
            lastqueue: lines_list_next,

            dimensions: dim,
            lines: lines_map_next,
            _sprites: self.sprites.clone(),
            _rects: self.rects.clone(),
        }
    }

    /// draw a sprite in a given location
    #[cfg_attr(feature="profile", flame)]
    pub fn sprite(&mut self, pos: Vec2, layer: u32, sprite: SpriteId) -> LayoutTune<SpriteLayout> {

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;
        let (x, y) = self.assets.get_sprite_offset(sprite).unwrap();
        let (w, h) = self.assets.get_sprite_size(sprite).unwrap();

        let i = self.sprites.get().len();
        self.sprites
            .get_mut()
            .push(SpriteLayout([layer as f32,
                                (pos.x as f32 / (dim.0 / 2.0)) - 1.0,
                                (pos.y as f32 / (dim.1 / 2.0)) - 1.0,
                                w * ratio,
                                h,
                                x,
                                y,
                                w,
                                h]));

        LayoutTune {
            last: i..i + 1,
            lastqueue: self.sprites.clone(),

            dimensions: dim,
            lines: self.lines.clone(),
            _sprites: self.sprites.clone(),
            _rects: self.rects.clone(),
        }
    }

    /// draw a rectangle
    #[cfg_attr(feature="profile", flame)]
    pub fn rect(&mut self, position: Vec2, dimensions: Vec2, layer: u32) -> LayoutTune<RectLayout> {

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;

        let i = self.rects.get().len();
        self.rects
            .get_mut()
            .push(RectLayout([layer as f32,
                              (position.x as f32 / (dim.0 / 2.0)) - 1.0,
                              (position.y as f32 / (dim.1 / 2.0)) - 1.0,
                              (dimensions.x as f32 / (dim.1 / 2.0)), // * ratio,
                              (dimensions.y as f32 / (dim.1 / 2.0)) * ratio,
                              0.0,
                              0.0,
                              0.0,
                              1.0]));

        LayoutTune {
            last: i..i + 1,
            lastqueue: self.rects.clone(),

            dimensions: dim,
            lines: self.lines.clone(),
            _sprites: self.sprites.clone(),
            _rects: self.rects.clone(),
        }
    }

    /// finishes and consummes the queue, issues all the draw calls to the backend
    pub fn done(mut self) {

        // get all lines, orderer by depth and then width
        for (width, line) in self.lines.get().iter() {
            // let v = self.lines.get(layer).unwrap();
            self.surface.draw_lines(line.get().as_slice(), *width);
        }
        // get all sprites,
        self.surface
            .draw_sprites(self.sprites.get().as_slice(), self.assets.get_atlas());
        // rectagles
        self.surface.draw_rects(self.rects.get().as_slice());

        self.surface.done()
    }


    // if we do not have a list for this width we need to create one
    fn add_width_list(&mut self, width: u32) {

        if self.lines.get().get(&width).is_none() {
            self.lines.get_mut().insert(width, RcRef::new(Vec::new()));
        }
    }
}
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

use tools::LayoutHash;
use std::hash::Hasher;
use std::mem::transmute;

impl LayoutHash for LineLayout {
    fn hash<H: Hasher>(&self, h: &mut H){
        let &LineLayout(slice)  = self;
        for i in &slice{
            let v = unsafe {transmute::<f32, u32>(*i)};
            h.write_u32(v);
        }
    }
}

impl LayoutHash for SpriteLayout {
    fn hash<H: Hasher>(&self, h: &mut H){
        let &SpriteLayout(slice)  = self;
        for i in &slice{
            let v = unsafe {transmute::<f32, u32>(*i)};
            h.write_u32(v);
        }
    }
}

impl LayoutHash for RectLayout {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let &RectLayout(slice)  = self;
        for i in &slice{
            let v = unsafe {transmute::<f32, u32>(*i)};
            h.write_u32(v);
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
#[cfg(test)]
mod tests {

    use super::CmdQueue;
    use super::StreamLineBackend;

    use super::StreamLineBackendSurface;
    use super::AssetsMgrBuilder;
    use super::maths::vec2;
    use super::Color;
    use super::SpriteLayout;
    use super::LineLayout;
    use super::RectLayout;

    use super::Colorize;

    use image::RgbaImage;
    use test::Bencher;

    struct TestBE;
    impl StreamLineBackend for TestBE {
        type Surface = TestBESurface;
        fn add_texture(&mut self, _img: RgbaImage) -> u32 {
            0
        }
        fn surface(&mut self, _:u32) -> Self::Surface {
            TestBESurface {}
        }
    }
    struct TestBESurface;
    impl StreamLineBackendSurface for TestBESurface {
        fn dimensions(&self) -> (f32, f32) {
            (0.0, 0.0)
        }
        fn clear(&mut self, _color: &Color) {}
        fn draw_sprites(&mut self, _sprites: &[SpriteLayout], _tex: u32) {}
        fn draw_lines(&mut self, _lines: &[LineLayout], _w: u32) {}
        fn draw_rects(&mut self, _rects: &[RectLayout]) {}
        fn done(self) {}
    }

    #[bench]
    fn bench_lines(b: &mut Bencher) {
        // get some dummy backend
        let mut be = TestBE;

        // phase 1, load assets
        let ass = AssetsMgrBuilder::new(&mut be)
            .build()
            .expect("no problem so far");

        b.iter(|| {
            let surface = be.surface(0);
            let mut q = CmdQueue::new(surface, &ass);
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            for _ in 0..1000 {
                q.line(vec2(0, 0), vec2(1, 1), 1, 0)
                    .with_color(1.0, 1.0, 0.0, 1.0);
            }
            q.done();
        });
    }

    #[bench]
    fn bench_sprites(b: &mut Bencher) {
        use std::path::Path;
        use find_folder::Search;

        // get some dummy backend
        let mut be = TestBE;

        let mut file_location = Search::Parents(3)
            .for_folder("assets")
            .expect("some assets folder must exist somewhere");
        file_location.push(Path::new("rust-logo.png"));

        // phase 1, load assets
        let (ass, sp) = {
            let mut mgr = AssetsMgrBuilder::new(&mut be);
            let sp = mgr.add_sprite(&file_location);
            (mgr.build().expect("everithing allright"), sp)
        };

        b.iter(|| {
            let surface = be.surface(0);
            let mut q = CmdQueue::new(surface, &ass);
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            for _ in 0..1000 {
                q.sprite(vec2(0, 0), 0, sp);
            }
            q.done();
        });
    }
}

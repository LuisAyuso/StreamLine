#![feature(test)]
extern crate test;

extern crate image;
extern crate find_folder;
extern crate rect_packer;
extern crate time;

mod assets;
pub mod tools;
pub mod maths;

use image::RgbaImage;
use std::vec::Vec;
use std::collections::BTreeMap as Map;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use maths::Vec2;

pub type AssetsMgrBuilder<'a, BE> = assets::AssetsMgrBuilder<'a, BE>;
pub type AssetsMgr = assets::AssetsMgr;

pub type PerformaceCounters = tools::PerformaceCounters;

pub type SpriteId = usize;
pub type Color = [f32; 4];

/// sprite data layout:  offsets and sizes come from the texture atlas
/// { layer u32, pos(f32,f32), trg_size(f32, f32), sprite_offset(f32,f32), sprite_size(f32, f32) }
pub struct SpriteLayout (pub [f32; 9]);

/// sprite data layout:  offsets and sizes come from the texture atlas
/// { layer u32, pos(f32,f32), size(f32, f32), color(f32,f32,f32, f32) }
pub struct RectLayout (pub [f32; 9]);

/// line data layout:
/// { layer u32, src(f32, f32), trg(f32, f32), color(f32,f32,f32,f32) }
/// for different widths we may need to split them in different queues
pub struct LineLayout (pub [f32; 9]);

pub struct LayoutTune<T>
{
    last: usize,
    lastqueue: Rc<RefCell<Vec<T>>>,

    lines:   Rc<RefCell<Vec<LineLayout>>>,
    sprites: Rc<RefCell<Vec<SpriteLayout>>>,
    rects:   Rc<RefCell<Vec<RectLayout>>>,
}

/// this trait lets us color primitives
pub trait Colorize{
    fn with_color(self, r: f32, g: f32, b: f32, a: f32) -> Self;
}

impl Colorize for LayoutTune<LineLayout>{
    fn with_color(self, r: f32, g: f32, b: f32, a: f32) -> Self{
        {
            let &mut LineLayout(ref mut elem) = &mut self.lastqueue.borrow_mut()[self.last];

            elem[5] = r;
            elem[6] = g;
            elem[7] = b;
            elem[8] = a;
        }
        self
    }
}

impl Colorize for LayoutTune<RectLayout>{
    fn with_color(self, r: f32, g: f32, b: f32, a: f32) -> Self{
        {
            let &mut RectLayout(ref mut elem) = &mut self.lastqueue.borrow_mut()[self.last];

            elem[5] = r;
            elem[6] = g;
            elem[7] = b;
            elem[8] = a;
        }
        self
    }
}

/// trait to add a countour arround primitives
pub trait Contour{
    fn border(self); // -> LayoutTune<LineLayout>;
}

impl Contour for LayoutTune<RectLayout>{
    fn border(self) { // -> LayoutTune<LineLayout>{
        {
            let &mut RectLayout(elem) = &mut self.lastqueue.borrow_mut()[self.last];
            let lines = &mut self.lines.borrow_mut();

            let layer = elem[0];
            let x = elem[1];
            let y = elem[2];
            let w = elem[3];
            let h = elem[4];


            lines.push(LineLayout([layer,
                                  x,y, x, y+h,
                                  1.0, 1.0, 1.0, 1.0]));
            lines.push(LineLayout([layer,
                                  x+w,y, x+w, y+h,
                                  1.0, 1.0, 1.0, 1.0]));
            lines.push(LineLayout([layer,
                                  x,y+h, x+w, y+h,
                                  1.0, 1.0, 1.0, 1.0]));
            lines.push(LineLayout([layer,
                                  x,y, x+w, y,
                                  1.0, 1.0, 1.0, 1.0]));

        }
    }
}

/// the trait that hides the backend in use
pub trait StreamLineBackend {
    type Surface;
    fn add_texture(&mut self, img: RgbaImage) -> u32;
    fn surface(&mut self) -> Self::Surface;
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
pub struct CmdQueue<'a, BE, S>
    where BE: StreamLineBackend + 'a,
          S: StreamLineBackendSurface
{
    be: &'a mut BE,
    surface: S,
    assets: &'a AssetsMgr,
    lines: Map<u32,   Rc<RefCell<Vec<LineLayout>>>>,
    sprites: Map<u32, Rc<RefCell<Vec<SpriteLayout>>>>,
    rects: Map<u32,   Rc<RefCell<Vec<RectLayout>>>>,
}

impl<'a, BE, S> CmdQueue<'a, BE, S>
    where BE: StreamLineBackend + 'a,
          S: StreamLineBackendSurface
{
    /// create a new queue
    pub fn new(be: &'a mut BE, surface: S, assets_mgr: &'a AssetsMgr) -> CmdQueue<'a, BE, S> {

        CmdQueue {
            be: be,
            surface: surface,
            assets: assets_mgr,
            lines:   Map::new(),
            sprites: Map::new(),
            rects:   Map::new(),
        }
    }

    fn add_layer(&mut self, layer: u32){
        self.lines.insert(layer, Rc::new(RefCell::new(Vec::new())));
        self.sprites.insert(layer, Rc::new(RefCell::new(Vec::new())));
        self.rects.insert(layer, Rc::new(RefCell::new(Vec::new())));
    }

    /// clear the current canvas, overwriting anything done before
    pub fn clear(&mut self, color: &Color) {
        self.surface.clear(color);
    }

    /// draw a line between two points
    pub fn line(&mut self, src: Vec2, dst: Vec2, layer: u32) -> LayoutTune<LineLayout>{
        if let None = self.lines.get(&layer) {
            self.add_layer(layer);
        }

        let dim = self.surface.dimensions();

        let mut i = 0;
        {
            let mut list = self.lines.get_mut(&layer).unwrap().borrow_mut();
            i = list.len();
            list.push(LineLayout([layer as f32,
                       (src.x as f32 / (dim.0 / 2.0)) - 1.0, 
                       (src.y as f32 / (dim.1 / 2.0)) - 1.0, 
                       (dst.x as f32 / (dim.0 / 2.0)) - 1.0, 
                       (dst.y as f32 / (dim.1 / 2.0)) - 1.0, 
                       1.0, 1.0, 1.0, 1.0]));
        }

        LayoutTune{
            last: i,
            lastqueue:  self.lines[&layer].clone(),

            sprites:  self.sprites[&layer].clone(),
            rects:  self.rects[&layer].clone(),
            lines:  self.lines[&layer].clone(),
        }
    }

    /// draw a sprite in a given location
    pub fn sprite(&mut self, pos: Vec2, layer: u32, sprite: SpriteId) -> LayoutTune<SpriteLayout>{
        if let None = self.sprites.get(&layer) {
            self.add_layer(layer);
        }

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;
        let (x, y) = self.assets.get_sprite_offset(sprite).unwrap();
        let (w, h) = self.assets.get_sprite_size(sprite).unwrap();

        let mut i = 0;
        {
            let mut list = self.sprites.get_mut(&layer).unwrap().borrow_mut();
            i = list.len();
            list.push(SpriteLayout([layer as f32,
                       (pos.x as f32 / (dim.0 / 2.0)) - 1.0,
                       (pos.y as f32 / (dim.1 / 2.0)) - 1.0, 
                       w * ratio, h, 
                       x, y, w, h]));
        }

        LayoutTune{
            last: i,
            lastqueue:  self.sprites[&layer].clone(),

            sprites:  self.sprites[&layer].clone(),
            rects:  self.rects[&layer].clone(),
            lines:  self.lines[&layer].clone(),
        }
    }

    /// draw a rectangle
    pub fn rect(&mut self, position: Vec2, dimensions: Vec2, layer: u32) -> LayoutTune<RectLayout>{
        if let None = self.rects.get(&layer) {
            self.add_layer(layer);
        }

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;

        let mut i = 0;
        {
            let mut list = self.rects.get_mut(&layer).unwrap().borrow_mut();
            i = list.len();
            list.push(RectLayout([layer as f32,
                       (position.x as f32 / (dim.0 / 2.0)) - 1.0,
                       (position.y as f32 / (dim.1 / 2.0)) - 1.0, 
                       (dimensions.x as f32 / (dim.0 / 2.0)) * ratio, 
                       (dimensions.y as f32 / (dim.1 / 2.0)),
                       1.0, 0.0, 1.0, 0.0]));
        }

        LayoutTune{
            last: i,
            lastqueue:  self.rects[&layer].clone(),

            sprites:  self.sprites[&layer].clone(),
            rects:  self.rects[&layer].clone(),
            lines:  self.lines[&layer].clone(),
        }
    }

    /// finishes and consummes the queue, issues all the draw calls to the backend
    pub fn done(mut self) {

        // get all lines, orderer by depth and then width
        for layer in self.lines.keys(){
            let v = self.lines.get(layer).unwrap();
            self.surface.draw_lines(v.borrow().as_slice(), 1);
        }
        // get all sprites,
        for layer in self.sprites.keys(){
            let v = self.sprites.get(layer).unwrap();
            self.surface.draw_sprites(v.borrow().as_slice(), self.assets.get_atlas());
        }
        // rectagles
        for layer in self.rects.keys(){
            let v = self.rects.get(layer).unwrap();
            self.surface.draw_rects(v.borrow().as_slice());
        }

        self.surface.done()
    }
}


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
        fn surface(&mut self) -> Self::Surface {
            TestBESurface {}
        }
    }
    struct TestBESurface;
    impl StreamLineBackendSurface for TestBESurface {
        fn dimensions(&self) -> (f32, f32){ (0.0, 0.0) }
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
        let ass = AssetsMgrBuilder::new(&mut be).build().expect("no problem so far");

        b.iter(|| {
            let surface = be.surface();
            let mut q = CmdQueue::new(&mut be, surface, &ass);
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            for _ in 0..1000{
                q.line(vec2(0, 0), vec2(1, 1), 0).with_color(1.0, 1.0, 0.0, 1.0);
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
            let surface = be.surface();
            let mut q = CmdQueue::new(&mut be, surface, &ass);
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            for _ in 0..1000{
                q.sprite(vec2(0, 0), 0, sp);
            }
            q.done();
        });
     }
}

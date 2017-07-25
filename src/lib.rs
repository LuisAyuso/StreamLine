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

use maths::Vec2;

pub type AssetsMgrBuilder<'a, BE> = assets::AssetsMgrBuilder<'a, BE>;
pub type AssetsMgr = assets::AssetsMgr;

pub type PerformaceCounters = tools::PerformaceCounters;

pub type SpriteId = usize;
pub type Color = [f32; 4];

/// sprite data layout:  offsets and sizes come from the texture atlas
/// { layer u32, pos(f32,f32), trg_size(f32, f32), sprite_offset(f32,f32), sprite_size(f32, f32) }
pub type SpriteLayout = [f32; 9];

/// sprite data layout:  offsets and sizes come from the texture atlas
/// { layer u32, pos(f32,f32), size(f32, f32), color(f32,f32,f32, f32) }
pub type RectLayout = [f32; 9];

/// line data layout:
/// { layer u32, src(f32, f32), trg(f32, f32), color(f32,f32,f32,f32) }
/// for different widths we may need to split them in different queues
pub type LineLayout = [f32; 9];

/// this trait lets us modify the lines
pub trait Line{
    /// change default color for lines
    fn with_color(&mut self, r: f32, g: f32, b: f32, a: f32);
}

impl Line for LineLayout{
    fn with_color(&mut self, r: f32, g: f32, b: f32, a: f32){
        self[5] = r;
        self[6] = g;
        self[7] = b;
        self[8] = a;
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
    lines: Map<u32, Vec<LineLayout>>,
    sprites: Map<u32, Vec<SpriteLayout>>,
    rects: Map<u32, Vec<RectLayout>>,
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
            lines: Map::new(),
            sprites: Map::new(),
            rects: Map::new(),
        }
    }

    /// clear the current canvas, overwriting anything done before
    pub fn clear(&mut self, color: &Color) {
        self.surface.clear(color);
    }

    /// draw a line between two points
    pub fn line(&mut self, src: Vec2, dst: Vec2, layer: u32) -> &mut LineLayout{
        if let None = self.lines.get(&layer) {
            self.lines.insert(layer, Vec::new());
        }

        let dim = self.surface.dimensions();

        let list = self.lines.get_mut(&layer).unwrap();
        let i = list.len();
        list.push([layer as f32,
                   (src.x as f32 / (dim.0 / 2.0)) - 1.0, 
                   (src.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   (dst.x as f32 / (dim.0 / 2.0)) - 1.0, 
                   (dst.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   1.0, 1.0, 1.0, 1.0]);

        &mut list[i]
    }

    /// draw a sprite in a given location
    pub fn sprite(&mut self, pos: Vec2, layer: u32, sprite: SpriteId) {
        if let None = self.sprites.get(&layer) {
            self.sprites.insert(layer, Vec::new());
        }

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;
        let (x, y) = self.assets.get_sprite_offset(sprite).unwrap();
        let (w, h) = self.assets.get_sprite_size(sprite).unwrap();

        let list = self.sprites.get_mut(&layer).unwrap();
        list.push([layer as f32,
                   (pos.x as f32 / (dim.0 / 2.0)) - 1.0,
                   (pos.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   w * ratio, h, 
                   x, y, w, h]);
    }

    /// draw a rectangle
    pub fn rect(&mut self, position: Vec2, dimensions: Vec2, layer: u32) -> &mut RectLayout{
        if let None = self.rects.get(&layer) {
            self.rects.insert(layer, Vec::new());
        }

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;

        let list = self.rects.get_mut(&layer).unwrap();
        let i = list.len();
        list.push([layer as f32,
                   (position.x as f32 / (dim.0 / 2.0)) - 1.0,
                   (position.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   dimensions.x as f32 * ratio, 
                   dimensions.y as f32, 
                   1.0, 0.0, 1.0, 0.0]);

        &mut list[i]
    }

    /// finishes and consummes the queue, issues all the draw calls to the backend
    pub fn done(mut self) {

        // get all sprites,
        for layer in self.sprites.keys(){
            let v = self.sprites.get(layer).unwrap();
            self.surface.draw_sprites(v.as_slice(), self.assets.get_atlas());
        }
        // get all lines, orderer by depth and then width
        for layer in self.lines.keys(){
            let v = self.lines.get(layer).unwrap();
            self.surface.draw_lines(v.as_slice(), 1);
        }
        for layer in self.rects.keys(){
            let v = self.rects.get(layer).unwrap();
            self.surface.draw_rects(v.as_slice());
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
    use super::Line;

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

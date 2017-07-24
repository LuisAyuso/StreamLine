extern crate image;
extern crate find_folder;
extern crate rect_packer;
extern crate time;

use image::RgbaImage;
use std::vec::Vec;
use std::collections::BTreeMap as Map;

mod assets;
pub mod tools;

pub type AssetsMgrBuilder<'a, BE> = assets::AssetsMgrBuilder<'a, BE>;
pub type AssetsMgr = assets::AssetsMgr;

pub type PerformaceCounters = tools::PerformaceCounters;

pub struct Pos {
    pub x: u32,
    pub y: u32,
}
pub fn pos(x: u32, y: u32) -> Pos {
    Pos { x: x, y: y }
}

pub type SpriteId = usize;
pub type Color = [f32; 4];

/// sprite data layout:  offsets and sizes come from the texture atlas
/// { pos(f32,f32), trg_size(f32, f32), sprite_offset(f32,f32), sprite_size(f32, f32) }
/// 64 bytes? it is this a nice transient size?
pub type SpriteLayout = [f32; 8];

/// line data layout:
/// { src(f32, f32), trg(f32, f32), color(f32,f32,f32,f32) }
/// for different widths we may need to split them in different queues
pub type LineLayout = [f32; 8];

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
        }
    }

    /// clear the current canvas, overwriting anything done before
    pub fn clear(&mut self, color: &Color) {
        self.surface.clear(color);
    }

    /// draw a sprite in a given location
    pub fn draw_sprite(&mut self, pos: Pos, layer: u32, sprite: SpriteId) {
        if let None = self.sprites.get(&layer) {
            self.sprites.insert(layer, Vec::new());
        }

        let dim = self.surface.dimensions();
        let ratio = dim.1 / dim.0;
        let (x, y) = self.assets.get_sprite_offset(sprite).unwrap();
        let (w, h) = self.assets.get_sprite_size(sprite).unwrap();

        let list = self.sprites.get_mut(&layer).unwrap();
        list.push([(pos.x as f32 / (dim.0 / 2.0)) - 1.0,
                   (pos.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   w * ratio, h, 
                   x, y, w, h]);
    }

    /// draw a line between two points
    pub fn draw_line(&mut self, src: Pos, dst: Pos, layer: u32) {
        if let None = self.lines.get(&layer) {
            self.lines.insert(layer, Vec::new());
        }

        let dim = self.surface.dimensions();

        let list = self.lines.get_mut(&layer).unwrap();
        list.push([(src.x as f32 / (dim.0 / 2.0)) - 1.0, 
                   (src.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   (dst.x as f32 / (dim.0 / 2.0)) - 1.0, 
                   (dst.y as f32 / (dim.1 / 2.0)) - 1.0, 
                   1.0, 1.0, 1.0, 1.0]);
    }

    /// finishes and consummes the queue, issues all the draw calls to the backend
    pub fn done(mut self) {

        // get all sprites,
        for layer in self.sprites.keys(){
            let v = self.sprites.get(layer).unwrap();
            self.surface.draw_sprites(v.as_slice(), 0);
        }
        // get all lines, orderer by depth and then width
        for layer in self.lines.keys(){
            let v = self.lines.get(layer).unwrap();
            self.surface.draw_lines(v.as_slice(), 1);
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
    use super::pos;
    use super::Color;
    use super::SpriteLayout;
    use super::LineLayout;

    use image::RgbaImage;

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
        fn done(self) {}
    }

    #[test]
    fn construct() {

        // get some dummy backend
        let mut be = TestBE;

        // phase 1, load assets
        let ass = AssetsMgrBuilder::new(&mut be).build().expect("no problem so far");
        let surface = be.surface();

        // phase 2, draw
        let mut q = CmdQueue::new(&mut be, surface, &ass);
        {
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            q.draw_line(pos(0, 0), pos(1, 1), 0);
            q.draw_line(pos(0, 0), pos(1, 1), 0);
            q.draw_line(pos(0, 0), pos(1, 1), 0);
            q.draw_line(pos(0, 0), pos(1, 1), 0);
        }
        q.done();
    }

    #[test]
    fn with_assets() {
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

        let surface = be.surface();

        // phase 2, draw
        let mut q = CmdQueue::new(&mut be, surface, &ass);
        {
            q.clear(&[0.0f32, 0.0, 0.0, 0.0]);
            q.draw_sprite(pos(0, 0), 0, sp);
        }
        q.done();
    }
}

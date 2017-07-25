
#[macro_use]
extern crate glium;
extern crate glutin;
extern crate streamline;
extern crate image;
extern crate find_folder;

mod line;
mod quad;

use streamline::StreamLineBackend;
use streamline::StreamLineBackendSurface;
use streamline::SpriteLayout;
use streamline::LineLayout;
use streamline::RectLayout;
use streamline::Color;

use line::LineDraw;
use quad::QuadDraw;

use glium::Surface;
use image::RgbaImage;

use std::collections::BTreeMap as Map;
use std::rc::Rc;
use std::ops::Deref;


pub struct GliumBackend {
    display: glium::Display,
    dimensions: (f32, f32),
    map: Rc<Map<u32, glium::texture::Texture2d>>,
}


impl GliumBackend {
    pub fn new(display: glium::Display, dim: (u32, u32)) -> GliumBackend {
        GliumBackend {
            display: display,
            dimensions: (dim.0 as f32, dim.1 as f32),
            map: Rc::new(Map::new()),
        }
    }
}

impl StreamLineBackend for GliumBackend {
    type Surface = GliumBackendSurface<glium::Display>;
    fn add_texture(&mut self, img: RgbaImage) -> u32 {

        let id = self.map.len() as u32;

        let dim = img.dimensions();
        // no idea why, but if not reversed, it just crashes
        // let tex = glium::texture::RawImage2d::from_raw_rgb(img.into_raw(),dim);
        let tex = glium::texture::RawImage2d::from_raw_rgba_reversed(&img.into_raw(), dim);
        let tex = glium::texture::Texture2d::new(&self.display, tex).unwrap();

        Rc::get_mut(&mut self.map).expect("no other one should be here").insert(id, tex);
        id
    }

    fn surface(&mut self) -> Self::Surface {
        let line_draw = LineDraw::new(&self.display);
        let quad_draw = QuadDraw::new(&self.display);
        GliumBackendSurface {
            frame: self.display.draw(),
            line_draw: line_draw,
            quad_draw: quad_draw,
            dimensions: self.dimensions,
            display: self.display.clone(),
            tex_map: self.map.clone(),
        }
    }
}

pub struct GliumBackendSurface<F>
    where F: glium::backend::Facade
{
    frame: glium::Frame,
    line_draw: LineDraw,
    quad_draw: QuadDraw,
    dimensions: (f32, f32),
    display: F,
    // Todo: find a more sophisticated way
    tex_map: Rc<Map<u32, glium::texture::Texture2d>>,
}

impl<F> StreamLineBackendSurface for GliumBackendSurface<F>
    where F: glium::backend::Facade
{

    fn dimensions(&self) -> (f32, f32){ self.dimensions }

    fn clear(&mut self, color: &Color) {
        self.frame.clear_color(color[0], color[1], color[2], color[3]);
        self.frame.clear_depth(1.0f32);

    }

    fn draw_sprites(&mut self, sprites: &[SpriteLayout], tex: u32) {
        if let Some(tex) = self.tex_map.deref().get(&tex) {
            self.quad_draw.draw_tex_quads(&self.display, &mut self.frame, sprites, tex);
        }
    }

    fn draw_lines(&mut self, lines: &[LineLayout], width: u32) {
        self.line_draw.draw_lines(&self.display, &mut self.frame, lines, width);
    }

    fn draw_rects(&mut self, rects: &[RectLayout]) {
        self.quad_draw.draw_color_quads(&self.display, &mut self.frame, rects);
    }

    fn done(self) {
        self.frame.finish().expect("could not finish frame");
    }
}


#[cfg(test)]
mod tests {

    use glium;
    use glutin;

    use super::*;

    use std::path::Path;
    use find_folder::Search;

    use streamline::AssetsMgrBuilder;

    #[test]
    fn lines() {

        let mut events_loop = glutin::EventsLoop::new();
        let window = glutin::WindowBuilder::new().with_dimensions(1024,1024);
        let context = glutin::ContextBuilder::new().with_depth_buffer(24);
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        // our backend
        let mut be = GliumBackend::new(display,(1024, 1024));

        let mut stop = false;
        let mut countdown = 40;
        while !stop && countdown > 0 {

            // ~~~~~~~~~~ raw drawing ~~~~~~~~~~~~~~~~
            let mut surface = be.surface();
            surface.clear(&[0.5f32, 0.4, 0.8, 1.0]);
            surface.draw_lines(&[[0.0, 0.0, 0.8, 0.8, 0.8, 1.0, 0.0, 0.0, 1.0f32]], 1);
            surface.draw_lines(&[[0.0, -0.5, 0.4, 0.4, 0.4, 1.0, 1.0, 0.0, 1.0f32]], 2);
            surface.draw_lines(&[[0.0, 100.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0f32]], 3);
            surface.draw_lines(&[[0.0, -100.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0f32]], 4);
            surface.draw_lines(&[[0.0, 0.0, 100.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0f32]], 5);
            surface.draw_lines(&[[0.0, 0.0, -100.0, 0.0, 0.0, 1.0, 0.5, 1.0, 1.0f32]], 6);
            surface.done();

            // ~~~~~~~~~~~   event ~~~~~~~~~~~~~~~~~
            events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, .. } => {
                        match event {
                            glutin::WindowEvent::Closed => stop = true,
                            _ => (),
                        }
                    }
                    _ => (),
                };
            });
            // println!("{}", countdown);
            countdown -= 1;
        }

        // asset generation
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

        let (x, y) = ass.get_sprite_offset(sp).unwrap();
        let (w, h) = ass.get_sprite_size(sp).unwrap();

        let mut stop = false;
        countdown = 120;
        while !stop && countdown > 0 {

            // ~~~~~~~~~~ raw drawing ~~~~~~~~~~~~~~~~
            let mut surface = be.surface();
            surface.clear(&[0.7f32, 0.8, 0.3, 1.0]);
            surface.draw_sprites(&[[0.0, 0.0, 0.0, h, w, x, y, w, h]], 0);
            surface.draw_sprites(&[[0.0, -0.5, -0.5, h*0.5, w*0.5, x, y, w, h]], 0);
            surface.done();

            // ~~~~~~~~~~~   event ~~~~~~~~~~~~~~~~~
            events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, .. } => {
                        match event {
                            glutin::WindowEvent::Closed => stop = true,
                            _ => (),
                        }
                    }
                    _ => (),
                };
            });
            // println!("{}", countdown);
            countdown -= 1;
        }
    }
}

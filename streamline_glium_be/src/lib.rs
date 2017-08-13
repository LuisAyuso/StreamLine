#![cfg_attr(feature="profile", feature(plugin, custom_attribute))]
#![cfg_attr(feature="profile", plugin(flamer))]

#[macro_use]
extern crate glium;
extern crate glutin;
extern crate cgmath;
extern crate streamline;
extern crate image;
extern crate find_folder;
extern crate lru;
extern crate seahash;
extern crate libc;
extern crate freetype_sys as freetype;

#[cfg(feature="profile")]
extern crate flame;

mod line;
mod quad;
mod text;
mod cache;

use streamline::StreamLineBackend;
use streamline::StreamLineBackendSurface;
use streamline::SpriteLayout;
use streamline::LineLayout;
use streamline::RectLayout;
use streamline::TextLayout;
use streamline::Color;
use streamline::tools::RcRef;

use line::LineDraw;
use quad::QuadDraw;
use text::TextDraw;

use glium::Surface;
use image::RgbaImage;

use std::collections::BTreeMap as Map;
use std::rc::Rc;
use std::ops::Deref;

pub struct GliumBackend<'a> {
    display: &'a glium::Display,
    dimensions: (f32, f32),
    map: Rc<Map<u32, glium::texture::Texture2d>>,

    line_draw: RcRef<LineDraw>,
    quad_draw: RcRef<QuadDraw>,
    text_draw: RcRef<TextDraw>,
}


impl<'a> GliumBackend<'a> {
    pub fn new(display: &glium::Display, dim: (u32, u32)) -> GliumBackend {

        GliumBackend {
            display: display,
            dimensions: (dim.0 as f32, dim.1 as f32),
            map: Rc::new(Map::new()),

            line_draw: RcRef::new(LineDraw::new(display)),
            quad_draw: RcRef::new(QuadDraw::new(display)),
            text_draw: RcRef::new(TextDraw::new(display)),
        }
    }
}

impl<'a> StreamLineBackend for GliumBackend<'a> {
    type Surface = GliumBackendSurface<glium::Display>;
    fn add_texture(&mut self, img: RgbaImage) -> u32 {

        let id = self.map.len() as u32;

        let dim = img.dimensions();
        // no idea why, but if not reversed, it just crashes
        // let texture = glium::texture::RawImage2d::from_raw_rgb(img.into_raw(),dim);
        let texture = glium::texture::RawImage2d::from_raw_rgba_reversed(&img.into_raw(), dim);
        let texture = glium::texture::Texture2d::new(self.display, texture).unwrap();

        Rc::get_mut(&mut self.map).expect("no other one should be here").insert(id, texture);
        id
    }

    fn add_font<FIO: std::io::Read>(&mut self, font: FIO) -> u32{
        return self.text_draw.get_mut().add_font(self.display, font);
    }

    fn surface(&mut self, layers: u32) -> Self::Surface {
        GliumBackendSurface {
            frame: self.display.draw(),
            line_draw: self.line_draw.clone(),
            quad_draw: self.quad_draw.clone(),
            text_draw: self.text_draw.clone(),
            dimensions: self.dimensions,
            layers: layers,
            display: self.display.clone(),
            tex_map: self.map.clone(),
        }
    }
}

pub struct GliumBackendSurface<F>
    where F: glium::backend::Facade
{
    frame: glium::Frame,
    line_draw: RcRef<LineDraw>,
    quad_draw: RcRef<QuadDraw>,
    text_draw: RcRef<TextDraw>,
    dimensions: (f32, f32),
    layers: u32,
    display: F,
    // TODO: find a more sophisticated way, maybe when we 
    // get lifetimes in associated types
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
            self.quad_draw.get_mut().draw_tex_quads(&self.display, &mut self.frame, sprites, tex, self.layers);
        }
    }

    fn draw_lines(&mut self, lines: &[LineLayout], width: u32) {
        self.line_draw.get_mut().draw_lines(&self.display, &mut self.frame, lines, width, self.layers);
    }

    fn draw_rects(&mut self, rects: &[RectLayout]) {
        self.quad_draw.get_mut().draw_color_quads(&self.display, &mut self.frame, rects, self.layers);
    }

    fn draw_texts(&mut self, texts: &[TextLayout]){
        self.text_draw.get_mut().draw_texts(&mut self.frame, texts, self.dimensions);
    }

    #[cfg_attr(feature="profile", flame)]
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
        let mut be = GliumBackend::new(&display,(1024, 1024));

        let mut stop = false;
        let mut countdown = 40;
        while !stop && countdown > 0 {

            // ~~~~~~~~~~ raw drawing ~~~~~~~~~~~~~~~~
            let mut surface = be.surface(6);
            surface.clear(&[0.5f32, 0.4, 0.8, 1.0]);
            surface.draw_lines(&[LineLayout([0.0, 0.0, 0.8, 0.8, 0.8, 1.0, 0.0, 0.0, 1.0f32])], 1);
            surface.draw_lines(&[LineLayout([0.0, -0.5, 0.4, 0.4, 0.4, 1.0, 1.0, 0.0, 1.0f32])], 2);
            surface.draw_lines(&[LineLayout([0.0, 100.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0f32])], 3);
            surface.draw_lines(&[LineLayout([0.0, -100.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0f32])], 4);
            surface.draw_lines(&[LineLayout([0.0, 0.0, 100.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0f32])], 5);
            surface.draw_lines(&[LineLayout([0.0, 0.0, -100.0, 0.0, 0.0, 1.0, 0.5, 1.0, 1.0f32])], 6);
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
        countdown = 40;
        while !stop && countdown > 0 {

            // ~~~~~~~~~~ raw drawing ~~~~~~~~~~~~~~~~
            let mut surface = be.surface(1);
            surface.clear(&[0.7f32, 0.8, 0.3, 1.0]);
            surface.draw_sprites(&[SpriteLayout([0.0, 0.0, 0.0, h, w, x, y, w, h])], 0);
            surface.draw_sprites(&[SpriteLayout([0.0, -0.5, -0.5, h*0.5, w*0.5, x, y, w, h])], 0);
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
            countdown -= 1;
        }
    }
}

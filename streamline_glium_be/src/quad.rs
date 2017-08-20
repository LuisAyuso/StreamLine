use glium;
use glium::Surface;
use streamline::SpriteLayout;
use streamline::RectLayout;
use streamline::tools::RcRef;
use cache::VbCache;

use std::vec::Vec;
use glium::texture::Texture2d;

#[derive(Debug, Copy, Clone)]
pub struct TexVertex {
    position: [f32; 3],
    coords: [f32; 2],
}
implement_vertex!(TexVertex, position, coords);

#[derive(Debug, Copy, Clone)]
pub struct ColorVertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(ColorVertex, position, color);

pub struct QuadDraw {
    tex_program: glium::Program,
    color_program: glium::Program,
    spr_cache: RcRef<VbCache<(glium::VertexBuffer<TexVertex>, glium::IndexBuffer<u32>)>>, 
    rec_cache: RcRef<VbCache<(glium::VertexBuffer<ColorVertex>, glium::IndexBuffer<u32>)>>, 
}


impl QuadDraw {
    pub fn new<F>(f: &F)  -> QuadDraw
        where F: glium::backend::Facade
    {

        let tex_program = program!(f,
        410 => {
            vertex: "
                #version 410 core
                in vec3 position;
                in vec2 coords;
                out vec2 vs_coords;
                void main() {
                    vs_coords = coords;
                    gl_Position = vec4(position, 1.0);
                }
            ",
            fragment: "
                #version 410 core
                uniform sampler2D atlas;
                in vec2  vs_coords;
                out vec4 fs_color;
                void main() {
                    fs_color = texture(atlas, (vs_coords.xy)); 
                    //fs_color = texelFetch(atlas, ivec2(vs_coords.xy), 0); 
                }
            ",
		});

        let color_program = program!(f,
        140 => {
            vertex: "
                #version 140
                in vec3 position;
                in vec4 color;
                out vec4 vs_color;
                void main() {
                    vs_color = color;
                    gl_Position = vec4(position, 1.0);
                }
            ",
            fragment: "
                #version 140
                in vec4  vs_color;
                out vec4 fs_color;
                void main() {
                    fs_color = vs_color;
                }
            ",
		});

        QuadDraw { 
            tex_program: tex_program.expect("line shaders do not compile"),
            color_program: color_program.expect("line shaders do not compile"),

            spr_cache: RcRef::new(VbCache::new()),
            rec_cache: RcRef::new(VbCache::new()),
        }
    }

    #[cfg_attr(feature="profile", flame)]
    pub fn draw_tex_quads<F>(&mut self, display: &F, frame: &mut glium::Frame, quads: &[SpriteLayout], texture: &Texture2d, layers: u32) 
        where F: glium::backend::Facade
    {

        // process lines vector, generate some kind of list, here is where the caching could come handy
        let mut cache_ptr = self.spr_cache.clone();
        let mut cache = cache_ptr.get_mut();
        let &(ref vertex_buffer, ref index_buffer) = cache.test(quads, || {
            let mut v = Vec::with_capacity(quads.len()*4);
            let mut i = Vec::with_capacity(quads.len()*6);
            for instance in quads.iter() {

                let &SpriteLayout(l) = instance;

                let depth = 1.0 - (l[0] / layers as f32);

                let x = l[1];
                let y = l[2];
                let w = l[3];
                let h = l[4];

                let t_x = l[5];
                let t_y = l[6];
                let t_w = l[7];
                let t_h = l[8];

                let a = v.len();
                v.push(TexVertex{
                        position: [x, y, depth],
                        coords: [t_x, t_y-t_h],
                        });
                let b = v.len();
                v.push(TexVertex{
                        position: [x+w, y, depth],
                        coords: [t_x + t_w, t_y-t_h],
                        });
                let c = v.len();
                v.push(TexVertex{
                        position: [x, y+h, depth],
                        coords: [t_x, t_y],
                        });

                let d = v.len();
                v.push(TexVertex{
                        position: [x+w, y+h, depth],
                        coords: [t_x + t_w, t_y],
                        });

                i.push(a as u32);
                i.push(b as u32);
                i.push(c as u32);

                i.push(d as u32);
                i.push(b as u32);
                i.push(c as u32);
            }

            (glium::VertexBuffer::new(display, &v).expect("something bad happen when creating vertex buffer"),
             glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &i).expect("something wrong with index generation")
             )
        });

        // some opengl stuff, that we will use as we need
        let uniforms = uniform!(
            atlas: texture
        );

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            polygon_mode: glium::PolygonMode::Fill,
            //polygon_mode: glium::PolygonMode::Line,
            line_width: Some(5.0),
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };

        frame.draw(vertex_buffer, index_buffer,
        		//&glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
         		&self.tex_program, &uniforms, &params).expect("failed to draw lines");
    }

    #[cfg_attr(feature="profile", flame)]
    pub fn draw_color_quads<F>(&mut self, display: &F, frame: &mut glium::Frame, quads: &[RectLayout], layers: u32) 
    where F: glium::backend::Facade
    {
        // process lines vector, generate some kind of list, here is where the caching could come handy
        let mut cache_ptr = self.rec_cache.clone();
        let mut cache = cache_ptr.get_mut();
        let &(ref vertex_buffer, ref index_buffer) = cache.test(quads, || {

            let mut v = Vec::new();
            let mut i = Vec::new();
            for instance in quads.iter() {

                let &RectLayout(l) = instance;

                let depth = 1.0 - (l[0] / layers as f32);

                let x = l[1];
                let y = l[2];
                let h = l[3];
                let w = l[4];

                let r = l[5];
                let g = l[6];
                let b = l[7];
                let a = l[8];

                let p1 = v.len();
                v.push(ColorVertex{
                        position: [x, y, depth],
                        color: [r,g,b,a],
                        });
                let p2 = v.len();
                v.push(ColorVertex{
                        position: [x+w, y, depth],
                        color: [r,g,b,a],
                        });
                let p3 = v.len();
                v.push(ColorVertex{
                        position: [x, y+h, depth],
                        color: [r,g,b,a],
                        });

                let p4 = v.len();
                v.push(ColorVertex{
                        position: [x+w, y+h, depth],
                        color: [r,g,b,a],
                        });

                i.push(p1 as u32);
                i.push(p2 as u32);
                i.push(p3 as u32);

                i.push(p4 as u32);
                i.push(p2 as u32);
                i.push(p3 as u32);
            }

            (glium::VertexBuffer::new(display, &v).expect("something bad happen when creating vertex buffer"),
             glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &i).expect("something wrong with index generation")
             )
        });

        // some opengl stuff, that we will use as we need
        let uniforms = glium::uniforms::EmptyUniforms {};

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            polygon_mode: glium::PolygonMode::Fill,
            ..Default::default()
        };

        frame.draw(vertex_buffer, index_buffer,
                  &self.color_program,
                  &uniforms,
                  &params)
            .expect("failed to draw lines");
    }
}

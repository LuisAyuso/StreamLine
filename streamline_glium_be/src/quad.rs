use glium;
use glium::Surface;
use streamline::SpriteLayout;
use streamline::RectLayout;

use std::vec::Vec;
use glium::texture::Texture2d;

#[derive(Debug, Copy, Clone)]
struct TexVertex {
    position: [f32; 3],
    coords: [f32; 2],
}
implement_vertex!(TexVertex, position, coords);

#[derive(Debug, Copy, Clone)]
struct ColorVertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(ColorVertex, position, color);

pub struct QuadDraw {
    tex_program: glium::Program,
    color_program: glium::Program,
}


impl QuadDraw {
    pub fn new<F>(f: &F) -> QuadDraw
        where F: glium::backend::Facade
    {

        let tex_program = program!(f,
        140 => {
            vertex: "
                #version 140
                in vec3 position;
                in vec2 coords;
                out vec2 vs_coords;
                void main() {
                    vs_coords = coords;
                    gl_Position = vec4(position, 1.0);
                }
            ",
            fragment: "
                #version 140
                uniform sampler2D atlas;
                in vec2  vs_coords;
                out vec4 fs_color;
                void main() {

                    fs_color = texture(atlas, (vs_coords.xy));
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
        }
    }

    pub fn draw_tex_quads<F>(&mut self, display: &F, frame: &mut glium::Frame, quads: &[SpriteLayout], texture: &Texture2d) 
        where F: glium::backend::Facade
    {

        // TODO:  optimizations can be done before this point, somehow we need to cache the vertex
        // list so we do not update it on every frame. maybe this needs to be done at the logic
        // level and not in the backend, the backend should just do.
        // The problem is that there is no concept of vertex buffer passed in advance. this is the
        // first time in execution that we see the vertices data


        // process lines vector, generate some kind of list, here is where the caching could come handy
        let vertex_buffer = {

            let mut v = Vec::new();
            for l in quads.iter() {

                let depth = l[0];

                let x = l[1];
                let y = l[2];
                let w = l[3];
                let h = l[4];

                let t_x = l[5];
                let t_y = l[6];
                let t_w = l[7];
                let t_h = l[8];

                v.push(TexVertex{
                        position: [x, y, depth],
                        coords: [t_x, t_y-t_h],
                        });
                v.push(TexVertex{
                        position: [x+w, y, depth],
                        coords: [t_x + t_w, t_y-t_h],
                        });
                v.push(TexVertex{
                        position: [x, y+h, depth],
                        coords: [t_x, t_y],
                        });

                v.push(TexVertex{
                        position: [x+w, y+h, depth],
                        coords: [t_x + t_w, t_y],
                        });
                v.push(TexVertex{
                        position: [x+w, y, depth],
                        coords: [t_x + t_w, t_y-t_h],
                        });
                v.push(TexVertex{
                        position: [x, y+h, depth],
                        coords: [t_x, t_y],
                        });
            }

                //println!("{:?}", v);
            glium::VertexBuffer::new(display, &v)
                .expect("something bad happen when creating vertex buffer")
        };

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

        frame.draw(&vertex_buffer,
                //&self.indices,
        		&glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
         		&self.tex_program, &uniforms, &params).expect("failed to draw lines");
    }

    pub fn draw_color_quads<F>(&mut self, display: &F, frame: &mut glium::Frame, quads: &[RectLayout]) 
    where F: glium::backend::Facade
    {

        // process lines vector, generate some kind of list, here is where the caching could come handy
        let vertex_buffer = {

            let mut v = Vec::new();
            for l in quads.iter() {

                let depth = l[0];

                let x = l[1];
                let y = l[2];
                let h = l[3];
                let w = l[4];

                let r = l[5];
                let g = l[6];
                let b = l[7];
                let a = l[8];

                v.push(ColorVertex{
                        position: [x, y, depth],
                        color: [r,g,b,a],
                        });
                v.push(ColorVertex{
                        position: [x+w, y, depth],
                        color: [r,g,b,a],
                        });
                v.push(ColorVertex{
                        position: [x, y+h, depth],
                        color: [r,g,b,a],
                        });

                v.push(ColorVertex{
                        position: [x+w, y+h, depth],
                        color: [r,g,b,a],
                        });
                v.push(ColorVertex{
                        position: [x+w, y, depth],
                        color: [r,g,b,a],
                        });
                v.push(ColorVertex{
                        position: [x, y+h, depth],
                        color: [r,g,b,a],
                        });
            }

            // println!("{:?}", v);
            glium::VertexBuffer::new(display, &v)
                .expect("something bad happen when creating vertex buffer")
        };

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

        frame.draw(&vertex_buffer,
                  &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                  &self.color_program,
                  &uniforms,
                  &params)
            .expect("failed to draw lines");
    }
}

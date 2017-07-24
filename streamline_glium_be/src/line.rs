use glium;
use glium::Surface;
use streamline_core::LineLayout;

use std::vec::Vec;


#[derive(Debug, Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}
implement_vertex!(Vertex, position, color);


pub struct LineDraw {
    program: glium::Program,
}


impl LineDraw {
    pub fn new<F>(f: &F) -> LineDraw
        where F: glium::backend::Facade
    {

        let program = program!(f,
        140 => {
            vertex: "
                #version 140

                in vec2 position;
                in vec4 color;

                out vec4 vs_color;

                void main() {
					vs_color = color;
                    gl_Position = vec4(position, 0.0, 1.0);
                }
            ",

            fragment: "
                #version 140

                in vec4 vs_color;
                out vec4 fs_color;

                void main() {
                    fs_color = vs_color;
                }
            ",
		});
        LineDraw { program: program.expect("line shaders do not compile") }
    }

    pub fn draw_lines<F>(&mut self,
                         display: &F,
                         frame: &mut glium::Frame,
                         lines: &[LineLayout],
                         width: u32)
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
            for l in lines.iter() {
                v.push(Vertex {
                    position: [l[0], l[1]],
                    color: [l[4], l[5], l[6], l[7]],
                });
                v.push(Vertex {
                    position: [l[2], l[3]],
                    color: [l[4], l[5], l[6], l[7]],
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
                test: glium::DepthTest::IfLess,
                write: false,
                ..Default::default()
            },
            polygon_mode: glium::PolygonMode::Line,
            line_width: Some(width as f32),
            ..Default::default()
        };

        frame.draw(&vertex_buffer,
                  &glium::index::NoIndices(glium::index::PrimitiveType::LinesList),
                  &self.program,
                  &uniforms,
                  &params)
            .expect("failed to draw lines");
    }
}

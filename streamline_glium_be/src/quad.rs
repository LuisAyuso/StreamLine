use glium;
use glium::Surface;
use streamline_core::SpriteLayout;

use std::vec::Vec;
use glium::texture::Texture2d;

#[derive(Debug, Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    coords: [f32; 2],
}
implement_vertex!(Vertex, position, coords);


pub struct QuadDraw {
    program: glium::Program,
}


impl QuadDraw {
    pub fn new<F>(f: &F) -> QuadDraw
        where F: glium::backend::Facade
    {

        let program = program!(f,
        140 => {
            vertex: "
                #version 140
                in vec2 position;
                in vec2 coords;
                out vec2 vs_coords;
                void main() {
                    vs_coords = coords;
                    gl_Position = vec4(position, 0.0, 1.0);
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

        QuadDraw { 
            program: program.expect("line shaders do not compile"),
        }
    }

    pub fn draw_quads<F>(&mut self, display: &F, frame: &mut glium::Frame, quads: &[SpriteLayout], texture: &Texture2d) 
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

                let x = l[0];
                let y = l[1];
                let w = l[2];
                let h = l[3];

                let t_x = l[4];
                let t_y = l[5];
                let t_w = l[6];
                let t_h = l[7];

                v.push(Vertex{
                        position: [x, y],
                        coords: [t_x, t_y-t_h],
                        });
                v.push(Vertex{
                        position: [x+w, y],
                        coords: [t_x + t_w, t_y-t_h],
                        });
                v.push(Vertex{
                        position: [x, y+h],
                        coords: [t_x, t_y],
                        });

                v.push(Vertex{
                        position: [x+w, y+h],
                        coords: [t_x + t_w, t_y],
                        });
                v.push(Vertex{
                        position: [x+w, y],
                        coords: [t_x + t_w, t_y-t_h],
                        });
                v.push(Vertex{
                        position: [x, y+h],
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
                test: glium::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            polygon_mode: glium::PolygonMode::Fill,
            //polygon_mode: glium::PolygonMode::Line,
            line_width: Some(5.0),
            ..Default::default()
        };

        frame.draw(&vertex_buffer,
                //&self.indices,
        		&glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
         		&self.program, &uniforms, &params).expect("failed to draw lines");
    }
}

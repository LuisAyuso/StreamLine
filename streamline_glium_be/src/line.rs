use glium;
use glium::Surface;
use streamline::LineLayout;
use streamline::tools::RcRef;
use cache::VbCache;

use std::vec::Vec;


#[derive(Debug, Copy, Clone)]
pub struct LineVertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(LineVertex, position, color);


pub struct LineDraw {
    program: glium::Program,
    vb_cache: RcRef<VbCache<glium::VertexBuffer<LineVertex>>>,
}


impl LineDraw {
    pub fn new<F>(f: &F) -> LineDraw
        where F: glium::backend::Facade
    {

        let program = program!(f,
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

                in vec4 vs_color;
                out vec4 fs_color;

                void main() {
                    fs_color = vs_color;
                }
            ",
		});
        LineDraw { 
            program: program.expect("line shaders do not compile") ,
            vb_cache: RcRef::new(VbCache::new()),
        }
    }

    #[cfg_attr(feature="profile", flame)]
    fn create_vb<F>(&mut self,
                         display: &F,
                         lines: &[LineLayout],
                         layers: u32) -> glium::VertexBuffer<LineVertex>
    where F: glium::backend::Facade{
        let mut v = Vec::with_capacity(lines.len()*2);
        for instance in lines.iter() {

            let &LineLayout(l) = instance;

            let depth = 1.0 - (l[0] / layers as f32);

            let x1 = l[1];
            let y1 = l[2];
            let x2 = l[3];
            let y2 = l[4];

            let r = l[5];
            let g = l[6];
            let b = l[7];
            let a = l[8];

            v.push(LineVertex {
                position: [x1, y1, depth],
                color: [r,g,b,a],
            });
            v.push(LineVertex {
                position: [x2, y2, depth],
                color: [r,g,b,a],
            });
        }

        // println!("{:?}", v);
        glium::VertexBuffer::new(display, &v)
            .expect("something bad happen when creating vertex buffer")
    }

    #[cfg_attr(feature="profile", flame)]
    pub fn draw_lines<F>(&mut self,
                         display: &F,
                         frame: &mut glium::Frame,
                         lines: &[LineLayout],
                         width: u32,
                         layers: u32)
        where F: glium::backend::Facade
    {
        // process lines vector, generate some kind of list, here is where the caching could come handy
        let mut cache_ptr = self.vb_cache.clone();
        let mut cache = cache_ptr.get_mut();
        let vertex_buffer = cache.test(lines, || self.create_vb(display, lines, layers) );

        // some opengl stuff, that we will use as we need
        let uniforms = glium::uniforms::EmptyUniforms {};

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            polygon_mode: glium::PolygonMode::Line,
            line_width: Some(width as f32),
            ..Default::default()
        };

        frame.draw(vertex_buffer,
                  &glium::index::NoIndices(glium::index::PrimitiveType::LinesList),
                  &self.program,
                  &uniforms,
                  &params)
            .expect("failed to draw lines");
    }
}

extern crate glium;
extern crate glutin;
extern crate streamline;
extern crate streamline_glium_be;
extern crate find_folder;

use streamline::StreamLineBackend;
use streamline::AssetsMgrBuilder;
use streamline::CmdQueue;

// modifiers
use streamline::Colorize;
use streamline::Contour;
use streamline::maths::vec2;
use streamline::tools::loop_with_report;

use streamline_glium_be::GliumBackend;

use std::path::Path;

const W: u32 = 800;
const H: u32 = 600;

fn main() {

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions(W, H);
    let context = glutin::ContextBuilder::new().with_depth_buffer(24);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // our backend
    let mut be = GliumBackend::new(display, (W, H));

    let file_location = find_folder::Search::Parents(3)
        .for_folder("assets")
        .expect("some assets folder must exist somewhere");

    let mut file1 = file_location.clone();
    let mut file2 = file_location.clone();

    file1.push(Path::new("rust-logo.png"));
    file2.push(Path::new("rust-logo2.png"));

    // phase 1, load assets
    let (ass_mgr, sp1, sp2) = {
        let mut mgr = AssetsMgrBuilder::new(&mut be);
        let sp1 = mgr.add_sprite(&file1);
        let sp2 = mgr.add_sprite(&file2);
        (mgr.build().expect("everithing allright"), sp1, sp2)
    };

    loop_with_report(&mut |_dt: f64| {

        // ~~~~~~~~~~ drawing ~~~~~~~~~~~~~~~~
        let surface = be.surface(8);
        let mut q = CmdQueue::new(surface, &ass_mgr);
        {
            q.clear(&[0.4f32, 0.2, 0.1, 1.0]);

            // sprites grid
            for i in 0..5{
                for j in 0..5{
                    q.sprite(vec2(i * 150, j * 150), 2, sp1);
                    q.sprite(vec2(i * 155, j * 155), 2, sp2);
                }
            }

            // line grid,  just behind the sprites
            //  NO color, therefore should default white
            for j in 0..H/10{
                q.line(vec2(0, j*10), vec2(W, j*10), 1, 0);
            }
            for i in 0..W/10{
                q.line(vec2(i*10, 0), vec2(i*10, H), 1, 0);
            }

            // cross lines, should be red and green
            q.line(vec2(10, 10), vec2(W-10, H-10), 1, 1).with_color(1.0,0.0,0.0,1.0);
            q.line(vec2(10, H-10), vec2(W-10, 10), 1, 1).with_color(0.0,1.0,0.0,1.0);

            q.rect(vec2(0,0), vec2(10, 10), 3).with_color(0.8,0.8,0.8,1.0);

            q.rect(vec2(W/2,H/2), vec2(20, 20), 3).with_color(1.0,0.0,1.0,1.0);

            q.rect(vec2(W/2 + 180, H/2 + 180), vec2(80, 80), 3).with_color(1.0,1.0,0.0,1.0);
            q.rect(vec2(W/2 + 200, H/2 + 200), vec2(40, 40), 4).with_color(0.0,1.0,1.0,1.0)
                .with_border(3).with_color(1.0, 0.0, 0.0, 1.0);

        }
        q.done();

        // ~~~~~~~~~~~   event ~~~~~~~~~~~~~~~~~
        events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } =  event {
                    if let glutin::WindowEvent::Closed = event {
                         std::process::exit(0);
                    }
            };
        });
    }, 3 // 3 seconds refresh
    );
}

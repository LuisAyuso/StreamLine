extern crate glium;
extern crate glutin;
extern crate streamline_core;
extern crate streamline_glium_be;
extern crate find_folder;

use streamline_core::StreamLineBackend;
use streamline_core::AssetsMgrBuilder;
use streamline_core::CmdQueue;
use streamline_core::pos;

use streamline_glium_be::GliumBackend;

use streamline_core::tools::loop_with_report;

use std::path::Path;


const W: u32 = 1500;
const H: u32 = 1500;

fn main() {

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions(W,H);
    let context = glutin::ContextBuilder::new().with_depth_buffer(24);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // our backend
    let mut be = GliumBackend::new(display, (W,H));

    let mut file_location = find_folder::Search::Parents(3)
        .for_folder("assets")
        .expect("some assets folder must exist somewhere");
    file_location.push(Path::new("rust-logo.png"));

    // phase 1, load assets
    let (ass, sp) = {
        let mut mgr = AssetsMgrBuilder::new(&mut be);
        let sp = mgr.add_sprite(&file_location);
        (mgr.build().expect("everithing allright"), sp)
    };

    loop_with_report(&mut |_delta: f64, _pc: &mut streamline_core::PerformaceCounters| {

        // ~~~~~~~~~~ drawing ~~~~~~~~~~~~~~~~
        let surface = be.surface();
        let mut q = CmdQueue::new(&mut be, surface, &ass);
        {
            q.clear(&[0.4f32, 0.2, 0.1, 1.0]);
            q.draw_line(pos(0,0), pos(10,10), 1);
            q.draw_line(pos(0,0), pos(100,100), 1);

            for i in 0..5{
                for j in 0..5{
                    q.draw_sprite(pos(i * 300, j * 300), 0, sp);
                }
            }

            for j in 0..H/10{
                q.draw_line(pos(0, j*10), pos(W, j*10), 0);
            }
            for i in 0..W/10{
                q.draw_line(pos(i*10, 0), pos(i*10, H), 0);
            }
            q.draw_line(pos(10, 10), pos(W-10, H-10), 0);
            q.draw_line(pos(10, H-10), pos(W-10, 10), 0);
        }
        q.done();

        // ~~~~~~~~~~~   event ~~~~~~~~~~~~~~~~~
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => {
                    match event {
                        glutin::WindowEvent::Closed => std::process::exit(0),
                        _ => (),
                    }
                }
                _ => (),
            };
        });
    }, 3 // 3 seconds refresh
    ); 
}

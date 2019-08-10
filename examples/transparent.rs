//! An incomplete reimplementation of Desktop Ponies, demonstrating the use of
//! a non-opaque window
use log::debug;
use std::time::{Duration, Instant};
use swsurface::{Format, SwWindow};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const FORMAT: Format = Format::Argb8888;
const FB_SIZE: [u32; 2] = [250, 150];

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let event_loop = EventLoop::new();

    let event_loop_proxy = event_loop.create_proxy();
    let sw_context = swsurface::ContextBuilder::new(&event_loop)
        .with_ready_cb(move |_| {
            let _ = event_loop_proxy.send_event(());
        })
        .build();

    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size((FB_SIZE[0], FB_SIZE[1]).into())
        .with_resizable(false)
        .with_always_on_top(true)
        .build(&event_loop)
        .unwrap();

    let sw_window = SwWindow::new(
        window,
        &sw_context,
        &swsurface::Config {
            opaque: false,
            ..Default::default()
        },
    );
    sw_window.update_surface_to_fit(FORMAT);
    sw_window.window().request_redraw();

    let assets: &'static Assets = Box::leak(Box::new(Assets::new()));
    let mut state = State::new(assets);

    let mut win_pos = randomize_wnd_pos(sw_window.window());
    sw_window.window().set_outer_position(win_pos);
    debug!("Placing the window at {:?}", win_pos);

    event_loop.run(move |event, _, control_flow| {
        let encourage_teleport = is_wnd_partially_escaping(sw_window.window());

        let (action, needs_redraw) = state.update(&assets, encourage_teleport);

        match action {
            None => {}
            Some(WndAction::Teleport) => {
                win_pos = randomize_wnd_pos(sw_window.window());
                sw_window.window().set_outer_position(win_pos);
                debug!("Teleporting the window to {:?}", win_pos);
            }
            Some(WndAction::Move(velocity_x)) => {
                win_pos.x += velocity_x as f64;
                sw_window.window().set_outer_position(win_pos);
            }
        }

        if needs_redraw {
            redraw(&sw_window, &state);
        }

        // Ideally this should be calculated based on the animation state
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(10));

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) | WindowEvent::HiDpiFactorChanged(_) => {
                    sw_window.update_surface_to_fit(FORMAT);
                    redraw(&sw_window, &state);
                }
                WindowEvent::RedrawRequested => {
                    redraw(&sw_window, &state);
                }
                _ => {}
            },
            Event::UserEvent(_) => {
                redraw(&sw_window, &state);
            }
            _ => {}
        }
    });
}

fn is_wnd_partially_escaping(window: &Window) -> bool {
    let monitor = window.current_monitor();

    let dpi = monitor.hidpi_factor();
    let mon_pos = monitor.position().to_logical(dpi);
    let mon_size = monitor.size().to_logical(dpi);

    let wnd_pos = window.outer_position().unwrap();
    let wnd_size = window.outer_size();

    wnd_pos.x < mon_pos.x
        || wnd_pos.y < mon_pos.y
        || wnd_pos.x > mon_pos.x + mon_size.width - wnd_size.width
        || wnd_pos.y > mon_pos.y + mon_size.height - wnd_size.height
}

fn randomize_wnd_pos(window: &Window) -> winit::dpi::LogicalPosition {
    use rand::{seq::SliceRandom, Rng};
    let mut rng = rand::thread_rng();

    let monitors: Vec<_> = window.available_monitors().collect();
    let monitor = monitors
        .choose(&mut rng)
        .expect("could not find any monitor");

    let dpi = monitor.hidpi_factor();
    let mon_pos = monitor.position().to_logical(dpi);
    let mon_size = monitor.size().to_logical(dpi);

    let wnd_size = window.outer_size();

    // `X_MARGIN` should be larger, otherwise the pony will escape too often
    const X_MARGIN: f64 = 200.0;
    const Y_MARGIN: f64 = 50.0;
    let x_range = X_MARGIN..mon_size.width - X_MARGIN - wnd_size.width;
    let y_range = Y_MARGIN..mon_size.height - Y_MARGIN - wnd_size.height;

    winit::dpi::LogicalPosition {
        x: rng.gen_range(x_range.start, x_range.end.max(x_range.start)) + mon_pos.x,
        y: rng.gen_range(y_range.start, y_range.end.max(y_range.start)) + mon_pos.y,
    }
}

fn redraw(sw_window: &SwWindow, state: &State<'_>) {
    if let Some(image_index) = sw_window.poll_next_image() {
        paint_image(
            &mut sw_window.lock_image(image_index),
            sw_window.image_info(),
            state,
        );

        sw_window.present_image(image_index);
    }
}

fn paint_image(pixels: &mut [u8], image_info: swsurface::ImageInfo, state: &State<'_>) {
    use image::{Pixel, Rgba};

    let mut native_image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        image_info.stride as u32 / 4,
        image_info.extent[1],
        pixels,
    )
    .unwrap();

    let mut image = image::ImageBuffer::<image::Rgba<u8>, _>::new(FB_SIZE[0], FB_SIZE[1]);

    for p in image.pixels_mut() {
        *p = Rgba([0, 0, 0, 0]);
    }

    let (off_x, off_y) = match state.mode {
        Mode::Arrive => (24, 42),
        Mode::Gallop => (17, 0),
        Mode::Magic => (20, 16),
        Mode::Stand => (6, 4),
        Mode::Teleport => (40, 40),
        Mode::Trot => (6, 6),
    };

    let off_x = if state.orientation {
        160 - (40 - off_x)
    } else {
        40 - off_x
    };
    let off_y = 42 - off_y;

    // Draw the current frame onto the framebuffer (`image`)
    let frame = state.cur_anim.frames[0].buffer();
    for y in 0..frame.height() {
        if state.orientation {
            assert!(off_x >= frame.width());
            for x in 0..frame.width() {
                image
                    .get_pixel_mut(off_x - x, y + off_y)
                    .blend(frame.get_pixel(x, y));
            }
        } else {
            assert!(off_x + frame.width() <= image.width());
            for x in 0..frame.width() {
                image
                    .get_pixel_mut(x + off_x, y + off_y)
                    .blend(frame.get_pixel(x, y));
            }
        }
    }

    for p in image.pixels_mut() {
        p.0.swap(0, 2);
    }

    // Upsample `image` to the native size
    for y in 0..native_image.height() {
        let in_y = y * image.height() / native_image.height();
        let mut in_x = 0;
        for x in 0..native_image.width() {
            *native_image.get_pixel_mut(x, y) = *image.get_pixel(in_x >> 16, in_y);
            in_x += (image.width() << 16) / native_image.width();
        }
    }
}

struct State<'a> {
    cur_anim: PlaybackState<'a>,
    orientation: bool,
    mode: Mode,
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Arrive,
    Gallop,
    Magic,
    Stand,
    Teleport,
    Trot,
}

#[derive(Debug, Clone, Copy)]
enum WndAction {
    Move(i32),
    Teleport,
}

impl<'a> State<'a> {
    fn new(assets: &'a Assets) -> Self {
        Self {
            cur_anim: PlaybackState::new(&assets.arrive),
            orientation: false,
            mode: Mode::Arrive,
        }
    }

    fn update(
        &mut self,
        assets: &'a Assets,
        encourage_teleport: bool,
    ) -> (Option<WndAction>, bool) {
        use rand::{seq::SliceRandom, Rng};
        let mut rng = rand::thread_rng();

        let mut action = None;

        let (done, updated) = self.cur_anim.update();
        if updated {
            // Synchronize the walk movement to the animation update interval
            let orient_int = [-1, 1][self.orientation as usize];
            match self.mode {
                Mode::Gallop => action = Some(WndAction::Move(orient_int * 14)),
                Mode::Trot => action = Some(WndAction::Move(orient_int * 6)),
                _ => {}
            }
        }

        if done {
            // Since mutable borrow in a pattern guard is illegal...
            let dice = rng.gen_range(0, 6);

            // Choose the next mode
            self.mode = match self.mode {
                Mode::Teleport => {
                    action = Some(WndAction::Teleport);
                    self.orientation = rng.gen();
                    Mode::Arrive
                }

                _ if encourage_teleport => Mode::Teleport,

                // Continue the current action
                x @ Mode::Gallop | x @ Mode::Trot if dice > 1 => x,

                _ => {
                    // Choose randomly
                    *[
                        Mode::Gallop,
                        Mode::Magic,
                        Mode::Stand,
                        Mode::Teleport,
                        Mode::Trot,
                    ]
                    .choose(&mut rng)
                    .unwrap()
                }
            };

            debug!("The next mode is {:?}", self.mode);

            match self.mode {
                Mode::Arrive => self.cur_anim = PlaybackState::new(&assets.arrive),
                Mode::Gallop => self.cur_anim = PlaybackState::new(&assets.gallop),
                Mode::Magic => self.cur_anim = PlaybackState::new(&assets.magic),
                Mode::Stand => self.cur_anim = PlaybackState::new(&assets.stand),
                Mode::Teleport => self.cur_anim = PlaybackState::new(&assets.teleport),
                Mode::Trot => self.cur_anim = PlaybackState::new(&assets.trot),
            }
        }

        (action, updated || done)
    }
}

struct PlaybackState<'a> {
    frames: &'a [Frame],
    timer: Instant,
    frame_start: Duration,
}

impl<'a> PlaybackState<'a> {
    fn new(frames: &'a [Frame]) -> Self {
        Self {
            frames,
            timer: Instant::now(),
            frame_start: Duration::new(0, 0),
        }
    }

    fn update(&mut self) -> (bool, bool) {
        let t = self.timer.elapsed();
        let mut updated = false;

        while self.frames.len() > 0 {
            let frame_dur = Duration::from_millis(self.frames[0].delay().to_integer() as _);
            let frame_end = self.frame_start + frame_dur;
            if t < frame_end {
                return (false, updated);
            }
            self.frame_start = frame_end;
            self.frames = &self.frames[1..];
            updated = true;
        }

        (true, updated)
    }
}

use image::Frame;

struct Assets {
    arrive: Vec<Frame>,
    magic: Vec<Frame>,
    stand: Vec<Frame>,
    teleport: Vec<Frame>,
    gallop: Vec<Frame>,
    trot: Vec<Frame>,
}

impl Assets {
    fn new() -> Self {
        static PACK: &[u8] = include_bytes!("horse.tar.zstd");

        use std::io::Cursor;
        let tar_data = zstd::decode_all(Cursor::new(PACK)).unwrap();

        use tar::Archive;
        let mut tar_reader = Archive::new(Cursor::new(&tar_data[..]));

        use std::{collections::HashMap, io::prelude::*};
        let files: HashMap<String, Vec<u8>> = tar_reader
            .entries()
            .unwrap()
            .map(|ent| {
                let mut ent = ent.unwrap();
                debug!("Loading file {:?}", ent.path().unwrap());
                let path = ent.path().unwrap().to_str().unwrap().to_owned();

                let mut data = Vec::new();
                ent.read_to_end(&mut data).unwrap();

                (path, data)
            })
            .collect();

        fn decode_gif(data: &[u8]) -> Vec<Frame> {
            use image::AnimationDecoder;

            debug!(
                "Decoding {:x?}... ({} bytes) as a GIF image",
                &data[0..30],
                data.len()
            );

            let decoder = image::gif::Decoder::new(Cursor::new(data)).unwrap();
            decoder.into_frames().collect_frames().unwrap()
        }

        Self {
            arrive: decode_gif(files.get("arrive_left.gif").unwrap()),
            magic: decode_gif(files.get("magic_twilight_left.gif").unwrap()),
            stand: decode_gif(files.get("stand_twilight_left.gif").unwrap()),
            teleport: decode_gif(files.get("teleport_left.gif").unwrap()),
            gallop: decode_gif(files.get("twilight_gallop_left.gif").unwrap()),
            trot: decode_gif(files.get("twilight_trot_left.gif").unwrap()),
        }
    }
}

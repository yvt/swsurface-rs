use swsurface::{Format, SwWindow};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("plasma")
        .build(&event_loop)
        .unwrap();

    let sw_context = swsurface::ContextBuilder::new(&event_loop).build();

    let sw_window = SwWindow::new(window, &sw_context, &Default::default());

    // Find the suitable pixel format. Wwe don't want to generate non-opaque
    // pixels, `Xrgb8888` is the ideal choice. `Argb8888` is acceptable too
    // because we can generate valid alpha values.
    let format = [Format::Xrgb8888, Format::Argb8888]
        .iter()
        .cloned()
        .find(|&fmt1| sw_window.supported_formats().any(|fmt2| fmt1 == fmt2))
        .unwrap();

    sw_window.update_surface_to_fit(format);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(_) | WindowEvent::HiDpiFactorChanged(_) => {
                sw_window.update_surface_to_fit(format);
                redraw(&sw_window);
            }
            WindowEvent::RedrawRequested => {
                redraw(&sw_window);
            }
            _ => {}
        },

        Event::EventsCleared => {
            sw_window.window().request_redraw();
        }
        _ => *control_flow = ControlFlow::Poll,
    });
}

fn redraw(sw_window: &SwWindow) {
    if let Some(image_index) = sw_window.wait_next_image() {
        paint_image(
            &mut sw_window.lock_image(image_index),
            sw_window.image_info(),
        );

        sw_window.present_image(image_index);
    }
}

fn paint_image(pixels: &mut [u8], image_info: swsurface::ImageInfo) {
    use std::{num::Wrapping, time::Instant};

    const TABLE_SIZE: usize = 256;

    lazy_static::lazy_static! {
        static ref SIN: Vec<u8> = (0..TABLE_SIZE).map(|i| {
            let x = (i as f32 * std::f32::consts::PI * 2.0 / TABLE_SIZE as f32).sin();
            (x * 127.0 + 127.0) as u8
        }).collect();
        static ref T: Instant = Instant::now();
    }

    let sin = &SIN[0..TABLE_SIZE];
    let get_sin = |i: Wrapping<u32>| Wrapping(sin[i.0 as usize % TABLE_SIZE] as u32);

    let t = Wrapping((T.elapsed().as_millis() * 20) as u32);

    let [size_w, size_h] = image_info.extent;
    for y in 0..size_h as usize {
        let row = pixels[y * image_info.stride..][..size_w as usize * 4].chunks_exact_mut(4);

        let mut phases = [
            Wrapping((y * 165) as u32),
            Wrapping((y * 17) as u32),
            Wrapping((y * 75) as u32),
            Wrapping((y * 23) as u32),
            Wrapping((y * 97) as u32),
            Wrapping((y * 53) as u32),
            Wrapping((y * 23) as u32),
            Wrapping((y * 150) as u32),
        ];

        for x in &mut phases {
            *x += t;
        }

        for (x, p) in row.enumerate() {
            const FAC1: Wrapping<u32> = Wrapping(256);
            const FAC2: Wrapping<u32> = Wrapping(2);

            let val1 = get_sin(phases[0] / FAC1 + get_sin(phases[1] / FAC1))
                + get_sin(phases[4] / FAC1 + get_sin(phases[5] / FAC1));
            let val2 = get_sin(phases[2] / FAC1 + get_sin(phases[3] / FAC1) * FAC2)
                + get_sin(phases[6] / FAC1 + get_sin(phases[7] / FAC1) * FAC2);
            let val3 = get_sin(phases[0] / FAC1 + get_sin(phases[3] / FAC1))
                + get_sin(phases[4] / FAC1 + get_sin(phases[1] / FAC1));

            let mask2 = 0u8.wrapping_sub((x as u8 >> 3).wrapping_add(y as u8 >> 2) & 1);
            let mask1 = 0u8.wrapping_sub((x as u8 >> 2).wrapping_add(y as u8 >> 2) & 1) & !mask2;

            p[0] = (val2.0 / 2) as u8 & mask2; // B
            p[1] = (val3.0 / 2) as u8 & mask1; // G
            p[2] = (val1.0 / 2) as u8 & !mask2; // R
            p[3] = 255;

            phases[0] += Wrapping(57);
            phases[1] += Wrapping(70);
            phases[2] += Wrapping(24);
            phases[3] += Wrapping(62);
            phases[4] += Wrapping(37);
            phases[5] += Wrapping(20);
            phases[6] += Wrapping(103);
            phases[7] += Wrapping(47);
        }
    }
}

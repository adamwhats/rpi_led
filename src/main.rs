mod e131;
mod ha;
mod noise_map;

use rs_ws281x::ChannelBuilder;
use rs_ws281x::Controller;
use rs_ws281x::ControllerBuilder;
use rs_ws281x::StripType;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use noise_map::NoiseMap;

const UNIVERSE1: u16 = 1;
const NUM_LEDS: i32 = 32;

// The PC counts as gone after this long without a packet (the sACN data loss timeout)
const PC_TIMEOUT: Duration = Duration::from_millis(2500);
// Another sender may take over once the followed one has been quiet this long.
// Longer than OpenRGB's keepalive interval, so a static colour isn't interrupted.
const SENDER_SWITCH_AFTER: Duration = Duration::from_millis(1500);
const FRAME_INTERVAL: Duration = Duration::from_micros(1_000_000 / 60);

fn main() {

    // Initialise E1.31 receiver
    let mut dmx_rcv = e131::Receiver::new(UNIVERSE1);

    // Initialise LED controller
    let mut controller = ControllerBuilder::new()
        .dma(10)
        .channel(
            0, // Channel Index
            ChannelBuilder::new()
                .pin(18) // PWM0
                .count(NUM_LEDS) // Number of LEDs
                .strip_type(StripType::Ws2812)
                .brightness(255) // default: 255
                .build(),
        )
        .build()
        .unwrap();

    let ha = ha::Ha::start();
    let mut noise = NoiseMap::new();
    let mut frame = [[0u8; 3]; NUM_LEDS as usize];
    let mut pc_frame = [[0u8; 3]; NUM_LEDS as usize];
    // Two OpenRGB instances on the PC stream different frames, which flickers
    // if both are shown, so only one sender is followed at a time
    let mut sender: Option<(SocketAddr, Instant)> = None;
    let mut last_pc_packet: Option<Instant> = None;
    let mut pc_streaming = false;
    let mut next_frame = Instant::now();

    // Main loop: wait for E1.31 until the next local frame is due
    loop {
        let timeout = next_frame.saturating_duration_since(Instant::now()).max(Duration::from_millis(1));
        let mut new_pc_frame = false;
        if let Some(packet) = dmx_rcv.recv(timeout) {
            let now = Instant::now();
            last_pc_packet = Some(now);
            let followed = sender.map(|(addr, _)| addr);
            let followed_quiet = sender.is_none_or(|(_, t)| t.elapsed() > SENDER_SWITCH_AFTER);
            if followed == Some(packet.sender) || followed_quiet {
                if followed != Some(packet.sender) {
                    println!("Following E1.31 sender {}", packet.sender);
                }
                sender = if packet.terminated { None } else { Some((packet.sender, now)) };
                for (px, chunk) in pc_frame.iter_mut().zip(packet.dmx.chunks_exact(3)) {
                    *px = [chunk[0], chunk[1], chunk[2]];
                }
                new_pc_frame = !packet.terminated;
            }
        }

        // The PC starting or stopping its stream switches the light on or off;
        // between those, only HA changes it
        let streaming = last_pc_packet.is_some_and(|t| t.elapsed() < PC_TIMEOUT);
        if streaming != pc_streaming {
            println!("PC {} streaming", if streaming { "started" } else { "stopped" });
            ha.set_on(streaming);
            pc_streaming = streaming;
            next_frame = Instant::now();
        }

        if !ha.is_on() {
            if Instant::now() >= next_frame {
                show(&mut controller, &[[0, 0, 0]; NUM_LEDS as usize]);
                next_frame = Instant::now() + Duration::from_millis(100);
            }
        } else if pc_streaming {
            if new_pc_frame {
                show(&mut controller, &pc_frame);
            }
            // Wake at least once a second to notice the PC going quiet
            next_frame = Instant::now() + Duration::from_secs(1);
        } else if Instant::now() >= next_frame {
            noise.render(&mut frame);
            show(&mut controller, &frame);
            next_frame = (next_frame + FRAME_INTERVAL).max(Instant::now());
        }
    }
}

// Takes RGB; the strip's wiring needs the channels swapped as below
fn show(controller: &mut Controller, frame: &[[u8; 3]]) {
    for (led, &[r, g, b]) in controller.leds_mut(0).iter_mut().zip(frame) {
        *led = [r, b, g, 0];
    }
    let _ = controller.render();
}

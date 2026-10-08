mod ha;
mod noise_map;

use rs_ws281x::ChannelBuilder;
use rs_ws281x::Controller;
use rs_ws281x::ControllerBuilder;
use rs_ws281x::StripType;

use sacn::packet::ACN_SDT_MULTICAST_PORT;
use sacn::receive::SacnReceiver;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use noise_map::NoiseMap;

const UNIVERSE1: u16 = 1;
const NUM_LEDS: i32 = 32;

// The PC counts as gone after this long without a packet (the sACN data loss timeout)
const PC_TIMEOUT: Duration = Duration::from_millis(2500);
const FRAME_INTERVAL: Duration = Duration::from_micros(1_000_000 / 60);

fn main() {

    // Initialise sACN receiver
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), ACN_SDT_MULTICAST_PORT);
    let mut dmx_rcv = SacnReceiver::with_ip(addr, None).unwrap();
    dmx_rcv.listen_universes(&[UNIVERSE1]).unwrap();

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
    let mut last_pc_packet: Option<Instant> = None;
    let mut pc_streaming = false;
    let mut next_frame = Instant::now();

    // Main loop: wait for E1.31 until the next local frame is due
    loop {
        let timeout = next_frame.saturating_duration_since(Instant::now()).max(Duration::from_millis(1));
        let packet = dmx_rcv.recv(Some(timeout)).ok();
        if packet.is_some() {
            last_pc_packet = Some(Instant::now());
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
            if let Some(dmx) = packet.as_ref().and_then(|p| p.first()) {
                for (px, chunk) in frame.iter_mut().zip(dmx.values[1..].chunks_exact(3)) {
                    *px = [chunk[0], chunk[1], chunk[2]];
                }
                show(&mut controller, &frame);
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

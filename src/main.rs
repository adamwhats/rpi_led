use rs_ws281x::ChannelBuilder;
use rs_ws281x::ControllerBuilder;
use rs_ws281x::StripType;

use sacn::packet::ACN_SDT_MULTICAST_PORT;
use sacn::receive::SacnReceiver;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

const UNIVERSE1: u16 = 1;
const NUM_LEDS: i32= 32;
const TIMEOUT: Option<Duration> = Some(Duration::from_secs(1));

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
                .pin(18) // GPIO 10 = SPI0 MOSI
                .count(NUM_LEDS) // Number of LEDs
                .strip_type(StripType::Ws2812)
                .brightness(255) // default: 255
                .build(),
        )
        .build()
        .unwrap();


    // Main loop
    loop {
        match dmx_rcv.recv(TIMEOUT) {
            Err(_e) => {
                for led in controller.leds_mut(0) {
                    *led = [0, 0, 0, 0];
                }
            }
            Ok(p) => {
                for (led, chunk) in controller.leds_mut(0).iter_mut().zip(p[0].values[1..].chunks_exact(3)) {
                    led[0] = chunk[0];
                    led[1] = chunk[2];
                    led[2] = chunk[1];
                    led[3] = 0;
                }
            }
        }

        let _ = controller.render();

    }
}
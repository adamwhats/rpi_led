import argparse
import logging
import time

from rpi_ws281x import Color, PixelStrip
from sacn import DataPacket, sACNreceiver

# Define the callback function
def update_strip(packet: DataPacket, strip: PixelStrip) -> None:
    if packet.dmxStartCode == 0x00:  # Ignore non-DMX-data packets
        if logging.getLogger().level <= logging.DEBUG:
            print("-" * 80)
        logging.debug(f"Packet received, size = {len(packet.dmxData)}")

        for pixel_idx in range(len(packet.dmxData) // 3):
            pixel_rgb = packet.dmxData[3 * pixel_idx:3 * pixel_idx + 3]
            logging.debug(f"Setting pixel {pixel_idx} to {pixel_rgb}")
            strip.setPixelColor(pixel_idx, Color(*pixel_rgb))
            # Break here to prevent unnecessary iteration as sacn normalizes the length of packet.dmxData to 512
            if pixel_idx == args.led_count - 1:
               break
        strip.show()

if __name__ == "__main__":
    # Parse CLI arguments
    parser = argparse.ArgumentParser(prog="rpi_led",
                                     description="A simple script for capturing E1.31 packets on a Raspberry Pi and controlling a strip of WS2812 LEDs. Written with the goal of integrating with OpenRGB",
                                     epilog="Adam Watts adamlwatts@msn.com")
    parser.add_argument("--sender", type=str, default="127.0.0.1",
                        help="Provide an IP address to bind to if you expect packets from a specific interface")
    parser.add_argument("--led_count", type=int, default=10,
                        help="Number of LEDs in the strip")
    parser.add_argument("--led_pin", type=int, default=18,
                        help="GPIO pin connected to the pixels. This should be a pin with PWM")
    parser.add_argument("--led_freq", type=int, default=800000,
                        help="LED signal frequency in hertz (usually 800khz)")
    parser.add_argument("--led_dma", type=int, default=10,
                        help="DMA channel to use for generating signal")
    parser.add_argument("--led_brightness", type=int, default=255,
                        help="Set to 0 for darkest and 255 for brightest")
    parser.add_argument("--led_invert", type=bool, default=False,
                        help="True to invert the signal (when using NPN transistor level shift)")
    parser.add_argument("--led-channel", type=int, default=0,
                        help="Set to '1' for GPIOs 13, 19, 41, 45 or 53")
    parser.add_argument("--log_level", type=str, default='INFO', choices=logging.getLevelNamesMapping().keys(),
                        help="The severity threshold for displaying logging messages")
    args = parser.parse_args()

    # Initialise logger
    logging.basicConfig(level=logging.getLevelName(args.log_level))

    # Initialise WS2812 strip
    strip = PixelStrip(args.led_count, args.led_pin, args.led_freq, args.led_dma,
                       args.led_invert, args.led_brightness, args.led_channel)
    strip.begin()

    # Initialise sACN receiver and register callback
    receiver = sACNreceiver()

    # Register the callback
    @receiver.listen_on('universe', universe=1)
    def universe_callback(packet: DataPacket) -> None:
        update_strip(packet, strip)

    # Main loop

    receiver.start()
    try:
        while True:
            time.sleep(0.05)
    finally:
        logging.warning("Closing rpi_led")
        receiver.stop()


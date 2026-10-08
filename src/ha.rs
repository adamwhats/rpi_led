// Home Assistant control over MQTT. Registers a single on/off light, "Office
// LEDs". The main loop also switches it when the PC starts or stops streaming.

use rumqttc::{Client, Event, LastWill, MqttOptions, Packet, QoS};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const BROKER_HOST: &str = "192.168.4.12";
const BROKER_PORT: u16 = 1883;

const TOPIC_AVAILABILITY: &str = "pi_led/availability";
const TOPIC_STATE: &str = "pi_led/light/state";
const TOPIC_SET: &str = "pi_led/light/set";

pub struct Ha {
    client: Client,
    // Not persisted: after a restart the light is off until the PC or HA turns it on
    on: Arc<AtomicBool>,
}

impl Ha {
    // Connects in a background thread, reconnecting forever, so the strip keeps
    // working from E1.31 alone while the broker is unreachable
    pub fn start() -> Self {
        let mut options = MqttOptions::new("pi_led", BROKER_HOST, BROKER_PORT);
        options.set_keep_alive(Duration::from_secs(30));
        options.set_last_will(LastWill::new(TOPIC_AVAILABILITY, "offline", QoS::AtLeastOnce, true));

        let (client, mut connection) = Client::new(options, 16);
        let on = Arc::new(AtomicBool::new(false));

        let (thread_client, thread_on) = (client.clone(), on.clone());
        thread::spawn(move || {
            // This thread drives the event loop, so it must only use try_* calls:
            // a blocking publish here would wait on itself once the queue fills
            for event in connection.iter() {
                match event {
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {
                        on_connect(&thread_client, thread_on.load(Ordering::Relaxed));
                    }
                    Ok(Event::Incoming(Packet::Publish(p))) if p.topic == TOPIC_SET => {
                        if let Some(command) = parse_command(&p.payload) {
                            thread_on.store(command, Ordering::Relaxed);
                            publish_state(&thread_client, command);
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("MQTT: {e}");
                        thread::sleep(Duration::from_secs(5));
                    }
                }
            }
        });

        Ha { client, on }
    }

    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    pub fn set_on(&self, on: bool) {
        self.on.store(on, Ordering::Relaxed);
        publish_state(&self.client, on);
    }
}

fn on_connect(client: &Client, on: bool) {
    let config = json!({
        "name": null,
        "unique_id": "office_leds",
        "schema": "json",
        "supported_color_modes": ["onoff"],
        "state_topic": TOPIC_STATE,
        "command_topic": TOPIC_SET,
        "availability_topic": TOPIC_AVAILABILITY,
        "icon": "mdi:led-strip-variant",
        "device": {
            "identifiers": ["pi_led"],
            "name": "Office LEDs",
            "manufacturer": "Raspberry Pi",
            "model": "Pi Zero W + WS2812",
        },
    });

    let publish = |topic: &str, payload: String| {
        let _ = client.try_publish(topic, QoS::AtLeastOnce, true, payload);
    };
    publish("homeassistant/light/pi_led/light/config", config.to_string());
    publish(TOPIC_AVAILABILITY, "online".into());
    publish_state(client, on);
    let _ = client.try_subscribe(TOPIC_SET, QoS::AtLeastOnce);
}

// Commands follow HA's JSON light schema, e.g. {"state":"ON"}
fn parse_command(payload: &[u8]) -> Option<bool> {
    let command = serde_json::from_slice::<Value>(payload).ok();
    let state = command.as_ref().and_then(|c| c["state"].as_str());
    if state.is_none() {
        eprintln!("MQTT: ignoring malformed command {:?}", String::from_utf8_lossy(payload));
    }
    state.map(|s| s == "ON")
}

fn publish_state(client: &Client, on: bool) {
    let state = json!({
        "state": if on { "ON" } else { "OFF" },
        "color_mode": "onoff",
    });
    let _ = client.try_publish(TOPIC_STATE, QoS::AtLeastOnce, true, state.to_string());
}

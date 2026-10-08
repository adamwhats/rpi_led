// Port of the OpenRGB Effects Plugin "Noise map" effect, so the strip can run
// the same pattern when OpenRGB on the PC isn't streaming. Transcribed from
// Effects/NoiseMap/NoiseMap.cpp (rainbow colours), SRombauts' SimplexNoise and
// OpenRGB's hsv2rgb, keeping their f32 arithmetic so output matches bit for bit.

// Settings from the "noise-map" effect profile in OpenRGB. motion_speed is 0
// there, so the motion offset is always zero and is left out.
const FREQUENCY: f32 = 0.3949;
const AMPLITUDE: f32 = 0.0001;
const LACUNARITY: f32 = 0.199;
const PERSISTENCE: f32 = 5.0;
const OCTAVES: usize = 3;
const SPEED: f64 = 21.0;
const FPS: f64 = 60.0;

// NoiseMap advances progress by this much per frame, so call render() at FPS
const STEP: f64 = 0.1 * SPEED / FPS;

const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186,
    3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59,
    227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70,
    221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178,
    185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241, 81,
    51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204, 176, 115,
    121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195,
    78, 66, 215, 61, 156, 180
];

fn fastfloor(fp: f32) -> i32 {
    let i = fp as i32;
    if fp < i as f32 { i - 1 } else { i }
}

fn hash(i: i32) -> i32 {
    PERM[i as u8 as usize] as i32
}

fn grad(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    (if h & 1 != 0 { -u } else { u }) + (if h & 2 != 0 { -v } else { v })
}

fn corner(gi: i32, x: f32, y: f32, z: f32) -> f32 {
    let t = 0.6 - x * x - y * y - z * z;
    if t < 0.0 {
        0.0
    } else {
        let t = t * t;
        t * t * grad(gi, x, y, z)
    }
}

fn noise(x: f32, y: f32, z: f32) -> f32 {
    const F3: f32 = 1.0 / 3.0;
    const G3: f32 = 1.0 / 6.0;

    let s = (x + y + z) * F3;
    let i = fastfloor(x + s);
    let j = fastfloor(y + s);
    let k = fastfloor(z + s);
    let t = (i + j + k) as f32 * G3;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1, 0, 0, 1, 1, 0)
        } else if x0 >= z0 {
            (1, 0, 0, 1, 0, 1)
        } else {
            (0, 0, 1, 1, 0, 1)
        }
    } else if y0 < z0 {
        (0, 0, 1, 0, 1, 1)
    } else if x0 < z0 {
        (0, 1, 0, 0, 1, 1)
    } else {
        (0, 1, 0, 1, 1, 0)
    };

    let gi0 = hash(i + hash(j + hash(k)));
    let gi1 = hash(i + i1 + hash(j + j1 + hash(k + k1)));
    let gi2 = hash(i + i2 + hash(j + j2 + hash(k + k2)));
    let gi3 = hash(i + 1 + hash(j + 1 + hash(k + 1)));

    let n0 = corner(gi0, x0, y0, z0);
    let n1 = corner(gi1, x0 - i1 as f32 + G3, y0 - j1 as f32 + G3, z0 - k1 as f32 + G3);
    let n2 = corner(gi2, x0 - i2 as f32 + 2.0 * G3, y0 - j2 as f32 + 2.0 * G3, z0 - k2 as f32 + 2.0 * G3);
    let n3 = corner(gi3, x0 - 1.0 + 3.0 * G3, y0 - 1.0 + 3.0 * G3, z0 - 1.0 + 3.0 * G3);
    32.0 * (n0 + n1 + n2 + n3)
}

fn fractal(x: f32, y: f32, z: f32) -> f32 {
    let mut output = 0.0;
    let mut denom = 0.0;
    let mut frequency = FREQUENCY;
    let mut amplitude = AMPLITUDE;
    for _ in 0..OCTAVES {
        output += amplitude * noise(x * frequency, y * frequency, z * frequency);
        denom += amplitude;
        frequency *= LACUNARITY;
        amplitude *= PERSISTENCE;
    }
    output / denom
}

// OpenRGB's integer hsv2rgb with saturation and value fixed at 255
fn rainbow(hue: u32) -> [u8; 3] {
    let (h, s, v) = ((hue % 360) as i32, 255, 255);
    let i = h / 60;
    let p = ((256 * v - s * v) / 256) as u8;
    let v8 = v as u8;
    if i & 1 != 0 {
        let q = ((256 * 60 * v - h * s * v + 60 * s * v * i) / (256 * 60)) as u8;
        match i {
            1 => [q, v8, p],
            3 => [p, q, v8],
            _ => [v8, p, q],
        }
    } else {
        let t = ((256 * 60 * v + h * s * v - 60 * s * v * (i + 1)) / (256 * 60)) as u8;
        match i {
            0 => [v8, t, p],
            2 => [p, v8, t],
            _ => [t, p, v8],
        }
    }
}

fn color(x: usize, progress: f64) -> [u8; 3] {
    let value = fractal(x as f32, 0.0, progress as f32);
    let frac = (1.0 + value) * 0.5;
    rainbow((360.0 * frac) as u32)
}

pub struct NoiseMap {
    progress: f64,
}

impl NoiseMap {
    pub fn new() -> Self {
        NoiseMap { progress: 0.0 }
    }

    // Fill one frame and advance the animation
    pub fn render(&mut self, frame: &mut [[u8; 3]]) {
        for (x, px) in frame.iter_mut().enumerate() {
            *px = color(x, self.progress);
        }
        self.progress += STEP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (frame number, RGB per LED) from a C transcription of the OpenRGB plugin
    const REFERENCE: [(u64, [[u8; 3]; 32]); 5] = [
        (0, [[0, 255, 255], [0, 161, 255], [0, 119, 255], [0, 68, 255], [0, 13, 255], [13, 0, 255], [51, 0, 255], [56, 0, 255], [30, 0, 255], [43, 0, 255], [39, 0, 255], [30, 0, 255], [47, 0, 255], [68, 0, 255], [77, 0, 255], [94, 0, 255], [98, 0, 255], [140, 0, 255], [161, 0, 255], [170, 0, 255], [161, 0, 255], [174, 0, 255], [161, 0, 255], [144, 0, 255], [119, 0, 255], [102, 0, 255], [132, 0, 255], [140, 0, 255], [153, 0, 255], [161, 0, 255], [161, 0, 255], [111, 0, 255]]),
        (1, [[0, 255, 255], [0, 161, 255], [0, 115, 255], [0, 68, 255], [0, 13, 255], [13, 0, 255], [51, 0, 255], [56, 0, 255], [30, 0, 255], [43, 0, 255], [43, 0, 255], [30, 0, 255], [47, 0, 255], [68, 0, 255], [77, 0, 255], [94, 0, 255], [98, 0, 255], [140, 0, 255], [161, 0, 255], [170, 0, 255], [161, 0, 255], [170, 0, 255], [161, 0, 255], [140, 0, 255], [119, 0, 255], [102, 0, 255], [136, 0, 255], [140, 0, 255], [153, 0, 255], [161, 0, 255], [161, 0, 255], [115, 0, 255]]),
        (600, [[115, 0, 255], [157, 0, 255], [161, 0, 255], [161, 0, 255], [199, 0, 255], [199, 0, 255], [212, 0, 255], [255, 0, 255], [255, 0, 233], [255, 0, 233], [255, 0, 229], [255, 0, 238], [255, 0, 250], [250, 0, 255], [225, 0, 255], [199, 0, 255], [157, 0, 255], [140, 0, 255], [123, 0, 255], [81, 0, 255], [17, 0, 255], [0, 17, 255], [0, 77, 255], [0, 132, 255], [0, 132, 255], [0, 153, 255], [0, 174, 255], [0, 174, 255], [0, 187, 255], [0, 216, 255], [0, 204, 255], [0, 204, 255]]),
        (123457, [[0, 255, 106], [0, 255, 89], [0, 255, 51], [0, 255, 26], [34, 255, 0], [68, 255, 0], [89, 255, 0], [119, 255, 0], [144, 255, 0], [144, 255, 0], [123, 255, 0], [98, 255, 0], [85, 255, 0], [77, 255, 0], [56, 255, 0], [72, 255, 0], [51, 255, 0], [26, 255, 0], [13, 255, 0], [17, 255, 0], [22, 255, 0], [34, 255, 0], [26, 255, 0], [13, 255, 0], [22, 255, 0], [26, 255, 0], [17, 255, 0], [0, 255, 13], [0, 255, 43], [0, 255, 60], [0, 255, 68], [0, 255, 64]]),
        (2000000, [[60, 255, 0], [64, 255, 0], [72, 255, 0], [102, 255, 0], [115, 255, 0], [123, 255, 0], [123, 255, 0], [106, 255, 0], [81, 255, 0], [85, 255, 0], [47, 255, 0], [51, 255, 0], [22, 255, 0], [30, 255, 0], [13, 255, 0], [0, 255, 0], [0, 255, 43], [0, 255, 72], [0, 255, 94], [0, 255, 153], [0, 255, 191], [0, 255, 229], [0, 233, 255], [0, 221, 255], [0, 225, 255], [0, 229, 255], [0, 233, 255], [0, 233, 255], [0, 216, 255], [0, 212, 255], [0, 216, 255], [0, 204, 255]]),
    ];

    #[test]
    fn matches_openrgb() {
        for (n, expected) in REFERENCE {
            let got: Vec<[u8; 3]> = (0..32).map(|x| color(x, n as f64 * STEP)).collect();
            assert_eq!(got, expected, "frame {n}");
        }
    }
}

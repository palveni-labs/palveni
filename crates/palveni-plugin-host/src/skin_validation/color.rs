//! CSS colour and shadow grammar shared by the skin format. Colours are
//! measured in sRGB, including alpha compositing, as in the workbench.

#[derive(Clone, Copy)]
pub(super) struct Color {
    rgb: [f64; 3],
    pub(super) alpha: f64,
}

impl Color {
    pub(super) fn contrast_on(self, bg: Self) -> f64 {
        let composite =
            std::array::from_fn(|i| self.rgb[i] * self.alpha + bg.rgb[i] * (1.0 - self.alpha));
        let front = luminance(composite);
        let back = luminance(bg.rgb);
        (front.max(back) + 0.05) / (front.min(back) + 0.05)
    }
}

fn luminance(rgb: [f64; 3]) -> f64 {
    let [r, g, b] = rgb.map(|v| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn oklab(l: f64, a: f64, b: f64) -> [f64; 3] {
    let x = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let y = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let z = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    [
        4.0767416621 * x - 3.3077115913 * y + 0.2309699292 * z,
        -1.2684380046 * x + 2.6097574011 * y - 0.3413193965 * z,
        -0.0041960863 * x - 0.7034186147 * y + 1.7076147010 * z,
    ]
    .map(|v| {
        if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    })
}

fn oklch(lightness: f64, chroma: f64, hue: f64) -> [f64; 3] {
    let (sin, cos) = (hue % 360.0).to_radians().sin_cos();
    let at = |c| oklab(lightness, c * cos, c * sin);
    let in_gamut = |rgb: [f64; 3]| rgb.iter().all(|v| (-0.0001..=1.0001).contains(v));
    let rgb = at(chroma);
    if in_gamut(rgb) {
        return rgb;
    }
    let (mut low, mut high) = (0.0, chroma);
    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        if in_gamut(at(mid)) {
            low = mid;
        } else {
            high = mid;
        }
    }
    at(low)
}

fn number(token: &str) -> Option<f64> {
    let value = token.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn amount(token: &str, percent_scale: f64) -> Option<f64> {
    if let Some(percent) = token.strip_suffix('%') {
        Some(number(percent)? / 100.0 * percent_scale)
    } else {
        number(token)
    }
}

pub(super) fn parse(value: &str) -> Option<Color> {
    let value = value.trim();
    if value.len() > 80 || !value.is_ascii() {
        return None;
    }
    if let Some(hex) = value.strip_prefix('#') {
        if !matches!(hex.len(), 3 | 4 | 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let width = if hex.len() <= 4 { 1 } else { 2 };
        let mut channels = [1.0; 4];
        for (i, chunk) in hex.as_bytes().chunks_exact(width).enumerate() {
            let v = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
            channels[i] = f64::from(v) / if width == 1 { 15.0 } else { 255.0 };
        }
        let [r, g, b, alpha] = channels;
        return Some(Color {
            rgb: [r, g, b],
            alpha,
        });
    }

    let (name, args) = value.strip_suffix(')')?.split_once('(')?;
    let name = name.to_ascii_lowercase();
    let (channels, alpha_token) = if args.contains(',') {
        if !matches!(name.as_str(), "rgb" | "rgba") {
            return None;
        }
        let mut channels = args.split(',').map(str::trim).collect::<Vec<_>>();
        let a = if channels.len() == 4 {
            channels.pop()
        } else {
            None
        };
        (channels, a)
    } else {
        let (values, alpha) = args
            .split_once('/')
            .map_or((args, None), |(v, a)| (v, Some(a.trim())));
        (values.split_ascii_whitespace().collect(), alpha)
    };
    let [x, y, z] = channels.as_slice() else {
        return None;
    };
    let alpha = match alpha_token {
        Some(a) => amount(a, 1.0)?,
        None => 1.0,
    };
    let rgb = match name.as_str() {
        "rgb" | "rgba" => [
            amount(x, 255.0)? / 255.0,
            amount(y, 255.0)? / 255.0,
            amount(z, 255.0)? / 255.0,
        ],
        "oklab" => oklab(amount(x, 1.0)?, amount(y, 0.4)?, amount(z, 0.4)?),
        "oklch" => oklch(
            amount(x, 1.0)?,
            amount(y, 0.4)?,
            number(z.strip_suffix("deg").unwrap_or(z))?,
        ),
        _ => return None,
    };
    if !rgb.iter().all(|v| v.is_finite()) {
        return None;
    }
    Some(Color {
        rgb: rgb.map(|v| v.clamp(0.0, 1.0)),
        alpha: alpha.clamp(0.0, 1.0),
    })
}

pub(super) fn shadow(value: &str) -> bool {
    if value.len() > 400 {
        return false;
    }
    let value = value.trim();
    if value == "none" {
        return true;
    }
    let mut start = 0;
    let mut parens = false;
    for (i, c) in value.char_indices() {
        match c {
            '(' if !parens => parens = true,
            ')' if parens => parens = false,
            '(' | ')' => return false,
            ',' if !parens => {
                if !shadow_layer(value[start..i].trim()) {
                    return false;
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    !parens && shadow_layer(value[start..].trim())
}

fn shadow_layer(mut rest: &str) -> bool {
    if let Some(r) = rest
        .strip_prefix("inset")
        .filter(|r| r.starts_with(char::is_whitespace))
    {
        rest = r.trim_start();
    }
    for n in 0..=4 {
        if n >= 2 && parse(rest).is_some() {
            // Shadow functions in the workbench accept plain decimal channels.
            return rest.starts_with('#')
                || rest
                    .strip_suffix(')')
                    .and_then(|s| s.split_once('('))
                    .is_some_and(|(name, args)| {
                        matches!(name, "rgb" | "rgba" | "oklab" | "oklch")
                            && args.bytes().all(|b| {
                                b.is_ascii_digit()
                                    || b.is_ascii_whitespace()
                                    || b".,%/+-".contains(&b)
                            })
                    });
        }
        let Some((token, tail)) = rest.split_once(char::is_whitespace) else {
            return false;
        };
        let length = token.strip_suffix("px").unwrap_or(token);
        let unsigned = length.strip_prefix('-').unwrap_or(length);
        let (int, frac) = unsigned
            .split_once('.')
            .map_or((unsigned, None), |(a, b)| (a, Some(b)));
        if int.is_empty()
            || !int.bytes().all(|b| b.is_ascii_digit())
            || frac.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit()))
            || number(length)
                .is_none_or(|v| (n == 2 && v < 0.0) || (!token.ends_with("px") && v != 0.0))
        {
            return false;
        }
        rest = tail.trim_start();
    }
    false
}

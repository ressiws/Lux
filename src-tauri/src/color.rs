use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64, // 0.0 to 1.0
}

impl ParsedColor {
    pub fn new(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self {
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    pub fn to_hex(&self) -> String {
        let a_u8 = (self.a * 255.0).round() as u8;
        if a_u8 == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, a_u8)
        }
    }

    pub fn to_rgb_string(&self) -> String {
        let a_u8 = (self.a * 255.0).round() as u8;
        if a_u8 == 255 {
            format!("{}, {}, {}", self.r, self.g, self.b)
        } else {
            let a_str = format_alpha(self.a);
            format!("{}, {}, {}, {}", self.r, self.g, self.b, a_str)
        }
    }

    pub fn to_css_rgb(&self) -> String {
        let a_u8 = (self.a * 255.0).round() as u8;
        if a_u8 == 255 {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            let a_str = format_alpha(self.a);
            format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, a_str)
        }
    }

    pub fn to_hsl_string(&self) -> String {
        let (h, s, l) = rgb_to_hsl_values(self.r, self.g, self.b);
        let a_u8 = (self.a * 255.0).round() as u8;
        if a_u8 == 255 {
            format!(
                "{}°, {}%, {}%",
                h.round() as i64,
                (s * 100.0).round() as i64,
                (l * 100.0).round() as i64
            )
        } else {
            let a_str = format_alpha(self.a);
            format!(
                "{}°, {}%, {}%, {}",
                h.round() as i64,
                (s * 100.0).round() as i64,
                (l * 100.0).round() as i64,
                a_str
            )
        }
    }

    pub fn to_css_hsl(&self) -> String {
        let (h, s, l) = rgb_to_hsl_values(self.r, self.g, self.b);
        let a_u8 = (self.a * 255.0).round() as u8;
        if a_u8 == 255 {
            format!(
                "hsl({}deg, {}%, {}%)",
                h.round() as i64,
                (s * 100.0).round() as i64,
                (l * 100.0).round() as i64
            )
        } else {
            let a_str = format_alpha(self.a);
            format!(
                "hsla({}, {}%, {}%, {})",
                h.round() as i64,
                (s * 100.0).round() as i64,
                (l * 100.0).round() as i64,
                a_str
            )
        }
    }
}

pub fn format_alpha(a: f64) -> String {
    let rounded = (a * 1000.0).round() / 1000.0;
    let s = format!("{rounded:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn rgb_to_hsl_values(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let d = max - min;

    if d.abs() < 1e-9 {
        return (0.0, 0.0, l);
    }

    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let mut h = if (max - rf).abs() < 1e-9 {
        (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }
    } else if (max - gf).abs() < 1e-9 {
        (bf - rf) / d + 2.0
    } else {
        (rf - gf) / d + 4.0
    };
    h *= 60.0;
    (h, s, l)
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    if s <= 0.0 {
        let val = (l * 255.0).round() as u8;
        return (val, val, val);
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = l - c / 2.0;
    let r = ((r1 + m) * 255.0).round() as u8;
    let g = ((g1 + m) * 255.0).round() as u8;
    let b = ((b1 + m) * 255.0).round() as u8;
    (r, g, b)
}

pub fn parse_hex(s: &str) -> Option<ParsedColor> {
    let s = s.trim();
    if !s.starts_with('#') {
        return None;
    }
    let hex_part = &s[1..];
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex_part.len() {
        3 => {
            let r = u8::from_str_radix(&hex_part[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex_part[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex_part[2..3].repeat(2), 16).ok()?;
            Some(ParsedColor::new(r, g, b, 1.0))
        }
        4 => {
            let r = u8::from_str_radix(&hex_part[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex_part[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex_part[2..3].repeat(2), 16).ok()?;
            let a_u8 = u8::from_str_radix(&hex_part[3..4].repeat(2), 16).ok()?;
            Some(ParsedColor::new(r, g, b, a_u8 as f64 / 255.0))
        }
        6 => {
            let r = u8::from_str_radix(&hex_part[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex_part[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex_part[4..6], 16).ok()?;
            Some(ParsedColor::new(r, g, b, 1.0))
        }
        8 => {
            let r = u8::from_str_radix(&hex_part[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex_part[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex_part[4..6], 16).ok()?;
            let a_u8 = u8::from_str_radix(&hex_part[6..8], 16).ok()?;
            Some(ParsedColor::new(r, g, b, a_u8 as f64 / 255.0))
        }
        _ => None,
    }
}

fn parse_color_channel(part: &str) -> Option<u8> {
    let part = part.trim();
    if let Some(percent_str) = part.strip_suffix('%') {
        let val: f64 = percent_str.trim().parse().ok()?;
        if !(0.0..=100.0).contains(&val) {
            return None;
        }
        Some((val / 100.0 * 255.0).round() as u8)
    } else {
        let val: i64 = part.parse().ok()?;
        if !(0..=255).contains(&val) {
            return None;
        }
        Some(val as u8)
    }
}

fn parse_alpha_channel(part: &str) -> Option<f64> {
    let part = part.trim();
    if let Some(percent_str) = part.strip_suffix('%') {
        let val: f64 = percent_str.trim().parse().ok()?;
        if !(0.0..=100.0).contains(&val) {
            return None;
        }
        Some(val / 100.0)
    } else {
        let val: f64 = part.parse().ok()?;
        if !(0.0..=1.0).contains(&val) {
            return None;
        }
        Some(val)
    }
}

pub fn parse_rgb(s: &str) -> Option<ParsedColor> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    let is_rgba = lower.starts_with("rgba(");
    let is_rgb = lower.starts_with("rgb(");
    if (!is_rgba && !is_rgb) || !s.ends_with(')') {
        return None;
    }
    let open_paren = s.find('(')?;
    let inner = s[open_paren + 1..s.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }

    if inner.contains(',') {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parse_color_channel(parts[0])?;
            let g = parse_color_channel(parts[1])?;
            let b = parse_color_channel(parts[2])?;
            Some(ParsedColor::new(r, g, b, 1.0))
        } else if parts.len() == 4 {
            let r = parse_color_channel(parts[0])?;
            let g = parse_color_channel(parts[1])?;
            let b = parse_color_channel(parts[2])?;
            let a = parse_alpha_channel(parts[3])?;
            Some(ParsedColor::new(r, g, b, a))
        } else {
            None
        }
    } else {
        // Modern space-separated syntax: rgb(255 87 51 / 0.5) or rgb(255 87 51)
        if inner.contains('/') {
            let slash_parts: Vec<&str> = inner.split('/').collect();
            if slash_parts.len() != 2 {
                return None;
            }
            let channels: Vec<&str> = slash_parts[0].split_whitespace().collect();
            if channels.len() != 3 {
                return None;
            }
            let r = parse_color_channel(channels[0])?;
            let g = parse_color_channel(channels[1])?;
            let b = parse_color_channel(channels[2])?;
            let a = parse_alpha_channel(slash_parts[1])?;
            Some(ParsedColor::new(r, g, b, a))
        } else {
            let channels: Vec<&str> = inner.split_whitespace().collect();
            if channels.len() == 3 {
                let r = parse_color_channel(channels[0])?;
                let g = parse_color_channel(channels[1])?;
                let b = parse_color_channel(channels[2])?;
                Some(ParsedColor::new(r, g, b, 1.0))
            } else if channels.len() == 4 {
                let r = parse_color_channel(channels[0])?;
                let g = parse_color_channel(channels[1])?;
                let b = parse_color_channel(channels[2])?;
                let a = parse_alpha_channel(channels[3])?;
                Some(ParsedColor::new(r, g, b, a))
            } else {
                None
            }
        }
    }
}

fn parse_hue(part: &str) -> Option<f64> {
    let part = part.trim();
    let lower = part.to_ascii_lowercase();
    let val = if let Some(deg) = lower.strip_suffix("deg") {
        deg.trim().parse::<f64>().ok()?
    } else if let Some(rad) = lower.strip_suffix("rad") {
        let r = rad.trim().parse::<f64>().ok()?;
        r * 180.0 / PI
    } else if let Some(grad) = lower.strip_suffix("grad") {
        let g = grad.trim().parse::<f64>().ok()?;
        g * 0.9
    } else if let Some(turn) = lower.strip_suffix("turn") {
        let t = turn.trim().parse::<f64>().ok()?;
        t * 360.0
    } else {
        part.parse::<f64>().ok()?
    };

    if !val.is_finite() {
        return None;
    }
    Some(val.rem_euclid(360.0))
}

fn parse_percentage(part: &str) -> Option<f64> {
    let part = part.trim();
    let percent_str = part.strip_suffix('%')?;
    let val: f64 = percent_str.trim().parse().ok()?;
    if !(0.0..=100.0).contains(&val) {
        return None;
    }
    Some(val / 100.0)
}

pub fn parse_hsl(s: &str) -> Option<ParsedColor> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    let is_hsla = lower.starts_with("hsla(");
    let is_hsl = lower.starts_with("hsl(");
    if (!is_hsla && !is_hsl) || !s.ends_with(')') {
        return None;
    }
    let open_paren = s.find('(')?;
    let inner = s[open_paren + 1..s.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }

    if inner.contains(',') {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let h = parse_hue(parts[0])?;
            let s = parse_percentage(parts[1])?;
            let l = parse_percentage(parts[2])?;
            let (r, g, b) = hsl_to_rgb(h, s, l);
            Some(ParsedColor::new(r, g, b, 1.0))
        } else if parts.len() == 4 {
            let h = parse_hue(parts[0])?;
            let s = parse_percentage(parts[1])?;
            let l = parse_percentage(parts[2])?;
            let a = parse_alpha_channel(parts[3])?;
            let (r, g, b) = hsl_to_rgb(h, s, l);
            Some(ParsedColor::new(r, g, b, a))
        } else {
            None
        }
    } else {
        // Space-separated
        if inner.contains('/') {
            let slash_parts: Vec<&str> = inner.split('/').collect();
            if slash_parts.len() != 2 {
                return None;
            }
            let channels: Vec<&str> = slash_parts[0].split_whitespace().collect();
            if channels.len() != 3 {
                return None;
            }
            let h = parse_hue(channels[0])?;
            let s = parse_percentage(channels[1])?;
            let l = parse_percentage(channels[2])?;
            let a = parse_alpha_channel(slash_parts[1])?;
            let (r, g, b) = hsl_to_rgb(h, s, l);
            Some(ParsedColor::new(r, g, b, a))
        } else {
            let channels: Vec<&str> = inner.split_whitespace().collect();
            if channels.len() == 3 {
                let h = parse_hue(channels[0])?;
                let s = parse_percentage(channels[1])?;
                let l = parse_percentage(channels[2])?;
                let (r, g, b) = hsl_to_rgb(h, s, l);
                Some(ParsedColor::new(r, g, b, 1.0))
            } else if channels.len() == 4 {
                let h = parse_hue(channels[0])?;
                let s = parse_percentage(channels[1])?;
                let l = parse_percentage(channels[2])?;
                let a = parse_alpha_channel(channels[3])?;
                let (r, g, b) = hsl_to_rgb(h, s, l);
                Some(ParsedColor::new(r, g, b, a))
            } else {
                None
            }
        }
    }
}

pub fn parse_color(s: &str) -> Option<ParsedColor> {
    let s = s.trim();
    if s.is_empty() || s.contains('\n') || s.contains('\r') {
        return None;
    }
    if s.starts_with('#') {
        return parse_hex(s);
    }
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("rgb(") || lower.starts_with("rgba(") {
        return parse_rgb(s);
    }
    if lower.starts_with("hsl(") || lower.starts_with("hsla(") {
        return parse_hsl(s);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_lengths() {
        // 3 digits
        let c3 = parse_hex("#F53").expect("Valid 3-digit hex");
        assert_eq!(c3.to_hex(), "#FF5533");
        assert_eq!(c3.a, 1.0);

        // 4 digits with alpha
        let c4 = parse_hex("#f538").expect("Valid 4-digit hex");
        assert_eq!(c4.to_hex(), "#FF553388");
        assert!((c4.a - (136.0 / 255.0)).abs() < 1e-4);

        // 6 digits
        let c6 = parse_hex("#3b3550").expect("Valid 6-digit hex");
        assert_eq!(c6.to_hex(), "#3B3550");
        assert_eq!((c6.r, c6.g, c6.b), (59, 53, 80));

        // 8 digits with alpha
        let c8 = parse_hex("#3b355080").expect("Valid 8-digit hex");
        assert_eq!(c8.to_hex(), "#3B355080");
        assert!((c8.a - (128.0 / 255.0)).abs() < 1e-4);

        // Invalid lengths
        assert_eq!(parse_hex("#"), None);
        assert_eq!(parse_hex("#1"), None);
        assert_eq!(parse_hex("#12"), None);
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("#1234567"), None);
        assert_eq!(parse_hex("#123456789"), None);

        // Non-hex characters
        assert_eq!(parse_hex("#gggggg"), None);
        assert_eq!(parse_hex("#123z"), None);
    }

    #[test]
    fn test_rgb_and_rgba() {
        // Standard comma syntax
        let c1 = parse_rgb("rgb(255, 87, 51)").expect("Valid rgb");
        assert_eq!((c1.r, c1.g, c1.b), (255, 87, 51));
        assert_eq!(c1.to_hex(), "#FF5733");
        assert_eq!(c1.a, 1.0);

        // Percentage syntax
        let c2 = parse_rgb("rgb(100%, 50%, 0%)").expect("Valid rgb percentage");
        assert_eq!((c2.r, c2.g, c2.b), (255, 128, 0));

        // RGBA float alpha
        let c3 = parse_rgb("rgba(255, 87, 51, 0.5)").expect("Valid rgba float");
        assert_eq!(c3.to_hex(), "#FF573380");
        assert_eq!(c3.a, 0.5);

        // RGBA percentage alpha
        let c4 = parse_rgb("rgba(255, 87, 51, 50%)").expect("Valid rgba percent");
        assert_eq!(c4.to_hex(), "#FF573380");
        assert_eq!(c4.a, 0.5);

        // Modern CSS Color 4 space syntax
        let c5 = parse_rgb("rgb(255 87 51 / 0.5)").expect("Valid modern rgb");
        assert_eq!(c5.to_hex(), "#FF573380");

        // Whitespace and case insensitivity
        let c6 = parse_rgb("  RGBA( 255 , 87 , 51 , 1.0 )  ").expect("Case & whitespace");
        assert_eq!(c6.to_hex(), "#FF5733");

        // Out of range & malformed
        assert_eq!(parse_rgb("rgb(256, 0, 0)"), None);
        assert_eq!(parse_rgb("rgb(-1, 0, 0)"), None);
        assert_eq!(parse_rgb("rgb(105%, 0%, 0%)"), None);
        assert_eq!(parse_rgb("rgba(255, 0, 0, 1.5)"), None);
        assert_eq!(parse_rgb("rgba(255, 0, 0, -0.1)"), None);
        assert_eq!(parse_rgb("rgb(255, 0)"), None);
        assert_eq!(parse_rgb("rgb(255, 0, 0, 0, 0)"), None);
        assert_eq!(parse_rgb("rgb(abc, def, ghi)"), None);
    }

    #[test]
    fn test_hsl_and_hsla() {
        // Standard HSL
        let c1 = parse_hsl("hsl(14, 100%, 60%)").expect("Valid hsl");
        assert_eq!((c1.r, c1.g, c1.b), (255, 99, 51));
        assert_eq!(c1.to_hex(), "#FF6333");
        assert_eq!(c1.a, 1.0);

        // With deg unit
        let c2 = parse_hsl("hsl(14deg, 100%, 60%)").expect("Valid hsl deg");
        assert_eq!(c2.to_hex(), "#FF6333");

        // HSLA float alpha
        let c3 = parse_hsl("hsla(14, 100%, 60%, 0.5)").expect("Valid hsla");
        assert_eq!(c3.to_hex(), "#FF633380");
        assert_eq!(c3.a, 0.5);

        // Modern space / slash syntax
        let c4 = parse_hsl("hsl(14 100% 60% / 50%)").expect("Valid modern hsl");
        assert_eq!(c4.to_hex(), "#FF633380");

        // Out of range & malformed
        assert_eq!(parse_hsl("hsl(14, 150%, 50%)"), None);
        assert_eq!(parse_hsl("hsl(14, -5%, 50%)"), None);
        assert_eq!(parse_hsl("hsl(14, 50%, 120%)"), None);
        assert_eq!(parse_hsl("hsl(14, 50, 50)"), None); // Missing %
        assert_eq!(parse_hsl("hsla(14, 100%, 50%, 1.5)"), None);
        assert_eq!(parse_hsl("hsl(14, 100%)"), None);
    }

    #[test]
    fn test_parse_color_unified() {
        assert!(parse_color("#FF5733").is_some());
        assert!(parse_color("rgb(255, 87, 51)").is_some());
        assert!(parse_color("hsl(14, 100%, 60%)").is_some());

        // Non-colors and arbitrary text containing color strings
        assert_eq!(parse_color("https://example.com/rgb(1,2,3)"), None);
        assert_eq!(parse_color("color: #FF5733;"), None);
        assert_eq!(parse_color("const c = rgb(255, 87, 51);"), None);
        assert_eq!(parse_color("issue #123"), None);
        assert_eq!(parse_color("plain text"), None);
    }
}

/**
 * Calculates perceived brightness / luminance from a color string.
 * Supports #RGB, #RGBA, #RRGGBB, #RRGGBBAA.
 * If predominantly transparent (alpha < 0.4), returns 0 so light text is chosen
 * on top of the dark container / checkerboard.
 */
export function getColorLuminance(colorStr: string): number {
  const clean = colorStr.trim().replace(/^#/, "");
  let r = 0,
    g = 0,
    b = 0,
    a = 1.0;

  if (clean.length === 3) {
    r = parseInt(clean[0] + clean[0], 16) || 0;
    g = parseInt(clean[1] + clean[1], 16) || 0;
    b = parseInt(clean[2] + clean[2], 16) || 0;
  } else if (clean.length === 4) {
    r = parseInt(clean[0] + clean[0], 16) || 0;
    g = parseInt(clean[1] + clean[1], 16) || 0;
    b = parseInt(clean[2] + clean[2], 16) || 0;
    a = (parseInt(clean[3] + clean[3], 16) || 255) / 255;
  } else if (clean.length === 6) {
    r = parseInt(clean.slice(0, 2), 16) || 0;
    g = parseInt(clean.slice(2, 4), 16) || 0;
    b = parseInt(clean.slice(4, 6), 16) || 0;
  } else if (clean.length >= 8) {
    r = parseInt(clean.slice(0, 2), 16) || 0;
    g = parseInt(clean.slice(2, 4), 16) || 0;
    b = parseInt(clean.slice(4, 6), 16) || 0;
    a = (parseInt(clean.slice(6, 8), 16) || 255) / 255;
  }

  // If predominantly transparent, the dark preview surface dominates
  if (a < 0.4) {
    return 0;
  }
  return (r * 299 + g * 587 + b * 114) / 1000;
}

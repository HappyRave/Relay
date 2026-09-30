//! Finding an image on the screen. Both are compared in gray, by normalized
//! cross-correlation (NCC), so a match survives a small change of brightness
//! or contrast. The search is coarse to fine: every scale is first tried on a
//! small copy of the screen, then the best spots are refined, one pyramid
//! level at a time, down to (nearly) full size.

/// A gray image, one value (0–255) per pixel, rows top to bottom.
#[derive(Clone, PartialEq)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub px: Vec<f32>,
}

impl std::fmt::Debug for Gray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gray({}×{})", self.w, self.h)
    }
}

impl Gray {
    /// From RGB bytes (3 per pixel).
    pub fn from_rgb(w: usize, h: usize, rgb: &[u8]) -> Gray {
        let px = rgb.as_chunks::<3>().0.iter().map(|&[r, g, b]| luma(r, g, b)).collect();
        Gray { w, h, px }
    }

    fn row(&self, y: usize) -> &[f32] {
        &self.px[y * self.w..(y + 1) * self.w]
    }

    /// Half the size, each pixel the average of a 2×2 block (an odd last
    /// row or column is dropped).
    fn half(&self) -> Gray {
        let (w, h) = (self.w / 2, self.h / 2);
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            let (a, b) = (self.row(2 * y), self.row(2 * y + 1));
            px.extend((0..w).map(|x| (a[2 * x] + a[2 * x + 1] + b[2 * x] + b[2 * x + 1]) * 0.25));
        }
        Gray { w, h, px }
    }

    /// Resampled to `w`×`h`: each new pixel is the average of the area it
    /// covers.
    pub fn resize(&self, w: usize, h: usize) -> Gray {
        let (xs, ys) = (area_weights(self.w, w), area_weights(self.h, h));
        let mut wide = Vec::with_capacity(w * self.h);
        for y in 0..self.h {
            let row = self.row(y);
            wide.extend(xs.iter().map(|ws| ws.iter().map(|&(i, k)| row[i] * k).sum::<f32>()));
        }
        let mut px = Vec::with_capacity(w * h);
        for ws in &ys {
            px.extend((0..w).map(|x| ws.iter().map(|&(j, k)| wide[j * w + x] * k).sum::<f32>()));
        }
        Gray { w, h, px }
    }

    /// The standard deviation of its pixels: how much detail there is to
    /// find.
    pub fn detail(&self) -> f32 {
        let n = self.px.len().max(1) as f64;
        let mean = self.px.iter().map(|&v| v as f64).sum::<f64>() / n;
        let var = self.px.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
        var.sqrt() as f32
    }
}

fn luma(r: u8, g: u8, b: u8) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// For each of `dst` cells over `src` pixels: the source pixels it covers,
/// each with its share of the cell (the shares add up to 1).
fn area_weights(src: usize, dst: usize) -> Vec<Vec<(usize, f32)>> {
    let step = src as f64 / dst as f64;
    (0..dst)
        .map(|i| {
            let (a, b) = (i as f64 * step, (i as f64 + 1.0) * step);
            let (first, last) = (a.floor() as usize, (b.ceil() as usize).min(src));
            (first..last)
                .filter_map(|j| {
                    let cover = (b.min(j as f64 + 1.0) - a.max(j as f64)) / step;
                    (cover > 1e-9).then_some((j, cover as f32))
                })
                .collect()
        })
        .collect()
}

/// Below this detail (standard deviation, in gray levels) an image is too
/// plain to find reliably.
pub const MIN_DETAIL: f32 = 4.0;

/// The scales tried go from `MIN_SCALE` to `MAX_SCALE`, each `SCALE_STEP`
/// times the one before.
pub const MIN_SCALE: f32 = 0.5;
pub const MAX_SCALE: f32 = 2.0;
const SCALE_STEP: f32 = 1.1;

/// The coarse search uses the smallest copy of the screen on which the
/// scaled image is still at least this many pixels on its short side.
const MIN_COARSE_SIDE: f32 = 5.0;
/// An image is never tried smaller than this (in screen pixels).
const MIN_SIDE: f32 = 8.0;
/// Refining stops at the pyramid level where the image fits in this many
/// pixels, so a big image doesn't cost a full-size comparison.
const MAX_FINE_AREA: usize = 128 * 128;
/// A coarse spot needs at least this score to be refined. It's low because
/// on a small copy of the screen, a few pixels of misalignment cost a lot.
const COARSE_FLOOR: f32 = 0.3;
/// A spot scoring below this on the way down isn't refined further: the
/// real image scores well above it once it's a dozen pixels across.
const PRUNE: f32 = 0.5;
/// How many spots per scale the coarse search keeps for refining.
const CANDIDATES: usize = 8;
/// Pyramid levels: 1, ½, ¼, ⅛, 1/16 of the screen.
const LEVELS: usize = 5;
/// A screen window whose standard deviation is below this (gray levels) is
/// flat, and can't be the image.
const FLAT: f64 = 1.0;

/// How to search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FindOpts {
    /// A score (0–1) that ends the search early: a match this good is taken.
    pub threshold: f32,
}

/// Where the image was found, in the searched image's pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The image's size on screen over its own size.
    pub scale: f32,
    /// The NCC score, 1 for a perfect match.
    pub score: f32,
}

/// The scales to try, in order: 1 (the image as it was taken), then the
/// rest from small to large.
pub fn scales() -> Vec<f32> {
    let mut out = vec![1.0];
    let mut s = MIN_SCALE;
    while s <= MAX_SCALE * 1.001 {
        if (s - 1.0).abs() > 0.03 {
            out.push(s);
        }
        s *= SCALE_STEP;
    }
    out
}

/// The best place for `image` in `screen`, at any scale from `MIN_SCALE` to
/// `MAX_SCALE`, or `None` if nothing on screen comes close. A match below
/// `opts.threshold` may still be returned: comparing its score is the
/// caller's job.
pub fn find(screen: &Gray, image: &Gray, opts: FindOpts) -> Option<Match> {
    let mut pyr = Pyramid::new(screen);
    let mut best: Option<Match> = None;
    for s in scales() {
        let Some(m) = find_at(&mut pyr, image, s) else { continue };
        if best.is_none_or(|b| m.score > b.score) {
            best = Some(m);
        }
        if m.score >= opts.threshold {
            break;
        }
    }
    best
}

/// The best place for `image` at about scale `s` (refined by ±3%).
fn find_at(pyr: &mut Pyramid, image: &Gray, s: f32) -> Option<Match> {
    let (sw, sh) = (image.w as f32 * s, image.h as f32 * s);
    let full = &pyr.levels[0];
    if sw.min(sh) < MIN_SIDE || sw > full.w as f32 || sh > full.h as f32 {
        return None;
    }
    let coarse = (0..LEVELS).rev().find(|&l| sw.min(sh) / (1 << l) as f32 >= MIN_COARSE_SIDE).unwrap_or(0);
    let fine = (0..=coarse).find(|&l| (sw * sh) as usize >> (2 * l) <= MAX_FINE_AREA).unwrap_or(coarse);
    let tpl = Template::new(image, sw, sh, coarse)?;
    let spots = pyr.coarse(coarse, &tpl, COARSE_FLOOR);
    spots
        .into_iter()
        .filter_map(|(x, y, _)| pyr.refine(image, s, (x, y), coarse, fine))
        .max_by(|a, b| a.score.total_cmp(&b.score))
}

/// The image, scaled for one pyramid level, as deviations from its mean.
struct Template {
    w: usize,
    h: usize,
    dev: Vec<f32>,
    norm: f64,
}

impl Template {
    /// `image` at size `sw`×`sh` (screen pixels), on pyramid level `level`.
    fn new(image: &Gray, sw: f32, sh: f32, level: usize) -> Option<Template> {
        let k = (1 << level) as f32;
        let (w, h) = (((sw / k).round() as usize).max(1), ((sh / k).round() as usize).max(1));
        let g = image.resize(w, h);
        let mean = g.px.iter().sum::<f32>() / g.px.len() as f32;
        let dev: Vec<f32> = g.px.iter().map(|v| v - mean).collect();
        let norm = dev.iter().map(|&d| (d as f64).powi(2)).sum::<f64>().sqrt();
        (norm > 1e-6).then_some(Template { w, h, dev, norm })
    }
}

/// The screen at 1, ½, ¼… size, with running sums for the coarse levels.
struct Pyramid {
    levels: Vec<Gray>,
    sums: Vec<Option<Sums>>,
}

/// Summed-area tables of the pixels and of their squares, so a window's mean
/// and spread cost four lookups.
struct Sums {
    w: usize,
    sum: Vec<f64>,
    sq: Vec<f64>,
}

impl Sums {
    fn new(g: &Gray) -> Sums {
        let w = g.w + 1;
        let (mut sum, mut sq) = (vec![0.0; w * (g.h + 1)], vec![0.0; w * (g.h + 1)]);
        for y in 0..g.h {
            let (mut rs, mut rq) = (0.0, 0.0);
            for (x, &v) in g.row(y).iter().enumerate() {
                rs += v as f64;
                rq += (v as f64) * (v as f64);
                sum[(y + 1) * w + x + 1] = sum[y * w + x + 1] + rs;
                sq[(y + 1) * w + x + 1] = sq[y * w + x + 1] + rq;
            }
        }
        Sums { w, sum, sq }
    }

    fn window(&self, x: usize, y: usize, w: usize, h: usize) -> (f64, f64) {
        let at = |t: &[f64], x: usize, y: usize| t[y * self.w + x];
        let area = |t: &[f64]| at(t, x + w, y + h) - at(t, x, y + h) - at(t, x + w, y) + at(t, x, y);
        (area(&self.sum), area(&self.sq))
    }
}

impl Pyramid {
    fn new(screen: &Gray) -> Pyramid {
        let mut levels = vec![screen.clone()];
        while levels.len() < LEVELS {
            let next = levels.last().unwrap().half();
            if next.w == 0 || next.h == 0 {
                break;
            }
            levels.push(next);
        }
        let sums = levels.iter().map(|_| None).collect();
        Pyramid { levels, sums }
    }

    /// The best spots for `tpl` on level `l` scoring at least `floor`, at
    /// most `CANDIDATES`, no two closer than half the template.
    fn coarse(&mut self, l: usize, tpl: &Template, floor: f32) -> Vec<(usize, usize, f32)> {
        let Some(g) = self.levels.get(l) else { return Vec::new() };
        if tpl.w > g.w || tpl.h > g.h {
            return Vec::new();
        }
        let sums = self.sums[l].get_or_insert_with(|| Sums::new(g));
        let n = (tpl.w * tpl.h) as f64;
        let (rx, ry) = (tpl.w.div_ceil(2), tpl.h.div_ceil(2));
        let mut best: Vec<(usize, usize, f32)> = Vec::with_capacity(CANDIDATES + 1);
        for y in 0..=g.h - tpl.h {
            for x in 0..=g.w - tpl.w {
                let (s, q) = sums.window(x, y, tpl.w, tpl.h);
                let var = q - s * s / n;
                if var < FLAT * FLAT * n {
                    continue;
                }
                let score = (correlate(g, tpl, x, y) / (tpl.norm * var.sqrt())) as f32;
                if score < floor || (best.len() == CANDIDATES && score <= best[CANDIDATES - 1].2) {
                    continue;
                }
                let near = best.iter().position(|&(bx, by, _)| bx.abs_diff(x) < rx && by.abs_diff(y) < ry);
                match near {
                    Some(i) if best[i].2 >= score => continue,
                    Some(i) => _ = best.remove(i),
                    None => {}
                }
                let at = best.partition_point(|b| b.2 >= score);
                best.insert(at, (x, y, score));
                best.truncate(CANDIDATES);
            }
        }
        best
    }

    /// Follows a coarse spot down from level `from` to level `to`, searching
    /// a few pixels around it at each step (more on the first, where the
    /// coarse template's rounding shows most). At the last one it also tries
    /// 3% smaller and larger, then 1.5% either side of the best of those.
    /// Gives up on a spot that scores below `PRUNE` on the way.
    fn refine(&self, image: &Gray, s: f32, (mut x, mut y): (usize, usize), from: usize, to: usize) -> Option<Match> {
        let mut found = None;
        let first = if from > to { from - 1 } else { from };
        for l in (to..=first).rev() {
            if l < from {
                (x, y) = (x * 2, y * 2);
            }
            let r = if l + 1 == from { 6 } else { 3 };
            let mut best = self.best_near(image, l, s, (x, y), r);
            if l == to {
                for k in [1.0 / 1.03, 1.03] {
                    best = better(best, self.best_near(image, l, s * k, (x, y), r));
                }
                if let Some((bx, by, _, bs)) = best {
                    for k in [1.0 / 1.015, 1.015] {
                        best = better(best, self.best_near(image, l, bs * k, (bx, by), 2));
                    }
                }
            }
            let (bx, by, score, sk) = best?;
            if l > to && score < PRUNE {
                return None;
            }
            (x, y) = (bx, by);
            let k = 1 << l;
            let (w, h) = ((image.w as f32 * sk).round() as i32, (image.h as f32 * sk).round() as i32);
            found = Some(Match { x: (bx * k) as i32, y: (by * k) as i32, w, h, scale: sk, score });
        }
        found
    }

    /// The best spot for `image` at scale `s` on level `l`, within `r`
    /// pixels of (x, y): (x, y, score, s).
    fn best_near(&self, image: &Gray, l: usize, s: f32, (x, y): (usize, usize), r: usize) -> Option<Spot> {
        let g = &self.levels[l];
        let tpl = Template::new(image, image.w as f32 * s, image.h as f32 * s, l)?;
        if tpl.w > g.w || tpl.h > g.h {
            return None;
        }
        let (cx, cy) = (x.min(g.w - tpl.w), y.min(g.h - tpl.h));
        let (x0, y0) = (cx.saturating_sub(r), cy.saturating_sub(r));
        let (x1, y1) = ((cx + r).min(g.w - tpl.w), (cy + r).min(g.h - tpl.h));
        let mut best: Option<Spot> = None;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let score = score_direct(g, &tpl, tx, ty);
                if best.is_none_or(|b| score > b.2) {
                    best = Some((tx, ty, score, s));
                }
            }
        }
        best
    }
}

type Spot = (usize, usize, f32, f32);

fn better(a: Option<Spot>, b: Option<Spot>) -> Option<Spot> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if b.2 > a.2 { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// Σ screen × deviation over the template at (x, y).
fn correlate(g: &Gray, tpl: &Template, x: usize, y: usize) -> f64 {
    (0..tpl.h).map(|j| dot(&g.row(y + j)[x..x + tpl.w], &tpl.dev[j * tpl.w..(j + 1) * tpl.w]) as f64).sum()
}

/// The NCC score at (x, y), without running sums.
fn score_direct(g: &Gray, tpl: &Template, x: usize, y: usize) -> f32 {
    let (mut s, mut q) = (0.0f64, 0.0f64);
    for j in 0..tpl.h {
        for &v in &g.row(y + j)[x..x + tpl.w] {
            s += v as f64;
            q += (v as f64) * (v as f64);
        }
    }
    let n = (tpl.w * tpl.h) as f64;
    let var = q - s * s / n;
    if var < FLAT * FLAT * n {
        return 0.0;
    }
    (correlate(g, tpl, x, y) / (tpl.norm * var.sqrt())) as f32
}

/// A dot product in eight lanes, so the compiler can vectorize it.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0.0f32; 8];
    let ((ca, ta), (cb, tb)) = (a.as_chunks::<8>(), b.as_chunks::<8>());
    for (x, y) in ca.iter().zip(cb) {
        for k in 0..8 {
            acc[k] += x[k] * y[k];
        }
    }
    acc.iter().sum::<f32>() + ta.iter().zip(tb).map(|(x, y)| x * y).sum::<f32>()
}

/// Why an image can't be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    #[error("This isn't a PNG or JPEG image.")]
    Unreadable,
    #[error("This image is too small. Pick one at least 8 pixels across.")]
    TooSmall,
    #[error("This image is too plain to find reliably. Pick a part with more detail.")]
    TooPlain,
}

/// An image's longest side is kept to this many pixels: plenty to find it,
/// and it keeps a `.rly` small.
pub const MAX_SIDE: usize = 512;

/// RGB pixels, 3 bytes each, rows top to bottom.
#[derive(Clone, PartialEq, Eq)]
pub struct Rgb8 {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl std::fmt::Debug for Rgb8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rgb8({}×{})", self.w, self.h)
    }
}

impl Rgb8 {
    pub fn gray(&self) -> Gray {
        Gray::from_rgb(self.w, self.h, &self.px)
    }

    /// The part at (x, y), `w`×`h`, clipped to the image.
    pub fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Rgb8 {
        let (x, y) = (x.min(self.w), y.min(self.h));
        let (w, h) = (w.min(self.w - x), h.min(self.h - y));
        let mut px = Vec::with_capacity(w * h * 3);
        for j in y..y + h {
            px.extend_from_slice(&self.px[(j * self.w + x) * 3..(j * self.w + x + w) * 3]);
        }
        Rgb8 { w, h, px }
    }

    /// Resampled to `w`×`h` by area averaging.
    fn resize(&self, w: usize, h: usize) -> Rgb8 {
        let (xs, ys) = (area_weights(self.w, w), area_weights(self.h, h));
        let mut wide = vec![0.0f32; w * self.h * 3];
        for y in 0..self.h {
            for (x, ws) in xs.iter().enumerate() {
                for &(i, k) in ws {
                    for c in 0..3 {
                        wide[(y * w + x) * 3 + c] += self.px[(y * self.w + i) * 3 + c] as f32 * k;
                    }
                }
            }
        }
        let mut px = vec![0.0f32; w * h * 3];
        for (y, ws) in ys.iter().enumerate() {
            for &(j, k) in ws {
                for i in 0..w * 3 {
                    px[y * w * 3 + i] += wide[j * w * 3 + i] * k;
                }
            }
        }
        Rgb8 { w, h, px: px.into_iter().map(|v| v.round().clamp(0.0, 255.0) as u8).collect() }
    }

    /// Reads a PNG or a JPEG. Transparent parts are drawn on white.
    pub fn decode(bytes: &[u8]) -> Result<Rgb8, ImageError> {
        if bytes.starts_with(b"\x89PNG") {
            decode_png(bytes).ok_or(ImageError::Unreadable)
        } else if bytes.starts_with(&[0xFF, 0xD8]) {
            decode_jpeg(bytes).ok_or(ImageError::Unreadable)
        } else {
            Err(ImageError::Unreadable)
        }
    }

    pub fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, self.w as u32, self.h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::High);
        // Writing to memory can't fail.
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(&self.px).expect("png data");
        writer.finish().expect("png end");
        out
    }
}

fn decode_png(bytes: &[u8]) -> Option<Rgb8> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width as usize, info.height as usize);
    let on_white = |v: u8, a: u8| ((v as u32 * a as u32 + 255 * (255 - a as u32)) / 255) as u8;
    let px = match info.color_type {
        png::ColorType::Rgb => buf,
        png::ColorType::Rgba => buf
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, a]| [on_white(r, a), on_white(g, a), on_white(b, a)])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&v| [v, v, v]).collect(),
        png::ColorType::GrayscaleAlpha => {
            buf.as_chunks::<2>().0.iter().flat_map(|&[v, a]| [on_white(v, a); 3]).collect()
        }
        png::ColorType::Indexed => return None,
    };
    (px.len() == w * h * 3).then_some(Rgb8 { w, h, px })
}

fn decode_jpeg(bytes: &[u8]) -> Option<Rgb8> {
    let mut dec = jpeg_decoder::Decoder::new(bytes);
    let data = dec.decode().ok()?;
    let info = dec.info()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let px = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => data,
        jpeg_decoder::PixelFormat::L8 => data.iter().flat_map(|&v| [v, v, v]).collect(),
        _ => return None,
    };
    (px.len() == w * h * 3).then_some(Rgb8 { w, h, px })
}

/// An image as Relay keeps it: read from a PNG or JPEG, no larger than
/// `MAX_SIDE`, detailed enough to find, and saved as a PNG.
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    check(Rgb8::decode(bytes)?).map(|img| img.encode_png())
}

/// `img` shrunk to `MAX_SIDE` if needed, or why it can't be used.
pub fn check(img: Rgb8) -> Result<Rgb8, ImageError> {
    if img.w.min(img.h) < MIN_SIDE as usize {
        return Err(ImageError::TooSmall);
    }
    let long = img.w.max(img.h);
    let img = if long > MAX_SIDE {
        let k = MAX_SIDE as f64 / long as f64;
        let (w, h) = ((img.w as f64 * k).round() as usize, (img.h as f64 * k).round() as usize);
        img.resize(w.max(1), h.max(1))
    } else {
        img
    };
    if img.gray().detail() < MIN_DETAIL {
        return Err(ImageError::TooPlain);
    }
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A light background with a few flat panels, like an app window.
    fn desktop(w: usize, h: usize, seed: u64) -> Gray {
        let mut rng = fastrand::Rng::with_seed(seed);
        let mut px = vec![235.0; w * h];
        for _ in 0..12 {
            let (pw, ph) = (rng.usize(20..w / 3), rng.usize(10..h / 3));
            let (px0, py0) = (rng.usize(0..w - pw), rng.usize(0..h - ph));
            let v = rng.u8(120..250) as f32;
            for y in py0..py0 + ph {
                px[y * w + px0..y * w + px0 + pw].fill(v);
            }
        }
        Gray { w, h, px }
    }

    /// A button: a dark frame with a blocky "label" inside.
    fn button(w: usize, h: usize, seed: u64) -> Gray {
        let mut rng = fastrand::Rng::with_seed(seed);
        let cells: Vec<f32> = (0..64).map(|_| if rng.bool() { 30.0 } else { 200.0 }).collect();
        let px = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 { 60.0 } else { cells[(y * 4 / h) * 16 + x * 16 / w] }
            })
            .collect();
        Gray { w, h, px }
    }

    fn paste(screen: &mut Gray, img: &Gray, x: usize, y: usize) {
        for j in 0..img.h {
            let row = &img.px[j * img.w..(j + 1) * img.w];
            screen.px[(y + j) * screen.w + x..(y + j) * screen.w + x + img.w].copy_from_slice(row);
        }
    }

    const OPTS: FindOpts = FindOpts { threshold: 0.85 };

    /// Plants `btn` scaled by `s` at (x, y) and finds it.
    fn planted(s: f32, x: usize, y: usize) -> (Match, Gray) {
        let btn = button(100, 40, 7);
        let mut screen = desktop(800, 500, 3);
        let shown = btn.resize((100.0 * s).round() as usize, (40.0 * s).round() as usize);
        paste(&mut screen, &shown, x, y);
        (find(&screen, &btn, OPTS).expect("fits"), btn)
    }

    fn assert_near(m: Match, x: i32, y: i32, s: f32) {
        let tol = (4.0 * s).max(3.0) as i32;
        assert!((m.x - x).abs() <= tol && (m.y - y).abs() <= tol, "{m:?} vs ({x}, {y})");
        assert!((m.scale / s - 1.0).abs() < 0.06, "{m:?} vs scale {s}");
        assert!(m.score >= 0.85, "{m:?}");
    }

    #[test]
    fn finds_the_image_at_its_own_size() {
        let (m, _) = planted(1.0, 321, 187);
        assert_eq!((m.x, m.y, m.w, m.h), (321, 187, 100, 40));
        assert!(m.score > 0.99, "{m:?}");
    }

    #[test]
    fn finds_the_image_scaled_up_or_down() {
        for (s, x, y) in [(0.5, 40, 30), (0.8, 600, 400), (1.25, 250, 90), (1.5, 500, 300), (2.0, 10, 380)] {
            let (m, _) = planted(s, x, y);
            assert_near(m, x as i32, y as i32, s);
        }
    }

    #[test]
    fn finds_the_image_against_the_edges() {
        let (m, _) = planted(1.0, 0, 0);
        assert_eq!((m.x, m.y), (0, 0));
        let (m, _) = planted(1.0, 700, 460);
        assert_eq!((m.x, m.y), (700, 460));
    }

    #[test]
    fn survives_a_change_of_brightness_and_some_noise() {
        let btn = button(100, 40, 7);
        let mut screen = desktop(800, 500, 3);
        let mut rng = fastrand::Rng::with_seed(1);
        let hovered =
            Gray { px: btn.px.iter().map(|v| v * 0.8 + 30.0 + rng.f32() * 16.0 - 8.0).collect(), ..btn.clone() };
        paste(&mut screen, &hovered, 200, 200);
        let m = find(&screen, &btn, OPTS).unwrap();
        assert_eq!((m.x, m.y), (200, 200));
        assert!(m.score > 0.9, "{m:?}");
    }

    #[test]
    fn a_missing_image_scores_low() {
        let screen = desktop(800, 500, 3);
        let m = find(&screen, &button(100, 40, 7), OPTS);
        assert!(m.is_none_or(|m| m.score < 0.7), "{m:?}");
    }

    #[test]
    fn tells_similar_buttons_apart() {
        let (ok, cancel) = (button(100, 40, 7), button(100, 40, 8));
        let mut screen = desktop(800, 500, 3);
        paste(&mut screen, &cancel, 100, 100);
        paste(&mut screen, &ok, 400, 300);
        let m = find(&screen, &ok, OPTS).unwrap();
        assert_eq!((m.x, m.y), (400, 300));
    }

    #[test]
    fn an_image_larger_than_the_screen_is_only_tried_smaller() {
        let mut screen = Gray { w: 90, h: 50, px: vec![235.0; 90 * 50] };
        paste(&mut screen, &button(100, 40, 7).resize(60, 24), 20, 10);
        let m = find(&screen, &button(100, 40, 7), OPTS).unwrap();
        assert_near(m, 20, 10, 0.6);
        assert_eq!(find(&Gray { w: 30, h: 30, px: vec![235.0; 900] }, &button(100, 40, 7), OPTS), None);
    }

    #[test]
    fn scales_start_with_one_and_cover_the_range_in_small_steps() {
        let s = scales();
        assert_eq!(&s[..2], &[1.0, MIN_SCALE]);
        assert!(s.iter().all(|&v| (MIN_SCALE..=MAX_SCALE * 1.001).contains(&v)));
        assert!(s.windows(2).skip(1).all(|p| p[1] / p[0] <= SCALE_STEP * 1.1));
        assert!(*s.last().unwrap() > MAX_SCALE / SCALE_STEP);
    }

    #[test]
    fn plain_images_have_no_detail() {
        assert!(Gray { w: 10, h: 10, px: vec![128.0; 100] }.detail() < MIN_DETAIL);
        assert!(button(100, 40, 7).detail() > MIN_DETAIL);
    }

    #[test]
    fn resize_keeps_the_average() {
        let g = button(100, 40, 7);
        let mean = |g: &Gray| g.px.iter().sum::<f32>() / g.px.len() as f32;
        for (w, h) in [(37, 15), (250, 100), (100, 40)] {
            assert!((mean(&g.resize(w, h)) - mean(&g)).abs() < 0.5);
        }
        assert_eq!(g.resize(100, 40), g);
    }

    fn rgb(w: usize, h: usize, f: impl Fn(usize, usize) -> [u8; 3]) -> Rgb8 {
        Rgb8 { w, h, px: (0..w * h).flat_map(|i| f(i % w, i / w)).collect() }
    }

    fn stripes(w: usize, h: usize) -> Rgb8 {
        rgb(w, h, |x, y| if (x / 4 + y / 4) % 2 == 0 { [20, 40, 200] } else { [240, 230, 10] })
    }

    #[test]
    fn a_png_round_trips() {
        let img = stripes(40, 24);
        let png = prepare(&img.encode_png()).unwrap();
        assert_eq!(Rgb8::decode(&png).unwrap(), img);
    }

    #[test]
    fn a_large_image_is_shrunk_to_the_maximum_side() {
        let png = prepare(&stripes(1200, 300).encode_png()).unwrap();
        let img = Rgb8::decode(&png).unwrap();
        assert_eq!((img.w, img.h), (MAX_SIDE, 128));
    }

    #[test]
    fn refuses_what_it_cant_use() {
        assert_eq!(prepare(b"GIF89a..."), Err(ImageError::Unreadable));
        assert_eq!(prepare(b"\x89PNG broken"), Err(ImageError::Unreadable));
        assert_eq!(prepare(&stripes(40, 6).encode_png()), Err(ImageError::TooSmall));
        assert_eq!(prepare(&rgb(40, 20, |_, _| [90, 90, 90]).encode_png()), Err(ImageError::TooPlain));
    }

    #[test]
    fn reads_transparency_on_white_and_gray_pngs() {
        let encode = |color, data: &[u8]| {
            let mut out = Vec::new();
            let mut enc = png::Encoder::new(&mut out, 2, 1);
            enc.set_color(color);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(data).unwrap();
            w.finish().unwrap();
            out
        };
        let img = Rgb8::decode(&encode(png::ColorType::Rgba, &[255, 0, 0, 255, 0, 0, 0, 0])).unwrap();
        assert_eq!(img.px, [255, 0, 0, 255, 255, 255]);
        let img = Rgb8::decode(&encode(png::ColorType::Grayscale, &[10, 200])).unwrap();
        assert_eq!(img.px, [10, 10, 10, 200, 200, 200]);
    }

    #[test]
    fn reads_a_jpeg() {
        let img = stripes(32, 16);
        let mut jpg = Vec::new();
        jpeg_encoder::Encoder::new(&mut jpg, 95).encode(&img.px, 32, 16, jpeg_encoder::ColorType::Rgb).unwrap();
        let back = Rgb8::decode(&jpg).unwrap();
        assert_eq!((back.w, back.h), (32, 16));
        assert!(back.px.iter().zip(&img.px).all(|(a, b)| a.abs_diff(*b) < 40));
    }

    #[test]
    fn crop_clips_to_the_image() {
        let img = rgb(10, 10, |x, y| [x as u8, y as u8, 0]);
        let c = img.crop(8, 7, 5, 5);
        assert_eq!((c.w, c.h), (2, 3));
        assert_eq!(&c.px[..3], &[8, 7, 0]);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(12))]
        #[test]
        fn a_planted_image_is_always_found(seed in 0u64..1000, s in 0.55f32..1.9, fx in 0.0f32..1.0, fy in 0.0f32..1.0) {
            let btn = button(80, 32, seed);
            let mut screen = desktop(640, 400, seed + 1);
            let shown = btn.resize((80.0 * s).round() as usize, (32.0 * s).round() as usize);
            let (x, y) = ((fx * (640 - shown.w) as f32) as usize, (fy * (400 - shown.h) as f32) as usize);
            paste(&mut screen, &shown, x, y);
            let m = find(&screen, &btn, OPTS).expect("found");
            assert_near(m, x as i32, y as i32, s);
        }
    }

    /// Run with `cargo test -p relay-core --release -- --ignored timing`.
    #[test]
    #[ignore]
    fn timing_on_a_large_screen() {
        let mut screen = desktop(2560, 1440, 3);
        let btn = button(100, 40, 7);
        let t = std::time::Instant::now();
        assert!(find(&screen, &btn, OPTS).is_none_or(|m| m.score < 0.85));
        let missing = t.elapsed();
        paste(&mut screen, &btn.resize(150, 60), 1700, 900);
        let t = std::time::Instant::now();
        let m = find(&screen, &btn, OPTS).unwrap();
        println!("missing: {missing:?}, found at 1.5: {:?}, {m:?}", t.elapsed());
    }
}

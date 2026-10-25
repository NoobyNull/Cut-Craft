//! Reading Vectric `.vfont` files: an OLE compound file whose streams hold MFC-serialized
//! glyphs (see `re/SPEC.md`, Fonts). Users point Cut Craft at the fonts their own Vectric
//! install has; none ship with Cut Craft.

use crate::font::{Font, Glyph};
use crate::geometry::{Path, Point, Segment};
use std::collections::{BTreeMap, HashMap};

const FREE: u32 = 0xFFFF_FFFF;
const END: u32 = 0xFFFF_FFFE;

pub fn read(bytes: &[u8]) -> Result<Font, String> {
    let streams = compound_streams(bytes)?;
    let info = streams
        .iter()
        .find(|(n, _)| n == "Info")
        .ok_or("no VectricFontInfo stream")?;
    let mut glyphs = Vec::new();
    for (name, data) in &streams {
        if name.starts_with("GLYPHS_") {
            glyphs.extend(read_glyphs(data)?);
        }
    }
    let (name, height, kerning) = read_info(&info.1)?;
    // fonts with a glyph at 129 (unused in Windows-1252) number their glyphs in code page 437
    let oem = glyphs.iter().any(|g| g.0 == 129);
    let decode = |code: u32| decode(code, oem);
    let glyphs: BTreeMap<char, Glyph> = glyphs
        .into_iter()
        .filter_map(|(code, g)| Some((decode(code)?, g)))
        .collect();
    let cap_height = height
        .or_else(|| glyphs.get(&'H').map(|g| top(&g.paths)))
        .filter(|h| *h > 0.0)
        .ok_or("the font has no height")?;
    let kerning = kerning
        .into_iter()
        .filter_map(|((a, b), k)| Some(((decode(a)?, decode(b)?), k)))
        .collect();
    Ok(Font {
        name,
        cap_height,
        // ponytail: 1.6 capital heights between baselines [inf]; the files carry no spacing
        line_pitch: 1.6 * cap_height,
        glyphs,
        kerning,
    })
}

fn top(paths: &[Path]) -> f64 {
    paths
        .iter()
        .map(|p| p.bounds().max.y)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// A byte code as a character: Windows-1252, or code page 437 for `oem` fonts.
fn decode(code: u32, oem: bool) -> Option<char> {
    const CP1252: &str = "€\u{81}‚ƒ„…†‡ˆ‰Š‹Œ\u{8d}Ž\u{8f}\u{90}‘’“”•–—˜™š›œ\u{9d}žŸ";
    const CP437: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»\
        ░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀\
        αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";
    let high = code.checked_sub(0x80).and_then(|i| usize::try_from(i).ok());
    match (code, high) {
        (0..0x80, _) => char::from_u32(code),
        (_, Some(i)) if oem => CP437.chars().nth(i),
        (0x80..0xA0, Some(i)) => CP1252.chars().nth(i),
        _ => char::from_u32(code).filter(|_| code < 0x100),
    }
}

/// Every stream of a compound file, by its own name (storages are not kept).
#[expect(
    clippy::many_single_char_names,
    reason = "the format's own short names"
)]
fn compound_streams(b: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    let bad = || "not a Vectric font (damaged compound file)".to_string();
    if b.get(..8) != Some(&MAGIC) {
        return Err("not a Vectric font".into());
    }
    let r = Reader { b, o: 0 };
    let sector = 1usize << r.u16_at(0x1E).ok_or_else(bad)?;
    let mini = 1usize << r.u16_at(0x20).ok_or_else(bad)?;
    let h = |at| r.u32_at(at).ok_or_else(bad);
    let (fat_sectors, dir0, cutoff, mini0, difat0, difat_sectors) =
        (h(0x2C)?, h(0x30)?, h(0x38)?, h(0x3C)?, h(0x44)?, h(0x48)?);
    // the last sector may be cut short at the end of the file
    let sec = |i: u32| {
        let at = (i as usize + 1) * sector;
        b.get(at..b.len().min(at + sector))
            .filter(|s| !s.is_empty())
            .ok_or_else(bad)
    };
    let ids = |s: &[u8]| -> Vec<u32> {
        s.as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect()
    };
    let mut difat: Vec<u32> = ids(&b[0x4C..0x200]);
    let mut d = difat0;
    for _ in 0..difat_sectors {
        let more = ids(sec(d)?);
        let (next, list) = more.split_last().ok_or_else(bad)?;
        difat.extend(list);
        d = *next;
    }
    let mut fat = Vec::new();
    for &s in difat.iter().take(fat_sectors as usize) {
        fat.extend(ids(sec(s)?));
    }
    let chain = |start: u32, table: &[u32]| -> Result<Vec<u32>, String> {
        let mut out = Vec::new();
        let mut i = start;
        while i != FREE && i != END {
            if out.len() > table.len() {
                return Err(bad());
            }
            out.push(i);
            i = *table.get(i as usize).ok_or_else(bad)?;
        }
        Ok(out)
    };
    let big = |start: u32| -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        for i in chain(start, &fat)? {
            out.extend_from_slice(sec(i)?);
        }
        Ok(out)
    };
    let dir = big(dir0)?;
    let entries: Vec<_> = dir.as_chunks::<128>().0.iter().map(|e| entry(e)).collect();
    let root = entries.first().ok_or_else(bad)?;
    let ministream = big(root.2)?;
    let mut minifat = Vec::new();
    for i in chain(mini0, &fat)? {
        minifat.extend(ids(sec(i)?));
    }
    let mut out = Vec::new();
    for (name, kind, start, size) in &entries {
        if *kind != 2 {
            continue;
        }
        let mut data = if *size < cutoff as usize {
            let mut d = Vec::new();
            for i in chain(*start, &minifat)? {
                let at = i as usize * mini;
                d.extend_from_slice(ministream.get(at..at + mini).ok_or_else(bad)?);
            }
            d
        } else {
            big(*start)?
        };
        if data.len() < *size {
            return Err(bad());
        }
        data.truncate(*size);
        out.push((name.clone(), data));
    }
    Ok(out)
}

/// A directory entry: name, kind (2 a stream, 5 the root), first sector, size.
fn entry(e: &[u8]) -> (String, u8, u32, usize) {
    let r = Reader { b: e, o: 0 };
    let len = usize::from(r.u16_at(64).unwrap_or(0)).clamp(2, 64);
    let units: Vec<u16> = e[..len - 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let size = r.u32_at(120).unwrap_or(0) as usize;
    (
        String::from_utf16_lossy(&units),
        e[66],
        r.u32_at(116).unwrap_or(END),
        size,
    )
}

struct Reader<'a> {
    b: &'a [u8],
    o: usize,
}

impl Reader<'_> {
    fn u16_at(&self, at: usize) -> Option<u16> {
        Some(u16::from_le_bytes(self.b.get(at..at + 2)?.try_into().ok()?))
    }
    fn u32_at(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.b.get(at..at + 4)?.try_into().ok()?))
    }
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self
            .b
            .get(self.o..self.o + n)
            .ok_or("the font ends too soon")?;
        self.o += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(
            self.take(8)?.try_into().expect("8 bytes"),
        ))
    }
    fn point(&mut self) -> Result<Point, String> {
        let p = Point::new(self.f64()?, self.f64()?);
        self.f64()?; // z, always 0
        Ok(p)
    }
}

/// MFC `CArchive` object reading: classes and objects share one numbering, so a later object
/// of a known class names it by number.
struct Archive<'a> {
    r: Reader<'a>,
    index: Vec<Option<String>>,
}

enum Obj {
    Char(u32, Glyph),
    Group(Vec<Path>),
    Contour(Path),
    Span(Point, Segment),
    Kerning(HashMap<(u32, u32), f64>),
}

impl<'a> Archive<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self {
            r: Reader { b, o: 0 },
            index: vec![None],
        }
    }

    fn obj(&mut self) -> Result<Option<Obj>, String> {
        let mut tag = u32::from(self.r.u16()?);
        let (class_bit, mask) = if tag == 0x7FFF {
            tag = self.r.u32()?;
            (0x8000_0000, 0x7FFF_FFFF)
        } else {
            (0x8000, 0x7FFF)
        };
        let class = match tag {
            0 => return Ok(None),
            0xFFFF => {
                self.r.u16()?; // schema
                let n = usize::from(self.r.u16()?);
                let name = String::from_utf8_lossy(self.r.take(n)?).into_owned();
                self.index.push(Some(name.clone()));
                name
            }
            t if t & class_bit != 0 => self
                .index
                .get((t & mask) as usize)
                .cloned()
                .flatten()
                .ok_or("the font refers to an unknown class")?,
            _ => return Err("the font shares an object; not expected in fonts".into()),
        };
        self.index.push(None);
        self.read(&class).map(Some)
    }

    fn read(&mut self, class: &str) -> Result<Obj, String> {
        let ver = self.r.u32()?;
        Ok(match class {
            "txtVectricChar" => {
                let code = self.r.u32()?;
                let advance = self.r.f64()?;
                let paths = match self.obj()? {
                    Some(Obj::Group(p)) => p,
                    None => vec![],
                    _ => return Err("a glyph holds something other than contours".into()),
                };
                Obj::Char(code, Glyph { advance, paths })
            }
            "vdContourGroup" => {
                let n = self.r.u32()?;
                let mut paths = Vec::new();
                for _ in 0..n {
                    if let Some(Obj::Contour(p)) = self.obj()? {
                        paths.push(p);
                    }
                }
                if ver >= 2 {
                    self.r.u8()?;
                }
                Obj::Group(paths)
            }
            "vdContour" => {
                self.r.take(4 + 8 + 8 + 2)?;
                let n = self.r.u32()?;
                let mut start = None;
                let mut segments = Vec::new();
                for _ in 0..n {
                    if let Some(Obj::Span(from, s)) = self.obj()? {
                        start.get_or_insert(from);
                        segments.push(s);
                    }
                }
                if ver >= 2 {
                    self.r.take(16)?;
                }
                let start = start.unwrap_or_default();
                let end = segments.last().map_or(start, Segment::end);
                let closed =
                    !segments.is_empty() && end.distance(start) < 1e-9 * (1.0 + start.length());
                Obj::Contour(Path {
                    start,
                    segments,
                    closed,
                })
            }
            "vdLineSpan" | "vdArcSpan" | "vdBezierSpan" => {
                self.r.u32()?;
                let from = self.r.point()?;
                let to = self.r.point()?;
                self.r.u32()?;
                let s = match class {
                    "vdLineSpan" => Segment::Line { to },
                    "vdArcSpan" => arc(from, to, self.r.f64()?),
                    _ => Segment::Cubic {
                        c1: Point::new(self.r.f64()?, self.r.f64()?),
                        c2: Point::new(self.r.f64()?, self.r.f64()?),
                        to,
                    },
                };
                Obj::Span(from, s)
            }
            "txtKerningPairs" => {
                let n = self.r.u32()?;
                self.r.f64()?;
                let mut pairs = HashMap::new();
                for _ in 0..n {
                    let a = u32::from(self.r.u16()?);
                    let b = u32::from(self.r.u16()?);
                    #[expect(clippy::cast_possible_wrap, reason = "stored as a signed 32-bit int")]
                    let k = self.r.u32()? as i32;
                    pairs.insert((a, b), f64::from(k));
                }
                Obj::Kerning(pairs)
            }
            other => return Err(format!("unknown font part {other}")),
        })
    }
}

/// An arc span: `bulge` is tan(sweep / 4), positive counter-clockwise.
fn arc(from: Point, to: Point, bulge: f64) -> Segment {
    if bulge.abs() < 1e-12 {
        return Segment::Line { to };
    }
    let d = to - from;
    let center = (from + to) / 2.0 + d.perp() * ((1.0 - bulge * bulge) / (4.0 * bulge));
    Segment::Arc {
        to,
        center,
        ccw: bulge > 0.0,
    }
}

fn read_glyphs(data: &[u8]) -> Result<Vec<(u32, Glyph)>, String> {
    let mut a = Archive::new(data);
    a.r.u32()?;
    let mut out = Vec::new();
    while a.r.o < data.len() {
        if let Some(Obj::Char(code, g)) = a.obj()? {
            out.push((code, g));
        }
    }
    Ok(out)
}

type Info = (String, Option<f64>, HashMap<(u32, u32), f64>);

/// Name, capital height and kerning. Version 3 files store the height; older ones do not.
fn read_info(data: &[u8]) -> Result<Info, String> {
    let mut a = Archive::new(data);
    let ver = a.r.u32()?;
    a.r.u32()?;
    let n = usize::from(a.r.u8()?);
    let name = String::from_utf8_lossy(a.r.take(n)?).into_owned();
    if ver < 3 {
        return Ok((name, None, HashMap::new()));
    }
    let v: Vec<f64> = (0..6).map(|_| a.r.f64()).collect::<Result<_, _>>()?;
    let kerning = match a.obj()? {
        Some(Obj::Kerning(k)) => k,
        _ => HashMap::new(),
    };
    Ok((name, Some(v[4]), kerning))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A compound file holding `streams`, every one in ordinary sectors (cut-off 0).
    fn compound(streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
        const S: usize = 512;
        let pad = |mut v: Vec<u8>| {
            v.resize(v.len().div_ceil(S).max(1) * S, 0);
            v
        };
        let mut body: Vec<Vec<u8>> = Vec::new(); // sector 0 is the FAT, 1 the directory
        let mut fat = vec![0xFFFF_FFFD_u32, END];
        let mut dir = vec![0u8; 128];
        dir[..10].copy_from_slice(
            &"Root"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .chain([0, 0])
                .collect::<Vec<_>>(),
        );
        dir[64..66].copy_from_slice(&10u16.to_le_bytes());
        dir[66] = 5;
        dir[116..120].copy_from_slice(&END.to_le_bytes());
        for (name, data) in streams {
            let mut e = vec![0u8; 128];
            let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
            e[..units.len()].copy_from_slice(&units);
            e[64..66].copy_from_slice(&u16::try_from(units.len() + 2).unwrap().to_le_bytes());
            e[66] = 2;
            let first = u32::try_from(fat.len()).unwrap();
            let sectors = pad(data.clone());
            let n = sectors.len() / S;
            for k in 0..n {
                fat.push(if k + 1 == n {
                    END
                } else {
                    first + u32::try_from(k).unwrap() + 1
                });
            }
            e[116..120].copy_from_slice(&first.to_le_bytes());
            e[120..124].copy_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            dir.extend(e);
            body.push(sectors);
        }
        assert!(
            dir.len() <= S && fat.len() <= S / 4,
            "test files stay one sector each"
        );
        let mut h = vec![0u8; S];
        h[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        h[0x1E..0x20].copy_from_slice(&9u16.to_le_bytes());
        h[0x20..0x22].copy_from_slice(&6u16.to_le_bytes());
        for (at, v) in [
            (0x2C, 1),
            (0x30, 1),
            (0x38, 0),
            (0x3C, END),
            (0x44, END),
            (0x48, 0),
        ] {
            h[at..at + 4].copy_from_slice(&u32::to_le_bytes(v));
        }
        for k in 0..109 {
            let v = if k == 0 { 0 } else { FREE };
            h[0x4C + 4 * k..0x50 + 4 * k].copy_from_slice(&v.to_le_bytes());
        }
        fat.resize(S / 4, FREE);
        let mut out = h;
        out.extend(fat.iter().flat_map(|v| v.to_le_bytes()));
        out.extend(pad(dir));
        for s in body {
            out.extend(s);
        }
        out
    }

    /// Writes objects the way `CArchive` does: a new class by name, later ones by number.
    #[derive(Default)]
    struct Writer {
        out: Vec<u8>,
        classes: Vec<(&'static str, u16)>,
        next: u16,
    }

    impl Writer {
        fn object(&mut self, class: &'static str, ver: u32) -> &mut Self {
            self.next = self.next.max(1);
            if let Some(&(_, n)) = self.classes.iter().find(|c| c.0 == class) {
                self.out.extend((0x8000 | n).to_le_bytes());
            } else {
                self.out.extend([0xFF, 0xFF, 1, 0]);
                self.out
                    .extend(u16::try_from(class.len()).unwrap().to_le_bytes());
                self.out.extend(class.as_bytes());
                self.classes.push((class, self.next));
                self.next += 1;
            }
            self.next += 1;
            self.u32(ver)
        }
        fn u32(&mut self, v: u32) -> &mut Self {
            self.out.extend(v.to_le_bytes());
            self
        }
        fn f64s(&mut self, v: &[f64]) -> &mut Self {
            for x in v {
                self.out.extend(x.to_le_bytes());
            }
            self
        }
        fn bytes(&mut self, v: &[u8]) -> &mut Self {
            self.out.extend(v);
            self
        }
        fn span(&mut self, class: &'static str, from: [f64; 2], to: [f64; 2]) -> &mut Self {
            self.object(class, 2)
                .u32(1)
                .f64s(&[from[0], from[1], 0.0, to[0], to[1], 0.0])
                .u32(1)
        }
        fn contour(&mut self, spans: u32) -> &mut Self {
            self.object("vdContour", 2)
                .u32(FREE)
                .f64s(&[0.001, 0.0])
                .bytes(&[0, 0])
                .u32(spans)
        }
    }

    /// 'H': one closed contour (line, arc, Bézier, line); 'I': an open stroke; then a gap.
    fn font_file() -> Vec<u8> {
        let mut g = Writer::default();
        g.u32(1);
        g.object("txtVectricChar", 1)
            .u32(u32::from(b'H'))
            .f64s(&[900.0]);
        g.object("vdContourGroup", 2).u32(1);
        g.contour(4)
            .span("vdLineSpan", [0.0, 0.0], [600.0, 0.0])
            .span("vdArcSpan", [600.0, 0.0], [600.0, 1000.0])
            .f64s(&[1.0])
            .span("vdBezierSpan", [600.0, 1000.0], [0.0, 1000.0])
            .f64s(&[400.0, 1100.0, 200.0, 1100.0])
            .span("vdLineSpan", [0.0, 1000.0], [0.0, 0.0])
            .bytes(&[0; 16]);
        g.bytes(&[1]);
        g.bytes(&[0, 0]); // an empty slot
        g.object("txtVectricChar", 1).u32(0xC9).f64s(&[300.0]);
        g.object("vdContourGroup", 2).u32(1);
        g.contour(1)
            .span("vdLineSpan", [100.0, 0.0], [100.0, 1000.0])
            .bytes(&[0; 16]);
        g.bytes(&[1]);
        let mut info = Writer::default();
        info.u32(3).u32(1).bytes(&[4]).bytes(b"Test");
        info.f64s(&[1.0, 0.001, 1.0, 0.0, 1000.0, 1.0]);
        info.object("txtKerningPairs", 1).u32(1).f64s(&[1.0]);
        info.bytes(&[b'H', 0, 0xC9, 0])
            .u32((-50_i32).cast_unsigned());
        compound(&[("Info", info.out), ("GLYPHS_0_255", g.out)])
    }

    #[test]
    fn reads_glyphs_spans_and_kerning() {
        let f = read(&font_file()).unwrap();
        assert_eq!(f.name, "Test");
        assert!((f.cap_height - 1000.0).abs() < 1e-9);
        let h = &f.glyphs[&'H'];
        assert!((h.advance - 900.0).abs() < 1e-9);
        let p = &h.paths[0];
        assert!(p.closed);
        assert_eq!(p.segments.len(), 4);
        match p.segments[1] {
            Segment::Arc { center, ccw, .. } => {
                // bulge 1 is a half circle, counter-clockwise: centre on the chord
                assert!((center - Point::new(600.0, 500.0)).length() < 1e-9);
                assert!(ccw);
            }
            s => panic!("expected an arc, got {s:?}"),
        }
        assert!(matches!(p.segments[2], Segment::Cubic { .. }));
        let e = &f.glyphs[&'É'];
        assert!(!e.paths[0].closed, "single-line strokes stay open");
        assert!((f.kerning[&('H', 'É')] + 50.0).abs() < 1e-9);
    }

    #[test]
    fn code_page_437_fonts_are_recognised_by_their_glyph_at_129() {
        assert_eq!(decode(0x80, false), Some('€'));
        assert_eq!(decode(0xC9, false), Some('É'));
        assert_eq!(decode(0x80, true), Some('Ç'));
        assert_eq!(decode(0x81, true), Some('ü'));
        assert_eq!(decode(0xFF, true), Some('\u{a0}'));
        assert_eq!(decode(0x41, true), Some('A'));
    }

    #[test]
    fn damaged_files_are_refused_not_panicked_on() {
        let good = font_file();
        assert!(read(b"not a font").is_err());
        // the padding after the last stream may go; its data may not
        for cut in [600, 1100, good.len() - 512 + 20] {
            assert!(read(&good[..cut]).is_err(), "cut at {cut}");
        }
    }
}

/// Reads every `.vfont` in `$VFONT_DIR` (a Vectric install's `Fonts/`); run by hand with
/// `VFONT_DIR=... cargo test -p cut-craft-core -- --ignored installed`.
#[cfg(test)]
#[test]
#[ignore = "needs a Vectric install"]
fn every_installed_font_reads() {
    let dir = std::env::var("VFONT_DIR").expect("set VFONT_DIR");
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if path.extension().is_some_and(|x| x == "vfont") {
            let f =
                read(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            let spans: usize = f
                .glyphs
                .values()
                .flat_map(|g| &g.paths)
                .map(|p| p.segments.len())
                .sum();
            assert!(f.glyphs.contains_key(&'A') && spans > 0, "{path:?}");
            println!(
                "{:28} {} glyphs, {spans} spans, H {:.0}",
                f.name,
                f.glyphs.len(),
                f.cap_height
            );
        }
    }
}

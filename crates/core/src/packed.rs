//! Bulk numbers in job files as one string: base64 of their little-endian bytes. A heightmap,
//! a picture's pixels or a toolpath's moves take 8, 1 and 25 bytes a value instead of a JSON
//! number (or object) each. Each reader also takes the plain JSON array older files hold.
// ponytail: raw doubles; f32 would halve heights and moves again, at a tenth of a micron

use crate::toolpath::{Move, MoveKind};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serializer};

fn encode<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&STANDARD.encode(bytes))
}

/// A packed string, or the array an older file holds.
#[derive(Deserialize)]
#[serde(untagged)]
enum Stored<T> {
    Packed(String),
    Plain(Vec<T>),
}

fn decode<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
    unpack: impl Fn(&[u8]) -> Result<Vec<T>, String>,
) -> Result<Vec<T>, D::Error> {
    match Stored::deserialize(d)? {
        Stored::Plain(values) => Ok(values),
        Stored::Packed(text) => {
            let bytes = STANDARD.decode(text).map_err(D::Error::custom)?;
            unpack(&bytes).map_err(D::Error::custom)
        }
    }
}

/// The bytes cut into `N`-byte records, each made a value.
fn records<const N: usize, T>(bytes: &[u8], from: impl Fn([u8; N]) -> T) -> Result<Vec<T>, String> {
    let (chunks, rest) = bytes.as_chunks::<N>();
    if !rest.is_empty() {
        return Err(format!(
            "{} packed bytes do not divide into {N}-byte values",
            bytes.len()
        ));
    }
    Ok(chunks.iter().map(|c| from(*c)).collect())
}

/// A component's heights, `Vec<f64>` with NaN where it has no shape; older files hold an
/// array with `null` there.
pub mod heights {
    use super::{Deserialize, Deserializer, STANDARD, Serializer, Stored, encode, records};
    use base64::Engine as _;
    use serde::de::Error as _;

    pub fn serialize<S: Serializer>(v: &[f64], s: S) -> Result<S::Ok, S::Error> {
        let bytes: Vec<u8> = v.iter().flat_map(|h| h.to_le_bytes()).collect();
        encode(&bytes, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
        match Stored::<Option<f64>>::deserialize(d)? {
            Stored::Plain(values) => {
                Ok(values.into_iter().map(|h| h.unwrap_or(f64::NAN)).collect())
            }
            Stored::Packed(text) => {
                let bytes = STANDARD.decode(text).map_err(D::Error::custom)?;
                records(&bytes, f64::from_le_bytes).map_err(D::Error::custom)
            }
        }
    }

    /// `Option<Vec<f64>>`.
    pub mod option {
        use super::super::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(v: &Option<Vec<f64>>, s: S) -> Result<S::Ok, S::Error> {
            match v {
                Some(v) => super::serialize(v, s),
                None => s.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<f64>>, D::Error> {
            #[derive(Deserialize)]
            struct Inner(#[serde(with = "super")] Vec<f64>);
            Ok(Option::<Inner>::deserialize(d)?.map(|i| i.0))
        }
    }
}

/// `Vec<u8>`.
pub mod u8s {
    use super::{Deserializer, Serializer, decode, encode};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        encode(v, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        decode(d, |b| Ok(b.to_vec()))
    }
}

/// `Vec<f32>`.
pub mod f32s {
    use super::{Deserializer, Serializer, decode, encode, records};

    pub fn serialize<S: Serializer>(v: &[f32], s: S) -> Result<S::Ok, S::Error> {
        let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        encode(&bytes, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<f32>, D::Error> {
        decode(d, |b| records(b, f32::from_le_bytes))
    }
}

/// `Vec<Move>`: for each, a kind byte (0 rapid, 1 plunge, 2 cut) then x, y and z.
pub mod moves {
    use super::{Deserializer, Move, MoveKind, Serializer, decode, encode, records};

    const SIZE: usize = 1 + 3 * 8;

    fn kind_byte(kind: MoveKind) -> u8 {
        match kind {
            MoveKind::Rapid => 0,
            MoveKind::Plunge => 1,
            MoveKind::Cut => 2,
        }
    }

    pub fn serialize<S: Serializer>(v: &[Move], s: S) -> Result<S::Ok, S::Error> {
        let mut bytes = Vec::with_capacity(v.len() * SIZE);
        for m in v {
            bytes.push(kind_byte(m.kind));
            for c in m.to.to_array() {
                bytes.extend(c.to_le_bytes());
            }
        }
        encode(&bytes, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Move>, D::Error> {
        decode(d, |b| {
            let unpacked = records(b, |c: [u8; SIZE]| {
                let at = |i: usize| f64::from_le_bytes(c[i..i + 8].try_into().expect("8 bytes"));
                let kind = match c[0] {
                    0 => Some(MoveKind::Rapid),
                    1 => Some(MoveKind::Plunge),
                    2 => Some(MoveKind::Cut),
                    _ => None,
                };
                kind.map(|kind| Move {
                    kind,
                    to: glam::DVec3::new(at(1), at(9), at(17)),
                })
            })?;
            unpacked
                .into_iter()
                .collect::<Option<Vec<Move>>>()
                .ok_or_else(|| "a packed move has an unknown kind".to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Serialize, Deserialize)]
    struct Everything {
        #[serde(with = "heights")]
        heights: Vec<f64>,
        #[serde(default, with = "heights::option")]
        unsculpted: Option<Vec<f64>>,
        #[serde(with = "u8s")]
        grey: Vec<u8>,
        #[serde(with = "f32s")]
        depth: Vec<f32>,
        #[serde(with = "moves")]
        moves: Vec<Move>,
    }

    /// Equal field by field; NaN prints as NaN, so holes match holes.
    fn same(a: &Everything, b: &Everything) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    fn sample() -> Everything {
        Everything {
            heights: vec![1.5, f64::NAN, -0.25, f64::MAX],
            unsculpted: Some(vec![f64::NAN, 2.0]),
            grey: vec![0, 127, 255],
            depth: vec![0.0, 0.5, 1.0],
            moves: vec![
                Move {
                    kind: MoveKind::Rapid,
                    to: glam::DVec3::new(1.0, 2.0, 3.0),
                },
                Move {
                    kind: MoveKind::Plunge,
                    to: glam::DVec3::new(-1.0, 0.0, -5.5),
                },
                Move {
                    kind: MoveKind::Cut,
                    to: glam::DVec3::new(10.0, 20.0, -1.0),
                },
            ],
        }
    }

    #[test]
    fn everything_round_trips_exactly_as_strings() {
        let json = serde_json::to_value(sample()).unwrap();
        assert!(json["heights"].is_string() && json["moves"].is_string());
        assert!(json["grey"].is_string() && json["depth"].is_string());
        assert!(json["unsculpted"].is_string());
        let back: Everything = serde_json::from_value(json).unwrap();
        assert!(same(&back, &sample()), "{back:?}");
    }

    #[test]
    fn the_arrays_older_files_hold_still_load() {
        let old = serde_json::json!({
            "heights": [1.5, null, -0.25, f64::MAX],
            "unsculpted": [null, 2.0],
            "grey": [0, 127, 255],
            "depth": [0.0, 0.5, 1.0],
            "moves": [
                {"kind": "rapid", "to": [1.0, 2.0, 3.0]},
                {"kind": "plunge", "to": [-1.0, 0.0, -5.5]},
                {"kind": "cut", "to": [10.0, 20.0, -1.0]},
            ],
        });
        let back: Everything = serde_json::from_value(old).unwrap();
        assert!(same(&back, &sample()), "{back:?}");
        let none: Everything =
            serde_json::from_str(r#"{"heights":[],"grey":[],"depth":[],"moves":[]}"#).unwrap();
        assert!(none.unsculpted.is_none());
        assert!(none.heights.is_empty() && none.moves.is_empty());
        let empty: Everything =
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap();
        assert!(same(&empty, &none), "empty packs round trip too");
    }

    #[derive(Serialize)]
    struct Heights<'a>(#[serde(with = "heights")] &'a [f64]);

    #[derive(Serialize)]
    struct Moves<'a>(#[serde(with = "moves")] &'a [Move]);

    /// Measured on a 400 x 800 mm job: a full-size model's heights 29.5 bytes a cell as
    /// pretty JSON, 10.7 packed; a 3D finishing toolpath 187 bytes a move, 33 packed.
    #[test]
    fn packed_is_a_fraction_of_the_pretty_json_size() {
        let heights: Vec<f64> = (0..10_000)
            .map(|i| (f64::from(i % 100) * 0.3).sin() + f64::from(i / 100) * 0.01)
            .collect();
        let packed = serde_json::to_string(&Heights(&heights)).unwrap().len();
        let plain = serde_json::to_string_pretty(&heights).unwrap().len();
        assert!(
            plain > 2 * packed,
            "heights: {packed} packed vs {plain} plain"
        );
        let moves: Vec<Move> = (0..1000)
            .map(|i| Move {
                kind: MoveKind::Cut,
                to: glam::DVec3::new(f64::from(i) * 0.5, 12.345, -(f64::from(i) * 0.3).sin()),
            })
            .collect();
        let packed = serde_json::to_string(&Moves(&moves)).unwrap().len();
        let plain = serde_json::to_string_pretty(&moves).unwrap().len();
        assert!(
            plain > 2 * packed,
            "moves: {packed} packed vs {plain} plain"
        );
    }

    #[derive(Deserialize)]
    #[expect(dead_code, reason = "only whether it loads matters")]
    struct LoadedHeights(#[serde(with = "heights")] Vec<f64>);

    #[derive(Deserialize)]
    #[expect(dead_code, reason = "only whether it loads matters")]
    struct LoadedMoves(#[serde(with = "moves")] Vec<Move>);

    #[test]
    fn damaged_packing_is_refused() {
        assert!(
            serde_json::from_str::<LoadedHeights>(r#""AAA""#).is_err(),
            "not 8-byte values"
        );
        assert!(serde_json::from_str::<LoadedHeights>(r#""not base64!""#).is_err());
        let bad_kind = STANDARD.encode([9u8; 25]);
        assert!(serde_json::from_str::<LoadedMoves>(&format!("\"{bad_kind}\"")).is_err());
    }
}

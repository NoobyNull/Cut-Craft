//! The Depth add-on: Depth Anything V2 Small, run on the CPU with candle, turns a photo into
//! relative depth (bigger is nearer) at the photo's own size. Only the Small model's weights
//! are Apache 2.0; the larger ones are for non-commercial use and must not be offered.

use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::depth_anything_v2::{DepthAnythingV2, DepthAnythingV2Config};
use candle_transformers::models::dinov2;
use image::{ImageBuffer, Luma, RgbImage, imageops::FilterType};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// The network sees the photo stretched to this many pixels square.
pub const SIDE: usize = 518;
/// The `ImageNet` mean and spread of each colour, which the network was trained on.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const SPREAD: [f32; 3] = [0.229, 0.224, 0.225];

/// The network's input: the photo (`rgb`, three bytes a pixel, rows from the top) stretched
/// to [`SIDE`] square, each colour normalised, colour by colour (red plane first).
pub fn network_input(rgb: &[u8], columns: u32, rows: u32) -> Vec<f32> {
    let photo = RgbImage::from_raw(columns, rows, rgb.to_vec()).expect("three bytes a pixel");
    #[expect(clippy::cast_possible_truncation, reason = "518")]
    let side = SIDE as u32;
    let square = image::imageops::resize(&photo, side, side, FilterType::CatmullRom);
    let mut planes = vec![0.0; 3 * SIDE * SIDE];
    for (i, pixel) in square.pixels().enumerate() {
        for colour in 0..3 {
            let v = f32::from(pixel[colour]) / 255.0;
            planes[colour * SIDE * SIDE + i] = (v - MEAN[colour]) / SPREAD[colour];
        }
    }
    planes
}

/// `values` (`from` columns by rows) resampled to `to`, blending between pixel centres.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "pixel positions"
)]
pub fn resample(values: &[f32], from: (usize, usize), to: (usize, usize)) -> Vec<f32> {
    let ((fc, fr), (tc, tr)) = (from, to);
    let at = |c: usize, r: usize| values[r * fc + c];
    // a destination pixel centre in source pixels, and its two neighbours with the blend
    let place = |i: usize, t: usize, f: usize| {
        let x = ((i as f32 + 0.5) * f as f32 / t as f32 - 0.5).clamp(0.0, (f - 1) as f32);
        let low = x.floor() as usize;
        (low, (low + 1).min(f - 1), x - x.floor())
    };
    let mut out = Vec::with_capacity(tc * tr);
    for r in 0..tr {
        let (up, down, below) = place(r, tr, fr);
        for c in 0..tc {
            let (left, right, across) = place(c, tc, fc);
            let top = at(left, up) * (1.0 - across) + at(right, up) * across;
            let bottom = at(left, down) * (1.0 - across) + at(right, down) * across;
            out.push(top * (1.0 - below) + bottom * below);
        }
    }
    out
}

/// `values` spread over the full 16-bit range: the lowest black, the highest white.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "scaled into 0..=65535"
)]
pub fn to_grey16(values: &[f32], columns: u32, rows: u32) -> ImageBuffer<Luma<u16>, Vec<u16>> {
    let lo = values.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (hi - lo).max(f32::EPSILON);
    let levels = values
        .iter()
        .map(|v| ((v - lo) / span * 65_535.0).round() as u16)
        .collect();
    ImageBuffer::from_raw(columns, rows, levels).expect("one value a pixel")
}

/// Depth Anything V2 Small, ready to run.
pub struct Estimator {
    model: DepthAnythingV2,
}

impl Estimator {
    /// Loads the weights: the official `depth_anything_v2_vits.pth` or that converted to
    /// safetensors ([`convert`]); the backbone under `pretrained.`, the head under
    /// `depth_head.`.
    pub fn load(weights: &Path) -> candle_core::Result<Self> {
        let tensors = read_weights(weights)?;
        Self::with(VarBuilder::from_tensors(
            completed(tensors)?,
            DType::F32,
            &Device::Cpu,
        ))
    }

    /// The network with every weight zero: for checking the plumbing without the download.
    pub fn untrained() -> candle_core::Result<Self> {
        Self::with(VarBuilder::zeros(DType::F32, &Device::Cpu))
    }

    fn with(vb: VarBuilder) -> candle_core::Result<Self> {
        let backbone = Arc::new(dinov2::vit_small(vb.pp("pretrained"))?);
        let model = DepthAnythingV2::new(backbone, DepthAnythingV2Config::vit_small(), vb)?;
        Ok(Self { model })
    }

    /// Relative depth of the photo (`rgb`, three bytes a pixel), bigger nearer, at its own
    /// size, rows from the top.
    pub fn estimate(&self, rgb: &[u8], columns: u32, rows: u32) -> candle_core::Result<Vec<f32>> {
        let input = Tensor::from_vec(
            network_input(rgb, columns, rows),
            (1, 3, SIDE, SIDE),
            &Device::Cpu,
        )?;
        let depth = self
            .model
            .forward(&input)?
            .flatten_all()?
            .to_vec1::<f32>()?;
        Ok(resample(
            &depth,
            (SIDE, SIDE),
            (columns as usize, rows as usize),
        ))
    }
}

/// The weights in `path`: safetensors, or a `PyTorch` `.pth` (read as data; candle's
/// reader runs none of the file's code).
fn read_weights(path: &Path) -> candle_core::Result<HashMap<String, Tensor>> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pth"))
    {
        Ok(candle_core::pickle::read_all(path)?.into_iter().collect())
    } else {
        // read whole (about 100 MB) rather than mapped, which would need unsafe code
        candle_core::safetensors::load_buffer(&std::fs::read(path)?, &Device::Cpu)
    }
}

/// The official `.pth` converted to safetensors, the form the add-on ships in.
pub fn convert(pth: &Path, out: &Path) -> candle_core::Result<()> {
    candle_core::safetensors::save(&read_weights(pth)?, out)
}

/// The weights with what candle's backbone asks for but Depth Anything never uses: the
/// backbone's classifier (`pretrained.head`), absent from the official file, as zeros.
fn completed(mut tensors: HashMap<String, Tensor>) -> candle_core::Result<HashMap<String, Tensor>> {
    const CLASSES: usize = 1000;
    const EMBED: usize = 384;
    if !tensors.contains_key("pretrained.head.weight") {
        tensors.insert(
            "pretrained.head.weight".into(),
            Tensor::zeros((CLASSES, 2 * EMBED), DType::F32, &Device::Cpu)?,
        );
        tensors.insert(
            "pretrained.head.bias".into(),
            Tensor::zeros(CLASSES, DType::F32, &Device::Cpu)?,
        );
    }
    tensors
        .into_iter()
        .map(|(name, t)| Ok((name, t.to_dtype(DType::F32)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_input_is_a_normalised_square_red_plane_first() {
        // one white pixel: every input pixel white
        let input = network_input(&[255, 255, 255], 1, 1);
        assert_eq!(input.len(), 3 * SIDE * SIDE);
        let white_red = (1.0 - MEAN[0]) / SPREAD[0];
        let white_blue = (1.0 - MEAN[2]) / SPREAD[2];
        assert!((input[0] - white_red).abs() < 1e-5);
        assert!((input[2 * SIDE * SIDE] - white_blue).abs() < 1e-5);
    }

    #[test]
    fn resampling_keeps_the_ends_and_blends_between() {
        let row = [0.0, 1.0];
        assert_eq!(resample(&row, (2, 1), (2, 1)), row);
        let wide = resample(&row, (2, 1), (4, 1));
        assert_eq!(wide.len(), 4);
        assert!(wide[0].abs() < 1e-6 && (wide[3] - 1.0).abs() < 1e-6);
        assert!((wide[1] - 0.25).abs() < 1e-6 && (wide[2] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn grey16_spreads_values_over_the_whole_range() {
        let grey = to_grey16(&[2.0, 3.0, 4.0], 3, 1);
        assert_eq!(grey.into_raw(), vec![0, 32_768, 65_535]);
    }

    #[test]
    fn the_unused_classifier_is_filled_in_and_everything_made_f32() {
        let half = Tensor::zeros(4, DType::F16, &Device::Cpu).unwrap();
        let tensors = completed(HashMap::from([("depth_head.x".to_string(), half)])).unwrap();
        assert_eq!(tensors["pretrained.head.weight"].dims(), [1000, 768]);
        assert_eq!(tensors["pretrained.head.bias"].dims(), [1000]);
        assert_eq!(tensors["depth_head.x"].dtype(), DType::F32);
    }

    #[test]
    fn converted_weights_read_back_the_same() {
        let dir = std::env::temp_dir().join(format!("cutcraft-depth-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("w.safetensors");
        let t = Tensor::new(&[1.0_f32, 2.0, 3.0], &Device::Cpu).unwrap();
        candle_core::safetensors::save(&HashMap::from([("a".to_string(), t)]), &file).unwrap();
        let back = read_weights(&file).unwrap();
        assert_eq!(back["a"].to_vec1::<f32>().unwrap(), [1.0, 2.0, 3.0]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_untrained_network_runs_and_answers_at_the_photo_size() {
        let estimator = Estimator::untrained().unwrap();
        let depth = estimator.estimate(&[128; 3 * 5 * 3], 5, 3).unwrap();
        assert_eq!(depth.len(), 15);
        assert!(depth.iter().all(|d| d.is_finite()));
    }
}

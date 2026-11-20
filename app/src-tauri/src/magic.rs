//! Magic layers' models (port of `studio/magic/{sam,ocr}.ts`): MobileSAM
//! (Segment Anything, through candle) finds the objects, ocrs reads the text.
//! Both run here, natively; the studio does the pixel work with what comes back.
//!
//! The models are downloaded on first use into the app's data folder (about
//! 52 MB) and kept there, so nothing is downloaded on computers that never
//! use Magic layers.

use crate::files::err;
use base64::Engine;
use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::segment_anything::sam::{IMAGE_SIZE, Sam};
use lattice_core::api::ApiError;
use lattice_core::canvas::magic::{LineBox, LowMask, PickOpts, pick_objects, share_inside};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

const MODELS: [(&str, &str); 3] = [
    ("mobile_sam-tiny-vitt.safetensors", "https://huggingface.co/lmz/candle-sam/resolve/main/mobile_sam-tiny-vitt.safetensors"),
    ("text-detection.rten", "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten"),
    ("text-recognition.rten", "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten"),
];

/// Prompt points per side: 16×16 finds most separate things on a page.
const GRID: usize = 16;

#[derive(Clone, Serialize)]
struct Step {
    label: String,
    progress: Option<f64>,
}

fn step(app: &AppHandle, label: &str, progress: Option<f64>) {
    let _ = app.emit("magic-progress", Step { label: label.into(), progress });
}

fn models_dir(app: &AppHandle) -> Result<PathBuf, ApiError> {
    let dir = app.path().app_data_dir().map_err(err)?.join("models");
    std::fs::create_dir_all(&dir).map_err(err)?;
    Ok(dir)
}

/// Downloads whichever models are missing, with progress. A partial file is
/// written beside the real one and renamed only when complete.
async fn ensure_models(app: &AppHandle) -> Result<PathBuf, ApiError> {
    let dir = models_dir(app)?;
    let client = reqwest::Client::builder().user_agent("lattice-lane").build().map_err(err)?;
    for (name, url) in MODELS {
        let path = dir.join(name);
        if path.exists() {
            continue;
        }
        let label = format!("Downloading {name} (first time only)");
        step(app, &label, Some(0.0));
        let mut last_err = None;
        for attempt in 1..=3u64 {
            match download(&client, url, &path, |p| step(app, &label, p)).await {
                Ok(()) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    tokio::time::sleep(std::time::Duration::from_millis(attempt * 1500)).await;
                }
            }
        }
        if let Some(e) = last_err {
            return Err(ApiError::Offline(format!("Could not download {name}: {e}")));
        }
    }
    Ok(dir)
}

async fn download(client: &reqwest::Client, url: &str, path: &Path, progress: impl Fn(Option<f64>)) -> Result<(), String> {
    let mut resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let total = resp.content_length();
    let tmp = path.with_extension("part");
    let mut bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        bytes.extend_from_slice(&chunk);
        progress(total.map(|t| bytes.len() as f64 / t as f64));
    }
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct OcrLine {
    text: String,
    #[serde(rename = "box")]
    line: LineBox,
    row_height: f64,
}

#[derive(Serialize)]
pub struct Mask {
    w: usize,
    h: usize,
    score: f32,
    /// Logits ×4, clamped to i8, base64: small enough to send, precise enough for soft edges.
    logits_q: String,
}

#[derive(Serialize)]
pub struct Analysis {
    lines: Vec<OcrLine>,
    masks: Vec<Mask>,
    /// Mask cells per source pixel.
    scale: f64,
}

/// Lines worth turning into text layers. ocrs gives no confidence, so lines
/// that are mostly not letters or digits (texture read as letters on photos)
/// are left alone; they stay part of the background, where they're harmless.
fn looks_like_text(s: &str) -> bool {
    let t = s.trim();
    let alnum = t.chars().filter(|c| c.is_alphanumeric()).count();
    let total = t.chars().filter(|c| !c.is_whitespace()).count();
    alnum >= 2 && total > 0 && alnum as f64 / total as f64 >= 0.6
}

fn read_lines(dir: &Path, rgb: &image::RgbImage) -> Result<Vec<OcrLine>, ApiError> {
    let load = |n: &str| rten::Model::load_file(dir.join(n)).map_err(|e| ApiError::Server(format!("Could not open the OCR model: {e}")));
    let engine = OcrEngine::new(OcrEngineParams { detection_model: Some(load("text-detection.rten")?), recognition_model: Some(load("text-recognition.rten")?), ..Default::default() })
        .map_err(|e| ApiError::Server(e.to_string()))?;
    let src = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions()).map_err(|e| ApiError::Server(e.to_string()))?;
    let input = engine.prepare_input(src).map_err(|e| ApiError::Server(e.to_string()))?;
    let words = engine.detect_words(&input).map_err(|e| ApiError::Server(e.to_string()))?;
    let lines = engine.find_text_lines(&input, &words);
    let texts = engine.recognize_text(&input, &lines).map_err(|e| ApiError::Server(e.to_string()))?;
    let (w, h) = (rgb.width() as i32, rgb.height() as i32);
    Ok(texts
        .into_iter()
        .flatten()
        .filter(|l| looks_like_text(&l.to_string()))
        .map(|l| {
            let r = l.bounding_rect();
            let (x0, y0) = (r.left().clamp(0, w), r.top().clamp(0, h));
            let (x1, y1) = (r.right().clamp(x0, w), r.bottom().clamp(y0, h));
            OcrLine {
                text: l.to_string().trim().to_string(),
                line: LineBox { x0: x0 as usize, y0: y0 as usize, x1: x1 as usize, y1: y1 as usize },
                row_height: f64::from(y1 - y0),
            }
        })
        .collect())
}

fn find_objects(app: &AppHandle, dir: &Path, rgb: &image::RgbImage) -> Result<(Vec<LowMask>, f64, usize, usize), ApiError> {
    let e = |x: candle_core::Error| ApiError::Server(format!("Finding objects failed: {x}"));
    let device = Device::Cpu;
    let weights = std::fs::read(dir.join("mobile_sam-tiny-vitt.safetensors")).map_err(err)?;
    let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device).map_err(e)?;
    let sam = Sam::new_tiny(vb).map_err(e)?;
    // The model sees the image with its long side at 1024.
    let (w, h) = rgb.dimensions();
    let k = IMAGE_SIZE as f64 / f64::from(w.max(h));
    let (rw, rh) = (((f64::from(w) * k).round() as u32).max(1), ((f64::from(h) * k).round() as u32).max(1));
    let small = image::imageops::resize(rgb, rw, rh, image::imageops::FilterType::Triangle);
    let img = Tensor::from_vec(small.into_raw(), (rh as usize, rw as usize, 3), &device).map_err(e)?.permute((2, 0, 1)).map_err(e)?;
    let embeddings = sam.embeddings(&img).map_err(e)?;
    // Masks come back at a quarter of 1024; only the top-left rw/4 × rh/4 is image.
    let (mw, mh) = ((rw as usize).div_ceil(4), (rh as usize).div_ceil(4));
    let mut masks = Vec::new();
    for j in 0..GRID {
        for i in 0..GRID {
            let point = ((i as f64 + 0.5) / GRID as f64, (j as f64 + 0.5) / GRID as f64, true);
            let (low, iou) = sam.forward_for_embeddings(&embeddings, rh as usize, rw as usize, &[point], true).map_err(e)?;
            let scores: Vec<f32> = iou.flatten_all().map_err(e)?.to_vec1().map_err(e)?;
            // SAM offers three readings per point (part, object, whole); keep the one it rates best.
            let (best, score) = scores.iter().copied().enumerate().fold((0, f32::MIN), |acc, (c, s)| if s > acc.1 { (c, s) } else { acc });
            let m = low.i((0, best)).map_err(e)?.i((..mh, ..mw)).map_err(e)?;
            let logits: Vec<f32> = m.flatten_all().map_err(e)?.to_vec1().map_err(e)?;
            masks.push(LowMask { logits, w: mw, h: mh, score });
        }
        step(app, "Finding objects", Some((j + 1) as f64 / GRID as f64));
    }
    Ok((masks, f64::from(rw) / 4.0 / f64::from(w), mw, mh))
}

/// Text lines and object masks in an image (the raw body: PNG or JPEG bytes).
#[tauri::command]
pub async fn magic_analyze(app: AppHandle, request: tauri::ipc::Request<'_>) -> Result<Analysis, ApiError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Invalid("The image must be sent as raw bytes.".into()));
    };
    let bytes = bytes.clone();
    let dir = ensure_models(&app).await?;
    step(&app, "Reading text", None);
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Analysis, ApiError> {
        let rgb = image::load_from_memory(&bytes).map_err(|_| ApiError::Invalid("That image could not be read.".into()))?.to_rgb8();
        let lines = read_lines(&dir, &rgb)?;
        step(&app2, "Finding objects", Some(0.0));
        let (masks, scale, _, _) = find_objects(&app2, &dir, &rgb)?;
        let cells: Vec<(f64, f64, f64, f64)> = lines
            .iter()
            .map(|l| (l.line.x0 as f64 * scale, l.line.y0 as f64 * scale, (l.line.x1 - l.line.x0) as f64 * scale, (l.line.y1 - l.line.y0) as f64 * scale))
            .collect();
        let kept: Vec<Mask> = pick_objects(&masks, PickOpts::default())
            .into_iter()
            .filter(|m| share_inside(m, &cells) < 0.5)
            .map(|m| {
                let q: Vec<u8> = m.logits.iter().map(|v| ((v * 4.0).round().clamp(-127.0, 127.0) as i8) as u8).collect();
                Mask { w: m.w, h: m.h, score: m.score, logits_q: base64::engine::general_purpose::STANDARD.encode(q) }
            })
            .collect();
        Ok(Analysis { lines, masks: kept, scale })
    })
    .await
    .map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_filter() {
        assert!(looks_like_text("Happy Diwali"));
        assert!(looks_like_text("₹ 1,250"));
        assert!(!looks_like_text("~ ."));
        assert!(!looks_like_text("|/\\;:"));
        assert!(!looks_like_text("a"));
    }
}

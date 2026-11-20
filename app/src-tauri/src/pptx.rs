//! PowerPoint export (port of `export-pptx.ts`, which used pptxgenjs): each
//! slide's shapes, photos and background become one full-bleed picture, and
//! its text layers become real text boxes on top, so wording stays editable.
//!
//! The UI draws each slide and sends it here with `pptx_add` (raw body: a
//! 4-byte big-endian JSON length, the JSON text boxes, then the PNG), then
//! `pptx_save` writes the file where the user chooses.

use crate::files::{ask_where, err};
use lattice_core::api::ApiError;
use lattice_core::canvas::Align;
use lattice_core::canvas::deck::{PX_PER_INCH, PptTextBox};
use std::io::Write;
use std::sync::Mutex;
use tauri::AppHandle;
use zip::write::SimpleFileOptions;

struct Slide {
    png: Vec<u8>,
    texts: Vec<(String, PptTextBox)>,
}

struct Deck {
    width_px: f64,
    height_px: f64,
    slides: Vec<Slide>,
}

static DECK: Mutex<Option<Deck>> = Mutex::new(None);

const EMU_PER_INCH: f64 = 914_400.0;

fn emu(inches: f64) -> i64 {
    (inches * EMU_PER_INCH).round() as i64
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[tauri::command(rename_all = "snake_case")]
pub fn pptx_begin(width_px: f64, height_px: f64) -> Result<(), ApiError> {
    *DECK.lock().map_err(err)? = Some(Deck { width_px, height_px, slides: Vec::new() });
    Ok(())
}

#[tauri::command]
pub fn pptx_add(request: tauri::ipc::Request<'_>) -> Result<(), ApiError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(ApiError::Invalid("The slide must be sent as raw bytes.".into()));
    };
    let len = bytes.get(..4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or_else(|| ApiError::Invalid("Empty slide.".into()))?;
    let json = bytes.get(4..4 + len).ok_or_else(|| ApiError::Invalid("Bad slide.".into()))?;
    let texts: Vec<(String, PptTextBox)> = serde_json::from_slice(json).map_err(|e| ApiError::Invalid(format!("Bad slide text: {e}")))?;
    let png = bytes.get(4 + len..).unwrap_or_default().to_vec();
    let mut deck = DECK.lock().map_err(err)?;
    let deck = deck.as_mut().ok_or_else(|| ApiError::Invalid("Start the export first.".into()))?;
    deck.slides.push(Slide { png, texts });
    Ok(())
}

/// Asks where, then writes the deck. Ok(None) if the dialog was cancelled.
#[tauri::command(rename_all = "snake_case")]
pub async fn pptx_save(app: AppHandle, file_name: String) -> Result<Option<String>, ApiError> {
    let deck = DECK.lock().map_err(err)?.take().ok_or_else(|| ApiError::Invalid("Nothing to export.".into()))?;
    let bytes = build(&deck)?;
    let name = if file_name.ends_with(".pptx") { file_name } else { format!("{file_name}.pptx") };
    let Some(path) = ask_where(&app, &name, "PowerPoint", "pptx").await? else { return Ok(None) };
    std::fs::write(&path, bytes).map_err(err)?;
    Ok(Some(path.display().to_string()))
}

const NS: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CT: &str = "application/vnd.openxmlformats-officedocument";

fn rels(items: &[(String, &str, String)]) -> String {
    let body: String = items.iter().map(|(id, kind, target)| format!(r#"<Relationship Id="{id}" Type="{REL}/{kind}" Target="{target}"/>"#)).collect();
    format!("{XML}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">{body}</Relationships>")
}

fn text_shape(id: usize, text: &str, b: &PptTextBox) -> String {
    let algn = match b.align {
        Align::Left => "l",
        Align::Center => "ctr",
        Align::Right => "r",
    };
    let alpha = ((100.0 - b.transparency) * 1000.0).round() as i64;
    let run_props = format!(
        r#"<a:rPr lang="en-US" sz="{}" b="{}" i="{}"{} spc="{}" dirty="0"><a:solidFill><a:srgbClr val="{}"><a:alpha val="{alpha}"/></a:srgbClr></a:solidFill><a:latin typeface="{}"/><a:cs typeface="{}"/></a:rPr>"#,
        (b.font_size * 100.0).round() as i64,
        u8::from(b.bold),
        u8::from(b.italic),
        if b.underline { r#" u="sng""# } else { "" },
        (b.char_spacing * 100.0).round() as i64,
        esc(&b.color),
        esc(&b.font_face),
        esc(&b.font_face),
    );
    let paragraphs: String = text
        .split('\n')
        .map(|line| {
            let para = format!(r#"<a:pPr algn="{algn}"><a:lnSpc><a:spcPct val="{}"/></a:lnSpc></a:pPr>"#, (b.line_spacing_multiple * 100_000.0).round() as i64);
            if line.is_empty() {
                format!("<a:p>{para}<a:endParaRPr lang=\"en-US\"/></a:p>")
            } else {
                format!("<a:p>{para}<a:r>{run_props}<a:t>{}</a:t></a:r></a:p>", esc(line))
            }
        })
        .collect();
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Text {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm rot="{}"><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square" lIns="0" tIns="0" rIns="0" bIns="0" rtlCol="0" anchor="t"><a:noAutofit/></a:bodyPr><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"#,
        (b.rotate * 60_000.0).round() as i64,
        emu(b.x),
        emu(b.y),
        emu(b.w).max(1),
        emu(b.h).max(1),
    )
}

fn slide_xml(s: &Slide, cx: i64, cy: i64) -> String {
    let texts: String = s.texts.iter().enumerate().filter(|(_, (t, _))| !t.trim().is_empty()).map(|(i, (t, b))| text_shape(i + 3, t, b)).collect();
    format!(
        r#"{XML}<p:sld {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr><p:pic><p:nvPicPr><p:cNvPr id="2" name="Slide picture"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>{texts}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
    )
}

const THEME: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Lattice Lane"><a:themeElements><a:clrScheme name="Lattice Lane"><a:dk1><a:srgbClr val="1F1F1F"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="2C332F"/></a:dk2><a:lt2><a:srgbClr val="FAF8EE"/></a:lt2><a:accent1><a:srgbClr val="54655B"/></a:accent1><a:accent2><a:srgbClr val="B5C2B1"/></a:accent2><a:accent3><a:srgbClr val="DDCF8B"/></a:accent3><a:accent4><a:srgbClr val="B08D57"/></a:accent4><a:accent5><a:srgbClr val="595959"/></a:accent5><a:accent6><a:srgbClr val="7F7F7F"/></a:accent6><a:hlink><a:srgbClr val="54655B"/></a:hlink><a:folHlink><a:srgbClr val="2C332F"/></a:folHlink></a:clrScheme><a:fontScheme name="Lattice Lane"><a:majorFont><a:latin typeface="Arial"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Arial"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Lattice Lane"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

const EMPTY_TREE: &str = r#"<p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree>"#;

/// The .pptx bytes.
fn build(deck: &Deck) -> Result<Vec<u8>, ApiError> {
    let (cx, cy) = (emu(deck.width_px / PX_PER_INCH), emu(deck.height_px / PX_PER_INCH));
    let n = deck.slides.len();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut put = |name: &str, body: String| files.push((name.to_string(), body.into_bytes()));

    let slide_overrides: String = (1..=n).map(|i| format!(r#"<Override PartName="/ppt/slides/slide{i}.xml" ContentType="{CT}.presentationml.slide+xml"/>"#)).collect();
    put(
        "[Content_Types].xml",
        format!(
            r#"{XML}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="png" ContentType="image/png"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="{CT}.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="{CT}.presentationml.slideMaster+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="{CT}.presentationml.slideLayout+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="{CT}.theme+xml"/><Override PartName="/ppt/presProps.xml" ContentType="{CT}.presentationml.presProps+xml"/><Override PartName="/ppt/viewProps.xml" ContentType="{CT}.presentationml.viewProps+xml"/><Override PartName="/ppt/tableStyles.xml" ContentType="{CT}.presentationml.tableStyles+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="{CT}.extended-properties+xml"/>{slide_overrides}</Types>"#
        ),
    );
    put(
        "_rels/.rels",
        rels(&[
            ("rId1".into(), "officeDocument", "ppt/presentation.xml".into()),
            ("rId2".into(), "../package/2006/relationships/metadata/core-properties", "docProps/core.xml".into()),
            ("rId3".into(), "extended-properties", "docProps/app.xml".into()),
        ])
        .replace(&format!("{REL}/../package/2006/relationships/metadata/core-properties"), "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties"),
    );
    put(
        "docProps/core.xml",
        format!(r#"{XML}<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><dc:title>Lattice Lane presentation</dc:title><dc:creator>Lattice Lane</dc:creator></cp:coreProperties>"#),
    );
    put(
        "docProps/app.xml",
        format!(r#"{XML}<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Lattice Lane</Application><Slides>{n}</Slides></Properties>"#),
    );
    let slide_ids: String = (1..=n).map(|i| format!(r#"<p:sldId id="{}" r:id="rId{}"/>"#, 255 + i, i + 1)).collect();
    put(
        "ppt/presentation.xml",
        format!(
            r#"{XML}<p:presentation {NS} saveSubsetFonts="1"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx="{cx}" cy="{cy}"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#
        ),
    );
    let mut pres_rels: Vec<(String, &str, String)> = vec![("rId1".into(), "slideMaster", "slideMasters/slideMaster1.xml".into())];
    pres_rels.extend((1..=n).map(|i| (format!("rId{}", i + 1), "slide", format!("slides/slide{i}.xml"))));
    pres_rels.push((format!("rId{}", n + 2), "theme", "theme/theme1.xml".into()));
    pres_rels.push((format!("rId{}", n + 3), "presProps", "presProps.xml".into()));
    pres_rels.push((format!("rId{}", n + 4), "viewProps", "viewProps.xml".into()));
    pres_rels.push((format!("rId{}", n + 5), "tableStyles", "tableStyles.xml".into()));
    put("ppt/_rels/presentation.xml.rels", rels(&pres_rels));
    put("ppt/presProps.xml", format!("{XML}<p:presentationPr {NS}/>"));
    put("ppt/viewProps.xml", format!("{XML}<p:viewPr {NS}/>"));
    put(
        "ppt/tableStyles.xml",
        format!(r#"{XML}<a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}"/>"#),
    );
    put("ppt/theme/theme1.xml", format!("{XML}{THEME}"));
    put(
        "ppt/slideMasters/slideMaster1.xml",
        format!(
            r#"{XML}<p:sldMaster {NS}><p:cSld>{EMPTY_TREE}</p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>"#
        ),
    );
    put(
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        rels(&[("rId1".into(), "slideLayout", "../slideLayouts/slideLayout1.xml".into()), ("rId2".into(), "theme", "../theme/theme1.xml".into())]),
    );
    put(
        "ppt/slideLayouts/slideLayout1.xml",
        format!(r#"{XML}<p:sldLayout {NS} type="blank" preserve="1"><p:cSld name="Blank">{EMPTY_TREE}</p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#),
    );
    put("ppt/slideLayouts/_rels/slideLayout1.xml.rels", rels(&[("rId1".into(), "slideMaster", "../slideMasters/slideMaster1.xml".into())]));
    for (i, s) in deck.slides.iter().enumerate() {
        let k = i + 1;
        put(&format!("ppt/slides/slide{k}.xml"), slide_xml(s, cx, cy));
        put(
            &format!("ppt/slides/_rels/slide{k}.xml.rels"),
            rels(&[("rId1".into(), "slideLayout", "../slideLayouts/slideLayout1.xml".into()), ("rId2".into(), "image", format!("../media/image{k}.png"))]),
        );
    }
    for (i, s) in deck.slides.iter().enumerate() {
        files.push((format!("ppt/media/image{}.png", i + 1), s.png.clone()));
    }

    let mut out = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in files {
        zip.start_file(name, opts).map_err(err)?;
        zip.write_all(&body).map_err(err)?;
    }
    zip.finish().map_err(err)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn writes_a_well_formed_package() -> Result<(), ApiError> {
        let tb = PptTextBox {
            x: 1.0,
            y: 2.0,
            w: 2.0,
            h: 1.0,
            rotate: 0.0,
            font_face: "Inter".into(),
            font_size: 24.0,
            bold: true,
            italic: false,
            underline: false,
            color: "54655B".into(),
            align: Align::Center,
            char_spacing: 2.0,
            line_spacing_multiple: 1.2,
            transparency: 0.0,
        };
        let deck = Deck { width_px: 1920.0, height_px: 1080.0, slides: vec![Slide { png: vec![137, 80, 78, 71], texts: vec![("Price : INR 3,250 & tax".into(), tb)] }] };
        let bytes = build(&deck)?;
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(err)?;
        let mut pres = String::new();
        zip.by_name("ppt/presentation.xml").map_err(err)?.read_to_string(&mut pres).map_err(err)?;
        assert!(pres.contains(r#"<p:sldSz cx="12192000" cy="6858000"/>"#), "1920×1080 px is 13.33″ × 7.5″");
        let mut slide = String::new();
        zip.by_name("ppt/slides/slide1.xml").map_err(err)?.read_to_string(&mut slide).map_err(err)?;
        assert!(slide.contains("INR 3,250 &amp; tax") && slide.contains(r#"sz="2400""#) && slide.contains(r#"algn="ctr""#));
        assert!(zip.by_name("ppt/media/image1.png").is_ok());
        Ok(())
    }
}

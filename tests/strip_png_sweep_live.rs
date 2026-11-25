//! SPEC-039 AC9, DEC-023 rule 6: the completeness sweep of the PNG chunk types,
//! and the sweep of ExifTool's `PNG::TextualData` keywords.
//!
//! The PNG strip decides what to remove from lists the builders typed
//! (`png::metadata::chunk_class`, the text keyword table), and a list typed by
//! the people who wrote the strip can only be complete against itself. This
//! file takes the candidates from ExifTool instead.
//!
//! **Type sweep.** Candidates: every four-character tag id of
//! `exiftool -listx`, the pinned subdirectory list ([`PINNED_SUBDIRECTORY_TYPES`]),
//! and all 16 ASCII-case spellings of every code the writer names. Each
//! candidate is one CRC-correct chunk between `IHDR` and `IDAT` of a minimal
//! PNG, in ten payload shapes ([`SHAPES`]). ExifTool runs over the files in two
//! batches (`-a -u -G1 -s`). A file FAILS when it prints a tag that the bare
//! file does not (other than an `Unknown_*` dump or a tag of the `File`,
//! `System` or `ExifTool` groups), and its code is neither in the writer's set
//! ([`known_codes`]: the codes `chunk_class` and the text, `iCCP`, `tIME` and
//! `eXIf` rules name, and every case of `eXIf` and `zxIf`) nor in the pinned
//! structure list ([`STRUCTURE`]) with exactly the tag names pinned there.
//!
//! **Keyword sweep.** Every keyword of the `PNG::TextualData` table, in a
//! `tEXt`, a `zTXt` and an `iTXt` chunk. The tag ExifTool prints, mapped through
//! the judge (`i3_judge::category_of`), must be the category
//! `text_keyword_category` gives the keyword.
//!
//! **The stated residual (DEC-023 rule 6).** `-listx` does not print the types
//! ExifTool decodes through a subdirectory (`tEXt`, `iCCP`, `pHYs`, ...), so
//! those come from the pinned list. A PNG chunk type that ExifTool decodes only
//! through a subdirectory, and that is in neither `-listx` nor the pinned list,
//! is not found. That gap is accepted and named here; it is not closed by
//! reading ExifTool's source (it is run, never read).
//!
//! Live only, never replayed: the answer is about the installed ExifTool.
//! Without it each test prints `SKIP` and passes, unless
//! `DIAMOND_EXIF_REQUIRE_ORACLE=1`.

#[path = "support/i3_judge.rs"]
mod i3_judge;
#[path = "support/i3_records.rs"]
mod i3_records;
#[path = "support/i3_shapes.rs"]
mod i3_shapes;
#[path = "support/oracle_png.rs"]
mod oracle_png;
#[path = "support/oracle_tiff.rs"]
mod oracle_tiff;
#[path = "support/oracle_webp.rs"]
mod oracle_webp;
#[path = "support/sweep.rs"]
mod sweep;
#[path = "support/tools.rs"]
mod tools;
#[path = "support/webp_carriers.rs"]
mod webp_carriers;
#[path = "support/webp_shapes.rs"]
mod webp_shapes;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use std::time::Duration;

use exif_core::audit::{text_keyword_category, Category};
use i3_judge::Cat;
use sweep::{exiftool_listx, listx_ids, live_gate, new_tags, parse_listing, unexplained, Scratch};

/// The PNG chunk types ExifTool decodes through a subdirectory, which `-listx`
/// does not print (DEC-023 rule 6; measured at design, 2026-11-24).
const PINNED_SUBDIRECTORY_TYPES: [&[u8; 4]; 26] = [
    b"IHDR", b"IDAT", b"IEND", b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"zxIf", b"iCCP", b"caBX",
    b"tXMP", b"meTa", b"seAl", b"cpIp", b"pHYs", b"cHRM", b"sCAL", b"sTER", b"vpAg", b"acTL",
    b"cICP", b"fcTL", b"fdAT", b"mDCV", b"cLLI", b"orNT",
];

/// Every code the writer names, typed by hand: the `chunk_class` codes, the
/// three text chunks, `iCCP`, `tIME`, `eXIf` and `zxIf`. The sweep also tries
/// all 16 case spellings of each.
const WRITER_CODES: [&[u8; 4]; 17] = [
    b"tEXt", b"zTXt", b"iTXt", b"iCCP", b"tIME", b"tXMP", b"meTa", b"seAl", b"caBX", b"dSIG",
    b"cpIp", b"gIFx", b"gIFg", b"gIFt", b"fRAc", b"eXIf", b"zxIf",
];

/// Chunk codes the strip never removes because they are image structure, and
/// the tag names ExifTool prints for them (pinned; a name outside the list is a
/// failure). DEC-020 clarification 4 plus `PLTE` (clarification 4 of the
/// 2026-11-24 amendment). Measured on ExifTool 13.55 over the ten shapes.
const STRUCTURE: [([u8; 4], &[&str]); 19] = [
    (*b"PLTE", &["Palette"]),
    (*b"acTL", &["AnimationFrames", "AnimationPlays"]),
    (*b"bKGD", &["BackgroundColor"]),
    (
        *b"cHRM",
        &[
            "BlueX",
            "BlueY",
            "GreenX",
            "GreenY",
            "RedX",
            "RedY",
            "WhitePointX",
            "WhitePointY",
        ],
    ),
    (
        *b"cICP",
        &[
            "ColorPrimaries",
            "MatrixCoefficients",
            "TransferCharacteristics",
            "VideoFullRangeFlag",
        ],
    ),
    (*b"gAMA", &["Gamma"]),
    (*b"gdAT", &["GainMapImage"]),
    (*b"hIST", &["PaletteHistogram"]),
    (*b"iDOT", &["AppleDataOffsets"]),
    (*b"oFFs", &["ImageOffset"]),
    (*b"pCAL", &["PixelCalibration"]),
    (
        *b"pHYs",
        &["PixelUnits", "PixelsPerUnitX", "PixelsPerUnitY"],
    ),
    (*b"sBIT", &["SignificantBits"]),
    (
        *b"sCAL",
        &["SubjectPixelHeight", "SubjectPixelWidth", "SubjectUnits"],
    ),
    (*b"sPLT", &["SuggestedPalette"]),
    (*b"sRGB", &["SRGBRendering"]),
    (*b"sTER", &["StereoMode"]),
    (*b"tRNS", &["Transparency"]),
    (
        *b"vpAg",
        &[
            "VirtualImageHeight",
            "VirtualImageWidth",
            "VirtualPageUnits",
        ],
    ),
];

/// The ten payload shapes of one candidate chunk.
const SHAPES: [&str; 10] = [
    "text",
    "tiff",
    "xmp",
    "zeros",
    "zlib-tiff",
    "exif-prefix",
    "keyword",
    "ztxt",
    "itxt",
    "jumbf",
];

/// The writer's codes that must print under at least one shape (so the sweep
/// proves something for them): all 17 did, on ExifTool 13.55.
const MUST_PRINT: [&[u8; 4]; 17] = WRITER_CODES;

fn spellings(base: [u8; 4]) -> Vec<[u8; 4]> {
    (0..16u8)
        .map(|mask| {
            let mut ty = base;
            for (i, b) in ty.iter_mut().enumerate() {
                if (mask >> i) & 1 == 1 {
                    *b ^= 0x20;
                }
            }
            ty
        })
        .collect()
}

/// The writer's set: its exact codes, and every case of `eXIf` and `zxIf`
/// (DEC-020 clarification 5: they decode in every case).
fn known_codes() -> BTreeSet<[u8; 4]> {
    let mut known: BTreeSet<[u8; 4]> = WRITER_CODES.iter().map(|c| **c).collect();
    known.extend(spellings(*b"eXIf"));
    known.extend(spellings(*b"zxIf"));
    known
}

fn candidates(listx: &str) -> BTreeSet<[u8; 4]> {
    let mut all = listx_ids(listx);
    all.extend(PINNED_SUBDIRECTORY_TYPES.iter().map(|c| **c));
    for code in WRITER_CODES {
        all.extend(spellings(*code));
    }
    all
}

/// A TIFF with one `Artist` entry (`Jane Doe`).
fn tiff_artist() -> Vec<u8> {
    let value = b"Jane Doe\0";
    let mut v = b"II*\0".to_vec();
    v.extend_from_slice(&8u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&0x013Bu16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&(value.len() as u32).to_le_bytes());
    v.extend_from_slice(&26u32.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(value);
    v
}

fn shape_bytes(shape: &str) -> Vec<u8> {
    match shape {
        "text" => webp_carriers::payload(webp_carriers::Payload::Text),
        "tiff" => tiff_artist(),
        "xmp" => webp_carriers::payload(webp_carriers::Payload::Xmp),
        "zeros" => webp_carriers::payload(webp_carriers::Payload::Zeros),
        "zlib-tiff" => i3_shapes::zlib_wrapped(&tiff_artist()),
        "exif-prefix" => [b"Exif\0\0".to_vec(), tiff_artist()].concat(),
        "keyword" => b"Artist\0Jane Doe".to_vec(),
        // keyword, NUL, compression method 0, zlib stream
        "ztxt" => [b"Artist\0\0".to_vec(), oracle_png::zlib_stored(b"Jane Doe")].concat(),
        // keyword, NUL, flag 0, method 0, language NUL, translated NUL, text
        "itxt" => b"Artist\0\0\0\0\0Jane Doe".to_vec(),
        "jumbf" => webp_carriers::payload(webp_carriers::Payload::Jumbf),
        other => panic!("unknown shape {other}"),
    }
}

/// One chunk between `IHDR` and `IDAT` in a minimal PNG.
fn file_of(code: &[u8; 4], shape: &str) -> Vec<u8> {
    i3_shapes::with(&[oracle_png::chunk(code, &shape_bytes(shape))])
}

fn exiftool_over(dir: &std::path::Path, limit: Duration) -> BTreeMap<String, BTreeSet<String>> {
    let args: Vec<String> = ["-a", "-u", "-G1", "-s"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let out = tools::run_tool_with_limit("exiftool", &args, dir, limit)
        .unwrap_or_else(|e| panic!("exiftool over {}: {e:?}", dir.display()));
    parse_listing(&String::from_utf8_lossy(&out.stdout))
}

// ── the type sweep ──────────────────────────────────────────────────────────

/// What the type sweep saw.
struct TypeSweep {
    codes: usize,
    files: usize,
    runs: usize,
    /// Each `(code, shape)` whose file prints tags the bare file does not.
    printed: Vec<([u8; 4], &'static str, Vec<String>)>,
}

/// The sweep, run once per process (the red-proof reuses it).
fn type_sweep() -> &'static TypeSweep {
    static SWEEP: OnceLock<TypeSweep> = OnceLock::new();
    SWEEP.get_or_init(|| {
        let codes: Vec<[u8; 4]> = candidates(&exiftool_listx()).into_iter().collect();
        assert!(
            codes.len() > 2000,
            "only {} candidate codes: did -listx change shape?",
            codes.len()
        );
        let scratch = Scratch::new("png-types");
        // Two ExifTool runs: shapes 0..5 in `a/`, shapes 5..10 in `b/`.
        let dirs = ["a", "b"];
        for d in dirs {
            std::fs::create_dir_all(scratch.0.join(d)).expect("batch dir");
            std::fs::write(scratch.0.join(d).join("bare.png"), i3_shapes::with(&[])).expect("bare");
        }
        let name = |i: usize, s: usize| format!("{i:05}_{s}.png");
        for (i, code) in codes.iter().enumerate() {
            for (s, shape) in SHAPES.iter().enumerate() {
                let dir = scratch.0.join(dirs[s / 5]);
                std::fs::write(dir.join(name(i, s)), file_of(code, shape)).expect("write");
            }
        }
        let mut listing = BTreeMap::new();
        let mut bares = Vec::new();
        for d in dirs {
            let mut one = exiftool_over(&scratch.0.join(d), Duration::from_secs(900));
            let bare = one.remove("bare.png").expect("the bare file");
            assert!(!bare.is_empty(), "the bare file printed nothing");
            bares.push(bare);
            listing.extend(one);
        }
        let mut printed = Vec::new();
        for (i, code) in codes.iter().enumerate() {
            for (s, shape) in SHAPES.iter().enumerate() {
                let file = name(i, s);
                let tags = new_tags(
                    listing
                        .get(&file)
                        .unwrap_or_else(|| panic!("no listing for {file}")),
                    &bares[s / 5],
                );
                if !tags.is_empty() {
                    printed.push((*code, *shape, tags));
                }
            }
        }
        TypeSweep {
            codes: codes.len(),
            files: codes.len() * SHAPES.len(),
            runs: dirs.len(),
            printed,
        }
    })
}

/// The failures of a sweep against a known set.
fn type_failures(sweep: &TypeSweep, known: &BTreeSet<[u8; 4]>) -> Vec<String> {
    sweep
        .printed
        .iter()
        .filter_map(|(code, shape, tags)| {
            unexplained(code, tags, known, &STRUCTURE).map(|why| {
                format!(
                    "{:?} ({shape}): ExifTool prints {why}, and the code is neither the writer's \
                     nor pinned as structure",
                    String::from_utf8_lossy(code)
                )
            })
        })
        .collect()
}

#[test]
fn every_png_code_exiftool_decodes_is_known_or_structure() {
    if !live_gate("every_png_code_exiftool_decodes_is_known_or_structure") {
        return;
    }
    let sweep = type_sweep();
    let known = known_codes();
    let printing: BTreeSet<[u8; 4]> = sweep.printed.iter().map(|p| p.0).collect();
    let failures = type_failures(sweep, &known);
    // The tag names each code outside the writer's set printed, over all shapes
    // (the table `STRUCTURE` pins).
    let mut observed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (code, _, tags) in &sweep.printed {
        if known.contains(code) {
            continue;
        }
        let names = observed
            .entry(String::from_utf8_lossy(code).into_owned())
            .or_default();
        names.extend(
            tags.iter()
                .map(|k| k.split_once(':').map_or(k.as_str(), |(_, n)| n).to_string()),
        );
    }
    println!(
        "png sweep: {} codes x {} shapes = {} files in {} ExifTool runs; {} codes print a tag the bare file does not ({} of them outside the writer's set); {} unexplained",
        sweep.codes,
        SHAPES.len(),
        sweep.files,
        sweep.runs,
        printing.len(),
        observed.len(),
        failures.len()
    );
    println!("png sweep observed outside the writer's set: {observed:?}");
    println!(
        "png sweep writer codes that printed: {:?}",
        printing
            .iter()
            .filter(|c| known.contains(*c))
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
    );
    assert!(failures.is_empty(), "{failures:#?}");
    // Not vacuous: the named codes printed, and the sweep saw a fair number.
    for code in MUST_PRINT {
        assert!(
            printing.contains(code),
            "{:?} printed nothing in any shape, so the sweep proves nothing for it",
            String::from_utf8_lossy(code)
        );
    }
    assert!(printing.len() >= 60, "{} printing codes", printing.len());
}

#[test]
fn the_sweep_goes_red_on_a_code_missing_from_the_known_set() {
    // The judge alone, on a made-up listing (no ExifTool).
    let code = *b"tXMP";
    let tags = vec!["XMP-dc:Creator".to_string()];
    let mut known = known_codes();
    assert!(unexplained(&code, &tags, &known, &STRUCTURE).is_none());
    known.remove(&code);
    assert!(unexplained(&code, &tags, &known, &STRUCTURE).is_some());

    // Against the real ExifTool: with `tXMP` out of the set the sweep fails and
    // names it; with it in, it does not.
    if !live_gate("the_sweep_goes_red_on_a_code_missing_from_the_known_set (live)") {
        return;
    }
    let sweep = type_sweep();
    assert!(type_failures(sweep, &known_codes()).is_empty());
    let failures = type_failures(sweep, &known);
    assert!(
        failures.iter().any(|f| f.contains("\"tXMP\"")),
        "removing tXMP must turn the sweep red: {failures:?}"
    );
    println!(
        "AC9 red (type sweep): without tXMP in the known set: {}",
        failures.first().cloned().unwrap_or_default()
    );
}

// ── the keyword sweep ───────────────────────────────────────────────────────

/// The `(id, name)` of every tag of the `PNG::TextualData` table in an
/// `exiftool -listx` document.
fn textual_data_tags(xml: &str) -> Vec<(String, String)> {
    let Some(start) = xml.find("<table name='PNG::TextualData'") else {
        return Vec::new();
    };
    let table = &xml[start..];
    let table = &table[..table.find("</table>").unwrap_or(table.len())];
    let unescape = |s: &str| {
        s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#39;", "'")
    };
    let mut out = Vec::new();
    let mut rest = table;
    while let Some(at) = rest.find("<tag id='") {
        rest = &rest[at + "<tag id='".len()..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        let id = unescape(&rest[..end]);
        let name = rest
            .find(" name='")
            .and_then(|n| {
                let tail = &rest[n + " name='".len()..];
                tail.find('\'').map(|e| unescape(&tail[..e]))
            })
            .unwrap_or_default();
        out.push((id, name));
    }
    out
}

/// The judge's category of a library category, for the categories a text
/// chunk can have.
fn cat_of(c: Category) -> Option<Cat> {
    match c {
        Category::Gps => Some(Cat::Gps),
        Category::DeviceId => Some(Cat::DeviceId),
        Category::Authorship => Some(Cat::Authorship),
        Category::Rights => Some(Cat::Rights),
        Category::FreeText => Some(Cat::FreeText),
        Category::Timestamps => Some(Cat::Timestamps),
        Category::CameraModel => Some(Cat::CameraModel),
        Category::Software => Some(Cat::Software),
        _ => None,
    }
}

/// One keyword in one chunk form, and the `PNG`-group tags ExifTool printed.
type KeywordRow = (String, &'static str, Vec<String>);

/// The failures of a keyword listing: each file must print exactly one `PNG`
/// tag, and `judge_map(tag)` must be the category the library gives the
/// keyword.
fn keyword_failures(rows: &[KeywordRow], judge_map: &dyn Fn(&str) -> Option<Cat>) -> Vec<String> {
    let mut out = Vec::new();
    for (keyword, form, tags) in rows {
        let [tag] = tags.as_slice() else {
            out.push(format!(
                "{keyword:?} in {form}: ExifTool printed {} PNG tags {tags:?}, expected one",
                tags.len()
            ));
            continue;
        };
        let theirs = judge_map(tag);
        let ours = cat_of(text_keyword_category(keyword.as_bytes()));
        if theirs != ours {
            out.push(format!(
                "{keyword:?} in {form}: ExifTool prints PNG:{tag}, which the judge files under \
                 {theirs:?}, but text_keyword_category says {ours:?}"
            ));
        }
    }
    out
}

/// The table's `(id, name)` pairs, and the rows read from the files.
type KeywordSweep = (Vec<(String, String)>, Vec<KeywordRow>);

/// ExifTool's reading of every keyword in every text-chunk form, once.
fn keyword_rows() -> &'static KeywordSweep {
    static ROWS: OnceLock<KeywordSweep> = OnceLock::new();
    ROWS.get_or_init(|| {
        let table = textual_data_tags(&exiftool_listx());
        let scratch = Scratch::new("png-keywords");
        std::fs::write(scratch.0.join("bare.png"), i3_shapes::with(&[])).expect("bare");
        let forms: [&'static str; 3] = ["tEXt", "zTXt", "iTXt"];
        let name = |i: usize, f: usize| format!("{i:03}_{f}.png");
        for (i, (id, _)) in table.iter().enumerate() {
            let k = id.as_bytes();
            let chunks = [
                oracle_png::text(k, b"Sweep Value"),
                oracle_png::chunk(
                    b"zTXt",
                    &[k, b"\0\0", &oracle_png::zlib_stored(b"Sweep Value")].concat(),
                ),
                oracle_png::chunk(b"iTXt", &[k, b"\0\0\0\0\0Sweep Value"].concat()),
            ];
            for (f, chunk) in chunks.iter().enumerate() {
                std::fs::write(
                    scratch.0.join(name(i, f)),
                    i3_shapes::with(std::slice::from_ref(chunk)),
                )
                .expect("write");
            }
        }
        let mut listing = exiftool_over(&scratch.0, Duration::from_secs(120));
        let bare = listing.remove("bare.png").expect("the bare file");
        let mut rows = Vec::new();
        for (i, (id, _)) in table.iter().enumerate() {
            for (f, form) in forms.iter().enumerate() {
                let file = name(i, f);
                let png_tags: Vec<String> = new_tags(
                    listing
                        .get(&file)
                        .unwrap_or_else(|| panic!("no listing for {file}")),
                    &bare,
                )
                .into_iter()
                .filter_map(|k| k.strip_prefix("PNG:").map(str::to_string))
                .collect();
                rows.push((id.clone(), *form, png_tags));
            }
        }
        (table, rows)
    })
}

#[test]
fn every_textual_data_keyword_has_the_judges_category() {
    if !live_gate("every_textual_data_keyword_has_the_judges_category") {
        return;
    }
    let (table, rows) = keyword_rows();
    // ExifTool's own table is the list the I3 keyword shapes were typed from.
    let ids: Vec<&str> = table.iter().map(|(id, _)| id.as_str()).collect();
    let mut shapes: Vec<&str> = i3_shapes::TEXTUAL_DATA_KEYWORDS.to_vec();
    shapes.sort_unstable();
    let mut theirs = ids.clone();
    theirs.sort_unstable();
    assert_eq!(
        theirs, shapes,
        "PNG::TextualData and i3_shapes::TEXTUAL_DATA_KEYWORDS differ"
    );
    let failures = keyword_failures(rows, &|t| i3_judge::category_of("PNG", t));
    assert!(failures.is_empty(), "{failures:#?}");
    println!(
        "keyword sweep: {} keywords x 3 chunk forms = {} files, one ExifTool run, {} failures",
        table.len(),
        rows.len(),
        failures.len()
    );
    assert_eq!(rows.len(), 3 * table.len());
}

#[test]
fn the_keyword_sweep_goes_red_on_a_missing_row() {
    // Canned: the judge's map without its `Make` row files `Make` as free
    // text (the fall-through of the map), and the sweep says so.
    let canned: Vec<KeywordRow> = vec![
        ("Make".into(), "tEXt", vec!["Make".into()]),
        ("Artist".into(), "tEXt", vec!["Artist".into()]),
    ];
    let real = |t: &str| i3_judge::category_of("PNG", t);
    let without_make = |t: &str| {
        if t == "Make" {
            Some(Cat::FreeText)
        } else {
            real(t)
        }
    };
    assert!(keyword_failures(&canned, &real).is_empty());
    let red = keyword_failures(&canned, &without_make);
    assert_eq!(red.len(), 1, "{red:?}");
    assert!(red[0].contains("\"Make\""), "{red:?}");

    // Against the real ExifTool's readings.
    if !live_gate("the_keyword_sweep_goes_red_on_a_missing_row (live)") {
        return;
    }
    let (_, rows) = keyword_rows();
    assert!(keyword_failures(rows, &real).is_empty());
    let red = keyword_failures(rows, &without_make);
    assert!(
        red.iter().any(|f| f.contains("\"Make\"")),
        "removing the Make row must turn the keyword sweep red: {red:?}"
    );
    println!(
        "AC9 red (keyword sweep): without the Make row in the judge: {}",
        red.first().cloned().unwrap_or_default()
    );
}

//! SPEC-039 AC10: the writer mutated-copy red-proofs W1 to W4, written once.
//!
//! A writer mutant is a one-expression fault injected into a COPY of `src/` at
//! a fact that the audit and the writer both read (`chunk_class`,
//! `text_keyword_category`), so the writer's own self-check (DEC-013: the
//! output is re-audited and must agree) cannot see it. Each mutant is run
//! through the probe's strip mode and judged four ways:
//!
//! 1. the CONTROL (the unmutated copy) reproduces the committed output digest;
//! 2. the MUTANT's `strip` returns `Ok` (it escapes the self-check);
//! 3. REPLAY: the mutant's output digest differs from the committed record's;
//! 4. LIVE (gated): ExifTool's reading of the mutant's output, judged against
//!    the committed input reading by `i3_judge`, fails and names the tag.
//!
//! Test support only. `#[path]`-included by `tests/png_mutation_oracle.rs` and
//! `tests/webp_mutation_oracle.rs`, after `i3_judge`, `i3_records`, `i3_shapes`,
//! `probe` and `tools`.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::i3_judge;
use crate::i3_records;
use crate::i3_shapes;
use crate::probe::{self, StripRun, TempDir, MARKER};

/// One writer mutant.
pub struct Mutant {
    pub id: &'static str,
    /// The I3 fixture it is run on.
    pub fixture: &'static str,
    /// The preset it is run under (the record's section name).
    pub policy: &'static str,
    /// The file of the staged `src/`, relative to it.
    pub file: &'static str,
    /// The exactly-once needle, and its replacement (without the marker).
    pub needle: &'static str,
    pub replacement: &'static str,
}

/// The four results of one mutant.
#[derive(Debug)]
pub struct Results {
    /// (1) the control's output digest equals the committed one.
    pub control_matches: bool,
    /// (2) the mutant's `strip` returned `Ok`.
    pub mutant_ok: bool,
    /// (3) why replay is red (the digest message), or `None` if it is green.
    pub replay_red: Option<String>,
    /// (4) the judge's failures on ExifTool's reading of the mutant's output;
    /// `None` when ExifTool was not run.
    pub live_failures: Option<Vec<String>>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn digest_message(fixture: &str, policy: &str, bytes: &[u8], len: usize, digest: u64) -> String {
    format!(
        "{fixture} under {policy}: the writer's output is {} bytes, digest {:016x}; the record \
         read {len} bytes, digest {digest:016x}",
        bytes.len(),
        i3_records::fnv1a64(bytes)
    )
}

/// Run one mutant. `live` also reads the mutant's output with ExifTool.
pub fn run(m: &Mutant, live: bool) -> Results {
    let fx = i3_shapes::fixtures()
        .into_iter()
        .find(|f| f.name == m.fixture)
        .unwrap_or_else(|| panic!("{}: no I3 fixture named {}", m.id, m.fixture));
    let record_path = root()
        .join(i3_records::record_dir(fx.ext))
        .join(format!("{}.txt", fx.name));
    let text = std::fs::read_to_string(&record_path)
        .unwrap_or_else(|e| panic!("{}: cannot read {}: {e}", m.id, record_path.display()));
    let record = i3_records::parse_record(&text).unwrap_or_else(|e| panic!("{}: {e}", m.id));
    let section = record
        .section(m.policy)
        .unwrap_or_else(|| panic!("{}: no section {}", m.id, m.policy));
    let (len, digest) = section
        .output
        .unwrap_or_else(|| panic!("{}: section {} has no output line", m.id, m.policy));
    let input = record.section("input").expect("input section");

    let files = TempDir::new(&format!("{}-file", m.id));
    let file = files.path().join(format!("{}.{}", fx.name, fx.ext));
    std::fs::write(&file, &fx.bytes).expect("write fixture");

    // (1) the control.
    let control_dir = TempDir::new(&format!("{}-control", m.id));
    probe::stage_probe_crate(control_dir.path(), None);
    let control_bin = probe::build_probe(control_dir.path())
        .unwrap_or_else(|e| panic!("{}: the CONTROL could not be built: {e}", m.id));
    let control = probe::run_probe_strip(&control_bin, &file, m.policy)
        .unwrap_or_else(|e| panic!("{}: the CONTROL could not be run: {e}", m.id));
    let control_matches = matches!(
        &control,
        StripRun::Ok(b) if b.len() == len && i3_records::fnv1a64(b) == digest
    );

    // The mutant.
    let mutant_dir = TempDir::new(&format!("{}-mutant", m.id));
    let inject = |src: &std::path::Path| {
        probe::inject_replace(
            &src.join(m.file),
            m.needle,
            &format!("{} {MARKER}", m.replacement),
        );
    };
    probe::stage_probe_crate(mutant_dir.path(), Some(&inject));
    let mutant_bin = probe::build_probe(mutant_dir.path())
        .unwrap_or_else(|e| panic!("{}: the MUTANT could not be built: {e}", m.id));
    let run = probe::run_probe_strip(&mutant_bin, &file, m.policy)
        .unwrap_or_else(|e| panic!("{}: the MUTANT could not be run: {e}", m.id));
    let StripRun::Ok(bytes) = run else {
        return Results {
            control_matches,
            mutant_ok: false,
            replay_red: None,
            live_failures: None,
        };
    };
    let replay_red = (bytes.len() != len || i3_records::fnv1a64(&bytes) != digest)
        .then(|| digest_message(&fx.name, m.policy, &bytes, len, digest));
    let live_failures = live.then(|| {
        let (lines, _) = i3_records::read_bytes(&format!("{}-out", m.id), fx.ext, &bytes)
            .unwrap_or_else(|e| panic!("{}: exiftool on the mutant's output: {e}", m.id));
        let carriers = record.section("carriers").map(|c| c.lines.as_slice());
        i3_judge::judge(
            &fx.name,
            fx.ext,
            &fx.bytes,
            m.policy,
            &input.lines,
            carriers,
            &lines,
        )
        .failures
    });
    Results {
        control_matches,
        mutant_ok: true,
        replay_red,
        live_failures,
    }
}

/// The four results as one report line, for the handback.
pub fn report(m: &Mutant, r: &Results) -> String {
    format!(
        "{}: control digest equal {}; strip Ok {}; replay red {}; live red {}",
        m.id,
        r.control_matches,
        r.mutant_ok,
        r.replay_red.is_some(),
        r.live_failures
            .as_ref()
            .map_or("not run".to_string(), |f| (!f.is_empty()).to_string())
    )
}

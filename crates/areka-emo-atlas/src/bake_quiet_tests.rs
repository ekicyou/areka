//! 記録を出さない焼きの入口 [`bake_quiet`] の檻（spec: emo-bake-log-overlap 要件 1.1〜1.3・1.6・1.7・3.2・3.4）。
//!
//! 0 寸・全透明・着せ替えの「無し」・抜き色の絵を持つ 1 つの検体を両入口で焼き、
//! 出す入口（[`bake`]）は 4 種を絵ごとに 1 行ずつ出し（対照）、出さない入口はその 4 種の文言が
//! 0 行で、戻り値は等しいことを確かめる。行は宛先まるごとでなく文言ごとに数える。

use super::*;
use areka_parsers::shell::{
    Animation, AppendTarget, DrawMethod, Element, ElementPath, Interval, Pattern, Surface,
};
use std::path::Path;

/// 黙らせる 4 本の文言（`lib.rs` の繰り返しの中）と、その絵の名前。
const KEY_DEBUG: &str = "抜き色（左上の 1 画素と同じ色）で透過しました";
const ZERO_WARN: &str = "元画像が 0 寸です";
const BIND_DEBUG: &str = "着せ替え（bind）のアニメーションからだけ参照される面の絵です";
const CLEAR_WARN: &str = "全透明（α=0）でトリム後 0 寸です";
const QUIETED: [(&str, &str); 4] = [
    (KEY_DEBUG, "key.png"),
    (ZERO_WARN, "zero.png"),
    (BIND_DEBUG, "null.png"),
    (CLEAR_WARN, "clear.png"),
];

fn elem(layer: u32, path: &str) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x: 0,
        y: 0,
    }
}

fn surface(id: u32, elements: Vec<Element>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// 面 0 が見える絵・抜き色の絵・0 寸の絵・全透明の絵・読めない絵を持ち、着せ替えの
/// アニメーションから面 3000（全透明の `null.png` だけ）を参照するシェル。
fn shell() -> Vec<Surface> {
    let mut base = surface(
        0,
        vec![
            elem(0, "solid.png"),
            elem(1, "key.png"),
            elem(2, "zero.png"),
            elem(3, "clear.png"),
            elem(4, "missing.png"),
        ],
    );
    base.animations = vec![Animation {
        id: 0,
        interval: Interval::Bind,
        interval_written: true,
        patterns: vec![Pattern {
            index: 0,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: 3000,
            target: "3000".into(),
            wait: 0,
            x: 0,
            y: 0,
        }],
    }];
    vec![base, surface(3000, vec![elem(0, "null.png")])]
}

fn decoder(base: &Path) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    let opaque = vec![
        10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
    ];
    dec.insert(base.join("solid.png"), 2, 2, 8, opaque.clone(), true);
    // α の無い絵は左上の色で抜く（左上だけが抜け、残りは見える）。
    dec.insert(base.join("key.png"), 2, 2, 8, opaque, false);
    dec.insert(base.join("zero.png"), 0, 0, 0, Vec::new(), true);
    dec.insert(base.join("clear.png"), 2, 2, 8, vec![0; 16], true);
    dec.insert(base.join("null.png"), 2, 2, 8, vec![0; 16], true);
    // "missing.png" は登録しない（読めない絵＝errors に 1 件）。
    dec.insert_pna(base.join("solid.png"));
    dec
}

/// 検体を `f` の入口で焼き、捕捉した記録と結果を返す。
fn bake_by(
    f: impl FnOnce(&[SurfaceSet<'_>], &MemoryDecoder, PackConfig) -> BakeResult,
) -> (String, BakeResult) {
    let base = Path::new("shell/master");
    let surfaces = shell();
    let dec = decoder(base);
    let sets = [SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    }];
    let mut captured: Option<BakeResult> = None;
    let out = crate::log_capture::capture_logs(|| {
        captured = Some(f(&sets, &dec, PackConfig::default()));
    });
    (out, captured.unwrap())
}

fn count(out: &str, text: &str) -> usize {
    out.lines().filter(|l| l.contains(text)).count()
}

/// 戻り値の指紋（索引表の鍵の並び・各エントリ・動く絵・頁・`errors`・`ignored_pna`）。
fn fingerprint(r: &BakeResult) -> String {
    let mut s = String::new();
    for i in 0..r.table.len() as u32 {
        let id = ElementId(i);
        s += &format!(
            "{:?} {:?} {:?}\n",
            r.table.key(id),
            r.table.entry(id),
            r.table.animation(id)
        );
    }
    for p in r.table.pages() {
        s += &format!("{}x{} {} {:?}\n", p.width, p.height, p.stride, &p.bytes[..]);
    }
    s + &format!("{:?} pna={}", r.errors, r.ignored_pna)
}

/// 対照: 出す入口は 4 種を絵ごとに 1 行ずつ出す。
#[test]
fn bake_emits_each_quieted_record_once_per_image() {
    let (out, _) = bake_by(bake);
    for (text, rel) in QUIETED {
        let lines: Vec<&str> = out.lines().filter(|l| l.contains(text)).collect();
        assert_eq!(lines.len(), 1, "{text} は 1 行: {out}");
        assert!(lines[0].contains(rel), "{text} は {rel} の行: {out}");
    }
}

/// 出さない入口はその 4 種の文言が 0 行で、戻り値は出す入口と等しい。
#[test]
fn bake_quiet_emits_none_of_the_quieted_records_and_same_result() {
    let (out, quiet) = bake_by(bake_quiet);
    for (text, _) in QUIETED {
        assert_eq!(count(&out, text), 0, "{text} は 0 行: {out}");
    }
    let (_, loud) = bake_by(bake);
    assert_eq!(quiet.errors.len(), 1, "読めない絵は errors に 1 件");
    assert_eq!(quiet.ignored_pna, 1, "`.pna` の数も同じ形で数える");
    assert_eq!(fingerprint(&quiet), fingerprint(&loud), "戻り値は等しい");
}

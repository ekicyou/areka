//! # actor_underline_present_tests — 下線を描く段で集めて寄せ、字の後に塗る（件 3・実 GPU）
//!
//! 出典 spec: `areka-P0-balloon-text-bugs`（要件 **3.1**・**3.2**・**3.3**・**3.4**・**3.5**・**3.6**・
//! **3.8**・**3.11**・**4.3**、design「areka が塗る下線（件 3）」・テストの方針「結合 2・3・5」）。
//!
//! 本番の 1 フレームの経路（`present_frame`・実 DirectWrite の行送り・実 GPU）を通して読み戻す。
//!
//! | 檻 | 内容 | 要件 |
//! |---|---|---|
//! | 実機の形 | 面の高さ 122・Yu Gothic UI 28 画素・4 行の最下行の作者の下線とアンカーの下線が面の中に途切れなく見える（拡大率 1.0・1.25・1.5） | 3.1, 3.2, 3.3 |
//! | 収まらない形 | 面の高さ 119 では線が面の下端の内側に寄せて塗られる | 3.11 |
//! | 字を変えない | 作者の下線の行で、読み戻しが下線の無い台詞と違うのは線の帯（太さ分の行）の線の右端までだけ（帯の行の線の右横も同じ） | 3.6 |
//! | ベースライン | 行の原点×拡大率が整数画素でない行でも、線の上端が字（`H`）の下端の直下の装置の画素 | 3.10 |
//! | 打ち消し線 | 打ち消し線の画素は下線を足しても変わらない | 3.8 |
//! | 箱 | 箱の文字の面でも実機の形・収まらない形が同じに成り立つ | 3.5 |
//! | 部分の描き替え | 最下行を 1 字ずつ出す（毎フレーム部分の描き替え）と、最後の面が 1 フレームで全部描いた面と 1 バイトも違わない（拡大率 1.25・1.5・寄せる形と寄せない形） | 3.1, 3.11, 4.3 |
//! | 縦書きの端の列 | 縦書き 2 方向（普通のバルーンと箱・拡大率 1.0・1.25・1.5）で、面の端の列を含む全部の列の作者の下線が字の列の左（今の側）に途切れなく見え、面に収まらない右から左の左端の列の線だけが面の左端へ太さを保って寄せられる | 3.4, 3.5 |
//!
//! 「違う画素」の述語は、同じ台本を 2 度撮って 1 バイトも違わないこと（較正）を先に確かめる。

use areka_parsers::balloon::parse_str;
use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use bevy_ecs::prelude::World;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, IDWriteFactory2,
};
use windows::core::{BOOL, HSTRING};
use wintf::com::dwrite::dwrite_create_factory;

use super::box_sync_tests::{emote, resolver};
use super::region_collapse_tests::box_layout;
use super::test_support::{com_world, cue};
use super::{TextLayerRuntime, TextSlotBinding, present_frame};
use crate::place::TextPlace;
use crate::state::TextLayerConfig;

/// 実機の書体。
const FONT: &str = "Yu Gothic UI";
/// 実機の字の大きさ（image px）。
const FONT_HEIGHT: f32 = 28.0;
/// 実機の文字の面（validrect ちょうど・高さ 122）。4 行目の行の箱の下端 90＋28＝118 で 4 行入る。
const REAL: (u32, u32) = (240, 122);
/// 収まらない形: 4 行目の行の箱（下端 118）は入るが、ベースラインの直下の線は面の外へ出る高さ。
const SHORT: (u32, u32) = (240, 119);
/// 4 行目の行の上端（行送り 30 の実機の値・image px）。
const BOTTOM_LINE_TOP: f32 = 90.0;
/// 行送り（実機の値・image px）。
const PITCH: f32 = 30.0;
/// 確かめる拡大率。
const SCALES: [f32; 3] = [1.0, 1.25, 1.5];
/// 全部の字が見えている時刻。
const LATE: f64 = 100.0;
/// インクとみなす α の下限（`decoration_readback_test` と同じ二値化）。
const INK_ALPHA_MIN: u8 = 128;

/// 普通のバルーンの定義（validrect は全域＝面は画像の大きさ）。
const BALLOON: &str = "charset,UTF-8\nfont.name,Yu Gothic UI\nfont.height,28\n";

/// 箱の束（実機の形と収まらない形の 2 つの箱をサーフェス 0 と 1 に置く）。
const SHELL: &str = "\
balloon.real
{
size,240,122
font.name,Yu Gothic UI
font.height,28
}
balloon.short
{
size,240,119
font.name,Yu Gothic UI
font.height,28
}
surface0
{
element1,balloon,real,0,0
}
balloon.vrl
{
size,118,120
font.name,Yu Gothic UI
font.height,28
writing_mode,vertical_rl
}
balloon.vlr
{
size,118,120
font.name,Yu Gothic UI
font.height,28
writing_mode,vertical_lr
}
surface1
{
element1,balloon,short,0,0
}
surface2
{
element1,balloon,vrl,0,0
}
surface3
{
element1,balloon,vlr,0,0
}
";

/// 4 行の台詞（最下行に印を付ける）。字の間に隙間のある仮名で、線だけが字の間を跨ぐ。
const LINES: [&str; 4] = ["あいう", "かきく", "さしす", "たちつ"];
/// ベースラインの檻の台詞（`H` の下端はベースラインちょうど）。
const H_LINES: [&str; 4] = ["HHH", "HHH", "HHH", "HHH"];

/// 行に付ける印。
#[derive(Clone, Copy, Debug)]
enum Mark {
    Plain,
    Author,
    Anchor,
    Strike,
    AuthorStrike,
}

fn font(tokens: &[&str]) -> CueCommand {
    CueCommand::command_carrier(
        FONT_TAG_CARRIER,
        tokens.iter().map(|t| (*t).to_owned()).collect(),
    )
}

/// 台本: 行を `\n` で区切り、`marked` の行を印で囲む。
fn script(lines: &[&str], mark: Mark, marked: impl Fn(usize) -> bool) -> Vec<CueCommand> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push(CueCommand::NewLine { ratio: 1.0 });
        }
        let on = marked(i);
        if on {
            match mark {
                Mark::Plain => {}
                Mark::Author => out.push(font(&["underline", "1"])),
                Mark::Strike => out.push(font(&["strike", "1"])),
                Mark::AuthorStrike => {
                    out.push(font(&["underline", "1"]));
                    out.push(font(&["strike", "1"]));
                }
                Mark::Anchor => out.push(CueCommand::AnchorBegin {
                    id: "OnAnchor".to_owned(),
                    references: Vec::new(),
                }),
            }
        }
        out.push(CueCommand::Text((*line).to_owned()));
        if on {
            match mark {
                Mark::Plain => {}
                Mark::Author => out.push(font(&["underline", "0"])),
                Mark::Strike => out.push(font(&["strike", "0"])),
                Mark::AuthorStrike => {
                    out.push(font(&["underline", "0"]));
                    out.push(font(&["strike", "0"]));
                }
                Mark::Anchor => out.push(CueCommand::AnchorEnd),
            }
        }
    }
    out
}

/// 最下行だけに印を付けた 4 行の台詞。
fn bottom(mark: Mark) -> Vec<CueCommand> {
    script(&LINES, mark, |i| i == LINES.len() - 1)
}

/// 読み戻した面（premultiplied BGRA 密配列＋物理寸）。
#[derive(PartialEq, Eq)]
struct Shot {
    bytes: Vec<u8>,
    w: u32,
    h: u32,
}

impl Shot {
    fn px(&self, x: u32, y: u32) -> &[u8] {
        let o = ((y * self.w + x) * 4) as usize;
        &self.bytes[o..o + 4]
    }
    fn alpha(&self, x: u32, y: u32) -> u8 {
        self.px(x, y)[3]
    }
    fn is_ink(&self, x: u32, y: u32) -> bool {
        self.alpha(x, y) >= INK_ALPHA_MIN
    }
}

fn physical(image: (u32, u32), k: f32) -> (u32, u32) {
    (
        (image.0 as f32 * k).ceil() as u32,
        (image.1 as f32 * k).ceil() as u32,
    )
}

fn apply(rt: &mut TextLayerRuntime, script: &[CueCommand]) {
    for c in script {
        rt.apply_cue(&cue("0", 0.0, c.clone()));
    }
}

fn present(rt: &mut TextLayerRuntime, world: &mut World) {
    present_frame(rt, world, LATE).expect("提示が Ok");
}

/// 台本を時刻の順に並べる（前の台詞を出し終えてから次の台詞が出始める＝最下行は最後に
/// 1 字ずつ出る）。返り値は (cue 列, 最後の字が出る時刻)。
fn timed(script: &[CueCommand]) -> (Vec<TalkCue>, f64) {
    let mut at = 0.0;
    let cues = script
        .iter()
        .map(|c| {
            let c = cue("0", at, c.clone());
            at += c.duration;
            c
        })
        .collect();
    (cues, at)
}

/// 普通のバルーン（面＝`image`・拡大率 `k`）へ台本を流して 1 フレーム提示し、面を読み戻す。
fn shoot_balloon(image: (u32, u32), k: f32, script: &[CueCommand]) -> Shot {
    shoot_described(BALLOON, image, k, script)
}

/// 定義 `descript` の普通のバルーンへ台本を流して 1 フレーム提示し、面を読み戻す。
fn shoot_described(descript: &str, image: (u32, u32), k: f32, script: &[CueCommand]) -> Shot {
    let cues: Vec<TalkCue> = script.iter().map(|c| cue("0", 0.0, c.clone())).collect();
    shoot_frames_described(descript, image, k, &cues, &[LATE])
}

/// 普通のバルーンへ cue 列を流し、時刻 `frames` の順にフレームを提示して、最後の面を読み戻す。
fn shoot_frames(image: (u32, u32), k: f32, cues: &[TalkCue], frames: &[f64]) -> Shot {
    shoot_frames_described(BALLOON, image, k, cues, frames)
}

/// [`shoot_frames`] の定義を選べる形。
fn shoot_frames_described(
    descript: &str,
    image: (u32, u32),
    k: f32,
    cues: &[TalkCue],
    frames: &[f64],
) -> Shot {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_binding(
        actor.clone(),
        TextSlotBinding::new(slot, window, k, physical(image, k), image),
        &parse_str(descript, None),
    );
    for c in cues {
        rt.apply_cue(c);
    }
    for &t in frames {
        present_frame(&mut rt, &mut world, t).expect("提示が Ok");
    }
    let surface = rt.surface(&actor).expect("装着済みの面");
    let (w, h) = surface.size();
    assert_eq!(
        (w, h),
        physical(image, k),
        "面は validrect の物理寸（前提）"
    );
    Shot {
        bytes: surface.read_back().expect("read_back"),
        w,
        h,
    }
}

/// シェルのサーフェス `surface`（0＝実機の形・1＝収まらない形）の箱へ台本を流して提示し、箱の面を読み戻す。
fn shoot_box(surface: u32, k: f32, script: &[CueCommand]) -> Shot {
    let (mut world, window, slot) = com_world();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_box_layout(&mut world, box_layout(SHELL), resolver(), vec![]);
    rt.apply_cue(&emote("0", &surface.to_string()));
    apply(&mut rt, script);
    let shell = TextSlotBinding::new(slot, window, k, physical(REAL, k), REAL);
    rt.sync_box_bindings(&mut world, &[(ActorKey::from("0"), Some((shell, surface)))]);
    present(&mut rt, &mut world);
    let key = rt
        .box_sites
        .keys()
        .find(|key| key.place != TextPlace::Balloon)
        .cloned()
        .expect("箱が登録されている");
    let surface = rt.surface_at(&key).expect("箱の面");
    let (w, h) = surface.size();
    Shot {
        bytes: surface.read_back().expect("read_back"),
        w,
        h,
    }
}

/// 2 枚の画素が違う行（y）の一覧。
fn diff_rows(a: &Shot, b: &Shot) -> Vec<u32> {
    assert_eq!((a.w, a.h), (b.w, b.h), "面の寸は台本によらない（前提）");
    (0..a.h)
        .filter(|&y| (0..a.w).any(|x| a.px(x, y) != b.px(x, y)))
        .collect()
}

/// 連続した 1 本の帯の両端（両端を含む）。
fn band(rows: &[u32], what: &str) -> (u32, u32) {
    assert!(!rows.is_empty(), "{what}: 違う行が 1 本も無い");
    let (first, last) = (rows[0], rows[rows.len() - 1]);
    assert_eq!(
        rows.len() as u32,
        last - first + 1,
        "{what}: 違う行が連続した 1 本の帯でない: {rows:?}"
    );
    (first, last)
}

/// 行 `y0..y1`（物理）のインクの列の左右端（両端を含む）。
fn ink_cols(shot: &Shot, y0: u32, y1: u32) -> (u32, u32) {
    let cols: Vec<u32> = (0..shot.w)
        .filter(|&x| (y0..y1.min(shot.h)).any(|y| shot.is_ink(x, y)))
        .collect();
    assert!(!cols.is_empty(), "行 {y0}..{y1} にインクが無い");
    (cols[0], cols[cols.len() - 1])
}

/// 行 `y` で `x0..=x1` の全部がインク（線が字の間を跨いで途切れない）。
fn full_row(shot: &Shot, y: u32, (x0, x1): (u32, u32)) -> bool {
    (x0..=x1).all(|x| shot.is_ink(x, y))
}

/// 書体の設計値（システムのフォント集から名前で引く）。
fn underline_thickness() -> f32 {
    let factory: IDWriteFactory2 =
        dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let metrics = unsafe {
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, false)
            .expect("システムのフォント集");
        let collection = collection.expect("フォント集");
        let (mut index, mut exists) = (0, BOOL(0));
        collection
            .FindFamilyName(&HSTRING::from(FONT), &mut index, &mut exists)
            .expect("FindFamilyName");
        assert!(exists.as_bool(), "{FONT} がこの機械に入っている");
        let font = collection
            .GetFontFamily(index)
            .expect("GetFontFamily")
            .GetFirstMatchingFont(
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
            )
            .expect("GetFirstMatchingFont");
        let mut metrics = DWRITE_FONT_METRICS::default();
        font.GetMetrics(&mut metrics);
        metrics
    };
    f32::from(metrics.underlineThickness) * FONT_HEIGHT / f32::from(metrics.designUnitsPerEm)
}

/// 最下行の線が面の中に途切れなく見えることを確かめ、線の帯（物理の行）を返す。
///
/// 線の行＝最下行の字の列の左右端の全部がインクの行（字の行は字の間に隙間があるので当たらない）。
fn assert_bottom_stroke_visible(plain: &Shot, styled: &Shot, k: f32, what: &str) -> (u32, u32) {
    let top = (BOTTOM_LINE_TOP * k).floor() as u32;
    let cols = ink_cols(plain, top, plain.h);
    let rows: Vec<u32> = (top..styled.h)
        .filter(|&y| full_row(styled, y, cols))
        .collect();
    assert!(
        !rows.is_empty(),
        "{what}: 最下行の線が面の中に途切れなく見えない（列 {cols:?}・面の高さ {}）",
        styled.h
    );
    assert!(
        (top..plain.h).all(|y| !full_row(plain, y, cols)),
        "{what}: 印の無い台詞に線の行がある（述語が空振りする）"
    );
    band(&rows, what)
}

/// 作者の下線: 違う画素は線の帯（太さ分の行）の、線の右端までだけ・帯の外と、帯の行の線の右横は
/// 下線の無い台詞と 1 画素も違わない（線は行の頭＝面の左端から始まる）。
fn assert_only_the_stroke_differs(plain: &Shot, styled: &Shot, k: f32, what: &str) -> (u32, u32) {
    let (r0, r1) = band(&diff_rows(plain, styled), what);
    let limit = (underline_thickness() * k).ceil() as u32 + 1;
    assert!(
        r1 - r0 < limit,
        "{what}: 下線の無い台詞と違う行 {r0}..={r1} が線の太さ（{limit} 行まで）より広い——字の画素が変わった"
    );
    // 線の右端＝帯の行で左端から途切れずにインクが続く列の終わり（帯の行のうち最も長いもの）。
    let stroke_end = (r0..=r1)
        .map(|y| (0..styled.w).take_while(|&x| styled.is_ink(x, y)).count() as u32)
        .max()
        .unwrap_or(0);
    assert!(stroke_end > 0, "{what}: 帯の行に左端から続く線が無い");
    // 線の右端の列は線の端の濃さ（AA）で違いうるので、その 1 列先までを線の内とする。
    let beside: Vec<u32> = (stroke_end + 1..styled.w)
        .filter(|&x| (r0..=r1).any(|y| plain.px(x, y) != styled.px(x, y)))
        .collect();
    assert!(
        beside.is_empty(),
        "{what}: 線の帯の行で、線の右端（列 {stroke_end}）より右の列 {beside:?} が変わった——字の画素が変わった"
    );
    (r0, r1)
}

// ════════════════════════════════════════════════════════════════════════════════════════

/// 較正: 同じ台本を 2 度撮ると 1 バイトも違わない（「違う画素」の述語が実行のばらつきで緑にならない）。
#[test]
fn the_same_script_paints_the_same_pixels_twice() {
    for k in [1.0, 1.25] {
        let a = shoot_balloon(REAL, k, &bottom(Mark::Author));
        let b = shoot_balloon(REAL, k, &bottom(Mark::Author));
        assert!(a == b, "k={k}: 同じ台本の 2 度の読み戻しが違う");
    }
}

/// 実機の形（面の高さ 122・Yu Gothic UI 28・4 行）の最下行の作者の下線とアンカーの下線が、
/// 拡大率 1.0・1.25・1.5 で面の中に途切れなく見える（要件 3.1〜3.3）。作者の下線の行は線の帯の
/// 外で下線の無い台詞と 1 画素も違わない（要件 3.6）。
#[test]
fn real_machine_shape_bottom_row_underlines_are_inside_the_surface() {
    for k in SCALES {
        let plain = shoot_balloon(REAL, k, &bottom(Mark::Plain));
        let author = shoot_balloon(REAL, k, &bottom(Mark::Author));
        let stroke = assert_bottom_stroke_visible(&plain, &author, k, &format!("作者 k={k}"));
        let diff = assert_only_the_stroke_differs(&plain, &author, k, &format!("作者 k={k}"));
        assert!(
            diff.0 <= stroke.0 && stroke.1 <= diff.1,
            "k={k}: 線の行 {stroke:?} が違う行 {diff:?} の中にある"
        );

        let anchor = shoot_balloon(REAL, k, &bottom(Mark::Anchor));
        assert_bottom_stroke_visible(&plain, &anchor, k, &format!("アンカー k={k}"));
    }
}

/// 線が面に収まらない形（面の高さ 119）では、線が面の下端の内側に寄せて塗られる（要件 3.11）。
/// 4 行目の行の箱は面に入るので、行の数は変わらない（字の画素は線の帯の外で同じ）。
#[test]
fn a_stroke_that_does_not_fit_is_painted_inside_the_bottom_edge() {
    let t = underline_thickness();
    for k in SCALES {
        let plain = shoot_balloon(SHORT, k, &bottom(Mark::Plain));
        let author = shoot_balloon(SHORT, k, &bottom(Mark::Author));
        let (s0, s1) = assert_bottom_stroke_visible(&plain, &author, k, &format!("k={k}"));
        assert_eq!(
            s1,
            author.h - 1,
            "k={k}: 寄せた線は面の下端の画素まで塗られる"
        );
        assert!(
            s1 + 1 - s0 >= (t * k).floor() as u32,
            "k={k}: 寄せた線の太さ（{} 行）が保たれていない（太さ {t}）",
            s1 + 1 - s0
        );
        assert_only_the_stroke_differs(&plain, &author, k, &format!("k={k}"));
    }
}

/// 線の上端は、どの行でも字（`H`）の下端の直下の装置の画素＝D2D が字を描いたベースライン
/// （要件 3.10）。行の原点×拡大率が整数画素でない行（拡大率 1.25・1.5）でも、線の上端の行は
/// 線に覆い尽くされる（α 255）——集める原点を描く原点と揃えないと DirectWrite の画素への寄せが
/// 字と違う位置を返し、線の上端が装置の画素の途中に落ちて上端の行が半端な濃さになる。
/// 実機の行送り（30）ではベースラインが次の行の帯の上端に来るので、字の下端は行の帯で測る。
/// 面の端で寄せない上の 3 行で確かめる。
#[test]
fn the_stroke_top_is_the_device_row_right_below_the_glyphs_on_every_row() {
    for k in SCALES {
        let plain = shoot_balloon(REAL, k, &script(&H_LINES, Mark::Plain, |_| true));
        let author = shoot_balloon(REAL, k, &script(&H_LINES, Mark::Author, |_| true));
        for line in 0..3u32 {
            let y0 = (line as f32 * PITCH * k).ceil() as u32;
            let y1 = ((line + 1) as f32 * PITCH * k).ceil() as u32;
            let glyph_last = (y0..y1)
                .filter(|&y| (0..plain.w).any(|x| plain.alpha(x, y) != 0))
                .max()
                .expect("行に字がある");
            let changed = |y: u32| (0..plain.w).any(|x| plain.px(x, y) != author.px(x, y));
            assert!(
                !changed(glyph_last),
                "k={k} 行 {line}: 線が字の下端の行 {glyph_last} にかかった（ベースラインより上）"
            );
            let top = glyph_last + 1;
            assert!(
                (0..plain.w).any(|x| plain.alpha(x, top) == 0 && author.alpha(x, top) == 255),
                "k={k} 行 {line}: 字の下端の直下の行 {top} が線に覆い尽くされていない（線の上端がベースラインからずれた）"
            );
        }
    }
}

/// 打ち消し線の画素は下線を足しても変わらない（要件 3.8）: 打ち消し線だけの台詞と、打ち消し線と
/// 下線の台詞は、下線の帯の外で 1 画素も違わない（打ち消し線の位置そのものは
/// `decoration_readback_test` の今日の値が固定する）。
#[test]
fn strikethrough_pixels_are_unchanged_by_the_painted_underline() {
    for k in [1.0, 1.25] {
        let plain = shoot_balloon(REAL, k, &bottom(Mark::Plain));
        let strike = shoot_balloon(REAL, k, &bottom(Mark::Strike));
        let author = shoot_balloon(REAL, k, &bottom(Mark::Author));
        let both = shoot_balloon(REAL, k, &bottom(Mark::AuthorStrike));
        let (u0, u1) = band(&diff_rows(&plain, &author), "下線の帯");
        let strike_rows = diff_rows(&plain, &strike);
        assert!(
            strike_rows.iter().any(|&y| y < u0 || y > u1),
            "k={k}: 打ち消し線が下線の帯の外に無い（述語が空振りする）"
        );
        let outside: Vec<u32> = diff_rows(&strike, &both)
            .into_iter()
            .filter(|&y| y < u0 || y > u1)
            .collect();
        assert!(
            outside.is_empty(),
            "k={k}: 下線を足すと下線の帯 {u0}..={u1} の外の行 {outside:?} が変わった（打ち消し線の画素が変わった）"
        );
    }
}

/// 箱の文字の面でも同じ（要件 3.5）: 実機の形の最下行の作者の下線とアンカーの下線が面の中に
/// 見え、収まらない形では面の下端の内側に寄せて塗られる。
#[test]
fn box_text_surfaces_paint_the_bottom_row_underline_inside_the_surface() {
    for k in SCALES {
        let plain = shoot_box(0, k, &bottom(Mark::Plain));
        let author = shoot_box(0, k, &bottom(Mark::Author));
        assert_eq!(
            (author.w, author.h),
            physical(REAL, k),
            "箱の面の寸（前提）"
        );
        assert_bottom_stroke_visible(&plain, &author, k, &format!("箱 作者 k={k}"));
        assert_only_the_stroke_differs(&plain, &author, k, &format!("箱 作者 k={k}"));
        let anchor = shoot_box(0, k, &bottom(Mark::Anchor));
        assert_bottom_stroke_visible(&plain, &anchor, k, &format!("箱 アンカー k={k}"));

        let plain = shoot_box(1, k, &bottom(Mark::Plain));
        let author = shoot_box(1, k, &bottom(Mark::Author));
        assert_eq!(
            (author.w, author.h),
            physical(SHORT, k),
            "箱の面の寸（前提）"
        );
        let (_, s1) = assert_bottom_stroke_visible(&plain, &author, k, &format!("箱 寄せ k={k}"));
        assert_eq!(s1, author.h - 1, "箱 k={k}: 寄せた線は面の下端の画素まで");
    }
}

/// 最下行を 1 字ずつ出すと（毎フレーム、出た字の所だけの部分の描き替え）、最後の面が同じ cue 列を
/// 1 フレームで全部描いた面（全域の描き替え）と 1 バイトも違わない（要件 3.1・3.11・4.3）。
/// 行の生成のときに原点 (0,0) で集めたはみ出しが、字を描く原点で集めた線を取りこぼすと、部分の
/// 描き替えの範囲の外に出た線の画素が塗られずに残る。行の原点×拡大率が整数画素でない拡大率
/// （1.25・1.5）で、寄せない形（実機の形）と寄せる形（収まらない形）の両方を確かめる。
/// 今の形では、行の描き替えの範囲（字の箱＋実測のはみ出し＋ガード 1 image px）が線を覆うので緑。
/// 較正: 描き替えの範囲のはみ出しを 0 にする欠陥を仕込むと、k=1.25 の実機の形で線の行（150・151）
/// だけが違って赤になる（この檻は範囲の外に出た線を見逃さない）。
#[test]
fn revealing_the_bottom_row_frame_by_frame_paints_the_same_pixels_as_one_frame() {
    /// 1 フレームの刻み（1 字の出る間隔と同じ）。
    const STEP: f64 = 0.05;
    for k in [1.25, 1.5] {
        for image in [REAL, SHORT] {
            for mark in [Mark::Author, Mark::Anchor] {
                let what = format!("k={k} 面 {image:?} {mark:?}");
                let (cues, end) = timed(&bottom(mark));
                let frames: Vec<f64> = (1..=(end / STEP).ceil() as usize + 2)
                    .map(|i| i as f64 * STEP)
                    .collect();
                let whole = shoot_frames(image, k, &cues, &[LATE]);
                // 較正: 最下行の途中の時刻の面は最後の面と違う（最下行が何フレームかに分けて出る）。
                let midway = shoot_frames(image, k, &cues, &[end - STEP * 1.5]);
                assert!(
                    midway != whole,
                    "{what}: 最下行が途中の時刻で出揃っている（前提）"
                );
                let progressive = shoot_frames(image, k, &cues, &frames);
                assert!(
                    progressive == whole,
                    "{what}: 1 字ずつ出した面が 1 フレームで描いた面と違う（違う行 {:?}）",
                    diff_rows(&whole, &progressive)
                );
            }
        }
    }
}

// ──────────────────────────── 縦書きの端の列（要件 3.4・3.5）

/// 縦書きの文字の面: 列送り 30 の 4 列目の列の箱（幅 28）が面の端にちょうど接する幅（4×30−2）。
/// 線は字の列の左に出るので、効くのは面の左端の列: 右から左では最後の列（字のインクが面の左端
/// から始まり、線は面の外へ出る＝寄せが効く）、左から右では最初の列（線は面に収まる＝寄せない）。
const VERTICAL: (u32, u32) = (118, 120);

/// 縦書きの普通のバルーンの定義（validrect は全域）。
fn vertical_balloon(mode: &str) -> String {
    format!("{BALLOON}writing_mode,{mode}\n")
}

/// 連続した列（x）の集まりごとの両端（両端を含む）。
fn runs(cols: &[u32]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &x in cols {
        match out.last_mut() {
            Some((_, end)) if *end + 1 == x => *end = x,
            _ => out.push((x, x)),
        }
    }
    out
}

/// 縦書きの全部の列に作者の下線を引いた面と下線の無い面を比べ、列ごとに確かめる:
/// 線（下線の無い面と違う列の帯）が字の列の左の 1/3 にあり（今の側＝D2D の既定の描き方・
/// `decoration_readback` の縦書きの檻と同じ側）、字の列の上端から下端まで途切れずに見える。
/// 端の列（面の左端・右端に接する列）を含む全部の列で成り立つ＝線は面の内側にある。
/// 線の帯（左の列から順）を返す。
fn assert_vertical_columns(plain: &Shot, styled: &Shot, what: &str) -> Vec<(u32, u32)> {
    let ink_x: Vec<u32> = (0..plain.w)
        .filter(|&x| (0..plain.h).any(|y| plain.is_ink(x, y)))
        .collect();
    let columns = runs(&ink_x);
    assert_eq!(
        columns.len(),
        LINES.len(),
        "{what}: 字の列が 4 本（前提）: {columns:?}"
    );
    let diff_x: Vec<u32> = (0..plain.w)
        .filter(|&x| (0..plain.h).any(|y| plain.px(x, y) != styled.px(x, y)))
        .collect();
    let bands = runs(&diff_x);
    assert_eq!(
        bands.len(),
        columns.len(),
        "{what}: 違う列の帯が字の列ごとに 1 本ずつでない: 帯 {bands:?}・列 {columns:?}"
    );
    for (&(b0, b1), &(c0, c1)) in bands.iter().zip(&columns) {
        // 側: 帯の中心が字の列の左の 1/3 より左（字の外を含む・2 倍の目盛りで半画素を避ける）。
        let offset = i64::from(b0 + b1) - 2 * i64::from(c0);
        assert!(
            3 * offset < 2 * i64::from(c1 - c0),
            "{what}: 線 {b0}..={b1} が字の列 {c0}..={c1} の左の 1/3 に無い（線の側が変わった）"
        );
        let ink_y: Vec<u32> = (0..plain.h)
            .filter(|&y| (c0..=c1).any(|x| plain.is_ink(x, y)))
            .collect();
        let (y0, y1) = (ink_y[0], ink_y[ink_y.len() - 1]);
        assert!(
            (b0..=b1).any(|x| (y0..=y1).all(|y| styled.is_ink(x, y))),
            "{what}: 列 {c0}..={c1} の線 {b0}..={b1} が字の上端 {y0} から下端 {y1} まで途切れずに見えない"
        );
    }
    bands
}

/// 面の左端の列の線の寄せ（要件 3.4「端の列で面に収まらないときだけ面の内側へ寄せる」）:
/// 右から左は線が面の外へ出るので、面の左端の画素から太さを保って塗る。左から右は収まるので
/// 寄せない（面の左端から離れている）。
fn assert_left_edge_pull(mode: &str, bands: &[(u32, u32)], k: f32, what: &str) {
    let (b0, b1) = bands[0];
    if mode == "vertical_rl" {
        assert_eq!(b0, 0, "{what}: 寄せた線は面の左端の画素から塗られる");
        assert!(
            b1 + 1 >= (underline_thickness() * k).floor() as u32,
            "{what}: 寄せた線の太さ（{} 列）が保たれていない",
            b1 + 1
        );
    } else {
        assert!(
            b0 > 0,
            "{what}: 収まる線が面の左端へ寄せられた: {b0}..={b1}"
        );
    }
}

/// 縦書き 2 方向の普通のバルーンで、端の列を含む全部の列の作者の下線が面の内側に、今の側
/// （字の列の左）に途切れなく見える（要件 3.4）。拡大率 1.0・1.25・1.5。
#[test]
fn vertical_edge_column_underlines_are_inside_the_surface_on_todays_side() {
    for mode in ["vertical_rl", "vertical_lr"] {
        let descript = vertical_balloon(mode);
        for k in SCALES {
            let what = format!("{mode} k={k}");
            let plain = shoot_described(
                &descript,
                VERTICAL,
                k,
                &script(&LINES, Mark::Plain, |_| true),
            );
            let author = shoot_described(
                &descript,
                VERTICAL,
                k,
                &script(&LINES, Mark::Author, |_| true),
            );
            assert_eq!(
                (author.w, author.h),
                physical(VERTICAL, k),
                "{what}: 面の寸（前提）"
            );
            let bands = assert_vertical_columns(&plain, &author, &what);
            assert_left_edge_pull(mode, &bands, k, &what);
        }
    }
}

/// 箱の文字の面でも同じ（要件 3.5）: 縦書き 2 方向の箱で、端の列を含む全部の列の作者の下線が
/// 面の内側に、今の側に途切れなく見える。
#[test]
fn vertical_box_edge_column_underlines_are_inside_the_surface_on_todays_side() {
    for (surface, mode) in [(2, "vertical_rl"), (3, "vertical_lr")] {
        for k in SCALES {
            let what = format!("箱 {mode} k={k}");
            let plain = shoot_box(surface, k, &script(&LINES, Mark::Plain, |_| true));
            let author = shoot_box(surface, k, &script(&LINES, Mark::Author, |_| true));
            assert_eq!(
                (author.w, author.h),
                physical(VERTICAL, k),
                "{what}: 箱の面の寸（前提）"
            );
            let bands = assert_vertical_columns(&plain, &author, &what);
            assert_left_edge_pull(mode, &bands, k, &what);
        }
    }
}

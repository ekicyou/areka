//! # line_pitch_readback_test — 裁定値の実フォント読み戻し
//!
//! 断りの無い要件の番号と design.md の参照は spec `areka-P0-emo-text-line-height-canon` のもの（別の spec のものはその場で名を書く）。
//!
//! 出典 spec: `areka-P0-emo-text-line-height-canon`（要件 **8.3**／**8.5**／**3.2**／**3.3**／
//! **5.6**／**3.6**／**1.1**／**1.2**・design.md §4.1 正典表・§「帯の防御式を保つ」）。
//!
//! ## このテストが固定するもの
//!
//! 行送りと文字の寸法は裁定（2026-09-05）で決まった値である（`font.height` は em・行間は
//! 定数 2 image px・したがって行送りは 30）。裁定は SSP と areka を並べた 200% 表示画像 2 枚の
//! 目視から導いたもので、**SSP の画素実測は行わない**（開発者方針 2026-09-05）。本ファイルは
//! その裁定値を、実フォント（`Yu Gothic UI`）を実際に描いた画素の読み戻しで定数として固定する
//! ——机上の式ではなく、画面に出た字で確かめる。
//!
//! 検査は 3 本:
//!
//! 1. [`canon_pitch_advance_and_ink_height_match_the_ruling`]——行送りの値・仮名 1 文字の送り・
//!    1 行のインクの縦範囲（要件 8.3／1.1／1.2／3.3）。
//! 2. [`two_lines_ink_does_not_overlap_at_the_ruled_pitch`]——2 行を並べたとき、上の行のインクの
//!    下端が下の行のインクの上端より上にある（要件 8.5／3.2）。
//! 3. [`hover_band_centers_in_the_line_box_and_contains_choice_ink`]——選択肢「閉じる」「もどる」を
//!    順にホバーし、塗り帯が行ボックスの中央へ寄って置かれ（28 では 4 画素）、文字のインクの
//!    上端を含み、帯の下端からのはみ出しが **0** である（要件 5.6／3.6・
//!    `areka-P0-emo2-conformance-e2e` R13）。同日の第 1 の裁定（2 画素のはみ出しを許容する）は
//!    開発者が実機の目視で覆した。事情はその関数の doc に書いた。塗り帯は不透明なので、
//!    α だけでインクを拾うと帯そのものを字と数えてしまう——文字は色で切り分けて拾う
//!    （[`support::is_hover_text`]）。
//!
//! ## 読み戻しの条件
//!
//! 拡大率 k は 1（image px ＝ 物理 px）。供給面は文字描画範囲（validrect）のみを覆うので、
//! 読み戻した画素の座標は validrect-local である（`draw_readback_test.rs` の封じ込め構造と同じ）。
//! いっぽう `ChoiceHitRow::rect` は**バルーン窓の物理 px**（validrect 原点を戻した座標）なので、
//! 面の中を走査するときは validrect 原点を差し引く。
//!
//! ## 実フォントが要る
//!
//! `Yu Gothic UI` はプロポーショナルで、仮名の送りは em より狭い。当該フォントが無い環境では
//! DirectWrite が等幅の代替へ落ち、本ファイルの数値の前提が崩れたまま緑になり得る。ゆえに
//! 先頭で共有の門（`support::fixture::assert_real_font_present`・本番の判断関数で判定）を通し、
//! 当該フォントが無ければ赤で止める。

#[path = "support/mod.rs"]
mod support;

use support::fixture::{assert_real_font_present, balloon_metrics};
use support::{
    Band, assert_hover_band, make_world_with_gpu, read_back, spawn_reserved_slot, surface_size,
    to_canvas_local, y_span_where,
};

use areka_emo_text::actor::{
    ChoiceHitRow, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::draw::DWriteMetrics;
use areka_emo_text::layout::GlyphMetrics;
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::prelude::World;

// ══ 裁定値（design.md §4.1 正典表・裁定 2026-09-05） ═════════════════════════════════════

/// 裁定日（行送り・文字寸法の正典が決まった日）。
const RULED_ON: &str = "2026-09-05";
/// 根拠の所在（表示画像 2 枚の目視読み取り値の表）。
const EVIDENCE: &str =
    ".kiro/specs/completed/areka-P0-emo-text-line-height-canon/verification/evidence/README.md";

/// 実物 `emo2-kakukaku` と同じ `font.height`（image px）。裁定 1 ＝ **em サイズ**であり、
/// 文字描画基盤へ値のまま渡す（要件 1.1／3.3）。
const FONT_HEIGHT: f32 = 28.0;

/// 行送りピッチ ＝ `font.height + 行間 2`（要件 1.2・design §4.1）。
///
/// 根拠画像（200% 表示・2026-09-05・[`EVIDENCE`]）の読み取りでは、SSP の行送りは
/// **58〜60 物理 px** ＝ 29〜30 image px であった（物理 px ＝ image px × 2）。
const PITCH_28: f32 = 30.0;

/// 仮名 1 文字の送り（image px・実フォント `Yu Gothic UI`）。
///
/// 根拠画像の読み取りでは 1 文字の送りが SSP・areka とも **≈ 45 物理 px** ＝ 約 23 image px。
/// `Yu Gothic UI` はプロポーショナルなので全角 ＝ em（28）ではない（要件 3.4）。
const ADVANCE_KANA_28: f32 = 23.0;
/// 上の送りの許容幅（目視読み取りは ±5 物理 px ＝ ±2.5 image px だが、実測に合わせて ±1 で締める）。
const ADVANCE_TOLERANCE: f32 = 1.0;

/// 1 行のインクの縦範囲（image px・実フォント `Yu Gothic UI`・`font.height,28`）。
///
/// 根拠画像の読み取りでは字のインクの丈が SSP・areka とも **≈ 45 物理 px** ＝ 約 22 image px。
/// インク丈 22 は行送り 30 より小さく、これが隣接行のインクが重ならない理由である（要件 3.2）。
const INK_HEIGHT_28: u32 = 22;
/// 上のインク丈の許容幅（アンチエイリアスの端の出方は字によって 1〜2 画素動く）。
const INK_HEIGHT_TOLERANCE: u32 = 2;

/// インクとみなす α の下限（アンチエイリアスの薄い端を落とす二値化・design §Data Models）。
const INK_ALPHA_MIN: u8 = 128;

/// 帯を行ボックスの中央へ寄せる量（image px・`font.height,28` の実フォント `Yu Gothic UI`）。
///
/// `round(max(0, 行ボックス丈 37.24 − 帯の丈 30) / 2) = round(3.62) = 4`
/// （`choice.rs` の帯の寄せの式で字の箱の上端 t = 0 のときの値。`areka-P0-emo2-conformance-e2e` 要件 13.1）。本ファイルは
/// 読み戻した帯の**絶対位置**をこの値で固定する——offset が 0 へ戻れば帯 y0..29 になって赤くなる。
const BAND_OFFSET_28: u32 = 4;

// ══ バルーン記述（実 parser を通す・実フォント指定） ═══════════════════════════════════

/// 読み戻し用バルーンの `descript`（実 `parse_str` を通す ＝ in-code モデルを組み立てない）。
///
/// 実物 `emo2-kakukaku` と同じ実フォント・同じ `font.height,28` を与え、文字描画範囲は
/// 3 行以上が余裕で収まる寸にする（画像 240×140 → validrect (5,5)-(235,135) ＝ 230×130）。
/// `cursor.*` は選択肢のホバー塗り帯を画素で見分けるために指定する（塗り (105,25,25)・
/// ホバー文字は白）。
const DESCRIPT: &str = "charset,UTF-8
font.name,Yu Gothic UI
font.height,28
font.color.r,0
font.color.g,0
font.color.b,0
validrect.top,5
validrect.left,5
validrect.right,-5
validrect.bottom,-5
cursor.style,square
cursor.brush.color.r,105
cursor.brush.color.g,25
cursor.brush.color.b,25
cursor.font.color.r,255
cursor.font.color.g,255
cursor.font.color.b,255
";

/// バルーン画像の原寸（image px・k=1 ゆえ物理原寸と同値）。
const IMAGE: (u32, u32) = (240, 140);
/// 解決後の文字描画範囲の原点（image px）＝供給面 (0,0) に対応する絶対座標。
const VR_ORIGIN: (f32, f32) = (5.0, 5.0);

/// インクの丈を測る 1 字（根拠画像が丈を読んだのと**同じ字**＝本体側 1 行目の「昼」）。
///
/// 根拠画像の読み取りは「字のインクの丈」——1 文字ぶんの上端から下端までであって、行全体の
/// 上端下端ではない。ゆえに読み戻しも同じ字 1 文字で測る（仮名だけの行や複数字の行は、字ごとの
/// 上端下端の合成になるので裁定値と同じものを測ったことにならない）。
///
/// なお「昼」は下部（旦）の横棒が上の日から離れているので、インクのある y は**とびとび**になる。
/// 丈は「インクのある y の最小と最大」で測る（連続した並びの長さではない）。
const INK_SAMPLE_CHAR: &str = "昼";

/// 行を並べる検査に使う 1 行の文字列（4 字）。
///
/// 「昼」1 字だとインクのある y がとびとびになり「行のかたまり」を数えられないので、字の合成で
/// 縦につながる実文を使う（この 4 字なら 1 行のインクが y 方向に連続する——検査 2 の走査方法の前提）。
const LINE_TEXT: &str = "昼間から";

/// ホバーの検査に使う選択肢（id・表示文字列）。要件 5.6／3.6 が名指ししている 2 つである。
///
/// 2 つとも測るのは、帯を行ボックスの中央へ寄せる量（[`BAND_OFFSET_28`]）が 1 行目だけでなく
/// 行送りの先の行にも効くことを確かめるためである。帯の上端は 1 つ目が y4、2 つ目が y64 にある。
const CHOICES: [(&str, &str); 2] = [("OnFirst", "閉じる"), ("OnSecond", "もどる")];

fn model() -> BalloonModel {
    parse_str(DESCRIPT, None)
}

// ══ GPU/COM フィクスチャ（choice_fixture_test.rs と同一方針・headless） ═══════════════

/// 実 descript の font 指定で組んだ字の寸法（GPU 不要）。
fn canon_metrics() -> DWriteMetrics {
    balloon_metrics(&ResolvedBalloonText::resolve(&model(), IMAGE))
}

// ══ cue（全リビール済みフレームで読む・duration を焼くのは Text だけで選択肢は 0） ══════

fn text_cue(text: &str) -> TalkCue {
    support::cue("0", 0.0, CueCommand::Text(text.into()), 0.05)
}

fn newline_cue() -> TalkCue {
    support::cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }, 0.0)
}

fn choice_cue(id: &str, text: &str) -> TalkCue {
    let command = CueCommand::Choice {
        id: id.into(),
        text: text.into(),
        references: vec![id.to_ascii_lowercase()],
    };
    support::cue("0", 0.0, command, 0.0)
}

/// cue 列を装着済み runtime へ流し、全リビール済みの時刻で 1 フレーム提示する。
fn present_cues(rt: &mut TextLayerRuntime, world: &mut World, cues: &[TalkCue]) {
    for cue in cues {
        rt.apply_cue(cue);
    }
    present_frame(rt, world, 100.0).expect("提示");
}

/// 装着済み runtime を組む（供給面は validrect 寸のみを覆う）。
fn make_runtime(world: &mut World) -> (TextLayerRuntime, ActorKey) {
    let (window, slot) = spawn_reserved_slot(world);
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&model(), IMAGE),
    );
    (rt, actor)
}

// ══ 画素の読み取り ═════════════════════════════════════════════════════════════════════

/// x 帯 `[x0, x1)`・y 帯 `[y0, y1)` の中で、述語に合う画素を 1 つ以上持つ y の**連続した並び**を
/// 列挙する。
///
/// 行と行の間には字の無い y が挟まるので、返る並びの本数がそのまま「インクの帯」の本数になる。
fn ink_runs(
    bytes: &[u8],
    width: u32,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in y0..y1 {
        let hit = (x0..x1).any(|x| {
            let i = ((y * width + x) * 4) as usize;
            pred(&bytes[i..i + 4])
        });
        if !hit {
            continue;
        }
        match runs.last_mut() {
            Some(last) if last.1 + 1 == y => last.1 = y,
            _ => runs.push((y, y)),
        }
    }
    runs
}

/// α が閾値以上の画素を「インク」とみなす述語（アンチエイリアスの薄い端を落とす二値化）。
fn is_ink(px: &[u8]) -> bool {
    px[3] >= INK_ALPHA_MIN
}

/// `ChoiceHitRow::rect`（バルーン窓物理 px）の行送り軸の上端を供給面の座標（validrect-local）へ写す。
fn row_top(row: &ChoiceHitRow) -> f32 {
    row.rect.top - VR_ORIGIN.1
}

/// [`VR_ORIGIN`] に書いた数値が、実解決の文字描画範囲の原点と一致することを確かめる。
///
/// 面の走査は `ChoiceHitRow::rect`（バルーン窓の物理 px）から [`VR_ORIGIN`] を差し引いて
/// validrect-local へ写す。この定数が実解決とずれると走査窓が丸ごとずれ、帯も字も見つからないか、
/// 別の行を測ったまま緑になる。ゆえに定数を実解決値（`choice_fixture_test.rs` が使うのと同じ
/// `TextRegion::resolve(...)` の left／top）と突き合わせる。
fn assert_vr_origin_matches_resolved_region() {
    let region = TextRegion::resolve(&model(), IMAGE, WritingMode::HorizontalTb);
    assert_eq!(
        (region.left(), region.top()),
        VR_ORIGIN,
        "VR_ORIGIN は実解決の文字描画範囲の原点と一致する（descript の validrect.left／.top ＝ 5）"
    );
}

// ══ 検査 1: 裁定値そのもの（要件 8.3／1.1／1.2／3.3） ═══════════════════════════════════

/// 観測可能な完了状態（1 本目）: 実フォント `Yu Gothic UI`・`font.height,28`・
/// 拡大率 1 で、(a) 行送りが裁定値 30 ちょうど、(b) 仮名 1 文字の送りが ≈ 23（±1）、
/// (c) 1 行のインクの縦範囲が ≈ 22（±2）である。
///
/// (a) は式（`font.height + 行間 2`）の値そのもので、(b)(c) は実際に描いた画素から読み戻す。
/// 根拠画像（[`EVIDENCE`]・[`RULED_ON`]・拡大率 200%）の読み取りは物理 px で送り ≈ 45・
/// インク丈 ≈ 45・行送り 58〜60 であり、image px へ直す（÷2）と 23／22／29〜30 になる。
/// SSP の画素実測は行わない（開発者方針 2026-09-05）。
#[test]
fn canon_pitch_advance_and_ink_height_match_the_ruling() {
    let resolved = ResolvedBalloonText::resolve(&model(), IMAGE);
    assert_eq!(
        resolved.font.height, FONT_HEIGHT,
        "descript の font.height が em サイズとしてそのまま解決される（裁定 1・要件 1.1／3.3）"
    );

    let metrics = canon_metrics();
    assert_real_font_present();

    // (a) 行送り ＝ 裁定値（要件 1.2）。
    assert_eq!(
        metrics.line_pitch(FONT_HEIGHT),
        PITCH_28,
        "行送りは font.height {FONT_HEIGHT} ＋ 行間 2 ＝ {PITCH_28}（裁定 {RULED_ON}・根拠 {EVIDENCE}）"
    );

    // (b) 仮名 1 文字の送り（実フォントの実測・要件 3.4）。
    for ch in ["あ", "い", "か", "ん"] {
        let advance = metrics.advance(ch, FONT_HEIGHT);
        assert!(
            (advance - ADVANCE_KANA_28).abs() <= ADVANCE_TOLERANCE,
            "仮名「{ch}」の送りは {ADVANCE_KANA_28}±{ADVANCE_TOLERANCE} image px（実測 {advance}）——根拠画像の ≈ 45 物理 px ÷ 2"
        );
        assert!(
            advance < FONT_HEIGHT,
            "仮名「{ch}」の送り {advance} は em {FONT_HEIGHT} より狭い（プロポーショナル・要件 3.4）"
        );
    }

    // (c) 1 字のインクの縦範囲（実際に描いた画素・要件 8.3）。
    let mut world = make_world_with_gpu();
    let (mut rt, actor) = make_runtime(&mut world);
    present_cues(&mut rt, &mut world, &[text_cue(INK_SAMPLE_CHAR)]);
    let (bytes, (w, h)) = (read_back(&rt, &actor), surface_size(&rt, &actor));

    let (top, bottom) = y_span_where(&bytes, w, 0, w, 0, h, is_ink)
        .expect("1 字を描いたのでインクが在る（無ければ描画が退化している）");
    let ink_height = bottom - top + 1;
    let lo = INK_HEIGHT_28 - INK_HEIGHT_TOLERANCE;
    let hi = INK_HEIGHT_28 + INK_HEIGHT_TOLERANCE;
    assert!(
        (lo..=hi).contains(&ink_height),
        "「{INK_SAMPLE_CHAR}」1 字のインクの縦範囲は {INK_HEIGHT_28}±{INK_HEIGHT_TOLERANCE} image px（実測 y{top}..{bottom} ＝ {ink_height}px・α ≥ {INK_ALPHA_MIN}）——根拠画像の ≈ 45 物理 px ÷ 2"
    );
    assert!(
        ink_height < PITCH_28 as u32,
        "インク丈 {ink_height} は行送り {PITCH_28} より小さい（隣接行が重ならない根拠・要件 3.2）"
    );
}

// ══ 検査 2: 2 行のインクが重ならない（要件 8.5／3.2） ═══════════════════════════════════

/// 観測可能な完了状態（2 本目）: 同じ文字列の 2 行を裁定の行送りで並べると、
/// 上の行のインクの下端が下の行のインクの上端より**上**にある（インクが重ならない）。
///
/// 行ボックス丈（ascent ＋ descent ＝ 37.24）は行送り 30 を超えるが、判定は**インク**で行う
/// （要件 3.2）——箱が重なることは許容し、字が重ならなければよい。
///
/// 走査の方法: 面の全 y について「α が閾値以上の画素が 1 つでもあるか」を見て、連続する y を
/// ひとまとまりにする。2 行なら 2 まとまりになり、まとまりの間の空白が「重なっていない」ことの
/// 証拠になる。まとまりが 1 本しか無ければ 2 行のインクがつながっている ＝ 重なりであり、
/// その場合は本検査が赤になる。
#[test]
fn two_lines_ink_does_not_overlap_at_the_ruled_pitch() {
    assert_real_font_present();
    let mut world = make_world_with_gpu();
    let (mut rt, actor) = make_runtime(&mut world);
    present_cues(
        &mut rt,
        &mut world,
        &[text_cue(LINE_TEXT), newline_cue(), text_cue(LINE_TEXT)],
    );
    let (bytes, (w, h)) = (read_back(&rt, &actor), surface_size(&rt, &actor));

    let runs = ink_runs(&bytes, w, 0, w, 0, h, is_ink);
    assert_eq!(
        runs.len(),
        2,
        "2 行を描いたのでインクの帯は 2 本に分かれる（つながっていたら隣接行のインクが重なっている・実測 {runs:?}）"
    );
    let (top1, bottom1) = runs[0];
    let (top2, bottom2) = runs[1];
    assert!(
        bottom1 < top2,
        "上の行のインク下端 y{bottom1} は下の行のインク上端 y{top2} より上にある（要件 8.5／3.2・実測 y{top1}..{bottom1} と y{top2}..{bottom2}）"
    );
    // 同じ文字列なので、2 行のインクの上端の差は行送りそのものになる。
    assert_eq!(
        top2 - top1,
        PITCH_28 as u32,
        "2 行のインク上端の差は行送り {PITCH_28}（実測 y{top1}..{bottom1} と y{top2}..{bottom2}）"
    );
}

// ══ 検査 3: ホバー塗り帯と文字のインク（要件 5.6／3.6・裁定 2026-09-06 第 2 回＝`areka-P0-emo2-conformance-e2e` R13） ═══

/// 観測可能な完了状態（3 本目・`areka-P0-emo2-conformance-e2e` R13.3 で締め直し）:
/// 選択肢「閉じる」「もどる」の**どちらをホバーしても**、塗り帯の丈が行送り（30）と一致し、
/// 帯が行ボックスの中央へ [`BAND_OFFSET_28`] 画素だけ寄って置かれ（ordinal 0 は y4..33）、
/// 帯が文字のインクの**上端を含み**（上の余白 ≥ 0）、帯の**下端からのはみ出しが 0** である。
///
/// # 測り方（2 つの取り違えを避ける）
///
/// 1. **帯そのものを文字と数えない。** 塗り帯は不透明なので、α だけでインクを拾うと帯の画素に
///    当たり、返る縦範囲は帯の縦範囲になる。それでは「字の上端が帯の中にある」が字の有無に
///    関わらず成り立ってしまう（字が消えても緑）。ゆえに文字は色で分けて拾い（[`support::is_hover_text`]）、
///    **字が実際に在ること**（画素数 > 0）も併せて確かめる。
/// 2. **窓で上端も下端も切らない。** 走査窓を帯とぴったり同じにすると、はみ出したインクが窓の
///    外へ落ちて下端の判定が恒真になり、同じ理屈で上端の判定も——窓の上端が帯の上端と同じなら
///    「字の上端は帯の上端以上」は窓の取り方だけで必ず成り立つ——恒真になる。窓は帯の上下へ
///    [`support::SCAN_MARGIN`] 画素ずつ開け、字の上端・下端がどちらも窓の内側にあることも確かめる。
///
/// ホバーは 2 度入れる。どちらの選択肢でも帯の上端は自分の行の上端より [`BAND_OFFSET_28`] 画素下に
/// あるので、窓はその分だけ上へ開く（ordinal 0 は行の上端が面の縁で、帯 y4..33・窓の上端 y0）。字がベースラインごと
/// 上へずれれば「字の上端が帯の上端以上」が破れて赤くなる。
///
/// # 裁定 2026-09-06（第 2 回・`areka-P0-emo2-conformance-e2e` R13）＝ **帯を行ボックスの中央へ**
///
/// 実フォント `Yu Gothic UI`・正典の `font.height,28`・拡大率 1 では、DirectWrite が
/// 行ボックス 37.24（(2210+514)/2048 × 28）の中にインクを中央寄りに置く一方、帯は行送り 30 で
/// 頭打ちになる。帯を行矩形のブロック軸近端（上端）へ揃えていた頃の実測は
///
/// | 選択肢 | 帯 | 文字（白）| 上の余白 | 下のはみ出し |
/// |---|---|---|---|---|
/// | 閉じる（ordinal 0）| y0..29（丈 30）| y7..31 | 7 画素 | 2 画素 |
/// | もどる（ordinal 1）| y60..89（丈 30）| y66..91 | 6 画素 | 2 画素 |
///
/// で、**上に 7 画素の余白を残したまま下で 2 画素を切っていた**。開発者はこれを
/// 「色反転位置が 2 ドット程上すぎる」と判じ、同日の第 1 の裁定（2 画素のはみ出しを許容する）を
/// 覆した。新しい規則は帯を `round(max(0, 行ボックス丈 − 帯の丈) / 2)` だけ近端から内側へ
/// 寄せることで（28 では [`BAND_OFFSET_28`]＝4）、寄せた後の実測は
///
/// | 選択肢 | 帯 | 文字（白）| 上の余白 | 下のはみ出し |
/// |---|---|---|---|---|
/// | 閉じる（ordinal 0）| y4..33（丈 30）| y7..31 | 3 画素 | **0** |
/// | もどる（ordinal 1）| y64..93（丈 30）| y66..91 | 2 画素 | **0** |
///
/// である（α ≥ 128 の二値化・α ≠ 0 でも α ≥ 250 でも同じ y。閾値の取り方の問題ではない）。
///
/// **帯は広げない**——帯を行送りより広げると隣接する行の帯と重なり、どの選択肢を指しているかの
/// 一意性が壊れる（`areka-P0-emo-text-line-height-canon` design §「帯の防御式を保つ」）。直したのは
/// 帯の**位置**だけであり、丈もヒット行の出どころも変えていない。隣の行の帯は
/// `offset + 丈 = 34 > 行送り 30` でも `30 + 4 = 34` から始まるので**接して重ならない**。
///
/// 走査の窓: ホバー行の x 帯だけを見て、y はその行のヒット矩形の上端の [`support::SCAN_MARGIN`] 上から
/// 帯の丈 ＋ [`support::SCAN_MARGIN`] までとする（2 つの選択肢の間に空行を 1 行入れてあるので、
/// 次の行のインクが窓へ入らない）。
#[test]
fn hover_band_centers_in_the_line_box_and_contains_choice_ink() {
    assert_real_font_present();
    assert_vr_origin_matches_resolved_region();
    let mut world = make_world_with_gpu();
    let (mut rt, actor) = make_runtime(&mut world);
    // 2 つの選択肢の間に改行 2 つ ＝ 空行 1 行を挟み、次の行のインクが走査窓へ入らないようにする。
    present_cues(
        &mut rt,
        &mut world,
        &[
            choice_cue(CHOICES[0].0, CHOICES[0].1),
            newline_cue(),
            newline_cue(),
            choice_cue(CHOICES[1].0, CHOICES[1].1),
        ],
    );
    let rows: Vec<ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(
        rows.len(),
        CHOICES.len(),
        "2 選択肢 ＝ 2 ヒット行（実測 {}）",
        rows.len()
    );
    assert_eq!(
        row_top(&rows[1]) - row_top(&rows[0]),
        PITCH_28 * 2.0,
        "選択肢の行は空行 1 行を挟んで行送り 2 つぶん離れている"
    );
    // ヒット矩形そのものが offset を持つ（描画の帯と同じ値を配る・`areka-P0-emo2-conformance-e2e` R13.2）。1 行目は文字描画範囲の
    // 書き出し（validrect 原点）から始まるので、その上端はそのまま offset になる。
    assert_eq!(
        row_top(&rows[0]),
        BAND_OFFSET_28 as f32,
        "先頭行のヒット矩形の上端は行矩形の上端（0）＋ offset {BAND_OFFSET_28}（近端揃えなら 0 で赤）"
    );

    // 2 つの選択肢を順にホバーし、それぞれ 1 フレーム提示して読み戻す。
    for (ordinal, (_, label)) in CHOICES.iter().enumerate() {
        rt.inject_choice_hover(&actor, Some(ordinal));
        present_frame(&mut rt, &mut world, 100.0).expect("ホバー提示");
        let (bytes, (w, h)) = (read_back(&rt, &actor), surface_size(&rt, &actor));

        // 窓は行の書き出し（＝帯の上端）を基準に、上下へ SCAN_MARGIN ずつ開ける。窓の下端は
        // 「次の行」の上端（帯の上端 ＋ 行送り）＋ SCAN_MARGIN。ordinal 0 も帯の上端が y4 にあるので、
        // 窓は面の縁（y0）まで 4 画素上へ開く。行の書き出しは ordinal ごとに行送り 2 つぶんずつ
        // 下がる——帯は ordinal 0 が y4..33・ordinal 1 が y64..93。offset を 0 へ戻すと
        // y0..29／y60..89 になって赤くなる。
        let row = to_canvas_local(&rows[ordinal], VR_ORIGIN.0, VR_ORIGIN.1);
        let next_row = (row.0, row.1 + PITCH_28, row.2, row.3 + PITCH_28);
        assert_hover_band(
            &format!("「{label}」（ordinal {ordinal}・font.height {FONT_HEIGHT}）"),
            &bytes,
            (w, h),
            row,
            next_row,
            Band {
                line_top: (ordinal as u32) * (PITCH_28 as u32) * 2,
                height: PITCH_28 as u32,
                offset: BAND_OFFSET_28,
            },
        );
    }
}

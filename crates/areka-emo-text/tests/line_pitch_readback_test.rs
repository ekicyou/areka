//! # line_pitch_readback_test — 裁定値の実フォント読み戻し（task 5.2）
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
//! 3. [`hover_band_contains_choice_ink_within_the_two_pixel_ruling`]——選択肢「閉じる」「もどる」を
//!    順にホバーし、塗り帯が文字のインクの上端を含み、帯の下端からのはみ出しが 2 画素以内である
//!    （要件 5.6／3.6）。上限 2 は**裁定 2026-09-06（第 2 回）**で決まった値である（正典の
//!    `font.height,28` を実フォントで読み戻すと、字によって 2 画素はみ出す）。事情はその関数の
//!    doc に書いた。塗り帯は不透明なので、α だけでインクを拾うと帯そのものを字と数えてしまう
//!    ——文字は色で切り分けて拾う（[`is_hover_text`]）。
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
//! 先頭で「あ」の送りが em 未満であることを確かめ、縮退していたら赤で止める
//! （`kero_menu_capacity_test.rs` の同趣旨の門と同じ考え方）。

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
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::com::dwrite::dwrite_create_factory;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

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

/// ホバー塗り帯の下端から文字のインクがはみ出してよい上限（image px）。
///
/// **裁定 2026-09-06（第 2 回・要件 3.6）**: 正典の `font.height,28` を実フォント
/// `Yu Gothic UI` で読み戻すと、帯の下端に対して文字のインクの下端が字により最大 2 画素下へ
/// 出る。開発者の裁定は「**2 画素のはみ出しを許容する**」である。第 1 回の裁定（同日・1 画素）
/// は `choice_fixture_test.rs` の fixture（`font.height,20`）の読み戻しで測ったもので、正典の
/// 28 で測り直した本ファイルの実測がそれを上書きした。
///
/// これは**裁定による意味の変更**であって、都合に合わせて許容幅を緩めたものではない。帯は
/// 広げない——帯を行送りより広げると隣接する行の帯と重なり、どの選択肢を指しているかの
/// 一意性が壊れる（design §「帯の防御式を保つ」）。**3 画素以上のはみ出しが出た場合は帯を
/// 広げず、はみ出しの画素数を数値で添えて改めて開発者の裁定を仰ぐこと。**
const BAND_OVERHANG_MAX: u32 = 2;

/// 走査窓を塗り帯の**上下**へ余分に伸ばす画素数。
///
/// 下側: 窓の下端を帯の下端と揃えると、帯からはみ出したインクが窓の外に落ちて**測れないまま
/// 緑になる**（要件 7.3 の「緑のまま意味を失う」）。上限 2 を超えたはみ出しを検出できるよう、
/// 窓は帯より 4 画素深く取る。
///
/// 上側: 窓の上端を帯の上端と揃えると、「字の上端が帯の上端以上」は窓の取り方だけで必ず成り立ち、
/// 字が上へずれても窓に切られて緑のままになる（判定が反証にならない）。ゆえに窓は帯より 4 画素
/// 上からも見る。帯より上に来るのはホバーしていない行で、その字は黒（premultiplied の青チャネルは
/// 0）ゆえ [`is_hover_text`] に当たらず、塗り色 (25,25,105,255) でもないので帯の検出も 1 本のままである。
///
/// 上下いずれの側も、隣の選択肢の行は行送り 2 つぶん（60）離れているのでこの窓には入らない。
const SCAN_MARGIN: u32 = 4;

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
/// 2 つとも測るのは、1 つ目の帯の上端が面の先頭（y0）にあり、それだけでは「字の上端が帯の中」の
/// 判定が反証にならないからである。2 つ目の帯の上端は y60 にある。
const CHOICES: [(&str, &str); 2] = [("OnFirst", "閉じる"), ("OnSecond", "もどる")];

fn model() -> BalloonModel {
    parse_str(DESCRIPT, None)
}

// ══ GPU/COM フィクスチャ（choice_fixture_test.rs と同一方針・headless） ═══════════════

/// `GraphicsCore` ＋ `WucGraphicsResource` を実資源として載せた wintf World（headless・MTA）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo-present `VisualMount` と同型の予約スロット（(window, slot) を返す）。
fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-line-pitch-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

/// DirectWrite factory（計測に要るのは factory だけ・GPU 不要）。
fn factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DirectWrite factory を生成できる")
}

/// 実 descript の font 指定で組んだ字の寸法（GPU 不要・各検査の実フォント門に使う）。
fn canon_metrics() -> DWriteMetrics {
    let resolved = ResolvedBalloonText::resolve(&model(), IMAGE);
    DWriteMetrics::new(
        &factory(),
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("実 descript のフォントで DWriteMetrics を生成できる")
}

/// 実フォントの送り幅が代替フォントへ縮退していないことを確かめる（縮退なら赤で止める）。
fn assert_real_font_present(metrics: &DWriteMetrics) {
    let a = metrics.advance('あ', FONT_HEIGHT);
    assert!(
        a < FONT_HEIGHT,
        "実フォント Yu Gothic UI が見つからない（「あ」の送りが {a} ＝ em {FONT_HEIGHT} 以上の等幅値へ縮退している）。本ファイルの期待値は実フォントの実測を前提にしているので、代替フォントのまま緑にしない"
    );
}

// ══ cue（全リビール済みフレームで読むので duration は素直に焼く） ══════════════════════

fn text_cue(text: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text(text.into()),
        duration: text.chars().count() as f64 * 0.05,
    }
}

fn newline_cue() -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::NewLine { ratio: 1.0 },
        duration: 0.0,
    }
}

fn choice_cue(id: &str, text: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: vec![id.to_ascii_lowercase()],
        },
        duration: text.chars().count() as f64 * 0.05,
    }
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

/// 供給面の読み戻し（premultiplied BGRA 密配列）と面の寸。
fn read_back(rt: &TextLayerRuntime, actor: &ActorKey) -> (Vec<u8>, u32, u32) {
    let surface = rt.surface(actor).expect("装着済み actor の供給面");
    let (w, h) = surface.size();
    (surface.read_back().expect("read_back"), w, h)
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

/// x 帯 `[x0, x1)`・y 帯 `[y0, y1)` で述語に合う画素が現れた y の**最小と最大**（無ければ `None`）。
///
/// 字の中にインクの無い y が挟まっても（「昼」の下部の横棒のように）1 つの丈として測れる。
fn ink_extent(
    bytes: &[u8],
    width: u32,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> Option<(u32, u32)> {
    let runs = ink_runs(bytes, width, x0, x1, y0, y1, pred);
    Some((runs.first()?.0, runs.last()?.1))
}

/// x 帯 `[x0, x1)`・y 帯 `[y0, y1)` で述語に合う画素の**個数**（退化していないことの確認用）。
fn ink_count(
    bytes: &[u8],
    width: u32,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                n += 1;
            }
        }
    }
    n
}

/// α が閾値以上の画素を「インク」とみなす述語（アンチエイリアスの薄い端を落とす二値化）。
fn is_ink(px: &[u8]) -> bool {
    px[3] >= INK_ALPHA_MIN
}

/// ホバー塗り帯の色（`cursor.brush.color` (105,25,25)・premultiplied α=255 ゆえ BGRA=(25,25,105,255)）。
fn is_band_fill(px: &[u8]) -> bool {
    px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
}

/// ホバー行の**文字だけ**を選ぶ述語（塗り帯そのものを文字と数えないための切り分け）。
///
/// [`is_ink`]（α ≥ 128）は塗り帯の画素にも当たる——帯は不透明だからである。それで縦範囲を測ると
/// 「帯の縦範囲」が返り、**字が 1 画素も無くても帯の上端 ＝ 帯の上端**になって上端の判定が
/// 恒真になる。そこで色で分ける:
///
/// - ホバー文字は白（`cursor.font.color` 255,255,255）。premultiplied では B ＝ G ＝ R ＝ α なので、
///   帯の外（背景が透明）では青チャネル ＝ α。帯の上に載った字は塗りと混ざり
///   B ＝ 25 ＋ 230 × 被覆率 になる。
/// - 塗り帯は (105,25,25) ゆえ青チャネルは 25 止まり。帯の縁が半透明になっても premultiplied なので
///   25 より小さくなるだけである。
///
/// ゆえに「青チャネルが 128 以上」は白文字だけを拾う。α の閾値も併せて掛け、帯の外の薄い端は
/// [`is_ink`] と同じ被覆率（≈ 0.5）で落とす。
fn is_hover_text(px: &[u8]) -> bool {
    px[3] >= INK_ALPHA_MIN && px[0] >= INK_ALPHA_MIN
}

/// `ChoiceHitRow::rect`（バルーン窓物理 px）の行内軸を供給面の座標（validrect-local）へ写す。
fn row_x_span(row: &ChoiceHitRow, width: u32) -> (u32, u32) {
    let x0 = (row.rect.left - VR_ORIGIN.0).floor().max(0.0) as u32;
    let x1 = ((row.rect.right - VR_ORIGIN.0).ceil().max(0.0) as u32).min(width);
    (x0, x1)
}

/// 同上（行送り軸の上端）。
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

/// 観測可能な完了状態（task 5.2 の 1 本目）: 実フォント `Yu Gothic UI`・`font.height,28`・
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
    assert_real_font_present(&metrics);

    // (a) 行送り ＝ 裁定値（要件 1.2）。
    assert_eq!(
        metrics.line_pitch(FONT_HEIGHT),
        PITCH_28,
        "行送りは font.height {FONT_HEIGHT} ＋ 行間 2 ＝ {PITCH_28}（裁定 {RULED_ON}・根拠 {EVIDENCE}）"
    );

    // (b) 仮名 1 文字の送り（実フォントの実測・要件 3.4）。
    for ch in ['あ', 'い', 'か', 'ん'] {
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
    let (bytes, w, h) = read_back(&rt, &actor);

    let (top, bottom) = ink_extent(&bytes, w, 0, w, 0, h, is_ink)
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

/// 観測可能な完了状態（task 5.2 の 2 本目）: 同じ文字列の 2 行を裁定の行送りで並べると、
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
    assert_real_font_present(&canon_metrics());
    let mut world = make_world_with_gpu();
    let (mut rt, actor) = make_runtime(&mut world);
    present_cues(
        &mut rt,
        &mut world,
        &[text_cue(LINE_TEXT), newline_cue(), text_cue(LINE_TEXT)],
    );
    let (bytes, w, h) = read_back(&rt, &actor);

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

// ══ 検査 3: ホバー塗り帯と文字のインク（要件 5.6／3.6・裁定 2026-09-06） ═══════════════

/// 観測可能な完了状態（task 5.2 の 3 本目）: 選択肢「閉じる」「もどる」の**どちらをホバーしても**、
/// 塗り帯の丈が行送り（30）と一致し、帯が文字のインクの**上端を含み**、帯の**下端からのはみ出しが
/// 2 画素以内**である。
///
/// # 測り方（2 つの取り違えを避ける）
///
/// 1. **帯そのものを文字と数えない。** 塗り帯は不透明なので、α だけでインクを拾うと帯の画素に
///    当たり、返る縦範囲は帯の縦範囲になる。それでは「字の上端が帯の中にある」が字の有無に
///    関わらず成り立ってしまう（字が消えても緑）。ゆえに文字は色で分けて拾い（[`is_hover_text`]）、
///    **字が実際に在ること**（画素数 > 0）も併せて確かめる。
/// 2. **窓で上端も下端も切らない。** 走査窓を帯とぴったり同じにすると、はみ出したインクが窓の
///    外へ落ちて下端の判定が恒真になり、同じ理屈で上端の判定も——窓の上端が帯の上端と同じなら
///    「字の上端は帯の上端以上」は窓の取り方だけで必ず成り立つ——恒真になる。窓は帯の上下へ
///    [`SCAN_MARGIN`] 画素ずつ開け、字の上端・下端がどちらも窓の内側にあることも確かめる（要件 7.3）。
///
/// ホバーは 2 度入れる。1 つ目の選択肢（「閉じる」）は帯の上端が面の縁（y0）にあり、窓をそれより
/// 上へ開けようがないので、上端の判定はこの選択肢では赤にならない。**上端の反証可能性は 2 つ目の
/// 選択肢（「もどる」）が担う**——帯の上端が y60・窓の上端が y56 にあるので、字がベースラインごと
/// 上へずれれば「字の上端が帯の上端より下」が破れて赤くなる。
///
/// # 裁定 2026-09-06（第 2 回・要件 3.6）＝ **2 画素を許容する**
///
/// 実フォント `Yu Gothic UI`・正典の `font.height,28`・拡大率 1 で読み戻すと、帯の下端
/// （y29・帯の丈 30 ＝ 行送り）に対して文字のインクの下端が字により 1〜2 画素下へ出る
/// （DirectWrite がベースラインを design metrics の ascent ＝ 2210/2048 × 28 ≈ 30.2 に
/// 置く帰結）。字ごとの実測は
///
/// | 字 | インクの下端 | 帯の下端からのはみ出し |
/// |---|---|---|
/// | は・い・じ・る・ど・整 | y30 | 1 画素 |
/// | 閉・も・調・頻・度 | y31 | **2 画素** |
///
/// である（α ≥ 128 の二値化・α ≠ 0 でも α ≥ 250 でも同じ y。閾値の取り方の問題ではない）。
/// 仕様が 5.6／3.6 の読み戻しに名指ししている選択肢「閉じる」「もどる」はどちらも 2 画素側で、
/// `menu.pasta` の「おしゃべり頻度」「調整」も同じである。行としての実測は
///
/// | 選択肢 | 帯 | 文字（白）| 上の余白 | 下のはみ出し |
/// |---|---|---|---|---|
/// | 閉じる（ordinal 0）| y0..29（丈 30）| y7..31 | 7 画素 | **2 画素** |
/// | もどる（ordinal 1）| y60..89（丈 30）| y66..91 | 6 画素 | **2 画素** |
///
/// **第 1 回の裁定（同日・1 画素）は、`choice_fixture_test.rs` の fixture（`font.height,20`・
/// 選択肢「はい」）の読み戻しで測った値であり、正典の 28 で測り直した本ファイルの実測が
/// それを上書きした。** 開発者の裁定は「**2 画素を許容する**」である。
///
/// **帯は広げない**——帯を行送りより広げると隣接する行の帯と重なり、どの選択肢を指しているかの
/// 一意性が壊れる（design §「帯の防御式を保つ」）。ヒット行の出どころも変えない。すなわち
/// これは**裁定による意味の変更**であって、都合に合わせて許容幅を緩めたものではない。
/// **3 画素以上のはみ出しが出た場合は帯を広げず、画素数を添えて改めて開発者の裁定を仰ぐこと。**
///
/// 走査の窓: ホバー行の x 帯だけを見て、y はその行の書き出しから帯の丈 ＋ [`SCAN_MARGIN`] までとする
/// （2 つの選択肢の間に空行を 1 行入れてあるので、次の行のインクが窓へ入らない）。
#[test]
fn hover_band_contains_choice_ink_within_the_two_pixel_ruling() {
    assert_real_font_present(&canon_metrics());
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

    // 2 つの選択肢を順にホバーし、それぞれ 1 フレーム提示して読み戻す。
    for (ordinal, (_, label)) in CHOICES.iter().enumerate() {
        rt.inject_choice_hover(&actor, Some(ordinal));
        present_frame(&mut rt, &mut world, 100.0).expect("ホバー提示");
        let (bytes, w, h) = read_back(&rt, &actor);

        let (x0, x1) = row_x_span(&rows[ordinal], w);
        // 窓は行の書き出し（＝帯の上端）を基準に、上下へ SCAN_MARGIN ずつ開ける。
        // ordinal 0 は書き出しが面の縁（0）なので上へは開かない（`saturating_sub` が 0 に留める）。
        let block_top = row_top(&rows[ordinal]).floor().max(0.0) as u32;
        let y0 = block_top.saturating_sub(SCAN_MARGIN);
        let y1 = (block_top + PITCH_28 as u32 + SCAN_MARGIN).min(h);
        assert!(
            x0 < x1 && y0 < y1,
            "走査窓が非退化: 「{label}」 x{x0}..{x1} y{y0}..{y1}"
        );

        // ── 塗り帯 ──
        let band = ink_runs(&bytes, w, x0, x1, y0, y1, is_band_fill);
        assert_eq!(
            band.len(),
            1,
            "「{label}」のホバー行の塗り帯は 1 本（実測 {band:?}）"
        );
        let (band_top, band_bottom) = band[0];
        assert_eq!(
            band_bottom - band_top + 1,
            PITCH_28 as u32,
            "「{label}」の塗り帯の丈は行送り {PITCH_28}（帯 ＝ clamp(行ボックス丈 37.24, 28, 30)・実測 y{band_top}..{band_bottom}）"
        );

        // ── 文字だけのインク（帯と切り分ける・字が在ることも確かめる） ──
        let text_pixels = ink_count(&bytes, w, x0, x1, y0, y1, is_hover_text);
        assert!(
            text_pixels > 0,
            "「{label}」のホバー行に白文字の画素が 1 つも無い（字が描かれていないのに帯だけで緑になる取り違えを塞ぐ・窓 x{x0}..{x1} y{y0}..{y1}）"
        );
        let (text_top, text_bottom) = ink_extent(&bytes, w, x0, x1, y0, y1, is_hover_text)
            .expect("画素数が 0 でない以上、白文字の縦範囲は在る");
        assert!(
            text_bottom < y1 - 1,
            "「{label}」の文字のインクの下端 y{text_bottom} が走査窓の下端 y{y1} に接している——窓が下端を切っている疑いがあり、はみ出しの測定が信用できない（窓を深くすること・要件 7.3）"
        );
        // 上端側の対の門。これが在って初めて、下の「字の上端が帯の上端の内」が窓の取り方ではなく
        // 描画結果を見た判定になる（字が上へずれたら、まず窓の上端に届いてここが赤くなる）。
        // ordinal 0 では帯の上端が面の縁（y0 ＝ 0）なので窓を上へ開けられず、この門も上端の判定も
        // 赤にならない——上端の反証可能性は ordinal 1（「もどる」・窓の上端 y56・帯の上端 y60）が担う。
        assert!(
            text_top > y0,
            "「{label}」の文字のインクの上端 y{text_top} が走査窓の上端 y{y0} に接している——窓が上端を切っている疑いがあり、上端の判定が信用できない（窓を上へ開けること・要件 7.3）"
        );

        eprintln!(
            "[line-pitch-readback] 「{label}」（ordinal {ordinal}・font.height {FONT_HEIGHT}）: 窓 x{x0}..{x1} y{y0}..{y1} / 帯 y{band_top}..{band_bottom}（丈 {}）/ 文字 y{text_top}..{text_bottom}（{text_pixels} 画素）/ 上の余白 {} 画素（負なら帯の上へはみ出し）/ 下のはみ出し {} 画素",
            band_bottom - band_top + 1,
            text_top as i64 - band_top as i64,
            text_bottom.saturating_sub(band_bottom)
        );

        // 上端は帯の中に収まる（文字の上が帯からはみ出さない・要件 5.6）。
        assert!(
            text_top >= band_top,
            "「{label}」の文字のインクの上端 y{text_top} は塗り帯の上端 y{band_top} の内にある（要件 5.6・窓 y{y0}..{y1}）"
        );
        // 下端のはみ出しは 2 画素以内（裁定 2026-09-06 第 2 回・要件 3.6）。
        let overhang = text_bottom.saturating_sub(band_bottom);
        assert!(
            overhang <= BAND_OVERHANG_MAX,
            "塗り帯の下端 y{band_bottom} から文字のインクの下端 y{text_bottom} が {overhang} 画素はみ出した（選択肢「{label}」・font.height {FONT_HEIGHT}・帯の丈 {PITCH_28}）。裁定 2026-09-06（第 2 回・要件 3.6）が許すのは {BAND_OVERHANG_MAX} 画素までである。**帯を広げてはならない**（隣接する行の帯と重なり、どの選択肢を指しているかの一意性が壊れる）。**許容幅を裁定なしで緩めてもならない**。この数値を添えて開発者の裁定を仰ぐこと（本関数の doc に字ごとの実測表がある）"
        );
    }
}

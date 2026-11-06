//! # scale_invariance_test — DPI/スケールの写像と画像座標空間のレイアウトの絶対値（task 10.3・R4.6/R10.4/R11.9）
//!
//! 合成スケール k による**写像**（物理寸＝`ceil(寸 × k)`・オフセット＝`原点 × k`）と、
//! 画像座標空間で解いた**レイアウト決定**（折返し・行送り・スクロール発火）の絶対値を、
//! 実描画なし（DirectWrite/D2D/COM/GPU 不使用・[`FixedMetrics`] 注入）の headless テストで固定する
//! （design.md「Testing Strategy > Unit Tests」6. スケール写像 が正典）。
//!
//! - **R4.6**: レイアウトは画像座標空間（前半）・物理化は合成スケール共有点 k の一点適用（後半）。
//! - **R10.4**: 画像/物理の 2 空間のみ（論理 px 不在）・変換は [`ScaleContract`] の一点定義。
//! - **R11.9**: レイアウト決定部（折返し・行送り・スクロール発火・validrect 整合）の値を固定する
//!   （実 DPI の実描画観測は task 9.3 example 手順の領分）。
//!
//! k ごとにレイアウトを解き直して比べる形は置かない。テストが自分で画像の寸法を解決へ渡すと
//! k は入力に入らず、同じ純関数を同じ値で呼び直すだけになる（本番が物理寸を渡す形に変わっても
//! 緑のまま）。本番が画像の寸法を渡す向きは `tests/attach_wiring_test.rs` が実の presenter で守る。
//!
//! 端数檻の方針（tasks.md Implementation Notes 2.4）: round/ceil の正準式は割り切れる値
//! だけでなく端数ケースで檻化し、floor/ceil/round の取り違え変異を殺す値を選ぶ
//! （例 321×1.25=401.25→402 は round 変異を殺す）。

#[path = "support/mod.rs"]
mod support;

use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use areka_emo_text::region::{ImagePx, PhysicalPx, ScaleContract};
use areka_emo_text::state::{TextItem, TextLayerState};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

// ── テスト土台 ──────────────────────────────────────────────────────────────

/// レイアウト共通の画像原寸（image px・region.rs／layout.rs の檻と同一値）。
const IMAGE: (u32, u32) = (400, 224);

/// テスト用 BalloonModel 生成ヘルパ（validrect は top,bottom,left,right・
/// writing_mode は生文字列転記＝解釈は本層 `WritingMode::resolve`）。
fn model(
    origin: (Option<i32>, Option<i32>),
    wordwrap: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    writing_mode: Option<&str>,
) -> BalloonModel {
    support::geometry_model(origin, wordwrap, validrect, None, writing_mode)
}

/// reveal 間隔（0.25＝2 の冪・正確表現）。reveal は配送 duration 由来（`interval = duration / N`）
/// ゆえ Text cue へ `N × REVEAL_INTERVAL` を焼き込むと interval=0.25 の決定論的リビール時刻列を得る
/// （旧 char_wait=0.25 檻と機能等価・pipeline_test.rs と同方針）。
const REVEAL_INTERVAL: f64 = 0.25;

/// テスト用 cue（共有の部品の間隔つき cue へ本ファイルの間隔を渡す）。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    support::cue(actor, at, command, REVEAL_INTERVAL)
}

// ══ 物理寸・オフセット写像: 物理寸=ceil(寸×k)・オフセット=画像×k（R4.6/R10.4） ══

/// present_frame と同一の導出（物理寸＝`ceil(validrect 寸 × k)`・オフセット＝
/// `validrect 原点 × k`）を k=1.0/1.25/2.0 で檻化する。fixture 実測 validrect
/// （(36,46)-(356,168)＝幅 320・高さ 122）に対する期待値は手計算の絶対値。
///
/// 端数檻: 122×1.25=152.5→**153**（floor 変異=152 を殺す）。round 変異は次テストの
/// 401.25 が殺す（152.5 は round half away でも 153 になり round を殺せないため）。
#[test]
fn physical_size_and_offset_map_from_region_at_each_scale() {
    let m = model(
        (None, None),
        (Some(-49), Some(0)),
        (Some(46), Some(-56), Some(36), Some(-44)),
        None,
    );
    // (k, 期待物理寸, 期待物理オフセット)——いずれも f32 で正確に表現できる値。
    let expected: [(f32, (u32, u32), (f32, f32)); 3] = [
        (1.0, (320, 122), (36.0, 46.0)),
        (1.25, (400, 153), (45.0, 57.5)), // 320×1.25=400・122×1.25=152.5→ceil 153
        (2.0, (640, 244), (72.0, 92.0)),
    ];
    let resolved = ResolvedBalloonText::resolve(&m, IMAGE);
    let region = &resolved.region;
    // レイアウト決定（validrect 絶対矩形）は画像座標空間の値。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (36.0, 46.0, 356.0, 168.0),
        "validrect 解決は画像座標空間のまま"
    );
    for (k, want_size, want_offset) in expected {
        // 物理化は ScaleContract の一点変換のみ（present_frame と同一の導出式）。
        let contract = ScaleContract::new(k, None);
        let physical_size = (
            contract.physical_extent(ImagePx(region.right() - region.left())),
            contract.physical_extent(ImagePx(region.bottom() - region.top())),
        );
        let physical_offset = (
            contract.to_physical(ImagePx(region.left())).0,
            contract.to_physical(ImagePx(region.top())).0,
        );
        assert_eq!(
            physical_size, want_size,
            "k={k}: 物理寸 = ceil(validrect 寸 × k)"
        );
        assert_eq!(
            physical_offset, want_offset,
            "k={k}: 物理オフセット = validrect 原点 × k"
        );
        // 物理→画像の逆写像が原値へ戻る（2 空間の往復整合）。
        assert_eq!(
            contract.to_image(PhysicalPx(physical_offset.0)),
            ImagePx(region.left()),
            "k={k}: to_image(to_physical(left)) は原値へ戻る"
        );
    }
}

/// ceil の丸め正準を round/floor 変異から守る端数檻: 幅 321・高さ 123 の validrect
/// （left35/right-44・top45/bottom-56）で 321×1.25=401.25→**402**（round/floor
/// 変異=401 を殺す）・123×1.25=153.75→**154**（floor 変異=153 を殺す）。
#[test]
fn physical_extent_ceils_fractional_values_killing_round_and_floor() {
    let m = model(
        (None, None),
        (None, None),
        (Some(45), Some(-56), Some(35), Some(-44)),
        None,
    );
    let resolved = ResolvedBalloonText::resolve(&m, IMAGE);
    let region = &resolved.region;
    assert_eq!(
        (
            region.right() - region.left(),
            region.bottom() - region.top()
        ),
        (321.0, 123.0)
    );
    let contract = ScaleContract::new(1.25, None);
    assert_eq!(
        contract.physical_extent(ImagePx(region.right() - region.left())),
        402,
        "321 × 1.25 = 401.25 → ceil 402（round/floor なら 401）"
    );
    assert_eq!(
        contract.physical_extent(ImagePx(region.bottom() - region.top())),
        154,
        "123 × 1.25 = 153.75 → ceil 154（floor なら 153）"
    );
}

// ══ R11.9 構造側: 画像座標空間のレイアウト決定（横書き・同一 cue 通し） ══

/// 画像座標空間で解いたレイアウト決定の絶対値: 自動折返し（閾値 351・font 40 で
/// 7 グリフ/行）・明示改行・3 行あふれ→縦スクロール（first_visible_line=1・
/// block_offset=−42）まで通しで効いていること。
///
/// 幾何は fixture 実測値（負値=反対辺基準・wordwrappoint.x,-49／validrect right,-44 等）を
/// 使う——負値の解決は画像原寸に依存する。
#[test]
fn horizontal_layout_decision_in_image_space() {
    // fixture 実測: validrect (36,46)-(356,168)・折返し閾値 400-49=351・
    // origin は未宣言＝書字開始角 (36,46) から書き始める。font 40 → pitch 40+2=42。
    let m = model(
        (None, None),
        (Some(-49), Some(0)),
        (Some(46), Some(-56), Some(36), Some(-44)),
        None,
    );
    // 同一 cue 列（typewriter 進行込み・reveal interval=0.25）: 自動折返し＋明示改行＋あふれ。
    let actor = ActorKey::from("0");
    let mut state = TextLayerState::default();
    for c in [
        // 10 グリフ: x=36 起点・8 個目は 316+40=356 > 351 で自動折返し（7＋3）。
        cue("0", 0.0, CueCommand::Text("あいうえおかきくけこ".into())),
        cue("0", 2.5, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 2.5, CueCommand::Text("さし".into())),
    ] {
        state.apply_cue(&c);
    }
    let resolved = ResolvedBalloonText::resolve(&m, IMAGE);
    let items = state
        .actor_state(&actor)
        .map(|s| s.items().to_vec())
        .unwrap_or_default();
    // t=10.0 は全量リビール済みの時刻。
    let visible = state.visible_glyphs(&actor, 10.0);
    let lines = LayoutEngine::layout(
        &items,
        visible,
        &resolved.region,
        resolved.mode,
        40.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);

    // 折返し（7＋3）＋明示改行で 3 行・あふれ発火。
    assert_eq!(visible, 12, "全 12 グリフがリビール済み");
    assert_eq!(lines.len(), 3, "自動折返し＋明示改行で 3 行");
    assert_eq!(
        lines[0].glyphs.len(),
        7,
        "行 0 は閾値 351（負値 -49 の反対辺解決）で 7 グリフ（8 個目で自動折返し）"
    );
    assert_eq!(
        lines[0].glyphs[0].inline_pos, 36.0,
        "行内開始＝書字開始角 x=36（origin 未宣言時の縮退・image px）"
    );
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -42.0
        },
        "3 行目の下端 46+2×42+40=170 > 168（bottom,-56 の反対辺解決）で縦スクロール発火（横書き）"
    );
}

// ══ R11.9 構造側（縦書き）: vertical_rl／vertical_lr の可視窓決定 ══

/// 縦書き両方向のレイアウト決定（列送り・横スクロール可視窓）の絶対値。
/// あふれ境界の辺は**負値（反対辺基準）**で与える（rl: left,-40→360・lr: right,-360→40）。
/// font 10 → 列送り 10+2=12: vertical_rl は最新列の左端 400−10−3×12=354
/// < validrect.left 360 で内容が右（+12）へ、vertical_lr は最新列の右端 10+3×12=46 >
/// validrect.right 40 で内容が左（−12）へ
/// （layout.rs の檻と同一の解決後幾何・軸読み替え正準表）。
#[test]
fn vertical_layout_decision_in_image_space() {
    // (writing_mode, validrect(top,bottom,left,right)・境界辺は負値, 期待オフセット)。
    let cases: [(&str, (i32, i32, i32, i32), f32); 2] = [
        ("vertical_rl", (0, 224, -40, 400), 12.0),
        ("vertical_lr", (0, 224, 0, -360), -12.0),
    ];
    // 4 列（各列 全角 1 グリフ・明示改行区切り）——4 列目であふれ発火する幾何。
    let mut items = Vec::new();
    for i in 0..4 {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::glyph("あ"));
    }

    for (mode_str, (top, bottom, left, right), want_offset) in cases {
        let m = model(
            (None, None),
            (None, None),
            (Some(top), Some(bottom), Some(left), Some(right)),
            Some(mode_str),
        );
        let resolved = ResolvedBalloonText::resolve(&m, IMAGE);
        let lines = LayoutEngine::layout(
            &items,
            4,
            &resolved.region,
            resolved.mode,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
        assert_eq!(lines.len(), 4, "{mode_str}: 4 列が配置される");
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: want_offset
            },
            "{mode_str}: 4 列目のあふれで横スクロール発火"
        );
    }
}

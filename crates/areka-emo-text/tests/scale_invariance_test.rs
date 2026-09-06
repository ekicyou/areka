//! # scale_invariance_test — DPI/スケールの構造検証（task 10.3・R4.6/R10.4/R11.9）
//!
//! 合成スケール k が 1 以外（1.25・2.0・および k<1）の場合の**写像**（物理寸・オフセット・
//! 画像原寸の透過）と、**レイアウト決定のスケール非依存**（design.md「DPI/スケール契約」:
//! 折返し/行送り/スクロール決定はすべて画像座標空間＝k 非依存）を、実描画なし
//! （DirectWrite/D2D/COM/GPU 不使用・[`FixedMetrics`] 注入）の headless 構造テストで檻化する
//! （design.md「Testing Strategy > Unit Tests」6. スケール写像 が正典）。
//!
//! - **R4.6**: レイアウトは画像座標空間（前半）・物理化は合成スケール共有点 k の一点適用（後半）。
//! - **R10.4**: 画像/物理の 2 空間のみ（論理 px 不在）・変換は [`ScaleContract`] の一点定義。
//! - **R11.9**: レイアウト決定部（折返し・行送り・スクロール発火・validrect 整合）の
//!   スケール非依存を構造テストで検証する（実 DPI の実描画観測は task 9.3 example 手順の領分）。
//!
//! 端数檻の方針（tasks.md Implementation Notes 2.4）: round/ceil の正準式は割り切れる値
//! だけでなく端数ケースで檻化し、floor/ceil/round の取り違え変異を殺す値を選ぶ
//! （例 321×1.25=401.25→402 は round 変異を殺す）。
//!
//! **2026-07-30**: image px 原寸は `round(物理 / k)` の逆写像で復元するのをやめ、presenter の
//! native をそのまま透過する形へ是正した（k<1 では順写像が縮小写像＝単射でなく、逆写像が
//! 原理的に存在しないため）。旧導出の端数檻は透過檻＋k<1 の衝突檻へ置き換えてある。

use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding};
use areka_emo_text::layout::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow, WrapPlan};
use areka_emo_text::region::{ImagePx, PhysicalPx, ScaleContract};
use areka_emo_text::state::{TextItem, TextLayerState};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::prelude::World;

// ── テスト土台 ──────────────────────────────────────────────────────────────

/// 本檻で回す合成スケール集合（design Testing Strategy 6 の正準値: 1.0/1.25/2.0）。
const SCALES: [f32; 3] = [1.0, 1.25, 2.0];

/// レイアウト共通の画像原寸（image px・region.rs／layout.rs の檻と同一値）。
const IMAGE: (u32, u32) = (400, 224);

/// テスト用 BalloonModel 生成ヘルパ（validrect は top,bottom,left,right・
/// writing_mode は生文字列転記＝解釈は本層 [`WritingMode::resolve`]）。
fn model(
    origin: (Option<i32>, Option<i32>),
    wordwrap: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    writing_mode: Option<&str>,
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(wordwrap.0, wordwrap.1),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        writing_mode.map(str::to_owned),
        None,
    )
}

/// binding 構築ヘルパ（slot/window entity は World から採番——値は本檻の関心外）。
///
/// `surface_size`＝物理原寸（k 適用後）／`image_size`＝作者画像空間の原寸（k 不変）。
/// 本番では前者を `TextSlotView::physical_size()`・後者を `surface_size()`（native）が供給する。
fn binding(
    world: &mut World,
    k: f32,
    surface_size: (u32, u32),
    image_size: (u32, u32),
) -> TextSlotBinding {
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();
    TextSlotBinding::new(slot, window, k, surface_size, image_size)
}

/// 画像原寸 `image` を k で物理化した surface 寸（`物理 = ceil(画像 × k)`——
/// 上流（emo-present/placement）が同一バルーンを k 別に物理化した状況の再現）。
fn physical_surface(image: (u32, u32), k: f32) -> (u32, u32) {
    (
        (image.0 as f32 * k).ceil() as u32,
        (image.1 as f32 * k).ceil() as u32,
    )
}

/// reveal 間隔（0.25＝2 の冪・正確表現）。reveal は配送 duration 由来（`interval = duration / N`）
/// ゆえ Text cue へ `N × REVEAL_INTERVAL` を焼き込むと interval=0.25 の決定論的リビール時刻列を得る
/// （旧 char_wait=0.25 檻と機能等価・pipeline_test.rs と同方針）。
const REVEAL_INTERVAL: f64 = 0.25;

/// テスト用 cue 生成ヘルパ（pipeline_test.rs と同型）。Text cue には配送
/// duration = `N × REVEAL_INTERVAL` を焼き込む（reveal interval=0.25）。他は瞬時（duration=0）。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// 1 観測点のレイアウト決定結果（注入時刻ごとの可視グリフ数・行列・可視窓——
/// 「レイアウト決定の出力」の全体をビット一致で比較する値型）。
#[derive(Clone, Debug, PartialEq)]
struct LayoutDecision {
    t: f64,
    visible: usize,
    lines: Vec<PositionedLine>,
    window: VisibleWindow,
}

/// 同一 balloon 定義・同一 cue 状態に対し、k 別 binding の `image_size` から
/// レイアウト決定（visible_glyphs→layout→visible_window）を通しで評価する。
///
/// 本関数は [`present_frame`](areka_emo_text::actor::present_frame) の純粋部分と同じ
/// 一点導出経路（`TextSlotBinding::new` の `image_size`→`ResolvedBalloonText::resolve`）
/// を通る——物理 px をレイアウトへ渡す誤配線（k 混入）はここで赤くなる。
fn decide_layout(
    state: &TextLayerState,
    actor: &ActorKey,
    model: &BalloonModel,
    binding: &TextSlotBinding,
    font_height: f32,
    probe_times: &[f64],
) -> Vec<LayoutDecision> {
    let resolved = ResolvedBalloonText::resolve(model, binding.image_size);
    let items = state
        .actor_state(actor)
        .map(|s| s.items().to_vec())
        .unwrap_or_default();
    probe_times
        .iter()
        .map(|&t| {
            let visible = state.visible_glyphs(actor, t);
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &resolved.region,
                resolved.mode,
                font_height,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
            LayoutDecision {
                t,
                visible,
                lines,
                window,
            }
        })
        .collect()
}

// ══ 画像原寸: presenter native の透過（k 不変・R10.4） ══

/// 同一バルーン（画像原寸 400×224）を k 別に物理化した surface からの binding 構築で、
/// `image_size` が k によらず同一の画像原寸を保つ（レイアウト入力の同一性の前提）。
///
/// **k<1 を含む**（2026-07-30）: image px 原寸は逆写像で復元せず presenter の native を
/// そのまま透過するため、k の大小に依らず厳密に一致する。旧実装（`round(物理 / k)`）は
/// k<1 で 1px ずれた——順写像の ±0.5 物理px 誤差が k で割られて ±0.5/k 画像px へ増幅されるため。
#[test]
fn bindings_at_different_scales_keep_identical_image_size() {
    let mut world = World::new();
    for k in SCALES {
        let surface = physical_surface(IMAGE, k);
        let b = binding(&mut world, k, surface, IMAGE);
        assert_eq!(b.scale, k, "k={k}: scale は透過保持される");
        assert_eq!(
            b.surface_size, surface,
            "k={k}: 物理原寸はそのまま保持される"
        );
        assert_eq!(
            b.image_size, IMAGE,
            "k={k}: image px 原寸は presenter native の透過＝k 不変"
        );
    }
}

/// k<1 の往復欠陥回帰檻（2026-07-30 新設）: **旧導出 `image_size = round(物理 / k)` を
/// 復活させると落ちる**値で固定する。
///
/// # k<1 の順写像は単射でない（＝逆写像は原理的に存在しない）
///
/// 丸め権威 `ScaleRatio::scale_len` は k=4/5 で `142 → round(113.6) = 114` と
/// `143 → round(114.4) = 114` を返す——**異なる原寸が同一の物理寸へ潰れる**。潰れた情報は
/// 割り算では戻らず、旧導出はこの 2 つのうち必ずどちらかで 1px 間違える（実測では 114 から
/// 143 を返すため、原寸 142 側が落ちる）。同様に 77 と 78 はともに 62 へ潰れる。
///
/// k>1 で表面化しないのは、拡大写像が異なる原寸を異なる物理寸へ分離するからにすぎない
/// （縮小写像である k<1 のみ鳩ノ巣で衝突する）。「k≥1 で緑」は k<1 の証拠にならない。
///
/// 本番到達性: `parse_author_dpi` は宣言値を素通しするため、`dpi,120` のゴーストを
/// 96 DPI（100%）モニタで表示すれば k=96/120=4/5 になる。
#[test]
fn sub_unity_scale_keeps_image_size_exact_where_the_old_inverse_lost_a_pixel() {
    let mut world = World::new();
    // (native, k 適用後の物理寸)＝scale_len(native, 4/5) の実値。
    // 142→114・77→62（143→114・78→62 と衝突する側を選ぶ）。
    const NATIVE_KLT1: (u32, u32) = (142, 77);
    const PHYSICAL_KLT1: (u32, u32) = (114, 62);

    let b = binding(&mut world, 0.8, PHYSICAL_KLT1, NATIVE_KLT1);
    assert_eq!(
        b.image_size, NATIVE_KLT1,
        "k<1 でも image px 原寸は native そのもの（旧導出は (143, 78) を返して 1px ずれた）"
    );
    assert_eq!(b.surface_size, PHYSICAL_KLT1, "物理原寸はそのまま保持");
}

/// 最小 1px クランプとの交差（k<1・2026-07-30 新設）: native (1,1) は k=2/3 でも
/// 物理 (1,1)（`scale_len` の min-1 クランプ）であり、image px 原寸は (1,1) のまま。
/// 旧導出は `round(1 / (2/3))` = `round(1.5)` = **2** と原寸を水増ししていた。
#[test]
fn sub_unity_scale_min_one_pixel_clamp_does_not_inflate_image_size() {
    let mut world = World::new();
    let b = binding(&mut world, 2.0 / 3.0, (1, 1), (1, 1));
    assert_eq!(
        b.image_size,
        (1, 1),
        "min-1 クランプ後の物理寸から原寸を割り戻さない（旧導出は (2, 2) へ膨らんだ）"
    );
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
    let mut world = World::new();
    // (k, 期待物理寸, 期待物理オフセット)——いずれも f32 で正確に表現できる値。
    let expected: [(f32, (u32, u32), (f32, f32)); 3] = [
        (1.0, (320, 122), (36.0, 46.0)),
        (1.25, (400, 153), (45.0, 57.5)), // 320×1.25=400・122×1.25=152.5→ceil 153
        (2.0, (640, 244), (72.0, 92.0)),
    ];
    for (k, want_size, want_offset) in expected {
        let b = binding(&mut world, k, physical_surface(IMAGE, k), IMAGE);
        let resolved = ResolvedBalloonText::resolve(&m, b.image_size);
        let region = &resolved.region;
        // レイアウト決定（validrect 絶対矩形）は全 k で同一の image px。
        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (36.0, 46.0, 356.0, 168.0),
            "k={k}: validrect 解決は画像座標空間のまま（k 非依存）"
        );
        // 物理化は ScaleContract の一点変換のみ（present_frame と同一の導出式）。
        let contract = ScaleContract::new(b.scale, None);
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
    let mut world = World::new();
    let b = binding(&mut world, 1.25, physical_surface(IMAGE, 1.25), IMAGE);
    let resolved = ResolvedBalloonText::resolve(&m, b.image_size);
    let region = &resolved.region;
    assert_eq!(
        (
            region.right() - region.left(),
            region.bottom() - region.top()
        ),
        (321.0, 123.0)
    );
    let contract = ScaleContract::new(b.scale, None);
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

// ══ R11.9 構造側: レイアウト決定の出力がスケール非依存（横書き・同一 cue 通し） ══

/// 観測可能な完了状態の中核: **同一 balloon 定義・同一 cue 列**に対し k=1.0/1.25/2.0 の
/// binding からのレイアウト決定（折返し位置・行矩形・グリフ配置・可視窓）が全注入時刻で
/// **ビット一致**する（レイアウトは画像座標空間で完結＝k 非依存の設計原則・R4.6 前半）。
///
/// 幾何は fixture 実測値（負値=反対辺基準・wordwrappoint.x,-49／validrect right,-44 等）を
/// 使う——負値解決は画像原寸に依存するため、k 混入（物理 px がレイアウトへ漏れる変異）は
/// 閾値/右辺のズレ→折返し位置の差としてここで赤くなる（絶対座標だけの幾何では k 混入を
/// 検出できない——檻の有意性の要）。
///
/// 一致だけの空虚な檻にしないため、絶対値も固定する: 自動折返し（閾値 351・font 40 で
/// 7 グリフ/行）・明示改行・3 行あふれ→縦スクロール（first_visible_line=1・
/// block_offset=−42）まで通しで効いていること。
#[test]
fn layout_decision_is_scale_independent_for_horizontal_text() {
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
    // 注入時刻列（リビール途中〜全量・2 の冪グリッド＋飽和点）。
    let probe_times = [0.0, 1.0, 2.25, 2.75, 10.0];

    let mut world = World::new();
    let baseline_binding = binding(&mut world, 1.0, physical_surface(IMAGE, 1.0), IMAGE);
    let baseline = decide_layout(&state, &actor, &m, &baseline_binding, 40.0, &probe_times);

    for k in [1.25f32, 2.0] {
        let b = binding(&mut world, k, physical_surface(IMAGE, k), IMAGE);
        let decisions = decide_layout(&state, &actor, &m, &b, 40.0, &probe_times);
        assert_eq!(
            decisions, baseline,
            "k={k}: レイアウト決定（折返し・行矩形・可視窓）は k=1.0 とビット一致する"
        );
    }

    // 絶対値の檻（t=10.0 全量リビール時）: 折返し（7＋3）＋明示改行で 3 行・あふれ発火。
    let full = baseline.last().expect("観測がある");
    assert_eq!(full.visible, 12, "全 12 グリフがリビール済み");
    assert_eq!(full.lines.len(), 3, "自動折返し＋明示改行で 3 行");
    assert_eq!(
        full.lines[0].glyphs.len(),
        7,
        "行 0 は閾値 351（負値 -49 の反対辺解決）で 7 グリフ（8 個目で自動折返し）"
    );
    assert_eq!(
        full.lines[0].glyphs[0].inline_pos, 36.0,
        "行内開始＝書字開始角 x=36（origin 未宣言時の縮退・image px）"
    );
    assert_eq!(
        full.window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -42.0
        },
        "3 行目の下端 46+2×42+40=170 > 168（bottom,-56 の反対辺解決）で縦スクロール発火（横書き）"
    );
}

/// レイアウト決定が依存してよいのは `image_size` だけで k そのものではない——
/// **異なる k・異なる物理 surface** でも `image_size` が同一なら、レイアウト決定が
/// ビット一致する（image px 原寸だけがレイアウト入力になる経路の檻）。
#[test]
fn same_image_size_yields_identical_layout_regardless_of_scale() {
    let m = model(
        (None, None),
        (Some(-49), Some(0)),
        (Some(46), Some(-56), Some(36), Some(-44)),
        None,
    );
    let items: Vec<TextItem> = "あいうえおかきくけこ"
        .chars()
        .map(|ch| TextItem::Glyph { ch })
        .collect();

    let mut world = World::new();
    // 同じ image px 原寸 401×398 を、k=1.0（物理 401×398）と k=1.25（物理 501×498）で与える。
    // k<1 側（k=0.8・物理 321×318）も加え、k の大小が判断に混入しないことまで固定する。
    let via_identity = binding(&mut world, 1.0, (401, 398), (401, 398));
    let via_quarter = binding(&mut world, 1.25, (501, 498), (401, 398));
    let via_sub_unity = binding(&mut world, 0.8, (321, 318), (401, 398));
    assert_eq!(via_identity.image_size, (401, 398));
    assert_eq!(
        via_quarter.image_size, via_identity.image_size,
        "k=1.25 でも image px 原寸は同一"
    );
    assert_eq!(
        via_sub_unity.image_size, via_identity.image_size,
        "k<1 でも image px 原寸は同一（旧導出はここで 1px ずれ得た）"
    );

    // font 40（全角送り 40）: 開始 x=36・閾値 352 → 7 グリフ目の終端 316 の次で
    // 316+40=356 > 352 が発火し 2 行になる（折返しが実際に効く幾何）。
    let layout_of = |b: &TextSlotBinding| -> (Vec<PositionedLine>, VisibleWindow) {
        let resolved = ResolvedBalloonText::resolve(&m, b.image_size);
        let lines = LayoutEngine::layout(
            &items,
            items.len(),
            &resolved.region,
            resolved.mode,
            40.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
        (lines, window)
    };
    let (lines_a, window_a) = layout_of(&via_identity);
    let (lines_b, window_b) = layout_of(&via_quarter);
    assert_eq!(
        lines_a, lines_b,
        "同一 image_size → 同一レイアウト（k 非依存）"
    );
    assert_eq!(window_a, window_b);
    // 空虚一致の排除: wordwrappoint.x,-49 → 401-49=352 で折返しが実際に起きている。
    assert!(
        lines_a.len() > 1,
        "折返しが発生する幾何であること（檻の有意性）"
    );
}

// ══ R11.9 構造側（縦書き）: vertical_rl／vertical_lr でも可視窓決定まで k 非依存 ══

/// 縦書き両方向でも「同一 balloon 定義（writing_mode 宣言込み）・同一 items」に対する
/// レイアウト決定（列送り・横スクロール可視窓）が k=1.0/1.25/2.0 でビット一致する。
/// あふれ境界の辺は**負値（反対辺基準）**で与える（rl: left,-40→360・lr: right,-360→40）
/// ——解決が画像原寸に依存するため k 混入の変異はスクロール発火の差として赤くなる。
/// 絶対値の檻（font 10 → 列送り 10+2=12）: vertical_rl は最新列の左端 400−10−3×12=354
/// < validrect.left 360 で内容が右（+12）へ、vertical_lr は最新列の右端 10+3×12=46 >
/// validrect.right 40 で内容が左（−12）へ
/// （layout.rs の檻と同一の解決後幾何・軸読み替え正準表）。
#[test]
fn layout_decision_is_scale_independent_for_vertical_modes() {
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
        items.push(TextItem::Glyph { ch: 'あ' });
    }

    let mut world = World::new();
    for (mode_str, (top, bottom, left, right), want_offset) in cases {
        let m = model(
            (None, None),
            (None, None),
            (Some(top), Some(bottom), Some(left), Some(right)),
            Some(mode_str),
        );
        let mut baseline: Option<(Vec<PositionedLine>, VisibleWindow)> = None;
        for k in SCALES {
            let b = binding(&mut world, k, physical_surface(IMAGE, k), IMAGE);
            let resolved = ResolvedBalloonText::resolve(&m, b.image_size);
            assert_eq!(
                resolved.mode,
                WritingMode::resolve(&m),
                "writing_mode 解決はスケールと無関係"
            );
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
            match &baseline {
                None => {
                    // 絶対値の檻（k=1.0 の初回・以降の k はこの値とビット一致で縛る）。
                    assert_eq!(lines.len(), 4, "{mode_str}: 4 列が配置される");
                    assert_eq!(
                        window,
                        VisibleWindow {
                            first_visible_line: 1,
                            block_offset: want_offset
                        },
                        "{mode_str}: 4 列目のあふれで横スクロール発火"
                    );
                    baseline = Some((lines, window));
                }
                Some((base_lines, base_window)) => {
                    assert_eq!(
                        &lines, base_lines,
                        "{mode_str} k={k}: 列レイアウトは k=1.0 とビット一致する"
                    );
                    assert_eq!(&window, base_window, "{mode_str} k={k}: 可視窓も一致する");
                }
            }
        }
    }
}

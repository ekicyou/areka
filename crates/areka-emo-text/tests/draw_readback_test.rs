//! # draw_readback_test — UI スレッド上の描画・読み戻しの通し構造検証
//!
//! 出典 spec: `areka-P0-emo-text-layer`（このファイルが挙げる要件の番号と design.md はこの spec のもの）。
//!
//! design.md「Testing Strategy > Integration Tests #4（描画実行）」の正典述語を、
//! **通し経路**（sink emit → UI ドレイン → `present_frame` → `read_back`）で檻化する
//! （R7.3/R8.5/R9.1/R9.3）:
//!
//! 1. **単調増加**: 可視グリフ数が増えるほど供給面の非透明ピクセルが単調に増加する。
//! 2. **Clear 後の全域透明**: `Clear` cue の通し適用（sink 経由＝`clear_cache` 込み）後、
//!    供給面は全域透明（premultiplied 全 0）へ戻る。
//! 3. **有効領域（validrect）外に非透明ピクセルなし**: design の構造（COM 層「装着」正典:
//!    物理寸＝`ceil(validrect 寸 × k)`・`Arrangement` offset＝validrect 原点 × k）により、
//!    テキストのインクは合成結果上 validrect の外に存在し得ない。これを観測可能な構造
//!    述語に落とす——(a) 供給面が validrect 寸**のみ**を覆う（`surface.size()`／readback
//!    長）、(b) 提示位置が validrect 原点に一致（`Arrangement`）、(c) 面内のインクが
//!    書字開始角（validrect-local 原点側）から始まる（領域オフセットの
//!    二重適用＝面内で (36,46) へずれる変異を殺す）。インクの行矩形はみ出し（縦書きで
//!    数 px 超え得る）は供給面クリップで validrect 内に閉じる。
//!
//! in-source の檻（`src/draw_oracle_tests.rs`）が **DrawExecutor 単体**（layout/canvas/render を
//! テストが直結）で同種の述語を張るのに対し、本テストは結線層〜COM 層の**実配線**
//! （`spawn_emo_text` の実 pump ドレイン・`present_frame` の装着/描画/提示）を通す。
//!
//! - GPU/WUC は headless 実資源（`GraphicsCore::new()`）＝ attach_wiring_test
//!   と同一方針。時刻は常に注入（`talk_time`）・実時間 sleep 不使用（決定論）。
//! - R9.1: 横書きテストは実 `EmoPresenter` の公開経路（`text_slot_view` →
//!   `register_actor_view`）で予約スロットへ到達する。
//! - R9.3: グリフ更新フレームは供給面の Present のみで完結する（World の entity 数
//!   不変・`GraphicsCommandList` 不挿入＝バルーン surface 本体の再合成を強要しない）。
//! - R8.5: 今の実挙動はテキスト描画のみ＝グリフ未リビール時（可視 0）と Clear 後に
//!   インク源が存在しない（Image/Surface シーム住人の inert は `src/canvas.rs` の檻が正典・
//!   本テストはその通し側の帰結「テキスト以外のインク源なし」を assert する）。

#[path = "support/mod.rs"]
mod support;

use support::{
    build_target_assets, make_world_with_gpu, opaque_count, pump_until_idle, read_back,
    show_surface, spawn_reserved_slot,
};

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_present::{EmoPresenter, TargetId};
use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame, spawn_emo_text,
};
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use bevy_ecs::prelude::World;
use wintf::ecs::{Arrangement, DPI, GraphicsCommandList, VisualGraphics};

// ── 幾何定数（fixture 実測値系の非退化 validrect——負値=反対辺の実解決を通す） ──────────

/// バルーン画像原寸（image px・k=1.0 恒常ゆえ物理原寸と同値）。
const IMAGE: (u32, u32) = (200, 150);
/// validrect 定義（top 46 / bottom -56 / left 36 / right -44——fixture 正準の実値）。
/// 解決結果: 絶対 (36,46)-(156,94) ＝ 原点 (36,46)・寸 (120,48)。
const VR_ORIGIN: (f32, f32) = (36.0, 46.0);
const VR_SIZE: (u32, u32) = (120, 48);
/// 既定フォント高さ 12（image px）・行送り pitch = 12 + 行間 2 = 14。
const FONT_H: u32 = 12;
const PITCH: u32 = 14;

/// 非退化 validrect 付きのテスト用 BalloonModel。**origin は宣言しない**＝描画開始点は
/// 書字開始角へ縮退する（region.rs の fixture 檻と同じ経路・要件 3.11）。
/// wordwrap 未指定＝折返し閾値は行末辺（横=right 156／縦=bottom 94）。
///
/// かつては `Origin::new(Some(0), Some(0))` を宣言していたが、これは解決後 validrect
/// (36,46)-(156,94) の外にあり、範囲外の宣言として書字開始角へ寄せられて
/// いた。本檻の意図は書字開始角の縮退（そこから始まるインクの観測）なので、正典推奨形の
/// 未宣言へ直してある（spec `areka-P0-balloon-vertical-canon` の DD5・要件 10.9）。
fn validrect_model(writing_mode: Option<&str>) -> BalloonModel {
    support::geometry_model(
        (None, None),
        (None, None),
        (Some(46), Some(-56), Some(36), Some(-44)),
        None,
        writing_mode,
    )
}

// ── 共通補助 ─────────────────────────────────────────────────────────────────────────

/// reveal 間隔（秒/グリフ）。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、
/// Text cue へ `N × REVEAL_INTERVAL` を焼き込むと interval=0.05 の per-glyph 進行を得る
/// （進行観測は安全マージン付き時刻 0.11/0.16/… で行う）。
const REVEAL_INTERVAL: f64 = 0.05;

/// テスト用 cue（共有の部品の間隔つき cue へ本ファイルの間隔を渡す）。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    support::cue(actor, at, command, REVEAL_INTERVAL)
}

/// 非透明ピクセルの外接矩形 (min_x, min_y, max_x, max_y)（インクなしは None）。
fn ink_bounds(bytes: &[u8], width: u32) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for (i, px) in bytes.chunks_exact(4).enumerate() {
        if px[3] != 0 {
            let (x, y) = (i as u32 % width, i as u32 / width);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    bounds
}

/// 供給面と提示位置の validrect 封じ込め構造（述語 3 の (a)(b)——design COM 層「装着」正典:
/// 物理寸＝ceil(validrect 寸 × k)・Arrangement offset＝validrect 原点 × k・k=1.0）。
/// 供給面が validrect **のみ**を覆い validrect 原点へ提示される＝合成結果上、テキストの
/// インクは validrect 外に存在し得ない（面外のインクは供給面クリップで消える）。
fn assert_validrect_containment(
    rt: &TextLayerRuntime,
    actor: &ActorKey,
    world: &World,
    slot: bevy_ecs::entity::Entity,
) {
    let surface = rt.surface(actor).expect("装着済み actor の供給面");
    assert_eq!(
        surface.size(),
        VR_SIZE,
        "供給面の物理寸は validrect 寸のみ（画像原寸 {IMAGE:?} ではない）"
    );
    let bytes = surface.read_back().expect("read_back");
    assert_eq!(
        bytes.len(),
        (VR_SIZE.0 * VR_SIZE.1 * 4) as usize,
        "readback は validrect 寸の BGRA 密配列＝validrect 外の画素は供給面に存在しない"
    );
    let arr = world
        .get::<Arrangement>(slot)
        .expect("slot に Arrangement（物理 px 直接）が装着される");
    assert_eq!(
        (arr.offset.x, arr.offset.y),
        VR_ORIGIN,
        "提示位置は validrect 原点 × k（k=1.0）＝供給面全体が validrect に一致する"
    );
    assert_eq!(
        (arr.size.width, arr.size.height),
        (VR_SIZE.0 as f32, VR_SIZE.1 as f32),
        "提示寸は validrect 寸 × k（k=1.0）"
    );
}

// ══ 横書き: 3 述語の通し檻（実 EmoPresenter 公開経路・R7.3/R8.5/R9.1/R9.3） ══════════════

/// 観測可能な完了状態（横書き）: 実 sink→UI ドレイン→present_frame→read_back の
/// 通し経路で、(1) 可視グリフ数増加に伴う非透明ピクセルの単調増加、(2) Clear 後の全域
/// 透明化、(3) 有効領域（validrect）外に非透明ピクセルが無いこと（封じ込め構造＋面内
/// 開始角）の 3 述語が green である。併せてスクロールの全域再描画（R7.3・残渣なし・
/// 決定論）と Present 完結（R9.3・World 非再合成）を檻化する。
#[test]
fn full_path_ink_monotonic_clear_transparent_and_contained_in_validrect() {
    let mut world = make_world_with_gpu();

    // ── R9.1: 実 EmoPresenter の公開経路（text_slot_view）で予約スロットへ到達する ──
    // 窓 entity には `DPI` component を**明示的に**載せる（areka-P0-emo-dpi-scaling task 3.2）。
    // component 不在は `derive_scale` の縮退分岐（error! ＋ k=1.0）を通るため、不在に頼ると
    // 縮退が正常経路になりすまして観測できない。作者基準 DPI 96 と揃えて k=1.0 を成立させる。
    let window = world.spawn(DPI::from_dpi(96, 96)).id();
    let (emo_world, atlas) = build_target_assets(IMAGE.0, IMAGE.1, 0);
    let mut presenter = EmoPresenter::new();
    presenter
        // 作者基準 DPI は正典既定の 96（ukadoc・D1）。本番は boot が descript の実値を
        // 供給する（本テストは窓 DPI 96 と揃えて k=1.0＝従来と同一の表示寸・描画結果）。
        .attach_target(TargetId(0), window, emo_world, atlas, 96);
    show_surface(&mut presenter, &mut world, TargetId(0));
    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(
        view.surface_size(),
        IMAGE,
        "バルーン原寸＝画像原寸（k=1.0）"
    );
    let slot = view.slot();

    let actor = ActorKey::from("0");
    let model = validrect_model(None);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor_view(actor.clone(), &view, &model);

    // ── セッション 1: 5 グリフ（at=0.1）を実 sink→UI ドレインで状態機械へ ──
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    sink.emit(cue("0", 0.1, CueCommand::Text("あいうえお".into())));
    pump_until_idle();

    // 装着フレーム（t=0.0 ＜ at=0.1 → 可視 0）: 装着は成立し、供給面は全域透明。
    // R8.5 の通し側: 実配線のインク源はテキストのみ＝グリフ未リビールなら面は空。
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム（可視 0）");
        assert!(
            rt.is_attached(&actor),
            "cue 到着済み actor は可視 0 でも装着される"
        );
    }
    let entities_after_attach = world.entities().len();
    {
        let rt = runtime.borrow();
        let bytes = read_back(&rt, &actor);
        assert!(
            bytes.iter().all(|&b| b == 0),
            "可視 0 グリフの供給面は全域透明（テキスト以外のインク源が無い・R8.5 通し側）"
        );
        // ── 述語 3（構造）: validrect 封じ込め＋装着契約（R9.1/R9.3） ──
        assert_validrect_containment(&rt, &actor, &world, slot);
        let vg = world
            .get::<VisualGraphics>(slot)
            .expect("slot に VisualGraphics（emo 自前 brush）が装着される");
        assert!(vg.is_valid(), "装着された brush は有効");
        assert!(
            world.get::<GraphicsCommandList>(slot).is_none(),
            "GraphicsCommandList は挿入しない（wintf 描画系と競合しない・R9.3 構造）"
        );
    }

    // ── 述語 1: 注入時刻フレームでの単調増加（r = [0.1, 0.15, 0.20, 0.25, 0.30]） ──
    let mut counts = vec![0usize];
    for t in [0.11, 0.16, 0.21, 0.26, 0.31] {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, t).expect("リビール進行フレーム");
        drop(rt);
        counts.push(opaque_count(&read_back(&runtime.borrow(), &actor)));
    }
    for i in 1..counts.len() {
        assert!(
            counts[i] > counts[i - 1],
            "非透明ピクセルは可視グリフ数とともに単調増加する（通し経路）: {counts:?}"
        );
    }

    // ── 述語 3（面内 (c)）: インクは書字開始角（validrect-local 原点側）
    //    から始まる——領域オフセットの二重適用（面内で (36,46) へずれ、下端の内容が
    //    validrect の外相当へ押し出される変異）を殺す ──
    {
        let rt = runtime.borrow();
        let (min_x, min_y, _, _) =
            ink_bounds(&read_back(&rt, &actor), VR_SIZE.0).expect("インクが描かれる");
        assert!(
            min_x < FONT_H && min_y < PITCH,
            "インクは書字開始角（validrect-local 原点側）から始まる: min=({min_x},{min_y})"
        );
    }

    // ── セッション 2: 行 1〜2 を追加（3 行＝validrect 高 48 に収まる上限） ──
    //    行 i の下端 = 14i + 12（pitch 14 ＝ 字の丈 12 ＋ 行間 2）→ 12／26／40／54。
    //    3 行目 40 ≤ 48 は収まり 4 行目 54 > 48 であふれる。
    sink.emit(cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 1.0, CueCommand::Text("■".into())));
    sink.emit(cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 1.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let before = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 5.0).expect("全リビール済みフレーム（3 行）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&before) > counts[5],
        "追加行のインクが載る（あふれ前・3 行とも可視）"
    );
    // 決定論: 同一注入時刻の再提示はバイト一致（全域再描画に差分累積なし・R7.3）。
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 5.0).expect("同一時刻の再提示フレーム");
        drop(rt);
        assert_eq!(
            read_back(&runtime.borrow(), &actor),
            before,
            "同一入力→同一ピクセル（通し経路の決定論）"
        );
    }

    // ── R7.3: 行 3 追加であふれ→スクロール＝validrect 寸キャンバスへの全域再描画 ──
    sink.emit(cue("0", 6.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 6.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let after = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 10.0).expect("あふれ後フレーム");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&after) < opaque_count(&before),
        "スクロールで先頭行（5 グリフ）が全域再描画により消える（残渣なし）: before={} after={}",
        opaque_count(&before),
        opaque_count(&after)
    );
    let (_, min_y, _, _) = ink_bounds(&after, VR_SIZE.0).expect("スクロール後もインクあり");
    assert!(
        min_y < PITCH,
        "可視窓の先頭行は領域先頭へ詰めて描かれる（min_y={min_y}）"
    );

    // ── 述語 2: Clear の通し適用（sink 経由＝clear_cache 込み）→ 全域透明 ──
    sink.emit(cue("0", 11.0, CueCommand::Clear));
    sink.close();
    pump_until_idle();
    {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 12.0).expect("Clear 後フレーム");
        drop(rt);
        let rt = runtime.borrow();
        let bytes = read_back(&rt, &actor);
        assert_eq!(
            bytes.len(),
            (VR_SIZE.0 * VR_SIZE.1 * 4) as usize,
            "Clear 後も供給面は validrect 寸のまま"
        );
        assert!(
            bytes.iter().all(|&b| b == 0),
            "Clear 後は全域が透明（premultiplied 全 0）へ戻る"
        );
    }

    // ── R9.3: 装着後の全フレーム（進行・スクロール・Clear）は Present のみで完結 ──
    assert_eq!(
        world.entities().len(),
        entities_after_attach,
        "グリフ更新フレームは World に entity を追加しない（供給面の提示のみ・R9.3）"
    );
    assert!(
        world.get::<GraphicsCommandList>(slot).is_none(),
        "全フレーム経過後も GraphicsCommandList は不挿入のまま（surface 再合成を強要しない）"
    );
}

// ══ 縦書き（vertical_rl）変種: 3 述語＋横方向スクロール（R6 系の軸切替を通し経路で） ══════

/// 観測可能な完了状態（縦書き変種）: 縦書き（vertical_rl）でも 3 述語が
/// green である——(1) 単調増加、(2) Clear 後全域透明、(3) validrect 封じ込め（縦書きの
/// インク行矩形はみ出しも供給面クリップで validrect 内に閉じる）。
/// 列は validrect 右端から左へ流れ、あふれは横方向スクロール（先頭列の消失）になる。
#[test]
fn vertical_rl_full_path_reveals_scrolls_horizontally_and_clears_within_validrect() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);

    let actor = ActorKey::from("0");
    let model = validrect_model(Some("vertical_rl"));
    let binding = TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE);
    let resolved = ResolvedBalloonText::resolve(&model, binding.image_size);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor(actor.clone(), binding, resolved);

    // ── セッション 1: 列 0＝■■（at=0.0）＋列 1〜7＝■ 各 1（at=0.5）
    //    → 列 i の左端は 120 − 12 − 14i（右上起点・pitch 14）。列 7 は左端 10 ≥ 0 で収まり、
    //    列 8 は左端 −4 < 0 であふれる＝8 列があふれ前の上限 ──
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    sink.emit(cue("0", 0.0, CueCommand::Text("■■".into())));
    for _ in 0..7 {
        sink.emit(cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }));
        sink.emit(cue("0", 0.5, CueCommand::Text("■".into())));
    }
    pump_until_idle();

    // ── 述語 1（縦書き）: r = [0.0, 0.05] → t=0.0 で 1 グリフ・t=0.06 で 2 グリフ ──
    let n1 = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.0).expect("縦書き 1 グリフ目フレーム");
        drop(rt);
        opaque_count(&read_back(&runtime.borrow(), &actor))
    };
    assert!(n1 > 0, "縦書き 1 グリフ目のインクが載る");
    // 書字開始角＝validrect 右上（軸読み替え正準表）: 列 0 は面の右端側
    // （x ∈ [幅−pitch, 幅)）に現れる——横書き配置（左端開始）との取り違えを殺す。
    {
        let rt = runtime.borrow();
        let (min_x, min_y, _, _) =
            ink_bounds(&read_back(&rt, &actor), VR_SIZE.0).expect("インクあり");
        assert!(
            min_x >= VR_SIZE.0 - PITCH - FONT_H,
            "縦書きの列 0 は validrect-local 右端側から始まる（min_x={min_x}）"
        );
        // ここの `PITCH` は行送り（ブロック軸）ではなく「上端側と言える小さな上界」として
        // 借りているだけ（縦書きでは y が字の並ぶ軸）。行送りの式が変わっても意味は変わらない。
        assert!(
            min_y < PITCH,
            "列内は上→下＝1 グリフ目は上端側（min_y={min_y}）"
        );
    }
    let n2 = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 0.06).expect("縦書き 2 グリフ目フレーム");
        drop(rt);
        opaque_count(&read_back(&runtime.borrow(), &actor))
    };
    assert!(
        n2 > n1,
        "縦書きでも非透明ピクセルは可視グリフ数とともに増加する: {n1} -> {n2}"
    );

    // 全リビール（8 列・■×9）＝あふれ前の基準。述語 3 の封じ込め構造も縦書きで成立。
    let before = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 10.0).expect("全リビール済みフレーム（8 列）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(opaque_count(&before) > n2, "8 列分のインクが載る");
    {
        let rt = runtime.borrow();
        assert_validrect_containment(&rt, &actor, &world, slot);
        let (min_x, _, max_x, _) = ink_bounds(&before, VR_SIZE.0).expect("インクあり");
        assert!(
            min_x < PITCH && max_x >= VR_SIZE.0 - PITCH - FONT_H,
            "列は右端（列 0）から左端（列 7）まで右→左に流れる: min_x={min_x} max_x={max_x}"
        );
    }

    // ── R7.3×縦書き: 列 8 追加であふれ→横方向スクロール（先頭列＝■■ の消失） ──
    sink.emit(cue("0", 11.0, CueCommand::NewLine { ratio: 1.0 }));
    sink.emit(cue("0", 11.0, CueCommand::Text("■".into())));
    pump_until_idle();
    let after = {
        let mut rt = runtime.borrow_mut();
        present_frame(&mut rt, &mut world, 20.0).expect("あふれ後フレーム（縦書き）");
        drop(rt);
        read_back(&runtime.borrow(), &actor)
    };
    assert!(
        opaque_count(&after) < opaque_count(&before),
        "横方向スクロールで先頭列（■■）が全域再描画により消える: before={} after={}",
        opaque_count(&before),
        opaque_count(&after)
    );
    let (_, _, max_x, _) = ink_bounds(&after, VR_SIZE.0).expect("スクロール後もインクあり");
    assert!(
        max_x >= VR_SIZE.0 - PITCH - FONT_H,
        "可視窓の先頭列は右端へ詰めて描かれる（max_x={max_x}）"
    );

    // ── 述語 2（縦書き）: Clear の通し適用 → 全域透明 ──
    sink.emit(cue("0", 21.0, CueCommand::Clear));
    sink.close();
    pump_until_idle();
    let mut rt = runtime.borrow_mut();
    present_frame(&mut rt, &mut world, 22.0).expect("Clear 後フレーム（縦書き）");
    drop(rt);
    let bytes = read_back(&runtime.borrow(), &actor);
    assert_eq!(bytes.len(), (VR_SIZE.0 * VR_SIZE.1 * 4) as usize);
    assert!(
        bytes.iter().all(|&b| b == 0),
        "縦書きでも Clear 後は全域が透明へ戻る"
    );
}

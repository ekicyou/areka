use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink};
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{GraphicsCommandList, GraphicsCore, VisualGraphics, WucGraphicsResource};

use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame, spawn_emo_text};
use crate::state::TextLayerConfig;
use crate::wrap::WrapMode;
use super::test_support::{cue, geo_model, opaque_count, pump_until_idle, spawn_reserved_slot};

// ── ログ檻（WARN/ERROR 件数を数える最小 Subscriber・sink.rs の檻パターン踏襲） ──

struct LevelCounter {
    warns: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,
}

impl tracing::Subscriber for LevelCounter {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        match *event.metadata().level() {
            tracing::Level::WARN => {
                self.warns.fetch_add(1, Ordering::SeqCst);
            }
            tracing::Level::ERROR => {
                self.errors.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// クロージャをログ檻の中で実行し、（結果, WARN 件数, ERROR 件数）を返す。
fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let warns = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));
    let subscriber = LevelCounter {
        warns: Arc::clone(&warns),
        errors: Arc::clone(&errors),
    };
    let out = tracing::subscriber::with_default(subscriber, f);
    (
        out,
        warns.load(Ordering::SeqCst),
        errors.load(Ordering::SeqCst),
    )
}

// ══ 実 pump 3 経路（R1.2/R1.3/R1.4/R1.5——観測可能な完了状態の前半） ══

/// ケース1: 終了指示（`close()`）——実 pump 上で cue が UI ドレイン経由で状態機械へ
/// 適用され（R1.2/R1.3）、Close 受領で drain が error ログなしにクリーン終了する（R1.4）。
/// drain 終了は「handler クロージャ（runtime の Rc clone を捕捉）の drop」を
/// `Rc::strong_count` で決定論観測する。
#[test]
fn close_terminates_drain_cleanly_after_applying_cues_on_real_pump() {
    let ((), _warns, errors) = with_log_cage(|| {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (mut sink, _handle) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
        assert_eq!(
            Rc::strong_count(&runtime),
            2,
            "drain handler が runtime を保持する"
        );

        sink.emit(cue("0", 0.0, CueCommand::Text("アヒル".into())));
        sink.emit(cue("0", 0.2, CueCommand::NewLine { ratio: 1.0 }));
        sink.close();

        pump_until_idle();

        // cue は UI ドレイン経由で状態機械へ適用済み（R1.2——描画状態の更新）。
        {
            let rt = runtime.borrow();
            let actor = ActorKey::from("0");
            let state = rt
                .state()
                .actor_state(&actor)
                .expect("actor state が生成される");
            assert_eq!(
                state.items().len(),
                4,
                "グリフ 3＋改行マーカー 1 が追記される"
            );
            // 注入時刻でのリビール進行（reveal interval=0.05〔duration 由来〕・丸め安全マージン付き時刻）。
            assert_eq!(rt.state().visible_glyphs(&actor, 0.0), 1);
            assert_eq!(rt.state().visible_glyphs(&actor, 0.06), 2);
            assert_eq!(rt.state().visible_glyphs(&actor, 0.11), 3);
        }

        // 送信元（sink）は生存したまま＝終了は Close（Ok(Break)）経路のみでありうる。
        assert_eq!(
            Rc::strong_count(&runtime),
            1,
            "Close 受領で drain future が drop されクリーン終了する"
        );
        drop(sink);
    });
    assert_eq!(
        errors, 0,
        "終了指示によるクリーン終了は error ログを伴わない"
    );
}

/// ケース2: 全送信元切断（全 `EmoTextSink` clone drop）——queue 済み cue を適用し切った
/// 上で error ログなしにクリーン終了する（R1.4）。
#[test]
fn dropping_all_sinks_terminates_drain_cleanly_after_applying_queued_cues() {
    let ((), _warns, errors) = with_log_cage(|| {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (mut sink, _handle) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
        let sink2 = sink.clone();

        sink.emit(cue("1", 0.0, CueCommand::Text("残置".into())));
        drop(sink);
        drop(sink2);

        pump_until_idle();

        let rt = runtime.borrow();
        let actor = ActorKey::from("1");
        let state = rt
            .state()
            .actor_state(&actor)
            .expect("切断前に queue 済みの cue は届く");
        assert_eq!(
            state.items().len(),
            2,
            "切断前の cue は破棄されず適用される"
        );
        drop(rt);

        assert_eq!(
            Rc::strong_count(&runtime),
            1,
            "全送信元切断で drain future が drop されクリーン終了する"
        );
    });
    assert_eq!(
        errors, 0,
        "全送信元切断によるクリーン終了は error ログを伴わない"
    );
}

/// ケース3: 個別失敗継続——runtime が借用中（UI スレッド上の別処理が保持）で cue 適用に
/// 失敗しても panic せず error 1 件で drain は継続し（R1.5）、借用解放後の後続 cue は
/// 適用され、Close でクリーン終了する。
#[test]
fn cue_failure_is_logged_and_drain_continues_processing_subsequent_cues() {
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");

    // 借用を保持したまま pump——drain handler は try_borrow_mut に失敗し Err（基盤が
    // error!＋継続）。当該 cue は失われるが drain は死なない。
    let ((), _warns, errors) = with_log_cage(|| {
        let guard = runtime.borrow_mut();
        sink.emit(cue("0", 0.0, CueCommand::Text("失われる".into())));
        pump_until_idle();
        drop(guard);
    });
    assert_eq!(
        errors, 1,
        "個別適用失敗はちょうど 1 件の error ログとして記録される"
    );
    assert_eq!(
        Rc::strong_count(&runtime),
        2,
        "個別失敗は終了経路ではない——drain は生き続ける"
    );

    // 借用解放後の後続 cue は受理・適用され、Close でクリーン終了する。
    let ((), _warns, errors) = with_log_cage(|| {
        sink.emit(cue("0", 0.5, CueCommand::Text("後続".into())));
        sink.close();
        pump_until_idle();
    });
    assert_eq!(
        errors, 0,
        "失敗後の後続 cue 適用と Close 終了は error ログを伴わない"
    );

    let rt = runtime.borrow();
    let actor = ActorKey::from("0");
    let state = rt
        .state()
        .actor_state(&actor)
        .expect("後続 cue が適用される");
    assert_eq!(
        state.items().len(),
        2,
        "失敗 cue は失われ、後続 cue のみ適用される"
    );
    drop(rt);
    assert_eq!(Rc::strong_count(&runtime), 1, "Close でクリーン終了する");
}

// ══ フレーム提示: 未解決 actor の蓄積＋スキップ＋再試行（純粋・COM 不要） ══

/// 未解決（binding 未登録）actor の cue は状態を蓄積し、present_frame は Ok のまま
/// 描画をスキップして次フレームで再試行する（actor ごと初回のみ warn!・以降 debug!——
/// design Error Handling）。
#[test]
fn present_frame_accumulates_and_skips_unresolved_actor_with_warn_once() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("蓄積".into())));
    let mut world = World::new(); // COM 資源なし——未解決 skip は資源に触れない

    let ((), warns, errors) = with_log_cage(|| {
        present_frame(&mut rt, &mut world, 0.0).expect("未解決 actor は skip＝frame は Ok");
        present_frame(&mut rt, &mut world, 0.1).expect("再試行フレームも Ok");
    });
    assert_eq!(
        warns, 1,
        "未解決 actor の warn は actor ごと初回のみ（2 フレーム目は debug）"
    );
    assert_eq!(
        errors, 0,
        "未解決 skip は error ではない（蓄積＋再試行の正常経路）"
    );

    let actor = ActorKey::from("0");
    assert!(!rt.is_attached(&actor), "未解決のまま装着されない");
    let state = rt
        .state()
        .actor_state(&actor)
        .expect("状態は蓄積継続（無損失）");
    assert_eq!(state.items().len(), 2, "skip 中も cue 状態は失われない");
}

/// binding 登録済みだが COM 資源（GraphicsCore/Compositor）が World に無い場合、
/// present_frame は panic せず `Device` エラーを返す（log-first・次フレーム再試行可能）。
#[test]
fn present_frame_reports_device_error_without_panic_when_resources_missing() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));

    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let binding = TextSlotBinding::new(slot, window, 1.0, (60, 40), (60, 40));
    rt.register_actor(
        ActorKey::from("0"),
        binding,
        ResolvedBalloonText::resolve(&geo_model(), (60, 40)),
    );

    let (result, _warns, errors) = with_log_cage(|| present_frame(&mut rt, &mut world, 0.0));
    let err = result.err().expect("COM 資源不在は Err（panic しない）");
    assert!(
        matches!(err, crate::TextLayerError::Device { .. }),
        "資源不在は Device エラー（log-first）: {err:?}"
    );
    assert!(errors >= 1, "失敗は真因文脈付き error ログを伴う");
    // 状態は無傷＝次フレーム再試行可能。
    assert!(rt.state().actor_state(&ActorKey::from("0")).is_some());
    assert!(!rt.is_attached(&ActorKey::from("0")));
}

// ══ フレーム提示: 装着＋注入時刻駆動の進行＋Present 完結（COM・headless） ══

/// 観測可能な完了状態（後半）: 登録後の present_frame が予約スロットへ装着し、
/// 注入時刻フレームごとに可視グリフ（非透明ピクセル）が進行する。装着は初回のみで、
/// 以降のグリフ更新は供給面の提示のみ（World の構造変更なし・GraphicsCommandList
/// 不挿入＝バルーン surface 本体の再合成を要求しない・R9.3）。Clear 後は全透明へ戻る。
#[test]
fn present_frame_attaches_once_and_progresses_glyphs_per_injected_frame() {
    // 本番 UI スレッド（MTA）を再現（WucGraphicsResource::new は DQTAT_COM_NONE を使う
    // ため COM 未初期化スレッドでは失敗する——wintf wuc_resource.rs テストと同一方針）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);

    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));

    // 未登録フレーム: 蓄積＋スキップ（COM 資源があっても binding 未解決なら触れない）。
    present_frame(&mut rt, &mut world, 0.0).expect("未解決 skip フレーム");
    assert!(!rt.is_attached(&actor));

    // 登録→次フレームで再試行が成功し装着される。
    let image = (120u32, 60u32);
    let binding = TextSlotBinding::new(slot, window, 1.0, image, image);
    rt.register_actor(
        actor.clone(),
        binding,
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム");
    assert!(rt.is_attached(&actor), "登録後の再試行で装着される");

    // 装着の構造契約: 有効な VisualGraphics・GraphicsCommandList 不挿入（R9.3 構造）。
    let vg = world
        .get::<VisualGraphics>(slot)
        .expect("slot に VisualGraphics（emo 自前 brush）が装着される");
    assert!(vg.is_valid());
    assert!(
        world.get::<GraphicsCommandList>(slot).is_none(),
        "GraphicsCommandList は挿入しない（wintf 描画系と競合しない）"
    );

    // 注入時刻フレームごとの進行（reveal interval=0.05〔duration 由来〕・r=[0.0, 0.05, 0.10]）。
    let read = |rt: &TextLayerRuntime| -> usize {
        opaque_count(
            &rt.surface(&actor)
                .expect("装着済み actor の供給面")
                .read_back()
                .expect("read_back"),
        )
    };
    let n1 = read(&rt);
    assert!(n1 > 0, "t=0.0 で 1 グリフ目が可視（非透明ピクセルあり）");

    let entities_after_attach = world.entities().len();
    present_frame(&mut rt, &mut world, 0.06).expect("フレーム t=0.06");
    let n2 = read(&rt);
    assert!(
        n2 > n1,
        "t=0.06 で 2 グリフ目まで可視（ピクセル単調増加）: {n1} -> {n2}"
    );

    present_frame(&mut rt, &mut world, 0.11).expect("フレーム t=0.11");
    let n3 = read(&rt);
    assert!(n3 > n2, "t=0.11 で 3 グリフ目まで可視: {n2} -> {n3}");

    // 装着済み actor の更新は Present のみで完結——World の構造は変わらない（R9.3）。
    assert_eq!(
        world.entities().len(),
        entities_after_attach,
        "グリフ更新フレームは World に entity を追加しない（供給面の提示のみ）"
    );
    assert!(
        world.get::<GraphicsCommandList>(slot).is_none(),
        "更新後も GraphicsCommandList は不挿入のまま"
    );

    // Clear cue → 次フレームで全透明へ戻る（clear_cache 経路込み）。
    rt.apply_cue(&cue("0", 0.15, CueCommand::Clear));
    present_frame(&mut rt, &mut world, 0.2).expect("Clear 後フレーム");
    assert_eq!(read(&rt), 0, "Clear 後の供給面は全域透明へ戻る");
}

/// 観測可能な完了状態（task 9・R3.5/R10.3）: 登録済み actor に対して present_frame 後の
/// 決定論観測統計を `TextLayerRuntime::draw_stats(actor)`（`surface(actor)` と同型の
/// additive アクセサ）から外部が読み出せる。未装着 actor は `None`。
#[test]
fn draw_stats_readable_after_present_frame() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);

    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

    // 未装着 actor は None（読み口の存在自体は装着に依存しない）。
    assert!(
        rt.draw_stats(&actor).is_none(),
        "未装着 actor の draw_stats は None"
    );

    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    let image = (120u32, 60u32);
    let binding = TextSlotBinding::new(slot, window, 1.0, image, image);
    rt.register_actor(
        actor.clone(),
        binding,
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // 装着＋初回描画フレーム（全域ダーティ）→ 統計が読み出せて描画が計上されている。
    present_frame(&mut rt, &mut world, 0.0).expect("装着フレーム");
    let stats = rt
        .draw_stats(&actor)
        .expect("装着済み actor の draw_stats は Some");
    assert!(
        stats.draw_text_layout_calls >= 1,
        "初回フレームで DrawTextLayout が計上される: {stats:?}"
    );
    assert!(
        stats.line_layout_creations >= 1,
        "初回フレームで行 TextLayout 生成が計上される: {stats:?}"
    );

    // 後続フレームで観測値が単調増加する（リビール進行＝現在行の再描画）。
    present_frame(&mut rt, &mut world, 0.06).expect("フレーム t=0.06");
    let stats2 = rt.draw_stats(&actor).expect("draw_stats（t=0.06）");
    assert!(
        stats2.draw_text_layout_calls >= stats.draw_text_layout_calls,
        "描画呼び出し回数は単調非減少: {} -> {}",
        stats.draw_text_layout_calls,
        stats2.draw_text_layout_calls
    );
}

/// task 5 配線の存在チェック（R4.2/R9.1・[test-only-decision-branches]）:
/// balloon model の `budoux_newline` が `ResolvedBalloonText.wrap` へ他の解決値
/// （mode/region/font）と同じ一点解決経路で反映される。ON model（`budoux_newline,1`）
/// → `BudouxWordWrap`・キー無し → `CharByChar`（既定）。`WrapMode::resolve` の受理語彙は
/// wrap.rs の檻、`segment_plan` の境界計算は segment.rs の檻ゆえここでは再テストしない
/// （配線は存在チェック 1 本）。ON 時のみ segment_plan が走る構造保証は present_actor の
/// `match`（OFF アームは segment_plan を呼ばない）が担い、性能/構造の主張につき檻化しない。
#[test]
fn resolve_wires_wrap_mode_from_balloon_model() {
    let image = (120u32, 60u32);
    // ON: budoux_newline,1 を持つ model（`new` の末尾 positional 引数）。
    let on_model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        Some("1".into()),
    );
    let resolved_on = ResolvedBalloonText::resolve(&on_model, image);
    assert_eq!(
        resolved_on.wrap,
        WrapMode::BudouxWordWrap,
        "budoux_newline,1 の model は wrap=BudouxWordWrap を束ねる（他の解決値と同一経路）"
    );
    // OFF: キー無し（geo_model）→ CharByChar（既定・ログなし正常系）。
    let resolved_off = ResolvedBalloonText::resolve(&geo_model(), image);
    assert_eq!(
        resolved_off.wrap,
        WrapMode::CharByChar,
        "キー無しの model は wrap=CharByChar（既定）"
    );
}

/// 観測可能な完了状態（task 6.1・R6.4/R7.4・#6）: `ClearAll` は純粋状態の全スコープ消去に
/// 加え、**装着済み全 actor の描画実行部**へ全域クリアを伝え、既描画サーフェスに古い
/// ピクセルを残さない（`Clear` は cue.actor スコープのみ）。2 actor を装着してインクを載せ、
/// 一方の actor 名で発行した `ClearAll` が**両**供給面を全域透明へ戻すことを readback で固定する
/// （提示層でも全 render をクリアしないと #6＝前会話残留が再現する）。
#[test]
fn clear_all_clears_every_attached_actor_render_not_just_cue_actor() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    let (window0, slot0) = spawn_reserved_slot(&mut world);
    let (window1, slot1) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);

    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

    // 2 actor へテキストを流し、各々の予約スロットへ装着する。
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    rt.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())));
    let image = (120u32, 60u32);
    rt.register_actor(
        sakura.clone(),
        TextSlotBinding::new(slot0, window0, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    rt.register_actor(
        kero.clone(),
        TextSlotBinding::new(slot1, window1, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // 全リビール済み時刻で提示——両供給面にインクが載る。
    present_frame(&mut rt, &mut world, 10.0).expect("装着＋描画フレーム");
    let read = |rt: &TextLayerRuntime, a: &ActorKey| -> usize {
        opaque_count(
            &rt.surface(a)
                .expect("装着済み供給面")
                .read_back()
                .expect("read_back"),
        )
    };
    assert!(read(&rt, &sakura) > 0, "actor 0 にインクが載る");
    assert!(read(&rt, &kero) > 0, "actor 1 にインクが載る");
    // ClearAll 前の FullClear 累計（基準）——ここまで FullClear は発生していない。
    let sakura_fc0 = rt.draw_stats(&sakura).expect("draw_stats(0)").full_clears;
    let kero_fc0 = rt.draw_stats(&kero).expect("draw_stats(1)").full_clears;

    // cue.actor="0" で ClearAll を発行——cue が名指ししない actor(1) を含む全スコープの
    // 状態が消え、両描画実行部へ全域クリア（request_clear）が伝わる。
    rt.apply_cue(&cue("0", 11.0, CueCommand::ClearAll));
    present_frame(&mut rt, &mut world, 12.0).expect("ClearAll 後フレーム");
    assert_eq!(
        read(&rt, &sakura),
        0,
        "ClearAll は cue.actor（0）の供給面を全域透明へ戻す"
    );
    assert_eq!(
        read(&rt, &kero),
        0,
        "ClearAll は cue が名指ししない actor（1）の供給面も全域透明へ戻す（#6・全 render クリア）"
    );
    // 両描画実行部が **FullClear**（request_clear 経由）を 1 回ずつ行ったことを固定する。
    // これがないと ClearAll が cue.actor の render しかクリアせず（＝退行）、
    // 名指しされない actor(1) の executor は request_clear を受けない（full_clears 不変）ため
    // 本 assert が退行を捕捉する（可視の透明化は planner の縮退経路でも起こり得るが、
    // FullClear 計上は request_clear が実際に全 render へ届いた証跡）。
    assert_eq!(
        rt.draw_stats(&sakura).expect("draw_stats(0)").full_clears,
        sakura_fc0 + 1,
        "ClearAll は cue.actor（0）の描画実行部に FullClear を 1 回起こす"
    );
    assert_eq!(
        rt.draw_stats(&kero).expect("draw_stats(1)").full_clears,
        kero_fc0 + 1,
        "ClearAll は名指しされない actor（1）の描画実行部にも FullClear を起こす（request_clear が全 render へ届いた証跡）"
    );
}


use super::TextSlotBinding;
use bevy_ecs::prelude::World;

/// k=1.0（現行契約の物理 1:1）: `image_size` は `surface_size` と同値、
/// slot/window/scale/surface_size は透過保持される。
#[test]
fn binding_identity_scale_keeps_surface_size() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 1.0, (434, 687), (434, 687));
    assert_eq!(binding.slot, slot);
    assert_eq!(binding.window, window);
    assert_eq!(binding.scale, 1.0);
    assert_eq!(binding.surface_size, (434, 687));
    assert_eq!(
        binding.image_size,
        (434, 687),
        "k=1.0 のとき image_size == surface_size（k≠1.0 では乖離する＝本 fixture は k=1.0）"
    );
}

/// image px 原寸は**導出せず透過する**（2026-07-30・旧「`round(物理 / k)` 一点導出」の後継檻）。
///
/// k と物理寸から image px 原寸を計算し直す変異（＝旧実装の復活）を殺す。k=1.25・物理
/// (127, 94) に対し旧導出は (102, 75) を返したが、正しい原寸は**呼び手が渡した値**である。
/// 端数を含む物理寸でも原寸は 1bit も動かない——これが「作者画像空間は k 不変」の意味。
#[test]
fn binding_passes_image_size_through_without_deriving_it() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 1.25, (127, 94), (101, 75));
    assert_eq!(
        binding.image_size,
        (101, 75),
        "image px 原寸は透過（旧導出 round(127/1.25)=102 を復活させると落ちる）"
    );
    assert_eq!(binding.surface_size, (127, 94), "物理原寸はそのまま保持");
}

/// k<1 の透過檻（2026-07-30 新設）: k=4/5・物理 (114, 62)・原寸 (142, 77)。
///
/// k<1 の順写像は**縮小写像＝単射でない**: `scale_len` は 142 も 143 も物理 114 へ、
/// 77 も 78 も 62 へ潰す。潰れた情報は割り算では戻らず、旧導出 `round(物理 / k)` は
/// この衝突対のどちらかで必ず 1px 間違える（実測では 114 から 143 を返すため 142 側が落ちる）。
/// 透過にした今は k の大小に依らず厳密。
#[test]
fn binding_passes_image_size_through_at_sub_unity_scale() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 0.8, (114, 62), (142, 77));
    assert_eq!(
        binding.image_size,
        (142, 77),
        "k<1 でも原寸は透過（旧導出は (143, 78) を返して 1px ずれた）"
    );
    assert_eq!(binding.surface_size, (114, 62), "物理原寸はそのまま保持");
}

/// 不正な k（0 以下・非有限）は ScaleContract の縮退規約（warn!＋1.0）へ乗り、
/// binding の scale が物理 1:1 で自己整合する（k の多重適用・混在の構造排除）。
#[test]
fn binding_degrades_invalid_scale_to_identity() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 0.0, (320, 240), (320, 240));
    assert_eq!(
        binding.scale, 1.0,
        "不正 k は 1.0 へ縮退（log-first・panic なし）"
    );
    assert_eq!(binding.image_size, (320, 240));
}

/// 検査用の読み口（`arrange_for_test`）は決まった字幅だけで本番と同じ配置の手順を通る（GPU の資源なし）。
/// 配置の入力の無い場所は `None`。
#[test]
fn arrange_for_test_lays_out_with_fixed_metrics_without_gpu() {
    use super::test_support::{cue, geo_model};
    use super::{ResolvedBalloonText, TextLayerRuntime};
    use crate::layout::FixedMetrics;
    use crate::place::PlaceKey;
    use crate::state::TextLayerConfig;
    use areka_sakura::contract::{ActorKey, CueCommand};

    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("あいう".into())));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    let lines = rt
        .arrange_for_test(&PlaceKey::balloon(&actor), &FixedMetrics, 10.0)
        .expect("配置の入力と状態のある場所は行の列を返す");
    let glyphs: usize = lines.iter().map(|line| line.glyphs.len()).sum();
    assert_eq!(glyphs, 3, "出終わった時刻には届いた 3 字がすべて並ぶ");
    assert!(
        rt.arrange_for_test(
            &PlaceKey::balloon(&ActorKey::from("1")),
            &FixedMetrics,
            10.0
        )
        .is_none(),
        "配置の入力の無い場所は None"
    );
}

/// 先渡しの受け取り（タスク 4.1・要件 2.1・4.1）: 本物の受け口（[`spawn_emo_text`](super::spawn_emo_text)
/// の取り出し）へ先渡しを積んで汲み出すと、実行時が空回しして区間の全文を持ち、受け取りの
/// `debug!` がちょうど 1 行（合図の数・求めた区間の数）出る。続く本番の合図は、状態を進める前に
/// 消去を数える（頭の全消去と途中の `\c` で、その場所の区間の番号が 1 つずつ進む）。
#[test]
fn preview_rehearses_once_and_applied_cues_count_clears() {
    use std::cell::RefCell;
    use std::rc::Rc;

    use areka_sakura::contract::{ActorKey, CueCommand, CueSink};
    use log_capture_kit::capture;

    use super::test_support::{cue, pump_until_idle};
    use super::{TextLayerRuntime, spawn_emo_text};
    use crate::place::PlaceKey;
    use crate::state::TextLayerConfig;

    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
    // 頭の全消去 → 「あい」 → `\c` → 「う」: 区間は「あい」（番号 1）と「う」（番号 2）の 2 つ。
    let cues = vec![
        cue("0", 0.0, CueCommand::ClearAll),
        cue("0", 0.0, CueCommand::Text("あい".into())),
        cue("0", 0.1, CueCommand::Clear),
        cue("0", 0.2, CueCommand::Text("う".into())),
    ];
    let ((), events) = capture(|| {
        sink.preview(&cues);
        pump_until_idle();
    });
    let received: Vec<_> = events
        .iter()
        .filter(|e| e.message() == "先渡しを受け取った——空回しで区間の全文を求めた")
        .collect();
    assert_eq!(received.len(), 1, "受け取りの debug! は 1 行: {events:?}");
    assert_eq!(received[0].level, tracing::Level::DEBUG);
    assert_eq!(received[0].field("cues"), Some("4"), "合図の数");
    assert_eq!(received[0].field("sections"), Some("2"), "求めた区間の数");

    let balloon = PlaceKey::balloon(&ActorKey::from("0"));
    assert_eq!(
        runtime.borrow().lookahead.number(&balloon),
        0,
        "先渡しを受け取った時点の番号は 0（本番の数えはここから）"
    );
    for c in cues {
        sink.emit(c);
    }
    pump_until_idle();
    assert_eq!(
        runtime.borrow().lookahead.number(&balloon),
        2,
        "本番の合図の全消去と `\\c` を 1 つずつ数える"
    );
}

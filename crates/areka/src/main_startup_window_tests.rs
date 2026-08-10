use super::*;
use wintf::ecs::WindowPos;

/// ダミー窓の press イベントを作る。
fn pressed_event(double_click: DoubleClick) -> PointerState {
    PointerState {
        double_click,
        ..Default::default()
    }
}

/// ビルダは最小の窓 entity を生成する: マーカー・Window・WindowStyle・BoxStyle サイズ・
/// OnPointerPressed を持ち、**WindowPos の位置主張を一切持たない**（配置・座標・DPI 非主張）。
#[test]
fn dummy_window_has_minimal_components_and_no_position_claim() {
    let mut world = World::new();
    let dummy = spawn_dummy_window(&mut world);

    // マーカー: despawn/auto-close がダミーのみを狙える。
    assert!(world.get::<DummyWindowMarker>(dummy).is_some());

    // 窓が存在するのに必要な最小コンポーネント。
    assert!(world.get::<Window>(dummy).is_some());
    assert!(world.get::<WindowStyle>(dummy).is_some());

    // 可視・クリック可能な最小サイズ（BoxStyle）を持つ。
    let box_style = world.get::<BoxStyle>(dummy).expect("BoxStyle");
    assert!(box_style.size.is_some(), "ダミー窓は最小サイズを持つべき");

    // ダブルクリック despawn 用の observer を持つ。
    assert!(world.get::<OnPointerPressed>(dummy).is_some());

    // 配置・座標・DPI を一切主張しない: ビルダは WindowPos を明示挿入せず、
    // 具体座標を主張しない。wintf の `on_window_add` フックが CreateWindow 前提として
    // `WindowPos::default()`（位置＝CW_USEDEFAULT ＝「Windows 既定に委ねる」＝座標非主張）を
    // 自動挿入するため、存在する WindowPos は必ず既定値と一致するはずである
    // （＝ビルダ由来の座標ロジックがない証明）。placement は window-placement の領分であり、
    // ダミー窓はそこへ踏み込まない（2026-07-05 placement リジェクト再発防止・R2.5）。
    if let Some(wp) = world.get::<WindowPos>(dummy) {
        assert_eq!(
            *wp,
            WindowPos::default(),
            "ダミー窓は具体座標を主張してはならない（既定 CW_USEDEFAULT のみ許容・liveness プローブに限る）"
        );
    }
    // 念のため: もし位置を持つなら CW_USEDEFAULT（座標非主張の番兵）であること。
    if let Some(pos) = world.get::<WindowPos>(dummy).and_then(|wp| wp.position) {
        assert_eq!(
            (pos.x, pos.y),
            (CW_USEDEFAULT, CW_USEDEFAULT),
            "ダミー窓の位置は CW_USEDEFAULT（既定placement・座標非主張）に限る"
        );
    }
}

/// ビルダはダミー窓に **可視・ヒット可能な子 Rectangle** を 1 枚付ける。
///
/// areka の窓は WUC/DComp GPU 合成（`WS_EX_NOREDIRECTIONBITMAP`）で描画されるため、
/// 描画内容が無い窓は完全透明で画面に見えず、手動ダブルクリックの標的にできない。
/// 不透明な診断用 Rectangle（`Brushes` 付き）を子（`ChildOf` = ダミー窓）として持つことで
/// 窓が実際にレンダリングされる（描画内容を持つ）ことを証明する。既定 `HitTest`（Bounds）で
/// 子はヒット可能＝子上のダブルクリックが窓の `OnPointerPressed` へ bubble する（proven mock pattern）。
#[test]
fn dummy_window_has_visible_rectangle_child() {
    let mut world = World::new();
    let dummy = spawn_dummy_window(&mut world);

    // Rectangle + Brushes を持つ子 entity が 1 枚あり、その ChildOf 親がダミー窓であること。
    let mut query = world.query::<(&Rectangle, &Brushes, &ChildOf)>();
    let children: Vec<_> = query.iter(&world).collect();
    assert_eq!(children.len(), 1, "ダミー窓は可視の子 Rectangle を 1 枚持つべき");

    let (_rect, _brushes, child_of) = children[0];
    assert_eq!(
        child_of.parent(),
        dummy,
        "可視 Rectangle の親はダミー窓であるべき（描画内容 = liveness surface）"
    );
}

/// ダブルクリック（左）でマーカー付きダミー窓を despawn し true を返す。
/// マーカーを持たない entity は残す。
#[test]
fn double_click_left_despawns_all_dummy_windows() {
    let mut world = World::new();
    let dummy = world.spawn(DummyWindowMarker).id();
    let dummy2 = world.spawn(DummyWindowMarker).id();
    let other = world.spawn_empty().id();

    let ev = Phase::Bubble(pressed_event(DoubleClick::Left));
    let handled = on_dummy_pressed(&mut world, dummy, dummy, &ev);

    assert!(handled);
    assert!(world.get_entity(dummy).is_err());
    assert!(world.get_entity(dummy2).is_err());
    assert!(world.get_entity(other).is_ok());
}

/// 左以外のダブルクリックでは despawn しない（false）。
#[test]
fn non_left_double_click_does_not_despawn_dummy() {
    let mut world = World::new();
    let dummy = world.spawn(DummyWindowMarker).id();

    for dc in [DoubleClick::None, DoubleClick::Right, DoubleClick::Middle] {
        let ev = Phase::Bubble(pressed_event(dc));
        assert!(!on_dummy_pressed(&mut world, dummy, dummy, &ev));
    }
    assert!(world.get_entity(dummy).is_ok());
}

/// Tunnel フェーズのダブルクリックは無視する（false・despawn しない）。
#[test]
fn tunnel_phase_double_click_is_ignored_for_dummy() {
    let mut world = World::new();
    let dummy = world.spawn(DummyWindowMarker).id();

    let ev = Phase::Tunnel(pressed_event(DoubleClick::Left));
    assert!(!on_dummy_pressed(&mut world, dummy, dummy, &ev));
    assert!(world.get_entity(dummy).is_ok());
}

// -- 自動 close ゲート `smoke_exit_ms_from`（純粋・env 非依存・task 2.3・R4.1） --

/// env 未設定（`None`）ではゲート発火なし（`None`）＝タスク不投入。
#[test]
fn smoke_exit_ms_unset_yields_none() {
    assert_eq!(smoke_exit_ms_from(None), None);
}

/// 空文字・空白のみは発火なし（`None`）。
#[test]
fn smoke_exit_ms_empty_or_whitespace_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("")), None);
    assert_eq!(smoke_exit_ms_from(Some("   ")), None);
    assert_eq!(smoke_exit_ms_from(Some("\t")), None);
}

/// 非数値は発火なし（`None`）。
#[test]
fn smoke_exit_ms_non_numeric_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("abc")), None);
    assert_eq!(smoke_exit_ms_from(Some("12ms")), None);
    assert_eq!(smoke_exit_ms_from(Some("1.5")), None);
}

/// `"0"` は即時発火（0ms）として `Some(0)`。周辺空白はトリムして受理する。
#[test]
fn smoke_exit_ms_zero_yields_some_zero() {
    assert_eq!(smoke_exit_ms_from(Some("0")), Some(0));
    assert_eq!(smoke_exit_ms_from(Some("  0  ")), Some(0));
}

/// 正の整数はその値をミリ秒として受理する。
#[test]
fn smoke_exit_ms_positive_yields_some() {
    assert_eq!(smoke_exit_ms_from(Some("500")), Some(500));
    assert_eq!(smoke_exit_ms_from(Some(" 1500 ")), Some(1500));
}

/// 負値・`u64` 溢れは発火なし（`None`）＝不正入力はゲート OFF。
#[test]
fn smoke_exit_ms_negative_or_overflow_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("-1")), None);
    // u64::MAX + 1（20 桁）は溢れて None。
    assert_eq!(smoke_exit_ms_from(Some("18446744073709551616")), None);
}

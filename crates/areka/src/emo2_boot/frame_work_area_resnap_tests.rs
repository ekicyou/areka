//! 作業領域の変化を契機とする現寸での再スナップ（task 5.2・設計 C6・要件 5.1／5.2／5.3／5.4／4.7）
//! の決定論テスト。
//!
//! # ここが押さえる是正
//!
//! task 5.1 の同期段は作業領域源を実行時に作り直すが、**窓は 1 枚も動かさない**。拡大率が
//! 一緒に動いたフレームでは拡大率の相（`Changed<DPI>` 駆動）が新しい下端へ射影し直すので
//! 接地点は追随するが、**拡大率が変わらず作業領域だけが変わった**とき（タスクバーを隠す・
//! 位置を変える・多段の表示切替）は `Changed<DPI>` が 1 件も立たない——相は何もせず、
//! 下端吸着のキャラ窓は古い下端へ接地したまま取り残される。
//!
//! [`a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write`] が
//! その形そのものである。是正前は接地点が旧下端に留まって赤くなり、拡大率の相の**後**に
//! 置いた再スナップが入ると、現在の寸のまま新しい下端へ 1 回の書込で移って緑になる。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 本タスクの完了条件には零件の主張が 2 つある——「同一表なら窓書込 0」と「変化の無い
//! フレームでは窓書込 0」。どちらも再スナップを丸ごと無操作にしても恒真で通るので、
//! **同じ駆動口（`FrameHarness::run_placement_phases`）が陽性側でも効くことを同じ
//! テスト本体の中で**続けて固定する
//! （[`an_unchanged_monitor_table_drives_no_resnap_while_a_changed_one_does`]／
//! [`steady_frames_write_nothing_until_the_work_area_moves_again`]）。
//! 窓ごとの側（自分の作業領域が動いていない窓）も同じ形で対にしてある
//! （[`a_change_that_moves_only_the_neighbor_monitor_writes_no_windows`]）。

use wintf::ecs::window::SetWindowPosCommand;

use crate::placement::diag::PlacementRoute;
use crate::placement::follow::{Anchored, BalloonFollow};
use crate::placement::resolver::{Anchor, RectPx};

use super::test_support::{
    FakeReports, FrameHarness, pos_of, s2_monitors_with_neighbor_work_area,
    s2_monitors_with_work_area, s2_neighbor_work_area, s2_taskbar_hidden_work_area,
    s2_work_area_for_dpi,
};

/// 本ファイルが回す拡大率。**遷移の間ずっと動かさない**——動かすと拡大率の相が窓を書いて
/// しまい、「作業領域だけが変わった」という主題が読めなくなる。
const STEADY_DPI: u16 = 96;

/// 拡大率が動く側の水準（拡大率の相との合流を見る 1 件だけが使う）。
const OTHER_DPI: u16 = 192;

/// 定常フレームを何コマ回して「書込 0」を主張するか。
const STEADY_FRAMES: u32 = 5;

/// 当該スコープのキャラ窓の接地点 Y（下端）。
fn ground_y(harness: &FrameHarness, scope: usize) -> i32 {
    harness.ground_point(scope).1
}

/// 指定スコープ・指定種別の窓書込だけを取り出す。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// 起動直後の整地——3 つの源と窓の拡大率をすべて [`STEADY_DPI`] へ揃え、キャラ窓を当該水準の
/// 作業領域下端へ接地させる。
///
/// 拡大率の相の初回 run は永続 `SystemState` の仕様で全窓へマッチするので、ここで 1 度
/// 空回しして消費する（以後のフレームでは真に変化した窓だけが対象になる）。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(STEADY_DPI);
    harness.set_monitor_table_for_dpi(STEADY_DPI);
    harness.set_window_dpi(STEADY_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            ground_y(harness, scope),
            s2_work_area_for_dpi(STEADY_DPI).bottom,
            "前提が崩れている: scope={scope} のキャラ窓が定常水準で作業領域下端へ接地していない"
        );
    }
}

/// タスクバーを隠す構成変更を実行時のモニタ表へ流し込む（拡大率は据え置き）。
fn hide_the_taskbar(harness: &mut FrameHarness) {
    harness.set_monitor_table(s2_monitors_with_work_area(
        STEADY_DPI,
        s2_taskbar_hidden_work_area(STEADY_DPI),
    ));
}

// ---------------------------------------------------------------------------
// 是正（要件 5.1・5.2）
// ---------------------------------------------------------------------------

/// **是正前は赤・是正後は緑**: 拡大率が変わらず作業領域だけが変わったフレームで、下端吸着の
/// キャラ窓が現在の寸のまま新しい下端へ **1 回だけ**書き直される。
///
/// 是正前は拡大率の相が `Changed<DPI>` を 1 件も見つけられず（拡大率は動いていない）、
/// 接地点は旧下端に留まる。
#[test]
fn a_work_area_only_change_lands_the_ground_point_on_the_new_bottom_in_one_write() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    let new_bottom = s2_taskbar_hidden_work_area(STEADY_DPI).bottom;
    let old_bottom = s2_work_area_for_dpi(STEADY_DPI).bottom;
    for scope in harness.scopes().to_vec() {
        assert_eq!(
            ground_y(&harness, scope),
            new_bottom,
            "scope={scope}: 接地点が新しい作業領域下端に載っていない（是正前は旧下端 {old_bottom} に留まる）"
        );
    }

    let writes = harness.drain_writes();
    let char0 = writes_for(&writes, 0, "char");
    assert_eq!(
        char0.len(),
        1,
        "キャラ窓の書込が 1 回ではない（中間矩形を挟んでいる）: {writes:?}"
    );
    assert_eq!(
        char0[0].tag.origin,
        PlacementRoute::WorkAreaResnap.as_str(),
        "経路語が作業領域再スナップの語になっていない（語の定義元は PlacementRoute 1 箇所）"
    );
}

/// 上の**探針の非退化**自己検査: タスクバーを隠す構成変更が実際に作業領域下端を動かす。
///
/// 合成レイアウトが「隠しても下端が同じ」へ縮退したら、再スナップの欠落はどの探針値でも
/// 観測できず、上のテストは「何も起きないから通る」空虚な緑になる。
#[test]
fn hiding_the_taskbar_really_moves_the_work_area_bottom() {
    let visible = s2_work_area_for_dpi(STEADY_DPI).bottom;
    let hidden = s2_taskbar_hidden_work_area(STEADY_DPI).bottom;
    assert_ne!(
        visible, hidden,
        "探針が退化している: タスクバーの表示切替で作業領域下端が動かない"
    );
}

/// 随伴バルーンは**同一フレーム**で窓相対の位置へ移り、追従 offset は 1 bit も変わらない
/// （要件 10.1・完了条件の「追従オフセットは補正しない」）。
///
/// 恒等式は**書込前に読んだ** offset に対して問う——後読み値だけで問うと、恒等式を作った
/// 当人に問い返す恒真形になる。
#[test]
fn the_resnap_moves_the_companion_balloon_in_the_same_frame_without_touching_the_offset() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let char0 = harness.char_window(0);
    let balloon0 = harness.balloon_window(0);
    let before = harness
        .world
        .get::<BalloonFollow>(char0)
        .expect("char 窓は BalloonFollow を持つ")
        .offset();
    let balloon_before = pos_of(&harness.world, balloon0).expect("balloon 位置がある");

    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    // 同一フレーム: 本フレームで積まれた書込にキャラ窓とバルーン窓の双方が居る。
    let writes = harness.drain_writes();
    assert_eq!(
        writes_for(&writes, 0, "char").len(),
        1,
        "キャラ窓が本フレームで書かれていない: {writes:?}"
    );
    assert_eq!(
        writes_for(&writes, 0, "balloon").len(),
        1,
        "随伴バルーンが同一フレームで追随していない: {writes:?}"
    );

    let char_pos = pos_of(&harness.world, char0).expect("char 位置がある");
    let balloon_pos = pos_of(&harness.world, balloon0).expect("balloon 位置がある");
    assert_eq!(
        (balloon_pos.x - char_pos.x, balloon_pos.y - char_pos.y),
        (before.x, before.y),
        "随伴恒等式 balloon − char ≡ BalloonFollow.offset が崩れている"
    );
    let after = harness
        .world
        .get::<BalloonFollow>(char0)
        .expect("char 窓は BalloonFollow を持つ")
        .offset();
    assert_eq!(
        (after.x, after.y),
        (before.x, before.y),
        "追従 offset を補正している（要件 10.1: 窓左上相対のまま補正しない）"
    );
    // 「相対不変」が「何も動かなかった」の言い換えに退化していないこと。
    assert_ne!(
        balloon_pos.y, balloon_before.y,
        "バルーンの絶対位置が動いていない（恒等式が空虚に成立している）"
    );
}

// ---------------------------------------------------------------------------
// 零件の主張と、その陽性の対（同じ駆動口・同じテスト本体）
// ---------------------------------------------------------------------------

/// 零件の主張⑴「同一表なら書込 0」と、その**陽性の対**を 1 本の本体で続けて固定する。
///
/// 前半: 表を触らずにフレームを進めても、同期段は差し替えを返さず（`None`）再スナップは
/// そもそも走らない＝窓書込 0。
/// 後半: **同じ駆動口**へ変化した表を渡せば書込が出る。前半だけだと、駆動が死んでいても
/// 再スナップを丸ごと無操作にしても緑になる。
#[test]
fn an_unchanged_monitor_table_drives_no_resnap_while_a_changed_one_does() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 前半（零件）: 表は据え置き。
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_none(),
        "表に変化が無いのに同期段が差し替えを報告している"
    );
    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "同一表のフレームで窓書込が出ている（要件 5.4・4.7）: {writes:?}"
    );

    // 後半（陽性の対）: 同じ駆動口・同じハーネスで表だけを動かす。
    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    let change = harness.run_placement_phases(&mut source);
    assert!(
        change.is_some(),
        "表を動かしたのに同期段が差し替えを報告しない（駆動が死んでいる）"
    );
    assert!(
        !harness.drain_writes().is_empty(),
        "作業領域が動いたのに窓書込が 1 件も出ない（再スナップが効いていない）"
    );
}

/// 零件の主張⑵「変化の無いフレームでは窓書込 0」と、その**陽性の対**。
///
/// 前半: 再スナップが 1 回書いた**後**に定常フレームを [`STEADY_FRAMES`] コマ回しても、
/// 窓書込は 1 件も出ない（要件 4.7 の churn なし）。
/// 後半: 同じ定常ループの続きで作業領域をもう一度動かせば、また書込が出る。
#[test]
fn steady_frames_write_nothing_until_the_work_area_moves_again() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    assert!(
        !harness.drain_writes().is_empty(),
        "前提が崩れている: 作業領域の変化で 1 度は書かれているはず"
    );

    // 前半（零件）: 何も動かさずに定常フレームを回す。
    for _ in 0..STEADY_FRAMES {
        harness.advance_frame();
        assert!(
            harness.run_placement_phases(&mut source).is_none(),
            "定常フレームで同期段が差し替えを報告している"
        );
        let writes = harness.drain_writes();
        assert!(
            writes.is_empty(),
            "定常フレームで窓書込が出ている（要件 4.7 の churn なし）: {writes:?}"
        );
    }

    // 後半（陽性の対）: 同じループの続きで作業領域を元へ戻す。
    harness.set_monitor_table_for_dpi(STEADY_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    assert!(
        !harness.drain_writes().is_empty(),
        "作業領域が戻ったのに窓書込が 1 件も出ない（定常ループが駆動を殺している）"
    );
    assert_eq!(
        ground_y(&harness, 0),
        s2_work_area_for_dpi(STEADY_DPI).bottom,
        "接地点が元の作業領域下端へ戻っていない"
    );
}

/// 窓ごとの側の零件の主張——**自分の作業領域が動いていない窓は再射影そのものを行わない**
/// （要件 5.2）——と、その陽性の対。
///
/// 前半は隣接モニタ（ゴースト窓が決して居ない側）の作業領域だけを動かす。表そのものは
/// 変わるので同期段は差し替えを報告する（＝駆動は生きている）が、窓書込は 0 でなければ
/// ならない。この形にすると「同期段が発火していないから 0 だった」という逃げ道が塞がる。
#[test]
fn a_change_that_moves_only_the_neighbor_monitor_writes_no_windows() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // 前半（零件）: 隣接モニタの作業領域だけを動かす。
    let neighbor = s2_neighbor_work_area();
    harness.set_monitor_table(s2_monitors_with_neighbor_work_area(
        STEADY_DPI,
        RectPx {
            bottom: neighbor.bottom - 40,
            ..neighbor
        },
    ));
    harness.advance_frame();
    assert!(
        harness.run_placement_phases(&mut source).is_some(),
        "前提が崩れている: 隣接モニタの変化でも表は作り直されるはず"
    );
    let writes = harness.drain_writes();
    assert!(
        writes.is_empty(),
        "自分の作業領域が動いていない窓を書き直している: {writes:?}"
    );
    assert_eq!(
        ground_y(&harness, 0),
        s2_work_area_for_dpi(STEADY_DPI).bottom,
        "隣接モニタの変化で接地点が動いている"
    );

    // 後半（陽性の対）: 同じ駆動口でゴーストの居るモニタを動かす。
    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    assert!(
        !harness.drain_writes().is_empty(),
        "自分の作業領域が動いたのに書き直されない"
    );
}

/// 下端吸着でないキャラ窓は再スナップの対象外（設計 C6 は Bottom アンカーだけを対象とする）。
/// 同じフレームの下端吸着窓が書かれることを対に置き、駆動の生存を同じ本体で示す。
#[test]
fn a_free_anchored_char_window_is_left_alone_while_a_bottom_anchored_one_moves() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let char1 = harness.char_window(1);
    harness
        .world
        .entity_mut(char1)
        .insert(Anchored(Anchor::Free));
    let free_pos = pos_of(&harness.world, char1).expect("char 位置がある");

    hide_the_taskbar(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    let writes = harness.drain_writes();
    assert!(
        writes_for(&writes, 1, "char").is_empty(),
        "位置固定（Free）のキャラ窓を再スナップが動かしている: {writes:?}"
    );
    assert_eq!(
        pos_of(&harness.world, char1).expect("char 位置がある"),
        free_pos,
        "位置固定のキャラ窓の位置が変わっている"
    );
    assert_eq!(
        writes_for(&writes, 0, "char").len(),
        1,
        "同じフレームの下端吸着窓が書かれていない（駆動が死んでいる）: {writes:?}"
    );
}

// ---------------------------------------------------------------------------
// 置き場（拡大率の相の後・同一フレーム）
// ---------------------------------------------------------------------------

/// 拡大率と作業領域が**同時に**動いたフレームで、キャラ窓の書込は 1 回のままである
/// （完了条件「相が既に書き終えた窓は同値のため書込を出さない」）。
///
/// 再スナップは相の**後**に置かれるので、相が新しい下端へ書き終えた窓に対しては導出値が
/// 現在値と一致し、`resize_window_to` のべき等 skip が書込ゼロで抜ける。相の**前**に置くと
/// 旧下端へ書いてから相が書き直す 2 段書込になる。
#[test]
fn a_window_already_written_by_the_scale_phase_is_not_written_again_in_the_same_frame() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    // OS 設定の拡大率変更（作業領域も同時に動く）。作業領域源は触らない——実行時のモニタ表
    // から作り直すのが同期段の仕事である。
    harness.set_monitor_table_for_dpi(OTHER_DPI);
    harness.set_window_dpi(OTHER_DPI);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    let writes = harness.drain_writes();
    assert_eq!(
        writes_for(&writes, 0, "char").len(),
        1,
        "拡大率と作業領域が同時に動いたフレームでキャラ窓の書込が 1 回になっていない: {writes:?}"
    );
    assert_eq!(
        ground_y(&harness, 0),
        s2_work_area_for_dpi(OTHER_DPI).bottom,
        "接地点が新しい水準の作業領域下端に載っていない"
    );
}

/// 再スナップの呼出は本番でも拡大率の相の**後**に置かれている（設計 C6）。
///
/// 順序そのものは上のテストが挙動で示すが、あちらはハーネスの駆動口が順に呼ぶ形ゆえ、
/// 本番の相順が入れ替わっても赤にならない。担い手は `frame.rs` の呼出順そのものなので、
/// その形を本文で名指しする（同期段が相より前であることは task 5.1 の兄弟テストが持つ）。
#[test]
fn the_resnap_is_called_after_the_scale_phase_in_the_frame_system() {
    let code = include_str!("frame.rs");
    let dpi = code
        .find("run_dpi_phase(&mut wiring, world)")
        .expect("拡大率の相の呼出が frame.rs に無い");
    let resnap = code
        .find("work_area_sync::resnap_for_work_area_change(")
        .expect("作業領域再スナップの呼出が frame.rs に無い");
    assert!(
        dpi < resnap,
        "再スナップが拡大率の相より前に置かれている（旧下端へ書いてから相が書き直す 2 段書込）"
    );
    let reconcile = code
        .find("reconcile_reported_sizes(&mut wiring.presenter, world)")
        .expect("報告寸の突合の呼出が frame.rs に無い");
    assert!(
        resnap < reconcile,
        "再スナップが報告寸の突合より後に置かれている（設計 C6 の置き場と食い違う）"
    );
}

/// 上端吸着のキャラ窓は再スナップの対象外である（要件 5.1 は**下端吸着**だけを名指しする）。
///
/// 作業領域の**上端も下端も**動く構成（タスクバーを上辺へ移す）で問う——上端だけが動く形に
/// すると、下端吸着窓の射影値が変わらず対の陽性側が空振りする。上端吸着窓は射影値が実際に
/// 変わる（`wa.top` が動く）ので、下端吸着だけを選ぶ絞りが**効いていなければ**書込が出る。
#[test]
fn a_top_anchored_char_window_is_left_alone_while_a_bottom_anchored_one_moves() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let char1 = harness.char_window(1);
    harness
        .world
        .entity_mut(char1)
        .insert(Anchored(Anchor::Top));
    let top_pos = pos_of(&harness.world, char1).expect("char 位置がある");

    // タスクバーを上辺へ移す（上端が下がり、下端はモニタ下端まで伸びる）。値は合成レイアウト
    // から導く（要件 5.6: 絶対 px を判定に直書きしない）。
    let visible = s2_work_area_for_dpi(STEADY_DPI);
    let hidden = s2_taskbar_hidden_work_area(STEADY_DPI);
    let taskbar_h = hidden.bottom - visible.bottom;
    let on_top = RectPx {
        top: visible.top + taskbar_h,
        bottom: hidden.bottom,
        ..visible
    };
    assert_ne!(
        on_top.top, visible.top,
        "探針が退化している: 作業領域の上端が動いていない（上端吸着の絞りを観測できない）"
    );
    harness.set_monitor_table(s2_monitors_with_work_area(STEADY_DPI, on_top));
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    let writes = harness.drain_writes();
    assert!(
        writes_for(&writes, 1, "char").is_empty(),
        "上端吸着のキャラ窓を再スナップが動かしている（要件 5.1 は下端吸着だけを対象とする）: {writes:?}"
    );
    assert_eq!(
        pos_of(&harness.world, char1).expect("char 位置がある"),
        top_pos,
        "上端吸着のキャラ窓の位置が変わっている"
    );
    assert_eq!(
        writes_for(&writes, 0, "char").len(),
        1,
        "同じフレームの下端吸着窓が書かれていない（駆動が死んでいる）: {writes:?}"
    );
    assert_eq!(
        ground_y(&harness, 0),
        on_top.bottom,
        "下端吸着窓の接地点が新しい作業領域下端に載っていない"
    );
}

use super::shared_test_support::wiring;
use super::test_support::{
    FakeSizes, all_shown, assert_no_write, dpi_world, pos_of, resnap_world, size_of,
};
use super::*;

// ── resnap シーム（resnap_from_sizes／resnap_shell_targets）の檻 ────────
//
// drain 後の shell サーフェス寸法変化検知を GPU 不要で headless に固定する。
// spawn_ghost_windows で 2 スコープの char/balloon 窓＋GhostWindows を組み（char 窓は
// Anchored 付き）、各窓へ偽 WindowHandle を注入し MonitorSnapshot を挿入した World 上で、
// 合成 (scope, SizePx) を resnap_from_sizes へ直接注入して観測する。

use areka_placement::resolver::SizePx;

use wintf::ecs::{Point, SizeI};

/// 異寸→resize＋re-snap。shown_size が現 WindowPos.size と異なると、当該 char 窓の
/// size が新寸・position が Anchored(Bottom) に沿って再射影される（y=下端−h'・x 保持）。
#[test]
fn resnap_from_sizes_drives_resize_and_resnap_on_size_change() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(434, 687)),
        "前提: 初期寸"
    );
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 757 }),
        "前提: 初期位置（bottom 不変量を満たす）"
    );

    // h 687→700 の異寸を注入。
    resnap_from_sizes(
        &mut world,
        [(0usize, SizePx { w: 434, h: 700 })].into_iter(),
    );

    // 新寸へ更新され、Bottom 再射影で下端固定（y=1444−700=744・x 保持）。
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(434, 700)),
        "新寸へ更新"
    );
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 744 }),
        "Bottom 再射影: y=work_area.bottom−h'（x 保持）"
    );

    // 同じ寸を 2・3 回繰り返しても、1 回目の適用後から窓状態は変わらない（多重には効かない）。
    for repeat in 2..=3 {
        resnap_from_sizes(
            &mut world,
            [(0usize, SizePx { w: 434, h: 700 })].into_iter(),
        );
        assert_eq!(
            size_of(&world, char0),
            Some(SizeI::new(434, 700)),
            "同寸 {repeat} 回目: size は不変"
        );
        assert_eq!(
            pos_of(&world, char0),
            Some(Point { x: 1483, y: 744 }),
            "同寸 {repeat} 回目: position は不変"
        );
    }
}

/// 非正/変換失敗→skip。非正寸（0・負）は resnap_from_sizes が弾き窓状態不変（二重防波堤）。
#[test]
fn resnap_from_sizes_skips_non_positive_sizes() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let size_before = size_of(&world, char0);
    let pos_before = pos_of(&world, char0);

    for bad in [
        SizePx { w: 0, h: 687 },
        SizePx { w: 434, h: 0 },
        SizePx { w: -5, h: 687 },
        SizePx { w: 434, h: -5 },
    ] {
        resnap_from_sizes(&mut world, [(0usize, bad)].into_iter());
    }

    assert_eq!(
        size_of(&world, char0),
        size_before,
        "非正寸は skip（size 不変）"
    );
    assert_eq!(
        pos_of(&world, char0),
        pos_before,
        "非正寸は skip（position 不変）"
    );
}

/// balloon で駆動しない。char 窓が resize されても同 scope の balloon 窓の
/// WindowPos.size は不変（resnap_from_sizes は scope→char_window のみ写像し balloon に触れない）。
#[test]
fn resnap_from_sizes_never_resizes_balloon_window() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let balloon0 = gw.balloon_window(0).unwrap();
    let balloon_size_before = size_of(&world, balloon0);
    assert_eq!(
        balloon_size_before,
        Some(SizeI::new(223, 158)),
        "前提: balloon 初期寸"
    );

    // char0 を異寸で駆動（balloon の寸へ仮に写せば 500×720 になるはずの値）。
    resnap_from_sizes(
        &mut world,
        [(0usize, SizePx { w: 500, h: 720 })].into_iter(),
    );

    // char は新寸へ（駆動された証拠）。
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(500, 720)),
        "char は resize される"
    );
    // balloon の寸は不変（balloon を resize 対象にしていない）。
    assert_eq!(
        size_of(&world, balloon0),
        balloon_size_before,
        "balloon 窓の size は resnap で不変（scope→char_window のみ写像）"
    );
}

/// 変異檻: resnap は **shell target の物理寸**で char 窓を駆動し、
/// balloon target のジオメトリは読まない。
///
/// `shell_target(scope)` → `balloon_target(scope)` の 1 トークン変異を**排他的に殺す**。
/// shell/balloon で寸を変えてあるため、変異すると char 窓が balloon 寸（223×158）へ
/// 縮み、Bottom 再射影で y も 1444−158=1286 へ跳ぶ——寸法・位置の両方で判別できる。
#[test]
fn resnap_reads_shell_targets_only_and_ignores_balloon_geometry() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let char1 = gw.char_window(1).unwrap();
    let balloon0 = gw.balloon_window(0).unwrap();
    let balloon_size_before = size_of(&world, balloon0);

    // shell=434×700（scope0 の初期寸 434×687 と異なる＝駆動される）／
    // balloon=223×158（fixture の balloon 実寸・変異したらこちらが char へ写る）。
    let fake = FakeSizes::new((434, 700), (223, 158));
    resnap_with(&fake, &mut world);

    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(434, 700)),
        "char0 は shell target の物理寸へ揃う（balloon 寸 223×158 なら変異）"
    );
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 744 }),
        "Bottom 再射影は shell 寸基準: y=1444−700（balloon 寸なら 1444−158=1286）"
    );
    assert_eq!(
        size_of(&world, char1),
        Some(SizeI::new(434, 700)),
        "char1 も shell target の物理寸で駆動される（scope 横断で同一判断）"
    );
    assert_eq!(
        size_of(&world, balloon0),
        balloon_size_before,
        "balloon 窓自体は書かれない（scope→char_window のみ写像）"
    );
}

/// 変異檻: 問い合わせた `TargetId` 集合が shell だけであること。
///
/// 上のジオメトリ檻と観測面を分ける——寸が偶然一致しても読み口の取り違えを捕まえる
/// （兄弟の `dpi_phase_first_run_matches_all_windows_without_churn` と同じ技法）。
#[test]
fn resnap_queries_shell_targets_only() {
    let (mut world, _gw) = resnap_world();

    let fake = FakeSizes::new((434, 700), (223, 158));
    resnap_with(&fake, &mut world);

    let mut queried = fake.queried.borrow().clone();
    queried.sort_unstable();
    assert_eq!(
        queried,
        vec![shell_target(0).0, shell_target(1).0],
        "resnap が引く target は shell のみ（balloon_target {:?}/{:?} は一度も引かない）",
        balloon_target(0),
        balloon_target(1)
    );
}

/// アダプタ存在チェック: resnap_shell_targets を target 未装着の EmoPresenter::new()
/// （target_physical_size 全 None）で呼ぶと全 scope skip の no-op（panic しない）・
/// GhostWindows 未挿入でも安全。
///
/// **注意**: 未装着 presenter は全 target が `None` ゆえ shell/balloon を判別できない
/// （本テストは変異を殺さない）。読み口の取り違えは
/// `resnap_reads_shell_targets_only_and_ignores_balloon_geometry` と
/// `resnap_queries_shell_targets_only` が担う。
#[test]
fn resnap_shell_targets_is_noop_with_unattached_presenter() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let size_before = size_of(&world, char0);
    let pos_before = pos_of(&world, char0);

    // 未装着 presenter は `PhysicalSizeSource`（本番は `target_physical_size`）が全 None → 全 scope skip（窓状態不変・panic しない）。
    let presenter = EmoPresenter::new();
    resnap_shell_targets(&presenter, &mut world);
    assert_eq!(
        size_of(&world, char0),
        size_before,
        "未装着は全 scope skip（size 不変）"
    );
    assert_eq!(
        pos_of(&world, char0),
        pos_before,
        "未装着は全 scope skip（position 不変）"
    );

    // GhostWindows 未挿入の素の World でも安全（no-op・panic しない）。
    let mut empty = World::new();
    resnap_shell_targets(&presenter, &mut empty);
}

// ── 同寸の繰り返しのべき等（回帰檻） ──────────────────────
//
// 同じ寸を繰り返し渡しても窓を一度も書かないことを固定する。違う寸の後に同じ寸を
// 繰り返す流れは `resnap_from_sizes_drives_resize_and_resnap_on_size_change` の後半が持つ。

/// 最初から現寸と**同一**の shown_size を反復して渡しても、一度も窓を書かない。書込 witness の
/// `dpi_world` で、窓状態の不変と単一ライター経路を通った痕跡の無さの両方を見る。
#[test]
fn resnap_from_sizes_same_size_repeats_never_write_a_window() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).unwrap();
    let balloon0 = gw.balloon_window(0).unwrap();
    let size_before = size_of(&world, char0);
    let pos_before = pos_of(&world, char0);

    for repeat in 1..=3 {
        resnap_from_sizes(
            &mut world,
            [(0usize, SizePx { w: 434, h: 687 })].into_iter(),
        );
        assert_eq!(
            size_of(&world, char0),
            size_before,
            "同寸反復 {repeat}: size 不変"
        );
        assert_eq!(
            pos_of(&world, char0),
            pos_before,
            "同寸反復 {repeat}: position 不変"
        );
        assert_no_write(&world, char0, "同寸反復の char 窓");
        assert_no_write(&world, balloon0, "同寸反復の balloon 窓");
    }
}

// ── 面の拡大の寄せと同じフレームの位置の指令 ──

/// 既定の位置にいる `free` の scope0 が面の再スナップで作業領域の下へはみ出すほど大きくなると、
/// 寄せた位置へ 1 回で書かれ、同じフレームの `\1\![move,--base=0,…]`（scope1 の左下を scope0 の
/// 左上へ）は寄せた後の矩形を基準に scope1 を置く。並べ直しはやり直さない＝どちらの窓も書き込みは 1 回ずつ。
#[test]
fn a_move_in_the_same_frame_as_a_grow_clamp_uses_the_clamped_rect() {
    use super::shared_test_support::steady_harness;
    use super::test_support::{PerTargetSizes, SPAWN_SIZE_1};
    use crate::move_cue::{PositionCue, parse_move_directive};
    use areka_placement::follow::Anchored;
    use areka_placement::resolver::Anchor;

    use wintf::ecs::WindowHandle;

    let mut harness = steady_harness();
    let (char0, char1) = (harness.char_window(0), harness.char_window(1));
    // 両方 free（scope1 も吸着で縦を戻されないように）。
    for e in [char0, char1] {
        harness.world.entity_mut(e).insert(Anchored(Anchor::Free));
    }

    let (tx, wiring) = wiring();
    let tokens: Vec<String> = [
        "--X=0",
        "--Y=0",
        "--base=0",
        "--base-offset=left.top",
        "--move-offset=left.bottom",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    tx.send(PositionCue::Move(
        parse_move_directive(1, &tokens).directive,
    ))
    .expect("送出は成功する");

    // scope0 は y=757 の 434×687（下端 1444＝作業領域の下端）から 434×900 へ。
    let sizes = PerTargetSizes::new([(0, Some((434, 900))), (1, Some(SPAWN_SIZE_1))]);
    harness.run_resnap(&sizes);
    // 前提: 窓はどれも最初の面を表示済み（位置の指令を預からない）。
    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    harness.run_chain_finalize(&sizes);

    let clamped = Point {
        x: 1483,
        y: 1444 - 900,
    };
    assert_eq!(
        pos_of(&harness.world, char0),
        Some(clamped),
        "下へはみ出した分だけ上げる"
    );
    assert_eq!(
        pos_of(&harness.world, char1),
        Some(Point {
            x: 1483,
            y: clamped.y - SPAWN_SIZE_1.1 as i32,
        }),
        "寄せた後の scope0 の左上を基準に置かれる（寄せる前の上辺 757 ではない）"
    );
    let hwnd = |e| harness.world.get::<WindowHandle>(e).expect("偽 HWND").hwnd;
    let (h0, h1) = (hwnd(char0), hwnd(char1));
    let writes = harness.drain_writes();
    assert_eq!(
        writes.iter().filter(|c| c.hwnd == h0).count(),
        1,
        "scope0 は 1 回で書く"
    );
    assert_eq!(
        writes.iter().filter(|c| c.hwnd == h1).count(),
        1,
        "scope1 は位置の指令の 1 回だけ（並べ直しをやり直さない）"
    );
}

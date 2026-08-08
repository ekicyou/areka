use super::*;
use super::test_support::{
    FakeSizes,
    pos_of,
    resnap_world,
    size_of,
};

// ── task 3.2: resnap シーム（resnap_from_sizes／resnap_shell_targets）の檻 ────────
//
// drain 後の shell サーフェス寸法変化検知を GPU 不要で headless に固定する。
// spawn_ghost_windows で 2 スコープの char/balloon 窓＋GhostWindows を組み（char 窓は
// Anchored 付き）、各窓へ偽 WindowHandle を注入し MonitorSnapshot を挿入した World 上で、
// 合成 (scope, SizePx) を resnap_from_sizes へ直接注入して観測する（Req1.3/3.1/3.4/4.5）。

use crate::placement::resolver::SizePx;

use wintf::ecs::{Point, SizeI};

/// 1.3/3.1: 異寸→resize＋re-snap。shown_size が現 WindowPos.size と異なると、当該 char 窓の
/// size が新寸・position が Anchored(Bottom) に沿って再射影される（y=下端−h'・x 保持）。
#[test]
fn resnap_from_sizes_drives_resize_and_resnap_on_size_change() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 687)), "前提: 初期寸");
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 757 }),
        "前提: 初期位置（bottom 不変量を満たす）"
    );

    // h 687→700 の異寸を注入。
    resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());

    // 新寸へ更新され、Bottom 再射影で下端固定（y=1444−700=744・x 保持）。
    assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 700)), "新寸へ更新");
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 744 }),
        "Bottom 再射影: y=work_area.bottom−h'（x 保持）"
    );
}

/// 3.1: 同寸→no-op。shown_size が現 WindowPos.size と同一なら resize は駆動されず窓状態不変。
#[test]
fn resnap_from_sizes_is_noop_on_same_size() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let size_before = size_of(&world, char0);
    let pos_before = pos_of(&world, char0);

    // 現寸と同一（434×687）→ 冗長駆動を避ける（非発火）。
    resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 687 })].into_iter());

    assert_eq!(size_of(&world, char0), size_before, "同寸は size 不変");
    assert_eq!(pos_of(&world, char0), pos_before, "同寸は position 不変（非発火）");
}

/// 3.4: 非正/変換失敗→skip。非正寸（0・負）は resnap_from_sizes が弾き窓状態不変（二重防波堤）。
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

    assert_eq!(size_of(&world, char0), size_before, "非正寸は skip（size 不変）");
    assert_eq!(pos_of(&world, char0), pos_before, "非正寸は skip（position 不変）");
}

/// 4.5: balloon で駆動しない。char 窓が resize されても同 scope の balloon 窓の
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
    resnap_from_sizes(&mut world, [(0usize, SizePx { w: 500, h: 720 })].into_iter());

    // char は新寸へ（駆動された証拠）。
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(500, 720)),
        "char は resize される"
    );
    // balloon の寸は不変（balloon を resize 対象にしていない・Req4.5）。
    assert_eq!(
        size_of(&world, balloon0),
        balloon_size_before,
        "balloon 窓の size は resnap で不変（scope→char_window のみ写像）"
    );
}

/// 4.5（変異檻・2026-07-30 新設）: resnap は **shell target の物理寸**で char 窓を駆動し、
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
        "balloon 窓自体は書かれない（scope→char_window のみ写像・Req4.5）"
    );
}

/// 4.5（変異檻・2026-07-30 新設）: 問い合わせた `TargetId` 集合が shell だけであること。
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

    // 未装着 presenter＝text_slot_view 全 None → 全 scope skip（窓状態不変・panic しない）。
    let presenter = EmoPresenter::new();
    resnap_shell_targets(&presenter, &mut world);
    assert_eq!(size_of(&world, char0), size_before, "未装着は全 scope skip（size 不変）");
    assert_eq!(pos_of(&world, char0), pos_before, "未装着は全 scope skip（position 不変）");

    // GhostWindows 未挿入の素の World でも安全（no-op・panic しない）。
    let mut empty = World::new();
    resnap_shell_targets(&presenter, &mut empty);
}

// ── task 4.1: 検知→反映の一連のべき等（回帰檻・Req1.5/3.1） ──────────────────────
//
// 既存の移動専用経路（enqueue_window_set_pos／move_window_to／on_char_drag）は本 task で
// 一切改変せず（follow.rs の move 系統合テスト群が無改変で緑＝単一ライター一般化の無影響）、
// ここでは「寸法検知（差分判定）→窓反映（resize_window_to のべき等 skip）」の一連の流れが
// 多重には効かないことを端から端まで固定する（Req3.1 の冗長回避・design「System Flows」
// 同寸同アンカー非発火／「resnap_from_sizes」Postconditions 同寸 no-op）。

/// 1.5/3.1（一連のべき等）: 寸法検知→窓反映の一連の流れが多重には効かないことを端から端まで
/// 固定する。まず現寸と**異なる** shown_size で 1 回駆動して resize 発火（size 新寸・position
/// Bottom 再射影）を確立し、続けて**同一**の shown_size を 2・3 回繰り返し駆動しても、1 回目
/// 適用後の position・size が**一切変化しない**（冗長な再配置・再書込が起きない）ことを反証する。
/// 空虚一致でないよう「1 回目で実際に size/position が変化した」ことも先に assert する。
/// 96 非倍数の work area 辺・寸法（bottom=1444／h=700）で dpi/96 再スケール混入の檻とする。
#[test]
fn resnap_from_sizes_is_idempotent_across_repeats_after_a_size_change() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();

    // 前提: 初期寸・初期位置（bottom 不変量を満たす・96 非倍数 work area 由来）。
    let size_initial = size_of(&world, char0);
    let pos_initial = pos_of(&world, char0);
    assert_eq!(size_initial, Some(SizeI::new(434, 687)), "前提: 初期寸");
    assert_eq!(pos_initial, Some(Point { x: 1483, y: 757 }), "前提: 初期位置");

    // (1) 現寸と異なる h=700 を 1 回駆動 → resize 発火（前提の確立）。
    resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());
    let size_after_first = size_of(&world, char0);
    let pos_after_first = pos_of(&world, char0);

    // 空虚一致でないことの担保: 1 回目で size・position が実際に変化した（新寸＋Bottom 再射影）。
    assert_eq!(size_after_first, Some(SizeI::new(434, 700)), "1 回目で新寸へ更新");
    assert_eq!(
        pos_after_first,
        Some(Point { x: 1483, y: 744 }),
        "1 回目で Bottom 再射影（y=1444−700=744・x 保持）"
    );
    assert_ne!(size_after_first, size_initial, "1 回目は実際に size が変化した（空虚でない）");
    assert_ne!(pos_after_first, pos_initial, "1 回目は実際に position が変化した（空虚でない）");

    // (2) 同一 shown_size を 2 回・3 回繰り返し駆動 → 窓の position・size が 1 回目適用後から
    //     一切変化しない（検知→反映の一連が多重には効かない＝冗長な再配置・再書込なし・Req3.1）。
    for repeat in 2..=3 {
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 700 })].into_iter());
        assert_eq!(
            size_of(&world, char0),
            size_after_first,
            "同寸 {repeat} 回目: size は 1 回目適用後から不変（多重には効かない）"
        );
        assert_eq!(
            pos_of(&world, char0),
            pos_after_first,
            "同寸 {repeat} 回目: position は 1 回目適用後から不変（非発火）"
        );
    }
}

/// 3.1（純べき等）: 最初から現寸と**同一**の shown_size を反復駆動しても、一度も窓状態が
/// 変化しない（検知段の同寸 skip が毎回効く・冗長駆動ゼロ）。size・position の**両方**が不変
/// であることを毎回見る。
#[test]
fn resnap_from_sizes_same_size_repeats_never_change_window_state() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let size_before = size_of(&world, char0);
    let pos_before = pos_of(&world, char0);

    // 現寸（434×687）と同一を 3 回反復 → 毎回 no-op（窓状態不変・冗長駆動なし・Req3.1）。
    for repeat in 1..=3 {
        resnap_from_sizes(&mut world, [(0usize, SizePx { w: 434, h: 687 })].into_iter());
        assert_eq!(size_of(&world, char0), size_before, "同寸反復 {repeat}: size 不変");
        assert_eq!(pos_of(&world, char0), pos_before, "同寸反復 {repeat}: position 不変");
    }
}

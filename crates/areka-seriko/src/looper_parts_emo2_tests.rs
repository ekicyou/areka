//! 実物の emo2 の表で、刻みが普通に届く間の発行列と乱数の消費を固定する檻
//! （spec: areka-P0-surface-element-nesting 要件 7.4・7.5・8.3）。
//!
//! # 期待値の出どころ
//!
//! 下の `EXPECTED_SHOWS`・`EXPECTED_RNG_CALLS` は、部品の経路を足す**前の HEAD**
//! （`eb1a7e77`）で本テストを走らせて採った値を、そのままリテラルで書き込んだもの。
//! 新しいコードどうしの比べ合いではない。部品の経路が入った後もこの値のまま緑であることが、
//! 要件 7.4（刻みが普通に届く間は emo2 の絵・発行列・乱数の消費が本 spec の前と同じ）の檻になる。
//!
//! emo2 では `\1` のまばたき（`surface.append10,2100`・`surface.append2200` の `animation0`）の
//! 2 枚目のコマ 2110／2210 が、自分のアニメーション（`surface.append2110`・`surface.append2210`）を
//! 持つ。2110／2210 が絵に出るのは発火の 40ms 後〜120ms 後だけで、次の抽選の境界とは、発火の
//! 刻みが境界から 880ms 以上遅れたときしか重ならない（design.md「emo2 についての事実」）。
//! 本テストの刻みの間隔は 33〜67ms なので、その端（要件 7.5）には触れない。
//!
//! # 着手前の確認（tasks.md 1.1）: 既存テストの刻みの飛び
//!
//! 刻みを回す既存テストで、隣り合う刻みの差が 880ms 以上の所は次のとおり（HEAD `eb1a7e77` で
//! ワークスペース全体の `send_tick`／`SerikoMsg::Tick`／`on_tick`／`inject_seriko_tick` を拾った）。
//! 実物の emo2 の表で seriko の刻みを回す既存テストは、**seriko クレートの中には 0 本**。
//! クレートの外では `crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs` の 5 本が、実物の emo2 の
//! シェルの表（`SpineHarness::boot_live`）に常に発火する乱数を注入して刻みを回す。
//! seriko クレートの `tests/regression.rs`・`tests/cue_sequence.rs`・`tests/bind_e2e.rs`・
//! `tests/balloon_face_e2e.rs` は emo2 の alias 表を使うが、ループは `SerikoLoopConfig::disabled()`
//! （空の表）で、seriko の刻みを 1 つも送らない。seriko クレートの中で刻みを回すテストはどれも
//! 手組みの表（emo2 の形を模した合成）を使う。本番の `emo2_boot/mod.rs` の刻みは実時刻でテストではなく、
//! `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の刻みは dispatcher 宛で seriko には届かない。
//!
//! - `crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs`（**実物の emo2 の表**・常に発火）:
//!   `spine_blink_smoke_send_tick_drives_loop_pattern_command`（40ms 刻み・飛びなし）・
//!   `spine_e2e_kero_blink_one_cycle_golden`（0→1000。その後 1040・1120）・
//!   `spine_e2e_sakura_blink_after_bind_one_cycle_golden`（0→1000。その後 1150・1172）・
//!   `spine_e2e_sakura_blink_default_off_emits_nothing`（0→1000→2000→…→5000・各 1000）・
//!   `spine_dpi_change_during_live_seriko_loop_keeps_loop_progressing`（0→1000。その後 1040・1120）
//! - `tests/loop_integration.rs`: `kero_negative_tail_restores_base_full_path`（0→1000）・
//!   `sakura_residual_tail_keeps_frame_full_path`（0→1000）・
//!   `playing_anim_not_relotteried_across_boundary_full_path`（0→1000・1000→2000）・
//!   `bindrandom_off_consumes_no_rng_full_path`（0→1000・1000→2000）・
//!   `bindrandom_on_fires_full_path`（0→1000）・`unchanged_tick_emits_nothing_full_path`（0→1000）・
//!   `surface_switch_clears_playback_and_frame_full_path`（0→1000・1000→2000）・
//!   `residual_immediately_cleared_on_refire_full_path`（0→1000・1100→2000）・
//!   `other_negative_surface_warns_once_and_spares_others_full_path`（0→1000・1040→2000）
//! - `src/looper_tests.rs`: 0→1000 が 21 本、ほかに `playing_anim_is_not_relotteried`（1000→2000）・
//!   `kero_negative_tail_restores_base_and_is_relotteriable`（1120→2000）・
//!   `residual_is_immediately_cleared_on_refire`（1100→2000）・
//!   `absent_scope_balloon_table_is_inert`（1000→2000）・`on_surface_changed_removes_playback`（1000→2000）・
//!   `disabled_config_is_inert`（1000→5000）・
//!   `playing_bind_anim_removed_from_binds_stops_and_does_not_revive`（1100→2000）
//! - `src/looper_replace_tests.rs`: 2 本とも 0→1000
//! - `src/actor_replace_tests.rs`: 2 本とも 0→1000
//! - `src/actor_bind_loop_tests.rs`（`SerikoMsg::Tick` の直投函）:
//!   `tick_with_no_shown_slot_is_complete_no_op`（0→1000・1000→5000・表示中のサーフェスが無いので
//!   抽選も発火も無い）・`tick_boundary_cross_emits_show_carrying_pattern`（0→1000）・
//!   `emote_surface_change_resets_loop_playback`（0→1000）。どれも手組みの表で、コマが指す
//!   サーフェス（2106・500 など）は自分のアニメーションを持たず、飛びの前に発火も無い
//!   （`send_tick_after_actor_stopped_logs_debug_no_panic` は停止後の 1 刻みだけ）
//! - `src/actor_dispatch_tests.rs`: 刻みなし（`tests/bind_e2e.rs`・`tests/balloon_face_e2e.rs` の
//!   `SakuraMsg::Tick` は台本の時計で、seriko の刻みではない）
//!
//! **端（要件 7.5）に当たるものは 0 件**。理由は次の 2 つ:
//!
//! 1. seriko クレートの中の手組みの表はどれも、pattern定義が指すサーフェス（2106・2110・500・3000 など）に
//!    自分のアニメーションを持たせていない。部品として抽選されるものが表に 1 つも無い。
//!    （実物の emo2 の表では 2110／2210 がアニメーションを持つので、`spine_seriko_loop_tests.rs` の 5 本には
//!    この理由は使えない。あの 5 本は下の 2 だけで端を外れる。）
//! 2. 飛びの直後の刻み（境界を跨ぐ刻み）で 2110／2210 が絵に出ているものが無い。0→1000 は最初の
//!    刻みが境界を遅延初期化するだけで、その前に発火は無い。それ以外の飛びの直前は、どれも
//!    コマ無し・停止（`-1`・`-2`）・末尾の残留（701・1410 など）・再生中の 2106 で、2110 は出ていない。
//!    `spine_seriko_loop_tests.rs` で 2110 が出るのは 1040（40ms 刻みの回では 1040・1080 など）だけで、
//!    その刻みは境界を跨がない。`spine_e2e_sakura_blink_default_off_emits_nothing` は着せ替えが OFF で
//!    発火そのものが無い。

use super::tests::{cfg, pattern_of};
use super::*;
use crate::bind::build_static_bindset;
use crate::resolve::SurfaceTarget;
use crate::state::ApplyOutcome;
use areka_emo_compose::{ComposeMethod, EmoWorld, PatternState};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 乱数の種（本番と同じ `seeded_rng`）。
const SEED: u64 = 20_261_004;
/// 回す長さ（ms）。`\1` のまばたきが何度も発火する長さ。
const RUN_MS: u64 = 60_000;
/// この時刻を越えた最初の刻みの前に、`\1` を 10 → 2200 へ切り替える（2210 側も回すため）。
const SWITCH_MS: u64 = 30_000;

/// 実物の emo2 のシェルの表（本番と同じ `EmoWorld::build` → `AnimationTable::from_world`）。
fn emo2_shell_table() -> AnimationTable {
    let path = crate::sample_test_support::emo2_root()
        .join("shell")
        .join("master")
        .join("surfaces.txt");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(&content)))
}

/// `seeded_rng` を包み、呼ばれた回数を数える乱数。
fn counting_seeded_rng(seed: u64) -> (LoopRng, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let mut inner = seeded_rng(seed);
    let rng: LoopRng = Box::new(move |bound| {
        counter.fetch_add(1, Ordering::Relaxed);
        inner(bound)
    });
    (rng, calls)
}

/// 33〜67ms の決まった揺らぎを持つ「普通の」刻みの間隔（880ms には遠く届かない）。
fn step_ms(i: u64) -> u64 {
    33 + (i * 13) % 35
}

/// 発行された `Show` 1 件を `"時刻 scope 面 [animation:コマ ...]"` の 1 行にする。
///
/// コマは emo2 では全部 overlay・位置 0,0 なのでここで確かめ、行には番号だけを残す。
/// `PatternState` が一番上のコマ以外を何も運んでいないことも、走査で組み直した値との
/// 等しさで確かめる（部品の欄が入った後に、普通の刻みで部品のコマが載らないことの檻）。
fn show_line(now_ms: u64, cmd: &DisplayCommand, binds: &areka_emo_compose::BindSet) -> String {
    let DisplayCommand::Show {
        scope,
        surface_id,
        binds: shown_binds,
        ..
    } = cmd
    else {
        panic!("Show を期待: {cmd:?}");
    };
    assert_eq!(shown_binds, binds, "着せ替えの集合は回す間ずっと同じ");
    let pattern = pattern_of(cmd);
    let mut rebuilt = PatternState::default();
    let mut frames = Vec::new();
    for (id, f) in pattern.iter() {
        assert!(
            matches!(f.method, ComposeMethod::Overlay) && f.x == 0 && f.y == 0,
            "emo2 のコマは overlay・0,0: {f:?}"
        );
        rebuilt.set(id, f.clone());
        frames.push(format!("{id}:{}", f.surface_id));
    }
    assert_eq!(&rebuilt, pattern, "一番上のコマ以外を運ばない");
    format!(
        "{now_ms} {} {surface_id} [{}]",
        scope.as_str(),
        frames.join(" ")
    )
}

/// 実物の emo2 の表で、`\0`＝1000（着せ替え・まばたき 1400 を ON）、`\1`＝10 → 2200 を
/// 普通の刻みで回し、`Show` の列と乱数の呼び出し回数を、HEAD で採ったリテラルと比べる
/// （要件 7.4・8.3）。
#[test]
fn emo2_normal_ticks_match_head_shows_and_rng_calls() {
    // emo2 の既定オン集合 {1100,1207,1302,1500,1800} に、`\0` のまばたき（1400・bind+random）を
    // 足す。`\0` 側も乱数を消費させ、scope 昇順の消費順まで固定するため。
    let binds = build_static_bindset(&[1100, 1207, 1302, 1400, 1500, 1800]);
    let (rng, calls) = counting_seeded_rng(SEED);
    let mut rt = LoopRuntime::new(cfg(emo2_shell_table(), rng));
    let mut states = ScopeStates::new(binds.clone());
    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    states.apply(&sakura, SurfaceTarget::Show(1000));
    states.apply(&kero, SurfaceTarget::Show(10));

    let mut lines = Vec::new();
    let mut switched = false;
    let mut now = 0;
    let mut i = 0;
    while now <= RUN_MS {
        if !switched && now > SWITCH_MS {
            // actor の Emote の扱い（apply が Changed なら発行し、その slot の再生を捨てる）と同じ手順。
            if let ApplyOutcome::Changed(cmd) = states.apply(&kero, SurfaceTarget::Show(2200)) {
                lines.push(format!("switch {}", show_line(now, &cmd, &binds)));
                rt.on_surface_changed(&kero, Slot::Shell);
            }
            switched = true;
        }
        for cmd in rt.on_tick(now, &mut states) {
            lines.push(show_line(now, &cmd, &binds));
        }
        now += step_ms(i);
        i += 1;
    }

    let actual_calls = calls.load(Ordering::Relaxed);
    assert_eq!(
        lines, EXPECTED_SHOWS,
        "emo2 の Show の列が HEAD と違う（rng 呼び出し {actual_calls} 回）\n実際:\n{lines:#?}"
    );
    assert_eq!(
        actual_calls, EXPECTED_RNG_CALLS,
        "emo2 の乱数の呼び出し回数が HEAD と違う"
    );
}

/// HEAD（`eb1a7e77`）で採った `Show` の列。
const EXPECTED_SHOWS: &[&str] = &[
    "4053 1 10 [0:2106]",
    "4138 1 10 [0:2110]",
    "4200 1 10 []",
    "9038 1 10 [0:2106]",
    "9079 1 10 [0:2110]",
    "9200 1 10 []",
    "10010 0 1000 [1400:1412]",
    "10190 0 1000 [1400:1410]",
    "12000 1 10 [0:2106]",
    "12089 1 10 [0:2110]",
    "12153 1 10 []",
    "14000 0 1000 [1400:1412]",
    "14000 1 10 [0:2106]",
    "14079 1 10 [0:2110]",
    "14138 1 10 []",
    "14175 0 1000 [1400:1410]",
    "15043 1 10 [0:2106]",
    "15104 1 10 [0:2110]",
    "15195 1 10 []",
    "17010 0 1000 [1400:1412]",
    "17190 0 1000 [1400:1410]",
    "21000 0 1000 [1400:1412]",
    "21175 0 1000 [1400:1410]",
    "22043 0 1000 [1400:1412]",
    "22043 1 10 [0:2106]",
    "22104 1 10 [0:2110]",
    "22195 0 1000 [1400:1411]",
    "22195 1 10 []",
    "22260 0 1000 [1400:1410]",
    "26000 1 10 [0:2106]",
    "26089 1 10 [0:2110]",
    "26153 1 10 []",
    "27043 0 1000 [1400:1412]",
    "27043 1 10 [0:2106]",
    "27109 1 10 [0:2110]",
    "27210 0 1000 [1400:1411]",
    "27210 1 10 []",
    "27245 0 1000 [1400:1410]",
    "29043 0 1000 [1400:1412]",
    "29195 0 1000 [1400:1411]",
    "29260 0 1000 [1400:1410]",
    "switch 30038 1 2200 []",
    "34043 1 2200 [0:2206]",
    "34109 1 2200 [0:2210]",
    "34210 1 2200 []",
    "36043 0 1000 [1400:1412]",
    "36195 0 1000 [1400:1411]",
    "36260 0 1000 [1400:1410]",
    "38010 1 2200 [0:2206]",
    "38053 1 2200 [0:2210]",
    "38143 1 2200 []",
    "42000 1 2200 [0:2206]",
    "42079 1 2200 [0:2210]",
    "42138 1 2200 []",
    "46053 0 1000 [1400:1412]",
    "46240 0 1000 [1400:1410]",
    "48043 0 1000 [1400:1412]",
    "48210 0 1000 [1400:1411]",
    "48245 0 1000 [1400:1410]",
    "51038 1 2200 [0:2206]",
    "51079 1 2200 [0:2210]",
    "51200 1 2200 []",
    "53053 1 2200 [0:2206]",
    "53138 1 2200 [0:2210]",
    "53200 1 2200 []",
    "54000 0 1000 [1400:1412]",
    "54153 0 1000 [1400:1411]",
    "54195 0 1000 [1400:1410]",
    "57043 1 2200 [0:2206]",
    "57104 1 2200 [0:2210]",
    "57195 1 2200 []",
    "59010 0 1000 [1400:1412]",
    "59190 0 1000 [1400:1410]",
];

/// HEAD（`eb1a7e77`）で採った乱数の呼び出し回数。
const EXPECTED_RNG_CALLS: usize = 118;

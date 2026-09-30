//! 定義の差し替えの依頼（[`SerikoMsg::Replace`]）の決定論テスト
//! （spec: areka-P0-shell-balloon-switch 要件 2.6・2.8・3.5・12.6）。
//!
//! 依頼はアクターの受信の閉包が捌くので、`spawn_seriko` の実のアクターへ cue・tick・依頼を
//! 同じ inbox で送り、出力先の記録の並びで確かめる（tick は注入の絶対時刻・乱数は常に発火）。

use super::test_support::*;
use super::*;
use crate::bind::BindOptionDecls;
use crate::looper::tests::{always_fire, cfg, pattern_of, table_single};
use crate::output::{DisplayCommand, MockSurfaceOutput, RebaseKind, RebasedShow};
use crate::table::AnimationTable;
use areka_emo_compose::BindSet;
use areka_parsers::shell::Interval;
use areka_sakura::ActorKey;
use dola::cue::CueSink;
use std::collections::BTreeMap;

/// 別名 1 件（`key` → `id`）の解決層。
fn resolver_of(key: &str, id: u32) -> SurfaceResolver {
    SurfaceResolver::new(BTreeMap::from([(key.to_string(), vec![id])]))
}

/// (腕, 伸び) → 1300 の sakura 解決層（既定のポリシー＝排他置換）。
fn arm_resolver() -> BindResolver {
    let sakura = BTreeMap::from([(("腕".to_string(), "伸び".to_string()), 1300)]);
    BindResolver::new(sakura, BTreeMap::new(), BindOptionDecls::default())
}

/// `\![bind,腕,伸び,1]` の cue。
fn arm_on_cue(scope: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::command_carrier(
            "bind",
            vec!["腕".to_string(), "伸び".to_string(), "1".to_string()],
        ),
        duration: 0.0,
    }
}

/// `\b[key]` の cue。
fn balloon_cue(scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::BalloonSurface { key: key.into() },
        duration: 0.0,
    }
}

/// 記録の中の `Rebased` の位置をすべて返す。
fn rebased_positions(records: &[DisplayCommand]) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, DisplayCommand::Rebased { .. }))
        .map(|(i, _)| i)
        .collect()
}

/// 指令の pattern に面 `sid` のコマが載っているか（pattern を持たない指令は偽）。
fn carries_frame(cmd: &DisplayCommand, sid: u32) -> bool {
    match cmd {
        DisplayCommand::Show { .. } | DisplayCommand::ShowBalloon { .. } => pattern_of(cmd)
            .get(0)
            .is_some_and(|frame| frame.surface_id == sid),
        _ => false,
    }
}

/// シェルの差し替え: 同じ面・新しい既定の着せ替えの `Rebased` が 1 件。前に処理した指令は前に、
/// 古い表のループの指令は後に 0 件。新しい表・新しい別名表で続く（要件 2.6・2.8・12.6）。
#[test]
fn shell_replace_emits_one_rebased_with_same_surface_and_new_default_binds() {
    // 古い表: 面 2100 で 2106 → 150ms 後に 2110。
    let old = table_single(
        2100,
        0,
        Interval::Random { k: 4 },
        &[(2106, 0), (2110, 150)],
    );
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        resolver_of("通常", 2100),
        BindSet::from_ids([1100, 1207]),
        arm_resolver(),
        cfg(old, always_fire()),
        out,
    );

    CueSink::emit(&mut sink, emote_cue(0.0, "0", "通常")); // Show 2100
    sink.send_tick(0);
    sink.send_tick(1000); // 古い表で発火（2106）
    CueSink::emit(&mut sink, arm_on_cue("0")); // 動的な着せ替え {1100,1207,1300}

    let new = table_single(2100, 0, Interval::Random { k: 4 }, &[(3000, 0)]);
    assert!(sink.send_replace(SerikoReplace::Shell {
        epoch: 7,
        resolver: resolver_of("笑顔", 2200),
        static_binds: BindSet::from_ids([1500]),
        bind_resolver: BindResolver::empty(),
        shell_table: new,
    }));
    sink.send_tick(1150); // 古い表なら 2110 が出る時刻
    sink.send_tick(2000); // 新しい表で発火（3000）
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "笑顔")); // 新しい別名表で引ける
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records.lock().expect("records mutex poisoned").clone();
    let at = rebased_positions(&recorded);
    assert_eq!(at.len(), 1, "Rebased はちょうど 1 件: {recorded:?}");
    assert_eq!(
        recorded[at[0]],
        DisplayCommand::Rebased {
            epoch: 7,
            kind: RebaseKind::Shell,
            shows: vec![RebasedShow {
                scope: ActorKey::from("0"),
                surface_id: Some(2100),
                binds: BindSet::from_ids([1500]),
            }],
        },
        "同じ面・新しい既定の着せ替え（動的な着せ替えは持ち越さない）"
    );

    let (before, after) = recorded.split_at(at[0]);
    // 前に処理した指令（cue の Show・古い表のコマ・着せ替えの Show）はすべて前に出る。
    assert_eq!(before.len(), 3, "前の指令は 3 件: {before:?}");
    assert!(before.iter().any(|c| carries_frame(c, 2106)));
    // 古い表のループの指令は後に 0 件。
    assert!(
        !after
            .iter()
            .any(|c| carries_frame(c, 2106) || carries_frame(c, 2110)),
        "古い表のコマは Rebased の後に 0 件: {after:?}"
    );
    // 新しい表で始め直し、新しい既定の着せ替えで出る。
    assert!(
        after.iter().any(|c| carries_frame(c, 3000)
            && matches!(c, DisplayCommand::Show { binds, .. } if *binds == BindSet::from_ids([1500]))),
        "新しい表のコマが新しい既定の着せ替えで出る: {after:?}"
    );
    // 新しい別名表で引けた面が出る。
    assert!(
        after.iter().any(|c| matches!(
            c,
            DisplayCommand::Show {
                surface_id: 2200,
                ..
            }
        )),
        "新しい別名表が効く: {after:?}"
    );
}

/// バルーンの差し替え: シェル側（面・着せ替え・ループの進行）は変えず、各スコープのバルーンの
/// 今の面（無ければ 0）を `Rebased` で 1 件出す（要件 3.5）。
#[test]
fn balloon_replace_keeps_shell_side_and_reports_balloon_surfaces() {
    let shell = table_single(
        2100,
        0,
        Interval::Random { k: 4 },
        &[(2106, 0), (2110, 150)],
    );
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        resolver_of("通常", 2100),
        BindSet::from_ids([1100, 1207]),
        arm_resolver(),
        cfg(shell, always_fire()),
        out,
    );

    CueSink::emit(&mut sink, emote_cue(0.0, "0", "通常"));
    CueSink::emit(&mut sink, balloon_cue("0", "5"));
    CueSink::emit(&mut sink, emote_cue(0.0, "1", "-1")); // scope 1 はシェルだけ（非表示）
    sink.send_tick(0);
    sink.send_tick(1000); // シェルのループが発火（2106）
    CueSink::emit(&mut sink, arm_on_cue("0"));

    assert!(sink.send_replace(SerikoReplace::Balloon {
        epoch: 9,
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), AnimationTable::empty())]),
    }));
    sink.send_tick(1150); // シェルのループは続く（2110）
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records.lock().expect("records mutex poisoned").clone();
    let at = rebased_positions(&recorded);
    assert_eq!(at.len(), 1, "Rebased はちょうど 1 件: {recorded:?}");
    assert_eq!(
        recorded[at[0]],
        DisplayCommand::Rebased {
            epoch: 9,
            kind: RebaseKind::Balloon,
            shows: vec![
                RebasedShow {
                    scope: ActorKey::from("0"),
                    surface_id: Some(5),
                    binds: BindSet::default(),
                },
                RebasedShow {
                    scope: ActorKey::from("1"),
                    surface_id: Some(0),
                    binds: BindSet::default(),
                },
            ],
        },
        "バルーンの今の面（無ければ 0）"
    );
    // シェル側は不変: 古い表のループが続き、動的な着せ替えも保たれる。
    let after = &recorded[at[0] + 1..];
    assert_eq!(after.len(), 1, "後の指令はシェルのループの 1 件: {after:?}");
    assert!(carries_frame(&after[0], 2110), "シェルのループは続く");
    assert!(
        matches!(&after[0], DisplayCommand::Show { surface_id: 2100, binds, .. }
            if *binds == BindSet::from_ids([1100, 1207, 1300])),
        "シェルの面と着せ替えは不変: {:?}",
        after[0]
    );
}

/// バルーンの差し替え: seriko が `\s`・`\b` を見ていないスコープも、新しい表の鍵（装着の全スコープ）
/// から面 0 で載る（起動時の装着は全スコープのバルーンを面 0 で確立している・要件 3.5）。
#[test]
fn balloon_replace_reports_unseen_scopes_from_new_tables_with_surface_zero() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        resolver_of("通常", 2100),
        BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    CueSink::emit(&mut sink, emote_cue(0.0, "0", "通常")); // seriko が見たのは scope 0 だけ
    assert!(sink.send_replace(SerikoReplace::Balloon {
        epoch: 3,
        balloon_tables: BTreeMap::from([
            (ActorKey::from("0"), AnimationTable::empty()),
            (ActorKey::from("1"), AnimationTable::empty()),
        ]),
    }));
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records.lock().expect("records mutex poisoned").clone();
    let at = rebased_positions(&recorded);
    assert_eq!(at.len(), 1, "Rebased はちょうど 1 件: {recorded:?}");
    let DisplayCommand::Rebased { shows, .. } = &recorded[at[0]] else {
        unreachable!("rebased_positions は Rebased だけを返す");
    };
    assert!(
        shows.contains(&RebasedShow {
            scope: ActorKey::from("1"),
            surface_id: Some(0),
            binds: BindSet::default(),
        }),
        "見ていない scope 1 も面 0 で載る: {shows:?}"
    );
}

/// 殻を経ずに既存の処理へ届いた依頼は `error!` を 1 件残して捨てる（状態・発行は不変）。
#[test]
fn replace_reaching_handle_message_directly_logs_error_and_is_dropped() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut loop_runtime = inert_runtime();
    let mut out = MockSurfaceOutput::new();
    let records = out.records();

    let (logs, flow) = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Replace(Box::new(SerikoReplace::Balloon {
                epoch: 1,
                balloon_tables: BTreeMap::new(),
            })),
        )
    });

    assert_eq!(flow, ControlFlow::Continue(()), "ループは続く");
    assert_eq!(
        logs.matches("level=ERROR").count(),
        1,
        "error! は 1 件: {logs}"
    );
    assert!(records.lock().unwrap().is_empty(), "発行は 0 件");
}

//! 表情の受け口が合図を受け取った場の判定の決定論テスト（spec: mcp-strict-errors 要件 2.7・4.1・4.2）。
//!
//! 知らせ付きの合図（`emit_with`）を実のアクターの受け口へ送り、知らせの口に届いた検出点の並びで
//! 確かめる。事実は手で組む（[`SkinFacts::new`]）。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::MockSurfaceOutput;
use crate::table::AnimationTable;
use areka_sakura::ActorKey;
use dola::cue::{CueFinding, CueProbe, CueSink};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

/// スコープ 0 の絵は `shell`、バルーンは `balloon`、別名 `笑顔` は 3。
fn facts(shell: &[u32], balloon: &[u32]) -> SkinFacts {
    let ids = |v: &[u32]| v.iter().copied().collect::<BTreeSet<u32>>();
    SkinFacts::new(
        BTreeMap::from([("笑顔".to_string(), vec![3])]),
        BTreeMap::from([(0, ids(shell))]),
        BTreeMap::from([(0, ids(balloon))]),
    )
}

fn balloon_cue(scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::BalloonSurface { key: key.into() },
        duration: 0.0,
    }
}

/// 届いた知らせを検出点の名前で控える口。
fn probe() -> (CueProbe, Arc<Mutex<Vec<&'static str>>>) {
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&got);
    let probe = CueProbe::new(move |f: CueFinding| sink.lock().unwrap().push(f.point));
    (probe, got)
}

fn spawn() -> (
    SerikoSink,
    areka_actor::ActorHandle,
    Arc<Mutex<Vec<DisplayCommand>>>,
) {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (sink, handle) = spawn_seriko(
        tiny_resolver(),
        crate::DefaultBinds::default(),
        BindResolver::empty(),
        cfg(AnimationTable::empty(), always_fire()),
        out,
    );
    (sink, handle, records)
}

fn names(points: &[DetectionPoint]) -> Vec<&'static str> {
    points.iter().map(|p| p.finding().point).collect()
}

/// 事実が無い（装着の前）ときは判定しないが、合図は今と同じに届ける。
#[test]
fn no_facts_judges_nothing_and_still_delivers() {
    let (mut sink, handle, out) = spawn();
    let (probe, got) = probe();
    sink.emit_with(emote_cue(0.0, "0", "-2"), &probe);
    sink.emit_with(balloon_cue("0", "99"), &probe);
    sink.emit_with(emote_cue(0.0, "0", "通常"), &probe);
    sink.close().unwrap();
    handle.join().unwrap();
    assert!(got.lock().unwrap().is_empty());
    assert!(
        out.lock()
            .unwrap()
            .iter()
            .any(|c| matches!(c, DisplayCommand::Show { .. })),
        "通常（2100）は今と同じに表示される"
    );
}

/// `\s`・数の `\b` を表情の本体と同じ解決器で判定する。名前の形・非表示・絵の無いスコープは判定しない。
#[test]
fn judges_surface_and_numeric_balloon_by_facts() {
    let (mut sink, handle, _out) = spawn();
    sink.set_facts(facts(&[0, 3], &[0]));
    let (probe, got) = probe();
    for cue in [
        emote_cue(0.0, "0", "笑顔"),    // 別名で在る
        emote_cue(0.0, "0", "-1"),      // 非表示
        emote_cue(0.0, "0", "-2"),      // 範囲外
        emote_cue(0.0, "0", "99999"),   // シェルに無い
        balloon_cue("0", "0"),          // 在る
        balloon_cue("0", "-1"),         // 非表示
        balloon_cue("0", "-2"),         // 範囲外
        balloon_cue("0", "99"),         // バルーンに無い
        balloon_cue("0", "バルーン１"), // 名前の形（文字の層の持ち分）
        emote_cue(0.0, "1", "-2"),      // スコープ 1 に絵が無い
        balloon_cue("1", "99"),         // スコープ 1 にバルーンが無い
    ] {
        sink.emit_with(cue, &probe);
    }
    sink.close().unwrap();
    handle.join().unwrap();
    assert_eq!(
        *got.lock().unwrap(),
        names(&[
            DetectionPoint::SurfaceUnresolved,
            DetectionPoint::SurfaceAbsent,
            DetectionPoint::BalloonInvalid,
            DetectionPoint::BalloonAbsent,
        ])
    );
}

/// 差し替えの前に配った合図は古い事実で、後に配った合図は新しい事実で判定する（要件 4.1）。
#[test]
fn replace_swaps_facts_between_cues() {
    let (mut sink, handle, _out) = spawn();
    sink.set_facts(facts(&[0], &[0]));
    let (probe, got) = probe();
    sink.emit_with(emote_cue(0.0, "0", "5"), &probe); // 古いシェルに 5 は無い
    sink.emit_with(balloon_cue("0", "2"), &probe); // 古いバルーンに 2 は無い
    let old = facts(&[0], &[0]);
    let shell_ids = BTreeMap::from([(0, BTreeSet::from([0, 5]))]);
    assert!(sink.send_replace(
        SerikoReplace::Shell {
            epoch: 1,
            resolver: tiny_resolver(),
            static_binds: crate::DefaultBinds::default(),
            bind_resolver: BindResolver::empty(),
            shell_table: AnimationTable::empty(),
            fallback: crate::ShellSurfaceFallback::default(),
        },
        old.with_shells(BTreeMap::new(), shell_ids),
    ));
    sink.emit_with(emote_cue(0.0, "0", "5"), &probe); // 新しいシェルに 5 は在る
    sink.emit_with(emote_cue(0.0, "0", "笑顔"), &probe); // 新しい別名表に笑顔は無い
    sink.emit_with(balloon_cue("0", "2"), &probe); // バルーンは前のまま
    let shell_now = SkinFacts::new(
        BTreeMap::new(),
        BTreeMap::from([(0, BTreeSet::from([0, 5]))]),
        BTreeMap::from([(0, BTreeSet::from([0]))]),
    );
    assert!(sink.send_replace(
        SerikoReplace::Balloon {
            epoch: 2,
            balloon_tables: BTreeMap::from([(ActorKey::from("0"), AnimationTable::empty())]),
        },
        shell_now.with_balloons(BTreeMap::from([(0, BTreeSet::from([0, 2]))])),
    ));
    sink.emit_with(balloon_cue("0", "2"), &probe); // 新しいバルーンに 2 は在る
    sink.emit_with(emote_cue(0.0, "0", "5"), &probe); // シェルは前のまま
    sink.close().unwrap();
    handle.join().unwrap();
    assert_eq!(
        *got.lock().unwrap(),
        names(&[
            DetectionPoint::SurfaceAbsent,
            DetectionPoint::BalloonAbsent,
            DetectionPoint::SurfaceUnresolved,
            DetectionPoint::BalloonAbsent,
        ])
    );
}

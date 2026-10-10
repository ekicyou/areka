//! 口パクの検体を本物の読み手・本物の表・本物のアクターで通す（spec: areka-P0-seriko-trigger-intervals
//! 要件 1.5・7.1・7.2・9.6・9.7・11.1・tasks.md 7.1・design「口パクの検体」）。
//!
//! 検体 `tests/fixtures/trigger-intervals/surfaces.txt` を読み手で読み、表を組み、アクターの受け口へ
//! 面の切り替え・文字・刻みを届いた順に流して、出てくる指令のコマで確かめる。固定するのは「読み手・
//! 表・アクターが検体について同じ答えを出す」ことだけで、境目の計算は下の層の檻が固定している。
//!
//! 時計は手で進める偽物で、どのメッセージもその直前に時計を合わせる（観測を追い越さない）。文字は
//! 1 字 50 ms。刻みは文字の現れる時刻・コマの替わる時刻ちょうどを避けて置く。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_emo_compose::{BindSet, EmoWorld};
use dola::cue::CueSink;
use log_capture_kit::{LineFormat, capture_lines};
use std::collections::BTreeMap;
use std::sync::Arc;

/// 検体の本文（面 9100〜9104。役の表は同じフォルダの `README.md`）。
const FIXTURE: &str = include_str!("../tests/fixtures/trigger-intervals/surfaces.txt");

/// 9100 の `talk,3`（口）・`runonce`（キラリ）・`periodic,2`（紅）の animation の番号。
const MOUTH: u32 = 0;
const ONCE: u32 = 1;
const CHEEK: u32 = 2;
/// 部品の面（9101 が置く）。animation 0 が `talk,2`・animation 1 が `runonce`。
const PART: u32 = 9102;

/// 本文を本番と同じ順（読み手 → 畳み込み → 表）で表にし、その走行の記録を 1 件 1 行で添える。
fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
    })
}

fn config() -> SerikoLoopConfig {
    cfg(table_and_logs(FIXTURE).0, always_fire())
}

fn count(lines: &[String], needle: &str) -> usize {
    lines.iter().filter(|l| l.contains(needle)).count()
}

/// 表を組むときの記録の件数を、検体を手で数えた数と比べる。
///
/// - 採録の `debug!` は 5 件＝9100 の 3 本（`talk,3`・`runonce`・`periodic,2`）＋ 9102 の 2 本
///   （`talk,2`・`runonce`）。
/// - `warn!` は検体の全体でちょうど 3 件＝9103 の無効な数値 2 件（`talk,abc`・`periodic,0`）＋ 9104 の
///   空のコマ列 1 件。9103 の `Talk,3`（大文字混じり）は今までどおり `debug!` だけ。
///
/// 採らなかった animation は表に無い（要件 1.5・7.1・7.2）。
#[test]
fn fixture_table_logs_five_adoptions_and_exactly_three_warnings() {
    let (table, logs) = table_and_logs(FIXTURE);

    assert_eq!(count(&logs, "引き金の語を採録"), 5, "{logs:#?}");
    assert_eq!(count(&logs, "level=WARN"), 3, "{logs:#?}");

    let invalid: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("talk/periodic の数値が無効ゆえ非採録"))
        .collect();
    assert_eq!(invalid.len(), 2, "{logs:#?}");
    for (line, animation, spelling) in [(invalid[0], 0, "talk,abc"), (invalid[1], 1, "periodic,0")]
    {
        assert!(
            line.contains("level=WARN")
                && line.contains("surface_id=9103")
                && line.contains(&format!("animation_id={animation}"))
                && line.contains(&format!("vocab=\"{spelling}\"")),
            "面の番号・animation の番号・元の綴りを添える: {line}"
        );
    }

    let empty: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("コマ列が空のアニメは非採録"))
        .collect();
    assert_eq!(empty.len(), 1, "{logs:#?}");
    assert!(
        empty[0].contains("level=WARN")
            && empty[0].contains("surface_id=9104")
            && empty[0].contains("animation_id=0"),
        "{}",
        empty[0]
    );

    let ids =
        |surface: u32| -> Vec<u32> { table.animations(surface).iter().map(|a| a.id).collect() };
    assert_eq!(ids(9100), [MOUTH, ONCE, CHEEK]);
    assert!(ids(9101).is_empty(), "9101 の一番上は 3 語を持たない");
    assert_eq!(ids(PART), [0, 1]);
    assert!(ids(9103).is_empty(), "無効な数値と大文字混じりは採らない");
    assert!(ids(9104).is_empty(), "空のコマ列は採らない");
}

/// 9100: 切り替えの指令が `runonce` の頭のコマを載せ（`periodic` はまだ鳴らない）、3 文字目が現れた
/// 時刻から口が動き、面に入った時刻から 2 秒ちょうどで `periodic` が鳴る。開始はどれも出来事の時刻で、
/// 刻みの時刻ではない。
#[test]
fn top_surface_fires_runonce_talk_and_periodic() {
    let mut rig = ClockRig::with_config(config());
    rig.tick(1000);
    let first = rig.hear(1000, emote_cue(0.0, "0", "9100"));
    assert_eq!(frames(&first, ONCE), [("0", Some(1700))], "切り替えで鳴る");
    assert_eq!(frames(&first, MOUTH), [("0", None)]);
    assert_eq!(
        frames(&first, CHEEK),
        [("0", None)],
        "`periodic` は切り替わった瞬間には鳴らない"
    );

    // 3 文字は 1000・1050・1100。口は 1200 → 80ms で 1202 → 80ms で `-1`。
    rig.hear(1000, clear_all());
    rig.hear(1000, text(0.0, "0", "あいう"));
    assert!(rig.tick(1090).is_empty(), "2 文字ではまだ");
    assert_eq!(frames(&rig.tick(1120), MOUTH), [("0", Some(1200))]);
    assert_eq!(
        frames(&rig.tick(1185), MOUTH),
        [("0", Some(1202))],
        "開始は 3 文字目の 1100（刻みの 1120 から数えれば 1200 まで 1 枚目のまま）"
    );
    let closed = rig.tick(1265);
    assert_eq!(frames(&closed, MOUTH), [("0", None)], "`-1` で口を消す");
    assert_eq!(frames(&closed, ONCE), [("0", Some(1700))], "キラリは 300ms");
    assert_eq!(frames(&rig.tick(1310), ONCE), [("0", None)]);

    // 面に入ったのは 1000。紅は 1600 → 500ms で `-1`。
    assert!(rig.tick(2990).is_empty(), "2 秒はまだ");
    let lap = rig.tick(3010);
    assert_eq!(frames(&lap, CHEEK), [("0", Some(1600))]);
    assert_eq!(
        frames(&lap, ONCE),
        [("0", None)],
        "`runonce` は 2 回目が無い"
    );
    assert_eq!(
        frames(&rig.tick(3505), CHEEK),
        [("0", None)],
        "開始は周の境目の 3000（刻みの 3010 から数えれば 3510 まで出たまま）"
    );
}

/// 9101 が部品 9102 を置く: 外側の面へ切り替えると部品の `runonce` が鳴り、文字が届くと部品の
/// `talk,2` が 2 文字目の時刻から鳴る。9101 の一番上は 3 語を持たないので、一番上の欄は空のまま。
#[test]
fn part_fires_runonce_and_talk_when_the_outer_surface_shows_it() {
    let mut rig = ClockRig::with_config(config());
    rig.tick(1000);
    let (surface, _, p) = single_show(rig.hear(1000, emote_cue(0.0, "0", "9101")));
    assert_eq!(surface, 9101);
    assert_eq!(part_frames(&p, PART), [(1, 1701)], "見えた瞬間に `runonce`");
    assert!(top_frames(&p).is_empty());

    // 3 文字は 1000・1050・1100。部品の口は 1203 → 100ms で `-1`。
    rig.hear(1000, clear_all());
    rig.hear(1000, text(0.0, "0", "あいう"));
    assert!(rig.tick(1040).is_empty(), "1 文字ではまだ");
    let (_, _, p) = single_show(rig.tick(1060));
    assert_eq!(part_frames(&p, PART), [(0, 1203), (1, 1701)]);
    assert!(top_frames(&p).is_empty());
    let (_, _, p) = single_show(rig.tick(1155));
    assert_eq!(
        part_frames(&p, PART),
        [(1, 1701)],
        "開始は 2 文字目の 1050（刻みの 1060 から数えれば 1160 まで出たまま）"
    );
    let (_, _, p) = single_show(rig.tick(1310));
    assert!(part_frames(&p, PART).is_empty(), "`runonce` は 300ms");
    assert!(top_frames(&p).is_empty());
}

/// 9103（無効な数値・大文字混じり）と 9104（空のコマ列）: 採らなかった animation はどれもコマを
/// 持つか `runonce` なので、採っていれば切り替え・文字・2 秒のどこかで指令が出る。出るのは切り替えの
/// 空の `Show` 1 件だけ。
#[test]
fn surfaces_whose_words_were_refused_stay_still() {
    for (key, surface) in [("9103", 9103), ("9104", 9104)] {
        let mut rig = ClockRig::with_config(config());
        rig.tick(1000);
        let (shown, _, p) = single_show(rig.hear(1000, emote_cue(0.0, "0", key)));
        assert_eq!(shown, surface);
        assert!(p.is_empty(), "{key}: 切り替えの指令は何も載せない: {p:?}");

        rig.hear(1000, clear_all());
        rig.hear(1000, text(0.0, "0", "あいうえおか"));
        for now in [1120, 1260, 2990, 3010, 3600] {
            assert!(rig.tick(now).is_empty(), "{key}: {now}");
        }
    }
}

/// 本物のアクター（別スレッド・`CueSink` から届く cue）でも 9100 の 3 語が鳴る。時計は 1000 に
/// 止めたままなので、cue はどれも 1000 に届いたことになる（刻みは自分の時刻を運ぶ）。
#[test]
fn spawned_actor_plays_the_three_words_of_the_fixture() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko_clocked(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([]),
        BindResolver::empty(),
        config(),
        out,
        Some(Arc::new(|| 1000)),
    );
    sink.send_tick(1000);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "9100"));
    CueSink::emit(&mut sink, clear_all());
    CueSink::emit(&mut sink, text(0.0, "0", "あいう"));
    for now in [1120, 1185, 1265, 1310, 3010, 3505] {
        sink.send_tick(now);
    }
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let records = records.lock().unwrap();
    // 行は指令 1 件・列は（口, キラリ, 紅）。
    let cells: Vec<[Option<u32>; 3]> = records
        .iter()
        .map(|c| match c {
            DisplayCommand::Show { pattern, .. } => {
                [MOUTH, ONCE, CHEEK].map(|anim| pattern.get(anim).map(|f| f.surface_id))
            }
            other => panic!("Show を期待: {other:?}"),
        })
        .collect();
    assert_eq!(
        cells,
        [
            [None, Some(1700), None],
            [Some(1200), Some(1700), None],
            [Some(1202), Some(1700), None],
            [None, Some(1700), None],
            [None, None, None],
            [None, None, Some(1600)],
            [None, None, None],
        ]
    );
}

/// 検体は実機の確かめで emo2 の `surfaces.txt` の末尾へ書き足す（tasks.md 9.2）。書き足した本文でも
/// 記録の件数は同じで（emo2 の側は `warn!` 0 件・3 語 0 本）、検体の番号は emo2 と当たらず、検体が
/// 番号で指す絵（コマの先の面・element定義のファイル）は emo2 に全部在る。
#[test]
fn fixture_appended_to_emo2_keeps_the_counts_and_finds_every_picture() {
    let shell = crate::sample_test_support::emo2_root().join("shell/master");
    let emo2 = std::fs::read_to_string(shell.join("surfaces.txt")).expect("emo2 の surfaces.txt");
    let ours = [9100, 9101, PART, 9103, 9104];

    let (_, alone) = table_and_logs(&emo2);
    assert_eq!(count(&alone, "level=WARN"), 0, "{alone:#?}");
    assert_eq!(count(&alone, "引き金の語を採録"), 0, "{alone:#?}");
    let emo2_world = EmoWorld::build(&areka_parsers::shell::parse(&emo2));
    for id in ours {
        assert!(emo2_world.surface(id).is_none(), "{id} は emo2 に無い番号");
    }

    let joined = format!("{emo2}\r\n{FIXTURE}");
    let (_, logs) = table_and_logs(&joined);
    assert_eq!(count(&logs, "level=WARN"), 3, "{logs:#?}");
    assert_eq!(count(&logs, "引き金の語を採録"), 5, "{logs:#?}");

    let world = EmoWorld::build(&areka_parsers::shell::parse(&joined));
    for id in ours {
        let surface = world.surface(id).unwrap_or_else(|| panic!("{id} が在る"));
        for frame in surface.animations.iter().flat_map(|a| &a.patterns) {
            if let Ok(target) = u32::try_from(frame.surface_id) {
                assert!(world.surface(target).is_some(), "{id} のコマの先 {target}");
            }
        }
        for element in &surface.elements {
            let file = element.path.as_str();
            let found = match file.parse::<u32>() {
                Ok(child) => world.surface(child).is_some(),
                Err(_) => shell.join(file).is_file(),
            };
            assert!(found, "{id} の element定義の絵 {file}");
        }
    }
}

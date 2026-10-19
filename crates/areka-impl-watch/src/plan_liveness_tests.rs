//! `plan::turns` の行き詰まりの無さのテスト（要件 3.6）: 名前のある場面と、参加者 3 人の小さな
//! 全探索。支えは親の `plan_test_support.rs` を読むだけ。

use std::collections::{BTreeSet, HashSet};

use super::super::test_support::{
    REPO, T0, assert_not_applied, give_back, load_test, request, run, state_with, stopped,
};
use super::replan;
use crate::plan::{Command, Event, Verdict};
use crate::state::{DeskSet, MergeDesk, ParticipantStatus, State};

/// 全探索の深さ（手数）。debug の `cargo test -p areka-impl-watch` の中で 0.5〜0.7 秒（7 は 1.0〜1.3 秒）
/// と確かめた値。機械の速さで変えない。
const DEPTH: usize = 6;

/// 協力的な進め方の手数の上限（3 人がそれぞれ申し込み・受け・返し・マージを済ませるのに足りる）。
const COOP_STEPS: usize = 40;

/// 全探索の参加者とリポジトリ。
const IDS: [&str; 3] = ["A", "B", "C"];
const REPOS: [&str; 2] = ["r1", "r2"];

fn merge(id: &str, repo: &str) -> Command {
    Command::Merge {
        id: id.to_owned(),
        name: None,
        repo: repo.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
    }
}

fn merged(id: &str) -> Command {
    Command::Merged {
        id: id.to_owned(),
        pr: "#1".to_owned(),
        sha: "abc".to_owned(),
    }
}

fn load_running(id: &str) -> Command {
    Command::LoadRunning {
        id: id.to_owned(),
        name: None,
        repo: REPO.to_owned(),
        purpose: "load".to_owned(),
    }
}

/// 設計の字母: `merge`（2 つのリポジトリ）・`request` の 3 つの組・名前なしの `return`・
/// 名指しの `return`（`test`・`sole`）・`loadrunning`・`merged`・`stopped`・`cancel`。
fn alphabet() -> Vec<Command> {
    let mut commands = Vec::new();
    for id in IDS {
        for repo in REPOS {
            commands.push(merge(id, repo));
        }
        for desks in [DeskSet::Test, DeskSet::Sole, DeskSet::TestSole] {
            commands.push(request(id, desks));
        }
        commands.push(give_back(id, None));
        commands.push(give_back(id, Some(DeskSet::Test)));
        commands.push(give_back(id, Some(DeskSet::Sole)));
        commands.push(load_running(id));
        commands.push(merged(id));
        commands.push(stopped(id));
        commands.push(Command::Cancel { id: id.to_owned() });
    }
    commands
}

/// 判断に効かず複製を重くするだけの項目（最近の出来事・見張りと待ちの記録・名前・リポジトリ・
/// 内容・spec・直前のマージ・停止要請の理由の中身）を空にする。生死の口は全員「居る」と答えるので、
/// 空にしても回収は起きず、番の決め方も変わらない。
fn bare(mut state: State) -> State {
    state.recent.clear();
    state.waits.clear();
    for participant in state.participants.values_mut() {
        participant.name.clear();
        participant.repo.clear();
        participant.watch = None;
        participant.awaiting_watch_since = None;
        if let Some(reason) = participant.stop_reason.as_mut() {
            reason.by.clear();
            reason.purpose.clear();
        }
    }
    for desk in state.merge.values_mut() {
        desk.last = None;
        desk.queue
            .iter_mut()
            .for_each(|request| request.spec.clear());
        if let Some(holder) = desk.holder.as_mut() {
            holder.spec.clear();
        }
    }
    state
        .requests
        .iter_mut()
        .for_each(|request| request.purpose.clear());
    if let Some(holder) = state.test.as_mut() {
        holder.purpose.clear();
    }
    if let Some(holder) = state.sole.as_mut() {
        holder.purpose.clear();
        holder.stopped.clear();
    }
    state
}

/// 参加者 3 人の入れ替え（`PERMS[k][n]` は `IDS[n]` の新しい番号）。先頭は入れ替えなし。
const PERMS: [[u8; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

/// 重複を除く鍵: 判断に効く項目だけを並べたバイト列を、参加者 3 人の入れ替え（6 通り）で最も
/// 小さいものにする（判断は名前に依らない）。リポジトリ 2 つの入れ替えは [`key_as`] が吸収する。
fn key(state: &State) -> Vec<u8> {
    let keys = PERMS.iter().map(|perm| key_as(state, perm));
    keys.min().expect("6 通り")
}

/// 参加者を `perm` で付け替えた鍵。時刻は固定・生死は全員「居る」なので、参加者は状態だけ・
/// 申し込みとマージの待ちは並びの順だけが効く。名前・内容・最近の出来事・直前のマージ・見張りの
/// 記録・空のマージの机は判断に効かないので外す。リポジトリ同士は独立なので、2 つの机の綴りは
/// 並べ替えてから繋ぐ。
fn key_as(state: &State, perm: &[u8; 3]) -> Vec<u8> {
    let code = |id: &str| match IDS.iter().position(|known| *known == id) {
        Some(nth) => b'a' + perm[nth],
        None => id.as_bytes()[0],
    };
    let holder = |id: Option<&String>| id.map_or(b'-', |id| code(id));
    let mut key = vec![b'-'; 3];
    for (nth, id) in IDS.iter().enumerate() {
        if let Some(participant) = state.participants.get(*id) {
            key[usize::from(perm[nth])] = match participant.status {
                ParticipantStatus::Working => b'w',
                ParticipantStatus::StopRequested => b'q',
                ParticipantStatus::Stopped => b's',
            };
        }
    }
    key.push(holder(state.test.as_ref().map(|holder| &holder.id)));
    key.push(holder(state.sole.as_ref().map(|holder| &holder.id)));
    key.push(u8::from(
        state.sole.as_ref().is_some_and(|sole| sole.running),
    ));
    for request in &state.requests {
        key.push(code(&request.id));
        key.push(u8::from(request.desks.has_test()) + 2 * u8::from(request.desks.has_sole()));
    }
    let mut desks: Vec<Vec<u8>> = REPOS
        .iter()
        .map(|repo| {
            let desk = state.merge.get(*repo);
            let empty = |desk: &&MergeDesk| desk.holder.is_none() && desk.queue.is_empty();
            let mut seg = vec![b'|'];
            if let Some(desk) = desk.filter(|desk| !empty(desk)) {
                seg.push(holder(desk.holder.as_ref().map(|holder| &holder.id)));
                seg.extend(desk.queue.iter().map(|waiting| code(&waiting.id)));
            }
            seg
        })
        .collect();
    desks.sort();
    key.extend(desks.concat());
    key
}

fn holds_merge(state: &State, id: &str) -> bool {
    state
        .merge
        .values()
        .any(|desk| desk.holder.as_ref().is_some_and(|holder| holder.id == id))
}

fn in_requests(state: &State, id: &str) -> bool {
    state.requests.iter().any(|request| request.id == id)
}

fn holds_desk(state: &State, id: &str) -> bool {
    state.test.as_ref().is_some_and(|holder| holder.id == id)
        || state.sole.as_ref().is_some_and(|holder| holder.id == id)
}

/// 到達した状態の不変の決まりと固定点。`trail` は失敗の見出しに出す手順。
fn check_invariants(state: &State, trail: &str) {
    let mut seen = BTreeSet::new();
    for request in &state.requests {
        assert!(
            !holds_desk(state, &request.id),
            "申し込みを待つ者が机を持つ: {trail}"
        );
        assert!(seen.insert(&request.id), "申し込みが 2 つ: {trail}");
    }
    if let Some(sole) = state.sole.as_ref().filter(|sole| sole.running) {
        let test = state.test.as_ref().map(|holder| holder.id.as_str());
        assert_eq!(test, Some(sole.id.as_str()), "走っている印の組: {trail}");
    }
    // 待ちの輪が生まれない形（4.7・4.8・2.8・5.7）。
    let owners = state.test.iter().map(|holder| &holder.id);
    let owners = owners.chain(state.sole.iter().map(|holder| &holder.id));
    let blocked: BTreeSet<&String> = owners
        .chain(state.requests.iter().map(|request| &request.id))
        .collect();
    for (repo, desk) in &state.merge {
        for waiting in &desk.queue {
            assert!(
                !blocked.contains(&waiting.id),
                "{} が {repo} のマージを待ちながら机を持つか申し込む: {trail}",
                waiting.id
            );
        }
    }
    // 固定点: `replan` を 2 度続けて呼んだ 2 度目は何も変えない。
    let mut once = state.clone();
    replan(&mut once, T0, &mut Vec::new());
    let mut twice = once.clone();
    let mut events = Vec::new();
    replan(&mut twice, T0, &mut events);
    assert_eq!(once, twice, "固定点: {trail}");
    assert!(events.is_empty(), "固定点の出来事: {trail}: {events:?}");
}

/// 協力的な進め方の次の 1 手（状態だけで決まる）。返す者がマージの持ち主なら、続けて `merged`
/// まで進める（完了の手順の本筋: `request --desk test` → 受けたら返す → `merged`）。
fn cooperative_move(state: &State) -> Option<Command> {
    // 排他テストの机とひとり占めの持ち主は返す。
    if let Some(id) = state.test.as_ref().map(|holder| &holder.id) {
        return Some(give_back(id, None));
    }
    if let Some(id) = state.sole.as_ref().map(|holder| &holder.id) {
        return Some(give_back(id, None));
    }
    // マージの持ち主は `request --desk test` を申し込む。別のリポジトリのマージを待つ間は断られる
    // （4.8）ので、排他テストを飛ばして `merged` へ進む（待ちの行列からも外れる）。
    for desk in state.merge.values() {
        let Some(id) = desk.holder.as_ref().map(|holder| &holder.id) else {
            continue;
        };
        if in_requests(state, id) {
            continue;
        }
        let waits = state.merge.values().flat_map(|desk| &desk.queue);
        return Some(if waits.into_iter().any(|request| request.id == *id) {
            merged(id)
        } else {
            request(id, DeskSet::Test)
        });
    }
    // 申し込みの無い停止要請中の者は `stopped`。ひとり占めを含む申し込みを待つ停止要請中の者も、
    // 見張りの停止要請に従って `stopped` する（番は止まったままでも来る・決まり 5）。排他テストの机
    // だけを待つ停止要請中の者は番が来る（決まり 3）ので待つ。止まった者は待つ。
    let only_test = |id: &str| {
        let mut own = state.requests.iter().filter(|request| request.id == id);
        own.any(|request| !request.desks.has_sole())
    };
    let asked = state.participants.values().find(|participant| {
        participant.status == ParticipantStatus::StopRequested && !only_test(&participant.id)
    });
    asked.map(|participant| stopped(&participant.id))
}

/// 協力的な進め方を上限まで進め、出来事を返す。申し込みとマージの待ち・持ち主が全部空にならなければ赤。
/// `drained` は空になると確かめた状態の鍵（進め方は状態だけで決まるので、途中で出会えばそこで済む）。
fn drive_cooperatively(
    state: &mut State,
    drained: &mut HashSet<Vec<u8>>,
    trail: &str,
) -> Vec<Event> {
    let mut path = Vec::new();
    let mut events = Vec::new();
    for _ in 0..COOP_STEPS {
        let raw = key_as(state, &PERMS[0]);
        if drained.contains(&raw) {
            break;
        }
        path.push(raw);
        let Some(cmd) = cooperative_move(state) else {
            break;
        };
        let merging = match &cmd {
            Command::Return { id, .. } if holds_merge(state, id) => Some(id.clone()),
            _ => None,
        };
        events.extend(run(state, &cmd, None, T0).events);
        if let Some(id) = merging {
            events.extend(run(state, &merged(&id), None, T0).events);
        }
    }
    let empty = state.requests.is_empty()
        && state.test.is_none()
        && state.sole.is_none()
        && state
            .merge
            .values()
            .all(|desk| desk.holder.is_none() && desk.queue.is_empty());
    let known = drained.contains(&key_as(state, &PERMS[0]));
    assert!(
        empty || known,
        "協力的な進め方で空にならない: {trail}: {state:?}"
    );
    drained.extend(path);
    events
}

/// 机の番が来た識別（出来事の順）。
fn granted_ids(events: &[Event]) -> Vec<String> {
    let granted = events.iter().filter_map(|event| match event {
        Event::DeskGranted { id, .. } => Some(id.clone()),
        _ => None,
    });
    granted.collect()
}

/// 名前のある場面: マージの持ち主 M の排他テストの申し込み・L の負荷テストの待ち・停止要請中の U の
/// UI テストの申し込みを `order` の順に重ね（U は L の後）、協力的な進め方で全部が渡ることを確かめる。
fn overlapping_scene(order: &[&str]) {
    let mut state = state_with(&["L", "M", "U"]);
    run(&mut state, &merge("M", "r1"), Some("M"), T0);
    assert!(holds_merge(&state, "M"));
    let mut events = Vec::new();
    for who in order {
        let cmd = match *who {
            "L" => load_test(who),
            _ => request(who, DeskSet::Test),
        };
        if *who == "U" {
            let status = state.participants["U"].status;
            assert_eq!(status, ParticipantStatus::StopRequested, "{order:?}");
        }
        let got = run(&mut state, &cmd, Some(who), T0);
        assert_eq!(got.verdict, Verdict::Applied, "{who} の申し込み: {order:?}");
        events.extend(got.events);
    }
    let trail = format!("{order:?}");
    check_invariants(&state, &trail);
    events.extend(drive_cooperatively(&mut state, &mut HashSet::new(), &trail));
    let granted: BTreeSet<String> = granted_ids(&events).into_iter().collect();
    assert_eq!(
        granted,
        BTreeSet::from(["L", "M", "U"].map(str::to_owned)),
        "{order:?}"
    );
    assert!(
        !state.participants.contains_key("M"),
        "M はマージを済ませた"
    );
}

#[test]
fn merge_holder_test_load_wait_and_stop_requested_ui_test_all_drain() {
    // L の負荷テストが先に待ち、停止要請中の U が UI テストを申し込み、マージの持ち主 M が足す。
    overlapping_scene(&["L", "U", "M"]);
    // 並べ替え（U は停止要請を受けてからなので L の後）。
    overlapping_scene(&["L", "M", "U"]);
    overlapping_scene(&["M", "L", "U"]);
}

#[test]
fn overlapping_scene_grants_every_request_through_the_cooperative_run() {
    let mut state = state_with(&["L", "M", "U"]);
    run(&mut state, &merge("M", "r1"), Some("M"), T0);
    run(&mut state, &load_test("L"), Some("L"), T0);
    assert_eq!(
        state.participants["U"].status,
        ParticipantStatus::StopRequested
    );
    run(&mut state, &request("U", DeskSet::Test), Some("U"), T0);
    run(&mut state, &request("M", DeskSet::Test), Some("M"), T0);
    let events = drive_cooperatively(&mut state, &mut HashSet::new(), "named");
    let mut granted = vec!["U".to_owned()];
    granted.extend(granted_ids(&events));
    // U（停止要請中で直ちに）→ M（マージの持ち主へ譲る）→ L（止まりとマージが済んでから）。
    assert_eq!(granted, ["U", "M", "L"]);
}

#[test]
fn test_holder_merge_and_merge_waiter_request_are_refused_byte_for_byte() {
    // 排他テストの机の持ち主の `merge`（4.7）。
    let mut state = state_with(&["A", "B"]);
    run(&mut state, &request("A", DeskSet::Test), Some("A"), T0);
    assert!(holds_desk(&state, "A"));
    let before = serde_json::to_string(&state).expect("直列化");
    let got = run(&mut state, &merge("A", "r1"), Some("A"), T0);
    assert_not_applied(&got, "排他テストの机の持ち主の merge");
    assert_eq!(serde_json::to_string(&state).expect("直列化"), before);

    // マージを待つ者の `request`（4.8）。
    let mut waiter = state_with(&["A", "B"]);
    run(&mut waiter, &merge("A", "r1"), Some("A"), T0);
    run(&mut waiter, &merge("B", "r1"), Some("B"), T0);
    assert!(waiter.merge["r1"].queue.iter().any(|q| q.id == "B"));
    for desks in [DeskSet::Test, DeskSet::Sole, DeskSet::TestSole] {
        let before = serde_json::to_string(&waiter).expect("直列化");
        let got = run(&mut waiter, &request("B", desks), Some("B"), T0);
        assert_not_applied(&got, "マージを待つ者の request");
        assert_eq!(serde_json::to_string(&waiter).expect("直列化"), before);
    }
}

/// 1 つの段の状態から字母の全部の手を打ち、状態が変わった行き先を（鍵, 状態, 手順）で返す。
fn expand(part: &[(State, String)], commands: &[Command]) -> Vec<(Vec<u8>, State, String)> {
    // 生の鍵（入れ替えをしない）で一度見た行き先は、入れ替えの 6 通りを数え直さない。
    let mut raw = HashSet::new();
    let mut out = Vec::new();
    for (state, trail) in part {
        for cmd in commands {
            let mut after = state.clone();
            if !run(&mut after, cmd, None, T0).changed {
                continue;
            }
            let after = bare(after);
            if raw.insert(key_as(&after, &PERMS[0])) {
                out.push((key(&after), after, format!("{trail} {cmd:?};")));
            }
        }
    }
    out
}

#[test]
fn every_reachable_state_keeps_the_invariants_and_drains_cooperatively() {
    let commands = alphabet();
    let start = bare(state_with(&IDS));
    let mut drained = HashSet::new();
    let mut seen = HashSet::from([key(&start)]);
    let mut frontier = vec![(start, String::new())];
    // 行き先の計算は段ごとに分けて並べて回し、段の順に繋ぐ（重複の除き方と順は一人で回すのと同じ）。
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    for depth in 0..=DEPTH {
        for (state, trail) in &frontier {
            check_invariants(state, trail);
            drive_cooperatively(&mut state.clone(), &mut drained, trail);
        }
        if depth == DEPTH {
            break;
        }
        let chunk = frontier.len().div_ceil(threads).max(1);
        let found: Vec<_> = std::thread::scope(|scope| {
            let parts = frontier.chunks(chunk);
            let workers: Vec<_> = parts
                .map(|part| scope.spawn(|| expand(part, &commands)))
                .collect();
            let joined = workers.into_iter().map(|w| w.join().expect("探索の糸"));
            joined.collect()
        });
        let found = found.into_iter().flatten();
        frontier = found
            .filter(|(key, ..)| seen.insert(key.clone()))
            .map(|(_, state, trail)| (state, trail))
            .collect();
    }
    // 探索が字母を実際に使えている（机・マージ・止まりが組み合わさる状態まで届く）。
    assert!(seen.len() > 1_000, "到達した状態の数: {}", seen.len());
}

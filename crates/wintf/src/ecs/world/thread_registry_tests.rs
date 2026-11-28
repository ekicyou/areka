//! `thread_registry` の決定論テスト。
//!
//! 名簿はプロセス共有で、テストは既定で並列に走る——**全体の件数は決して当てにしない**。
//! 各テストは自分の TID と用途札から一意な役割名を作り、その役割名の項目だけを検査する。
//! 実時間・CPU の閾値は 1 つも使わない。

use super::*;

/// このテストスレッド専用の一意な役割名を作る（他のテストの登録と混ざらないため）。
fn unique_role(tag: &str) -> String {
    role_actor(format!("test-{}-{}", get_current_thread_id(), tag))
}

/// 指定の役割名を持つ項目だけを名簿から抜き出す。
fn entries_with_role(role: &str) -> Vec<ThreadEntry> {
    snapshot().into_iter().filter(|e| e.role == role).collect()
}

#[test]
fn register_current_thread_puts_the_role_and_tid_into_the_snapshot() {
    let role = unique_role("register");
    let tid = get_current_thread_id();

    register_current_thread(role.as_str()).expect("自スレッドの登録は成功するはず");

    let found = entries_with_role(&role);
    assert_eq!(
        found.len(),
        1,
        "同じ役割名の項目は 1 件だけのはず: {found:?}"
    );
    assert_eq!(found[0].tid, tid, "登録した TID がそのまま一覧に出るはず");

    let current = std::thread::current();
    assert_eq!(
        found[0].name.as_deref(),
        current.name(),
        "OS 名は登録時のスレッド名をそのまま持つはず"
    );
}

fn folded_with_family(reading: &RegistryReading, family: &str) -> Vec<FoldedReading> {
    reading
        .folded
        .iter()
        .filter(|f| f.family == family)
        .cloned()
        .collect()
}

#[test]
fn a_joined_thread_leaves_the_live_entries_and_is_folded_into_its_family_total() {
    let role = unique_role("spawned");
    let role_for_thread = role.clone();
    let (gate_tx, gate_rx) = std::sync::mpsc::channel::<()>();
    let (seen_tx, seen_rx) = std::sync::mpsc::channel::<u32>();

    let joiner = std::thread::Builder::new()
        .name("wintf-registry-test-spawned".to_string())
        .spawn(move || {
            register_current_thread(role_for_thread.as_str()).expect("別スレッドの登録も成功する");
            let own = crate::api::duplicate_current_thread_handle().expect("自分のハンドルの複製");
            seen_tx.send(get_current_thread_id()).expect("TID を渡す");
            gate_rx.recv().expect("終わる合図を待つ");
            own
        })
        .expect("テスト用スレッドの生成は成功するはず");

    let spawned_tid = seen_rx.recv().expect("生成側の TID");
    let alive = entries_with_role(&role);
    assert_eq!(alive.len(), 1, "生きている間は 1 件見えるはず: {alive:?}");
    assert_eq!(alive[0].tid, spawned_tid);
    assert_ne!(
        alive[0].tid,
        get_current_thread_id(),
        "登録したのは別スレッド（対照）"
    );
    assert_eq!(
        alive[0].name.as_deref(),
        Some("wintf-registry-test-spawned"),
        "生成側の付けたスレッド名が読めるはず"
    );

    gate_tx.send(()).expect("終わる合図");
    let own = joiner.join().expect("テスト用スレッドは正常終了するはず");
    let last = crate::api::get_thread_times(&own).expect("終わったスレッドの最後の CPU");

    let reading = read_cpu();
    assert!(
        reading.live.iter().all(|live| live.role != role),
        "join の後は生きた項目に無いはず"
    );
    assert!(
        entries_with_role(&role).is_empty(),
        "snapshot にも残らないはず"
    );
    let folded = folded_with_family(&reading, &role);
    assert_eq!(
        folded,
        vec![FoldedReading {
            family: role.clone(),
            cpu: last,
            exited: 1,
        }],
        "その系統の畳んだ合計に最後の CPU が 1 本分入るはず"
    );
}

#[test]
fn role_family_folds_a_trailing_ascii_number_only() {
    for (role, family) in [
        ("actor:sakura-talk-17", "actor:sakura-talk-*"),
        ("actor:sakura-talk-0", "actor:sakura-talk-*"),
        ("actor:kanade", "actor:kanade"),
        ("vblank", "vblank"),
        ("-5", "-5"),
        ("actor:x-1a", "actor:x-1a"),
        ("actor:x-", "actor:x-"),
        ("actor:x-１", "actor:x-１"),
    ] {
        assert_eq!(role_family(role), family, "{role} の系統");
    }
}

#[test]
fn the_role_vocabulary_is_fixed() {
    assert_eq!(ROLE_VBLANK, "vblank");
    assert_eq!(ROLE_CURSOR_MONITOR, "cursor_monitor");
    assert_eq!(ROLE_UI, "ui");
    assert_eq!(ROLE_TICKER_DISPATCHER_KANADE, "ticker_dispatcher_kanade");
    assert_eq!(ROLE_TICKER_LOOP, "ticker_loop");
    assert_eq!(ROLE_ACTOR_PREFIX, "actor:");
    assert_eq!(ROLE_PERF_REPORT, "perf_report");
    assert_eq!(ROLE_UNREGISTERED_REST, "unregistered_rest");

    assert_eq!(
        ALL_ROLES,
        [
            "vblank",
            "cursor_monitor",
            "ui",
            "ticker_dispatcher_kanade",
            "ticker_loop",
            "actor:",
            "perf_report",
            "unregistered_rest",
        ]
        .as_slice(),
        "固定語彙は 8 語・この綴りと並びで固定する"
    );

    let mut sorted = ALL_ROLES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ALL_ROLES.len(), "語彙に重複は無い");
}

#[test]
fn role_actor_builds_a_prefixed_role_and_only_known_roles_are_accepted() {
    assert_eq!(role_actor("shell"), "actor:shell");
    assert!(is_known_role("actor:shell"));

    for role in ALL_ROLES {
        if *role == ROLE_ACTOR_PREFIX {
            assert!(
                !is_known_role(role),
                "接尾辞の無い `actor:` は役割名として成立しない"
            );
        } else {
            assert!(is_known_role(role), "固定語彙の {role} は既知のはず");
        }
    }

    assert!(!is_known_role(""));
    assert!(!is_known_role("actor"));
    assert!(!is_known_role("ticker"));
    assert!(!is_known_role("UI"), "大文字小文字は区別する");
}

#[test]
fn re_registering_the_same_thread_replaces_the_previous_entry() {
    let first = unique_role("replace-first");
    let second = unique_role("replace-second");

    register_current_thread(first.as_str()).expect("1 度目の登録");
    register_current_thread(second.as_str()).expect("2 度目の登録");

    assert!(
        entries_with_role(&first).is_empty(),
        "同じ TID の再登録で前の項目は残らない（報告器が二重計上しないため）"
    );
    let found = entries_with_role(&second);
    assert_eq!(found.len(), 1, "後から宣言した役割名だけが残るはず");
    assert_eq!(found[0].tid, get_current_thread_id());
    assert!(
        folded_with_family(&read_cpu(), &first).is_empty(),
        "生きている同じスレッドの登録し直しは畳まない"
    );
}

#[test]
fn cpu_times_are_readable_for_a_registered_thread_and_for_the_process() {
    let role = unique_role("cpu");
    register_current_thread(role.as_str()).expect("登録");

    let mut found = entries_with_role(&role);
    let entry = found.pop().expect("登録した項目が 1 件あるはず");

    let thread = entry
        .cpu_times()
        .expect("複製ハンドルから GetThreadTimes は成功するはず");
    assert_eq!(
        thread.total_100ns(),
        thread.kernel_100ns + thread.user_100ns,
        "合計はカーネルとユーザーの和"
    );
    assert_eq!(
        thread.total_us(),
        thread.total_100ns() / 10,
        "µs は 100ns の 1/10"
    );

    let process = get_process_times().expect("GetProcessTimes は成功するはず");
    assert_eq!(
        process.total_100ns(),
        process.kernel_100ns + process.user_100ns
    );
}

#[test]
fn a_running_thread_is_reported_as_not_exited() {
    let handle = crate::api::duplicate_current_thread_handle().expect("自スレッドのハンドルの複製");
    assert!(
        !crate::api::thread_has_exited(&handle).expect("生きたスレッドの見分けは成功するはず"),
        "走っている自スレッドは終わっていない"
    );
}

#[test]
fn a_joined_thread_is_reported_as_exited() {
    let handle = std::thread::Builder::new()
        .name("wintf-registry-test-exited".to_string())
        .spawn(|| crate::api::duplicate_current_thread_handle().expect("複製"))
        .expect("テスト用スレッドの生成は成功するはず")
        .join()
        .expect("テスト用スレッドは正常終了するはず");
    assert!(
        crate::api::thread_has_exited(&handle).expect("終わったスレッドの見分けは成功するはず"),
        "join した後の複製ハンドルは終わったと読めるはず"
    );
}

use std::collections::HashMap;

struct FakeProbe {
    exited: HashMap<u32, bool>,
    cpu: HashMap<u32, CpuTimes>,
}

impl FakeProbe {
    fn new() -> Self {
        Self {
            exited: HashMap::new(),
            cpu: HashMap::new(),
        }
    }

    fn alive(mut self, tid: u32) -> Self {
        self.exited.insert(tid, false);
        self
    }

    fn exited(mut self, tid: u32, kernel_100ns: u64, user_100ns: u64) -> Self {
        self.exited.insert(tid, true);
        self.cpu.insert(
            tid,
            CpuTimes {
                kernel_100ns,
                user_100ns,
            },
        );
        self
    }

    fn cpu_for(mut self, tid: u32, kernel_100ns: u64, user_100ns: u64) -> Self {
        self.cpu.insert(
            tid,
            CpuTimes {
                kernel_100ns,
                user_100ns,
            },
        );
        self
    }

    fn exited_unreadable(mut self, tid: u32) -> Self {
        self.exited.insert(tid, true);
        self
    }
}

impl ThreadProbe for FakeProbe {
    fn has_exited(&self, entry: &ThreadEntry) -> Result<bool> {
        Ok(self.exited[&entry.tid])
    }

    fn cpu_times(&self, entry: &ThreadEntry) -> Result<CpuTimes> {
        self.cpu
            .get(&entry.tid)
            .copied()
            .ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))
    }
}

fn fake_entry(role: &str, tid: u32) -> ThreadEntry {
    ThreadEntry {
        role: role.to_owned(),
        name: None,
        tid,
        handle: Arc::new(
            crate::api::duplicate_current_thread_handle().expect("自スレッドのハンドルの複製"),
        ),
    }
}

#[test]
fn a_reused_tid_after_exit_folds_the_old_cpu_into_the_old_family_once() {
    let mut registry = Registry::new();
    let none = registry.insert(
        fake_entry("actor:sakura-talk-1", 7),
        &FakeProbe::new().alive(7),
    );
    assert!(none.is_empty());

    let gone = FakeProbe::new().exited(7, 30, 12);
    let notes = registry.insert(fake_entry("ui", 7), &gone);
    assert!(notes.is_empty(), "{notes:?}");

    let alive = FakeProbe::new().alive(7).cpu_for(7, 100, 100);
    let (reading, notes) = registry.read(&alive);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(
        reading.folded,
        vec![FoldedReading {
            family: "actor:sakura-talk-*".to_owned(),
            cpu: CpuTimes {
                kernel_100ns: 30,
                user_100ns: 12
            },
            exited: 1,
        }],
        "前の系統へ前の CPU が 1 度だけ入るはず"
    );
    assert_eq!(reading.live.len(), 1, "生きた項目は新しい方だけ");
    assert_eq!(reading.live[0].role, "ui");
    assert_eq!(reading.live[0].tid, 7);

    let (again, _) = registry.read(&alive);
    assert_eq!(again.folded, reading.folded, "読み直しても二重に入らない");
}

#[test]
fn re_registering_a_live_tid_moves_neither_the_total_nor_the_count() {
    let mut registry = Registry::new();
    let probe = FakeProbe::new().alive(8);
    registry.insert(fake_entry("actor:sakura-talk-1", 8), &probe);
    registry.insert(fake_entry("actor:sakura-talk-2", 8), &probe);

    assert!(registry.folded.is_empty(), "{:?}", registry.folded);
    assert_eq!(registry.live.len(), 1);
    assert_eq!(registry.live[0].role, "actor:sakura-talk-2");
}

#[test]
fn an_exited_entry_with_unreadable_cpu_warns_with_tid_and_role_and_adds_only_the_count() {
    let mut registry = Registry::new();
    registry.insert(
        fake_entry("actor:sakura-talk-5", 9),
        &FakeProbe::new().alive(9),
    );

    let probe = FakeProbe::new().exited_unreadable(9);
    let ((), events) = log_capture_kit::capture(|| {
        tracing::warn!(target: "wintf::thread_registry_test_control", "control");
        let notes = registry.sweep(&probe);
        emit_notes(notes)
    });

    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.target == "wintf::thread_registry" && e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field("tid"), Some("9"), "{events:?}");
    assert_eq!(
        warns[0].field("role"),
        Some("actor:sakura-talk-5"),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e.target == "wintf::thread_registry_test_control"),
        "対照の事象が捕まらないなら捕捉が働いていない: {events:?}"
    );

    let total = registry.folded["actor:sakura-talk-*"];
    assert_eq!(total.exited, 1);
    assert_eq!((total.kernel_100ns, total.user_100ns), (0, 0));
    assert!(registry.live.is_empty());
}

fn sum_total_100ns(reading: &RegistryReading, family: &str) -> u64 {
    folded_with_family(reading, family)
        .iter()
        .map(|f| f.cpu.total_100ns())
        .sum()
}

#[test]
fn many_sequential_threads_fold_into_one_family_total_that_matches_the_last_cpu() {
    const COUNT: usize = 128;
    let prefix = unique_role("many");
    let family = format!("{prefix}-*");
    let mut last_sum = 0u64;
    let mut previous_total = 0u64;

    for i in 0..COUNT {
        let role = format!("{prefix}-{i}");
        let own = std::thread::Builder::new()
            .name(format!("wintf-registry-test-many-{i}"))
            .spawn(move || {
                register_current_thread(role).expect("別スレッドの登録も成功する");
                crate::api::duplicate_current_thread_handle().expect("自分のハンドルの複製")
            })
            .expect("テスト用スレッドの生成は成功するはず")
            .join()
            .expect("テスト用スレッドは正常終了するはず");
        last_sum += crate::api::get_thread_times(&own)
            .expect("終わったスレッドの最後の CPU")
            .total_100ns();

        if (i + 1) % 16 == 0 {
            let total = sum_total_100ns(&read_cpu(), &family);
            assert!(
                total >= previous_total,
                "畳んだ合計は減らないはず: {total} < {previous_total}"
            );
            previous_total = total;
        }
    }

    let reading = read_cpu();
    assert!(
        reading
            .live
            .iter()
            .all(|live| !live.role.starts_with(&prefix)),
        "全部 join した後は生きた項目が 0 のはず"
    );
    let folded = folded_with_family(&reading, &family);
    assert_eq!(folded.len(), 1, "畳んだ合計は系統に 1 つ: {folded:?}");
    assert_eq!(folded[0].exited, COUNT as u64);
    assert_eq!(
        folded[0].cpu.total_100ns(),
        last_sum,
        "畳んだ合計は join の後に読んだ最後の CPU の和と一致するはず"
    );
}

#[test]
fn inserting_many_exited_threads_never_grows_the_local_registry_past_one_live_and_one_total() {
    let mut registry = Registry::new();
    for i in 0..128 {
        let entry = std::thread::spawn(move || ThreadEntry {
            role: format!("actor:local-many-{i}"),
            name: None,
            tid: get_current_thread_id(),
            handle: Arc::new(
                crate::api::duplicate_current_thread_handle().expect("自分のハンドルの複製"),
            ),
        })
        .join()
        .expect("テスト用スレッドは正常終了するはず");

        registry.insert(entry, &Win32Probe);

        assert!(registry.live.len() <= 1, "{:?}", registry.live);
        assert!(registry.folded.len() <= 1, "{:?}", registry.folded);
    }
    assert_eq!(registry.folded["actor:local-many-*"].exited, 127);
}

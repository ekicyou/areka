//! `thread_registry` の決定論テスト。
//!
//! 名簿はプロセス共有で、テストは既定で並列に走る——**全体の件数は決して当てにしない**。
//! 各テストは自分の TID と用途札から一意な役割名を作り、その役割名の項目だけを検査する。
//! 実時間・CPU の閾値は 1 つも使わない（要件 6.5）。

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

#[test]
fn registration_from_another_thread_is_visible_from_this_thread() {
    let role = unique_role("spawned");
    let role_for_thread = role.clone();

    let joiner = std::thread::Builder::new()
        .name("wintf-registry-test-spawned".to_string())
        .spawn(move || {
            register_current_thread(role_for_thread.as_str()).expect("別スレッドの登録も成功する");
            get_current_thread_id()
        })
        .expect("テスト用スレッドの生成は成功するはず");
    let spawned_tid = joiner.join().expect("テスト用スレッドは正常終了するはず");

    let found = entries_with_role(&role);
    assert_eq!(
        found.len(),
        1,
        "別スレッドの登録が 1 件見えるはず: {found:?}"
    );
    assert_eq!(
        found[0].tid, spawned_tid,
        "生成側が登録した TID が見えるはず"
    );
    assert_ne!(
        found[0].tid,
        get_current_thread_id(),
        "登録したのは自分ではなく別スレッド（対照）"
    );
    assert_eq!(
        found[0].name.as_deref(),
        Some("wintf-registry-test-spawned"),
        "生成側の付けたスレッド名が読めるはず"
    );
    // 複製ハンドルは名簿が所有する——スレッドが終了した後でも CPU 時間は読めるはず。
    found[0]
        .cpu_times()
        .expect("終了済みスレッドでも複製ハンドルから CPU 時間は読める");
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

    // 語彙に無いもの（対照）。
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

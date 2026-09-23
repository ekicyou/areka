use super::*;
use crate::key::parse_dotted;
use crate::test_log_capture::{assert_logged, capture};
use tracing::Level;

fn all_families() -> Vec<PersistKey> {
    vec![
        PersistKey::WindowPos {
            scope: 0,
            axis: Axis::X,
        },
        PersistKey::WindowPos {
            scope: 2,
            axis: Axis::Y,
        },
        PersistKey::BalloonOffset {
            scope: 0,
            axis: Axis::X,
        },
        PersistKey::BalloonOffset {
            scope: 1,
            axis: Axis::Y,
        },
        PersistKey::BootCount,
        PersistKey::VanishCount,
    ]
}

// --- 正準 key 往復（Task 3.2 の申し送り: to_canonical_string と一致・読み口 1 本化の要石）---

#[test]
fn canonical_key_round_trips_with_to_canonical_string() {
    for key in all_families() {
        let s = key.to_canonical_key();
        let back = parse_dotted(&s).unwrap().to_canonical_string();
        assert_eq!(back, s, "canonical mismatch for {key:?}");
    }
}

#[test]
fn canonical_key_exact_strings() {
    assert_eq!(
        PersistKey::WindowPos {
            scope: 0,
            axis: Axis::X
        }
        .to_canonical_key(),
        "areka.window.scope(0).x"
    );
    assert_eq!(
        PersistKey::WindowPos {
            scope: 3,
            axis: Axis::Y
        }
        .to_canonical_key(),
        "areka.window.scope(3).y"
    );
    assert_eq!(
        PersistKey::BalloonOffset {
            scope: 0,
            axis: Axis::X
        }
        .to_canonical_key(),
        "areka.balloon.offset.scope(0).x"
    );
    assert_eq!(PersistKey::BootCount.to_canonical_key(), "areka.boot.count");
    assert_eq!(
        PersistKey::VanishCount.to_canonical_key(),
        "areka.vanish.count"
    );
}

// --- 4 key 族 put→load 往復（R6.6・完了条件）---

#[test]
fn four_family_put_load_value_round_trip() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/ghost")),
        ..ScopeRoots::default()
    };
    let entries = vec![
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "1024".to_string(),
        ),
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y,
            },
            "512".to_string(),
        ),
        (
            PersistKey::BalloonOffset {
                scope: 0,
                axis: Axis::X,
            },
            "30".to_string(),
        ),
        (
            PersistKey::BalloonOffset {
                scope: 0,
                axis: Axis::Y,
            },
            "-10".to_string(),
        ),
        (PersistKey::BootCount, "3".to_string()),
        (PersistKey::VanishCount, "0".to_string()),
    ];
    assert_eq!(
        save_scope(PersistScope::Ghost, &roots, &io, entries.clone()),
        PersistOutcome::Saved
    );
    let loaded = load_scope(PersistScope::Ghost, &roots, &io);
    for (k, v) in &entries {
        let found = loaded
            .iter()
            .find(|(lk, _)| lk == k)
            .map(|(_, lv)| lv.clone());
        assert_eq!(
            found.as_deref(),
            Some(v.as_str()),
            "family {k:?} did not round-trip"
        );
    }
}

#[test]
fn round_trip_preserves_negative_and_multi_scope() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    let entries = vec![
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "-5".to_string(),
        ),
        (
            PersistKey::WindowPos {
                scope: 1,
                axis: Axis::Y,
            },
            "200".to_string(),
        ),
    ];
    save_scope(PersistScope::Ghost, &roots, &io, entries.clone());
    let loaded = load_scope(PersistScope::Ghost, &roots, &io);
    for (k, v) in &entries {
        assert!(loaded.contains(&(*k, v.clone())), "missing {k:?}={v}");
    }
}

// --- TOML 写像の正しさ（保存 doc の配置）---

#[test]
fn toml_mapping_places_families_under_expected_tables() {
    let io = FakePersistIo::new();
    let path = PathBuf::from("/g/sylphya.toml");
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X,
                },
                "1024".into(),
            ),
            (
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::Y,
                },
                "-10".into(),
            ),
            (PersistKey::BootCount, "3".into()),
            (PersistKey::VanishCount, "0".into()),
        ],
    );
    let serialized = io.read(&path).unwrap().unwrap();
    // toml は数字のみの key を bare key として出力する（"0" → window.0）。
    assert!(
        serialized.contains("[window.0]"),
        "serialized=\n{serialized}"
    );
    assert!(
        serialized.contains("[balloon-offset.0]"),
        "serialized=\n{serialized}"
    );
    assert!(serialized.contains("[boot]"), "serialized=\n{serialized}");
    assert!(serialized.contains("[vanish]"), "serialized=\n{serialized}");
    assert!(
        serialized.contains("format-version = 1"),
        "serialized=\n{serialized}"
    );
    // 値の配置も確認（x は window 表下・count は boot/vanish 表下）。
    assert!(
        serialized.contains("x = \"1024\""),
        "serialized=\n{serialized}"
    );
}

// --- マージが無関係 key を温存（read-modify-write・clobber なし）---

#[test]
fn merge_preserves_unrelated_keys() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "7".into(),
        )],
    );
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "2".into())],
    );
    let loaded = load_scope(PersistScope::Ghost, &roots, &io);
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "7".into()
        )),
        "window.* が boot.count 保存で消えた: {loaded:?}"
    );
    assert!(
        loaded.contains(&(PersistKey::BootCount, "2".into())),
        "boot.count 未反映: {loaded:?}"
    );
}

#[test]
fn overwrite_same_key_updates_value() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "1".into())],
    );
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "5".into())],
    );
    let loaded = load_scope(PersistScope::Ghost, &roots, &io);
    assert!(loaded.contains(&(PersistKey::BootCount, "5".into())));
    assert_eq!(
        loaded
            .iter()
            .filter(|(k, _)| *k == PersistKey::BootCount)
            .count(),
        1
    );
}

// --- スコープ分離（R6.5・別 root は非混同）---

#[test]
fn scope_isolation_distinct_ghost_roots_do_not_cross() {
    let io = FakePersistIo::new();
    let roots_a = ScopeRoots {
        ghost: Some(PathBuf::from("/a")),
        ..ScopeRoots::default()
    };
    let roots_b = ScopeRoots {
        ghost: Some(PathBuf::from("/b")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots_a,
        &io,
        vec![(PersistKey::BootCount, "9".into())],
    );
    assert!(
        load_scope(PersistScope::Ghost, &roots_b, &io).is_empty(),
        "root B が root A の値を見た（混同）"
    );
    assert!(
        load_scope(PersistScope::Ghost, &roots_a, &io)
            .contains(&(PersistKey::BootCount, "9".into())),
        "root A 自身は自値を見られる"
    );
}

#[test]
fn scope_isolation_different_scopes_use_different_files() {
    // App と Ghost が別ルート → 別ファイル → 非混同。
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        app: Some(PathBuf::from("/app")),
        ghost: Some(PathBuf::from("/ghost")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::App,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "11".into())],
    );
    assert!(load_scope(PersistScope::Ghost, &roots, &io).is_empty());
    assert!(
        load_scope(PersistScope::App, &roots, &io).contains(&(PersistKey::BootCount, "11".into()))
    );
}

// --- root None → 寛容（R6.7・panic なし）---

#[test]
fn none_root_load_is_empty() {
    let io = FakePersistIo::new();
    assert!(load_scope(PersistScope::Ghost, &ScopeRoots::default(), &io).is_empty());
}

#[test]
fn none_root_save_is_degraded_no_panic() {
    let io = FakePersistIo::new();
    assert_eq!(
        save_scope(
            PersistScope::Ghost,
            &ScopeRoots::default(),
            &io,
            vec![(PersistKey::BootCount, "1".into())]
        ),
        PersistOutcome::Degraded
    );
}

// --- IO 障害の寛容縮退（read/commit 故障注入）---

#[test]
fn load_tolerates_read_failure() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "1".into())],
    );
    io.fail_next_read();
    assert!(
        load_scope(PersistScope::Ghost, &roots, &io).is_empty(),
        "read 障害は空縮退（panic なし）"
    );
}

#[test]
fn save_commit_failure_is_degraded_and_leaves_prior_intact() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "1".into())],
    );
    io.fail_next_commit();
    assert_eq!(
        save_scope(
            PersistScope::Ghost,
            &roots,
            &io,
            vec![(PersistKey::BootCount, "2".into())]
        ),
        PersistOutcome::Degraded
    );
    // 既存内容は無傷（原子的確定・R6.2）——1 のまま。
    assert!(
        load_scope(PersistScope::Ghost, &roots, &io).contains(&(PersistKey::BootCount, "1".into()))
    );
}

// === Task 4.4 ログ表明檻（criteria 明示要求の「警告」「error ログ」を capture 経由で檻へ）===
//
// criteria は寛容読取（Criterion 2）で **警告発火** を、commit 中断（Criterion 3）で **error
// ログ** を明示的に要求する。format 層の縮退 warn は `format.rs` 側で檻済みだが、load/save
// orchestration（本モジュール）の縮退アーム——load の read 障害段（R6.3「読み取れない」）と
// save の commit 失敗段（R6.2/R8.1）——は outcome のみ檻済みでログ未表明だった。ここを共有
// ヘルパ [`crate::test_log_capture`]（共有 crate `log-capture-kit` へ委譲して並列決定論化）
// 経由で檻へ入れる（bare `with_default` は使わない）。

// --- Criterion 2 (R6.3/R6.7): load_scope の read 障害段——warn ＋ 空縮退 ＋ 起動継続 ---
#[test]
fn load_read_failure_emits_warn_and_degrades() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "1".into())],
    );
    io.fail_next_read();
    let events = capture(|| {
        // 読み取れない → 空縮退（panic なし＝起動継続）。
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        assert!(loaded.is_empty(), "read 障害は空縮退（起動継続）");
    });
    // 縮退アームが LOG_TARGET・WARN で「persist read failed」を発火（無音失敗なし・R6.7/R8.1）。
    // ログ削除・語彙変更・レベル降格で赤になる。
    assert_logged(&events, Level::WARN, LOG_TARGET, "persist read failed");
}

// --- Criterion 3 (R6.2/R8.1): save_scope の commit 失敗段——既存無傷 ＋ error ログ ---
#[test]
fn save_commit_failure_emits_error_log() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![(PersistKey::BootCount, "1".into())],
    );
    io.fail_next_commit();
    let events = capture(|| {
        let outcome = save_scope(
            PersistScope::Ghost,
            &roots,
            &io,
            vec![(PersistKey::BootCount, "2".into())],
        );
        assert_eq!(
            outcome,
            PersistOutcome::Degraded,
            "commit 失敗は Degraded 縮退（panic なし）"
        );
    });
    // commit 失敗アームが LOG_TARGET・ERROR で「persist commit failed」を発火（R8.1 無音失敗禁止）。
    assert_logged(&events, Level::ERROR, LOG_TARGET, "persist commit failed");
    // 既存内容は無傷（temp→rename の原子的確定・R6.2）——1 のまま。
    assert!(
        load_scope(PersistScope::Ghost, &roots, &io).contains(&(PersistKey::BootCount, "1".into())),
        "commit 中断後も既存内容が無傷であること"
    );
}

// --- 非数値スコープ ID の寛容 skip（format 層は不透明文字列を許す）---

#[test]
fn non_numeric_scope_id_in_file_is_skipped() {
    let io = FakePersistIo::new();
    let path = PathBuf::from("/g/sylphya.toml");
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    // 手書き TOML: window の ID が非数値 "main" ＋ 正常な boot。
    io.commit(
        &path,
        "format-version = 1\n[window.\"main\"]\nx = \"5\"\n[boot]\ncount = \"1\"\n",
    )
    .unwrap();
    let loaded = load_scope(PersistScope::Ghost, &roots, &io);
    // 非数値 window は skip、boot は載る（panic なし）。
    assert!(loaded.contains(&(PersistKey::BootCount, "1".into())));
    assert!(
        loaded
            .iter()
            .all(|(k, _)| !matches!(k, PersistKey::WindowPos { .. }))
    );
}

// --- 決定論（同一入力→同一結果）---

#[test]
fn load_is_deterministic() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &io,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 2,
                    axis: Axis::X,
                },
                "9".into(),
            ),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X,
                },
                "1".into(),
            ),
            (PersistKey::VanishCount, "4".into()),
        ],
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &io),
        load_scope(PersistScope::Ghost, &roots, &io)
    );
}

#[test]
fn absent_file_loads_empty() {
    let io = FakePersistIo::new();
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/never-written")),
        ..ScopeRoots::default()
    };
    assert!(load_scope(PersistScope::Ghost, &roots, &io).is_empty());
}

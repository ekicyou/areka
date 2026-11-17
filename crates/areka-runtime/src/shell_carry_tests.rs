//! ghost の読み直しでシェルの面と着せ替えを持ち越す控えの組み方・面の決め方・渡し方の決定論テスト
//! （areka-P0-mcp-reload 要件 3.8・12.1）。
//!
//! ⑴ 面の決め方: 控えた面が在れば出す／無ければ既定の面へ戻す／どちらも無ければ出さない
//! ⑵ 控えの組み方: 表示中のスコープだけ面を載せ、表示したことのあるスコープは着せ替えも載せ、
//!    読み直しの前の既定も控える
//! ⑶ 結線が無い・ゴーストが無いときの渡しは警告して控えを捨てる
//!
//! 着せ替えの決め方と seriko への送り方は、`areka` の `runtime_tests/` の同じ名前のファイルにある。

use super::test_support::set;
use super::*;

/// 在る面の集合（読み直したシェルの代わり）。
fn shell_with(ids: &'static [u32]) -> impl Fn(u32) -> bool {
    move |id| ids.contains(&id)
}

/// ⑴ 控えた面が読み直したシェルに在ればその面、無ければ候補の並び（descript の既定の面 → 組み込みの
/// 既定の面）の最初に在る面、どれも無ければ出さない（descript-window-keys 要件 3.1・3.3・3.4・7.2）。
#[test]
fn decide_keeps_the_carried_surface_or_falls_back_to_the_default() {
    // 持ち越し: 控えた面が在れば、descript の既定の面が在っても替えない（要件 3.4）。
    assert_eq!(
        decide(1000, &[5, 0], shell_with(&[0, 5, 1000])),
        CarriedShow::Carried(1000)
    );
    // descript の面へ戻る（要件 3.1）。
    assert_eq!(
        decide(1000, &[5, 0], shell_with(&[0, 5])),
        CarriedShow::FellBack {
            missing: 1000,
            to: 5,
            passed_over: None,
        }
    );
    // descript の既定の面が組み込みと同じなら候補は 1 つ（飛ばした面は無い）。
    assert_eq!(
        decide(1000, &[0], shell_with(&[0])),
        CarriedShow::FellBack {
            missing: 1000,
            to: 0,
            passed_over: None,
        }
    );
    // 組み込みの面へ落ちる: 飛ばした descript の面が載る（要件 3.3）。
    assert_eq!(
        decide(2011, &[11, 10], shell_with(&[0, 10])),
        CarriedShow::FellBack {
            missing: 2011,
            to: 10,
            passed_over: Some(11),
        }
    );
    // どれも無い: 出さず、候補の先頭（descript の既定の面）を載せる（要件 3.3）。
    assert_eq!(
        decide(1000, &[11, 10], shell_with(&[0])),
        CarriedShow::Skipped {
            missing: 1000,
            descript: 11,
        }
    );
    assert_eq!(CarriedShow::Carried(7).surface(), Some(7));
    assert_eq!(
        CarriedShow::FellBack {
            missing: 7,
            to: 10,
            passed_over: Some(11),
        }
        .surface(),
        Some(10)
    );
    assert_eq!(
        CarriedShow::Skipped {
            missing: 7,
            descript: 10,
        }
        .surface(),
        None
    );
}

/// ⑵ 面は表示中のスコープだけ（scope 1 は非表示＝`None`）、着せ替えは表示したことのあるスコープだけ載る。
#[test]
fn snapshot_keeps_only_the_shown_scopes() {
    let carry = snapshot(
        [0, 1, 2],
        |scope| match scope {
            0 => Some(1000),
            2 => Some(2010),
            _ => None,
        },
        |scope| (scope != 2).then(|| set(&[scope + 100])),
        areka_seriko::DefaultBinds::uniform(set(&[100])),
    );
    assert_eq!(carry.surface(0), Some(1000));
    assert_eq!(carry.surface(1), None);
    assert_eq!(carry.surface(2), Some(2010));
    assert_eq!(carry.surface(3), None);
    assert_eq!(carry.binds(0), Some(&set(&[100])));
    assert_eq!(carry.binds(1), Some(&set(&[101])));
    assert_eq!(carry.binds(2), None);
    assert_eq!(
        carry.old_static(),
        &areka_seriko::DefaultBinds::uniform(set(&[100]))
    );
}

/// 控えを渡す先の結線が無ければ `warn!` を 1 件残して捨てる（panic しない）。
#[test]
fn hand_over_without_wiring_warns_and_drops_the_carry() {
    let mut world = World::new();
    let ((), events) = log_capture_kit::capture(|| {
        hand_over(&mut world, Some(ShellCarry::default()));
    });
    let warned = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_shell_carry_unavailable"))
        .count();
    assert_eq!(warned, 1, "{events:?}");
    assert!(!world.contains_non_send::<RuntimeWiring>());
}

/// 置き場にゴーストが居なければ「出し直す」の記録を出さず、`reload_shell_replay_unavailable` の
/// `warn!` を 1 件だけ残して何も送らない（62 番）。出すものが無ければ記録も 0 件。
#[test]
fn replay_without_a_ghost_warns_once_and_restores_nothing() {
    let world = World::new();
    let count = |events: &[log_capture_kit::CapturedEvent], level, name: &str| {
        events
            .iter()
            .filter(|e| e.level == level && e.field_str("event") == Some(name))
            .count()
    };
    let ((), events) = log_capture_kit::capture(|| {
        replay(&world, &[(0, CarriedShow::Carried(0))], &[]);
    });
    assert_eq!(
        count(
            &events,
            tracing::Level::WARN,
            "reload_shell_replay_unavailable"
        ),
        1,
        "{events:?}"
    );
    assert_eq!(
        count(&events, tracing::Level::INFO, "reload_shell_restored"),
        0,
        "{events:?}"
    );
    let ((), events) = log_capture_kit::capture(|| replay(&world, &[], &[]));
    assert!(events.is_empty(), "{events:?}");
}

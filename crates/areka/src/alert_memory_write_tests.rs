//! 記憶を書けないことの告知の文面の決定論テスト（areka-P0-write-failure-notice 要件 2.4・2.7・6.2）。
//!
//! 確かめること: 2 つの時点の題名と本文 5 行・本文に環境変数の名前と既定の置き場の場所が
//! 無いこと・説明書の「■ 記憶の置き場」への案内・覚えられなかったものの語が全 key と
//! 空の 4 スコープに返ること。今の 5 場面は `alert_tests.rs` がそのまま固定する。

use std::path::PathBuf;

use areka_sylphya::{Axis, PersistFailure, PersistFailureCause, PersistKey, PersistScope};

use super::*;

const FIX_LINE: &str = "直し方: この置き場を書ける状態にしてください。置き場の変え方は、説明書（README.txt）の「■ 記憶の置き場」にあります。";

fn dir() -> PathBuf {
    PathBuf::from(r"C:\areka-alert-tests\profile\areka")
}

fn failure(keys: Vec<PersistKey>, cause: PersistFailureCause) -> PersistFailure {
    PersistFailure {
        scope: PersistScope::App,
        dir: Some(dir()),
        keys,
        cause,
    }
}

fn scene(failure: PersistFailure, moment: WriteMoment) -> AlertScene {
    AlertScene::MemoryWrite { failure, moment }
}

#[test]
fn boot_moment_title_and_five_lines() {
    let f = failure(
        vec![PersistKey::LastRunning],
        PersistFailureCause::Io("アクセスが拒否されました。 (os error 5)".to_owned()),
    );
    let (title, body) = alert_text(&scene(f, WriteMoment::Boot));
    assert_eq!(title, "areka を起動できません");
    let expected = [
        "記憶を書けないので、ゴーストを起こさずに起動を止めました。".to_owned(),
        "覚えられなかったもの: 起動中の印".to_owned(),
        format!("置き場: {}", dir().display()),
        "理由: アクセスが拒否されました。 (os error 5)".to_owned(),
        FIX_LINE.to_owned(),
    ]
    .join("\n");
    assert_eq!(body, expected);
}

#[test]
fn running_moment_title_and_five_lines() {
    let f = PersistFailure {
        scope: PersistScope::Ghost,
        dir: None,
        keys: vec![],
        cause: PersistFailureCause::NoRoot,
    };
    let (title, body) = alert_text(&scene(f, WriteMoment::Running));
    assert_eq!(title, "areka の記憶を書けません");
    let expected = [
        "記憶を書けなくなったので、areka を終了しました。",
        "覚えられなかったもの: ゴーストの記憶",
        "置き場: 決まっていません",
        "理由: 記憶の置き場が決まっていません",
        FIX_LINE,
    ]
    .join("\n");
    assert_eq!(body, expected);
}

#[test]
fn body_has_no_env_names_nor_default_location_and_points_to_manual() {
    for moment in [WriteMoment::Boot, WriteMoment::Running] {
        for cause in [
            PersistFailureCause::NoRoot,
            PersistFailureCause::Io("x".to_owned()),
        ] {
            let (_, body) = alert_text(&scene(failure(vec![], cause), moment));
            assert!(!body.contains("AREKA_"), "{body}");
            assert!(!body.contains("LOCALAPPDATA"), "{body}");
            assert!(body.contains("「■ 記憶の置き場」"), "{body}");
            assert_eq!(body.lines().count(), 5, "{body}");
        }
    }
}

#[test]
fn remembered_text_covers_every_key() {
    let cases = [
        (PersistKey::LastRunning, "起動中の印"),
        (PersistKey::LastGhost, "最後に使ったゴースト"),
        (PersistKey::LastBalloon, "最後に使ったバルーン"),
        (PersistKey::LastShell, "最後に使ったシェル"),
        (PersistKey::BasewareRoots, "置き場の並び"),
        (
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            "窓の位置",
        ),
        (
            PersistKey::BalloonOffset {
                scope: 1,
                axis: Axis::Y,
            },
            "窓の位置",
        ),
        (PersistKey::BootCount, "ゴーストのプロパティ"),
        (PersistKey::VanishCount, "ゴーストのプロパティ"),
    ];
    for (key, word) in cases {
        let f = failure(vec![key], PersistFailureCause::NoRoot);
        assert_eq!(remembered_text(&f), word, "{key:?}");
    }
}

#[test]
fn remembered_text_joins_distinct_words_once() {
    let f = failure(
        vec![
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X,
            },
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y,
            },
            PersistKey::BalloonOffset {
                scope: 0,
                axis: Axis::X,
            },
            PersistKey::BootCount,
            PersistKey::LastGhost,
            PersistKey::VanishCount,
        ],
        PersistFailureCause::NoRoot,
    );
    assert_eq!(
        remembered_text(&f),
        "窓の位置・ゴーストのプロパティ・最後に使ったゴースト"
    );
}

#[test]
fn remembered_text_uses_scope_word_when_keys_are_empty() {
    let cases = [
        (PersistScope::App, "areka の記憶"),
        (PersistScope::Ghost, "ゴーストの記憶"),
        (PersistScope::Shell, "シェルの記憶"),
        (PersistScope::Balloon, "バルーンの記憶"),
    ];
    for (scope, word) in cases {
        let f = PersistFailure {
            scope,
            dir: None,
            keys: vec![],
            cause: PersistFailureCause::NoRoot,
        };
        assert_eq!(remembered_text(&f), word, "{scope:?}");
    }
}

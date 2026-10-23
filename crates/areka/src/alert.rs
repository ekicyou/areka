//! 無いときの告知（areka-P0-baseware-root-layout 要件 6）。
//!
//! 起動を止める 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）と
//! SHIORI が動かなくなった場面（areka-P0-shiori-fault-notice 要件 1・5）と
//! 記憶を書けない場面（areka-P0-write-failure-notice 要件 2.4・2.7・6.2）を
//! [`AlertScene`] 1 つの型で数え、利用者向けのメッセージボックス（`MessageBoxW`）と
//! 同じ内容の `error!` を出す。文面は [`alert_text`] が純粋に組む（テストで固定する）。
//! 告知は環境変数 `AREKA_NO_ALERT` で抑えられる（自動テストでモーダルが止まらない）。
//! 終了コードは呼び手（`main`）の責務で、ここは告げるだけ。
//!
//! はい／いいえを尋ねる口 [`ask_yes_no`]（areka-P0-ghost-install 要件 4.4）と、
//! OK だけの警告の口 [`warn_ok`]（areka-P0-emily-ghost-verification 要件 1.12）も置く。
//!
//! この module の `unsafe` は [`raise`]・[`ask_yes_no`]・[`warn_ok`] の `MessageBoxW` 呼び出しに閉じる。

use std::path::PathBuf;

use areka_kanade::{ShioriFault, ShioriFaultKind};
use areka_sylphya::{PersistFailure, PersistFailureCause, PersistKey, PersistScope};
use windows::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_ICONERROR, MB_ICONQUESTION, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND,
    MB_TOPMOST, MB_YESNO, MessageBoxW,
};
use windows::core::HSTRING;

use crate::roots::{RootError, RootsOrigin};

/// 告知を抑える環境変数（要件 6.5・裁定 2）。設定されていれば抑える。
/// ただし未設定・空・空白だけ・"0" は「設定されていない」と読む（`AREKA_TICK_GATE=0` と同じく 0 で切れる）。
pub(crate) const NO_ALERT_ENV: &str = "AREKA_NO_ALERT";

/// 告知の場面（要件 6.1・6.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AlertScene {
    /// 根が決まらない（要件 1.4）。
    RootMissing(RootError),
    /// ゴーストが無い（要件 4.7）／argv のフォルダがゴーストでない（要件 4.8・argv は Some）。
    GhostMissing {
        ghost_stores: Vec<PathBuf>,
        argv: Option<PathBuf>,
    },
    /// バルーンが無い（要件 5.8）。
    BalloonMissing { balloon_stores: Vec<PathBuf> },
    /// 起動窓を開けない（要件 6.4）。`reason` は失敗の内容（`PlacementError` の表示）。
    StartupWindow { reason: String },
    /// SHIORI が動かなくなった（起動時か会話中かは載せない）。
    ShioriFault {
        ghost_name: Option<String>,
        ghost_root: PathBuf,
        fault: ShioriFault,
    },
    /// 記憶を書けない（write-failure-notice 要件 2.1〜2.4）。起動の時点か途中かを載せる。
    MemoryWrite {
        failure: PersistFailure,
        moment: WriteMoment,
    },
}

/// 記憶を書けなかった時点（write-failure-notice 要件 6.2・足す告知は 2 つの時点だけ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteMoment {
    /// 起動の時点（ゴーストを起こす前）。
    Boot,
    /// 動いている途中。
    Running,
}

/// 題名（起動を止める 4 場面と、起動の時点で記憶を書けない場面で共通）。
const TITLE: &str = "areka を起動できません";
/// SHIORI が動かなくなった場面の題名（smoke の目印でもある）。
const SHIORI_FAULT_TITLE: &str = "SHIORI が動かなくなりました";
/// 動いている途中で記憶を書けなくなった場面の題名（起動の時点は [`TITLE`]）。
const MEMORY_WRITE_TITLE: &str = "areka の記憶を書けません";
/// 記憶を書けない場面の直し方（環境変数の名前と既定の置き場の場所は書かない・要件 2.4）。
/// 説明書の節の名前は user-data-root の `dist/README.txt` に合わせる。
const MEMORY_WRITE_FIX: &str = "直し方: この置き場を書ける状態にしてください。置き場の変え方は、説明書（README.txt）の「■ 記憶の置き場」にあります。";

/// 置き場の場面の文面（利用者向けの語は「置き場」・要件 9.5）。
const ROOT_SHAPE: &str = "置くもの: ghost フォルダと balloon フォルダを持つフォルダ";
const ROOT_BY_ENV: &str =
    "環境変数 AREKA_ROOT に、置き場のフォルダの絶対パスを ; で区切って複数渡せます。";
const GHOST_SHAPE: &str = r"置くもの: ghost\master\descript.txt を持つゴーストのフォルダ";
const BALLOON_SHAPE: &str = "置くもの: descript.txt を持つバルーンのフォルダ";

/// 失敗の種類を利用者向けの平易な語へ写す（純粋）。
pub(crate) fn fault_kind_text(kind: ShioriFaultKind) -> &'static str {
    match kind {
        ShioriFaultKind::ConnectFailed => "SHIORI に接続できなかった",
        ShioriFaultKind::Timeout => "SHIORI の応答が期限内に返らなかった",
        ShioriFaultKind::Disconnected => "SHIORI との通信が切れた",
        ShioriFaultKind::Internal => "areka 側の内部の失敗",
        ShioriFaultKind::Unknown => "原因不明",
    }
}

/// 覚えられなかったものの利用者向けの語（純粋・`memory_write_failed` の欄 `what` も同じ物）。
/// key ごとの語を出た順に同じ語は 1 回だけ「・」で繋ぐ。key が空（起動の前の確かめ）ならスコープの語。
pub(crate) fn remembered_text(failure: &PersistFailure) -> String {
    if failure.keys.is_empty() {
        return match failure.scope {
            PersistScope::App => "areka の記憶",
            PersistScope::Ghost => "ゴーストの記憶",
            PersistScope::Shell => "シェルの記憶",
            PersistScope::Balloon => "バルーンの記憶",
        }
        .to_owned();
    }
    let mut words: Vec<&str> = Vec::new();
    for key in &failure.keys {
        let word = match key {
            PersistKey::LastRunning => "起動中の印",
            PersistKey::LastGhost => "最後に使ったゴースト",
            PersistKey::LastBalloon => "最後に使ったバルーン",
            PersistKey::LastShell => "最後に使ったシェル",
            PersistKey::BasewareRoots => "置き場の並び",
            PersistKey::WindowPos { .. }
            | PersistKey::BalloonOffset { .. }
            | PersistKey::WindowAdjacent { .. } => "窓の位置",
            PersistKey::BootCount | PersistKey::VanishCount => "ゴーストのプロパティ",
        };
        if !words.contains(&word) {
            words.push(word);
        }
    }
    words.join("・")
}

/// 題名と本文（純粋・テストで文面を固定する）。題名は場面ごと。本文は行数可変（要件 9.5〜9.7）:
/// 置き場の場面は「理由」「試した場所（1 行ずつ・出どころつき）」「置くもの」「AREKA_ROOT の案内」、
/// ゴースト・バルーンの場面は「何が無いか」「探した場所（各置き場の `ghost\`／`balloon\` を 1 行ずつ）」
/// 「置くもの」、起動窓の場面は「何が起きたか」「失敗の内容」「確かめること」、
/// SHIORI の場面は「どのゴーストか」「失敗の種類」「理由」、
/// 記憶の場面は「何が起きたか」「覚えられなかったもの」「置き場」「理由」「直し方」の 5 行。
pub(crate) fn alert_text(scene: &AlertScene) -> (String, String) {
    let lines: Vec<String> = match scene {
        AlertScene::RootMissing(error) => {
            let (what, tried, origin) = match error {
                RootError::ExeLocationUnavailable { tried, origin, .. } => (
                    "置き場が決まりません: 実行ファイルの場所が取れません。",
                    tried.as_slice(),
                    *origin,
                ),
                RootError::EnvAllEmpty { .. } => (
                    "置き場が決まりません: 環境変数 AREKA_ROOT が空です。",
                    &[][..],
                    RootsOrigin::EnvVar,
                ),
                RootError::NoneFound { tried, origin, .. } => (
                    "置き場（ゴーストとバルーンを置くフォルダ）が見つかりません。",
                    tried.as_slice(),
                    *origin,
                ),
            };
            let from = origin_text(origin);
            std::iter::once(what.to_owned())
                .chain(
                    tried
                        .iter()
                        .map(|dir| format!("試した場所: {}（{from}）", dir.display())),
                )
                .chain([ROOT_SHAPE.to_owned(), ROOT_BY_ENV.to_owned()])
                .collect()
        }
        AlertScene::GhostMissing { ghost_stores, argv } => store_lines(
            match argv {
                None => "ゴーストが見つかりません。".to_owned(),
                Some(given) => format!(
                    "ゴーストが見つかりません: 渡されたフォルダ {} はゴーストではありません。",
                    given.display()
                ),
            },
            ghost_stores,
            GHOST_SHAPE,
        ),
        AlertScene::BalloonMissing { balloon_stores } => store_lines(
            "バルーンが見つかりません。".to_owned(),
            balloon_stores,
            BALLOON_SHAPE,
        ),
        AlertScene::StartupWindow { reason } => vec![
            "起動窓を開けません。".to_owned(),
            format!("失敗の内容: {reason}"),
            "モニタの接続とゴーストのファイルを確かめてください。".to_owned(),
        ],
        AlertScene::ShioriFault {
            ghost_name,
            ghost_root,
            fault,
        } => vec![
            match ghost_name {
                Some(name) => format!("ゴースト: {name}（{}）", ghost_root.display()),
                None => format!("ゴースト: {}", ghost_root.display()),
            },
            format!("失敗の種類: {}", fault_kind_text(fault.kind)),
            format!("理由: {}", fault.reason),
        ],
        AlertScene::MemoryWrite { failure, moment } => vec![
            match moment {
                WriteMoment::Boot => "記憶を書けないので、ゴーストを起こさずに起動を止めました。",
                WriteMoment::Running => "記憶を書けなくなったので、areka を終了しました。",
            }
            .to_owned(),
            format!("覚えられなかったもの: {}", remembered_text(failure)),
            match &failure.dir {
                Some(dir) => format!("置き場: {}", dir.display()),
                None => "置き場: 決まっていません".to_owned(),
            },
            match &failure.cause {
                PersistFailureCause::Io(reason) => format!("理由: {reason}"),
                PersistFailureCause::NoRoot => "理由: 記憶の置き場が決まっていません".to_owned(),
            },
            MEMORY_WRITE_FIX.to_owned(),
        ],
    };
    let title = match scene {
        AlertScene::ShioriFault { .. } => SHIORI_FAULT_TITLE,
        AlertScene::MemoryWrite {
            moment: WriteMoment::Running,
            ..
        } => MEMORY_WRITE_TITLE,
        AlertScene::MemoryWrite {
            moment: WriteMoment::Boot,
            ..
        } => TITLE,
        AlertScene::RootMissing(_)
        | AlertScene::GhostMissing { .. }
        | AlertScene::BalloonMissing { .. }
        | AlertScene::StartupWindow { .. } => TITLE,
    };
    (title.to_owned(), lines.join("\n"))
}

/// 並びの出どころを利用者向けの語へ（要件 9.5）。
fn origin_text(origin: RootsOrigin) -> &'static str {
    match origin {
        RootsOrigin::EnvVar => "環境変数 AREKA_ROOT",
        RootsOrigin::Memory => "アプリの記憶",
        RootsOrigin::Default => "既定（実行ファイルの隣）",
    }
}

/// 格納フォルダの場面の行: 「何が無いか」、探した場所を 1 行ずつ、「置くもの」（要件 9.6・9.7）。
fn store_lines(what: String, stores: &[PathBuf], shape: &str) -> Vec<String> {
    std::iter::once(what)
        .chain(
            stores
                .iter()
                .map(|dir| format!("探した場所: {}", dir.display())),
        )
        .chain([shape.to_owned()])
        .collect()
}

/// 値から抑止を判断する純粋な口（None／""／空白のみ／"0"（trim 後）→ false・それ以外 → true）。
pub(crate) fn suppressed_from(value: Option<&str>) -> bool {
    value.is_some_and(|v| !matches!(v.trim(), "" | "0"))
}

/// env `AREKA_NO_ALERT` を読んで [`suppressed_from`] へ渡す薄い口。
/// 非 UTF-8 の値も「設定されている」と読む（抑える）。
pub(crate) fn suppressed() -> bool {
    match std::env::var(NO_ALERT_ENV) {
        Ok(v) => suppressed_from(Some(&v)),
        Err(std::env::VarError::NotPresent) => false,
        Err(std::env::VarError::NotUnicode(_)) => true,
    }
}

/// `error!(event = "alert")` を必ず 1 件残し、`suppressed` が偽なら `MessageBoxW` を出す
/// （`hwnd` 無し＝`WinApp` の有無を問わない）。戻ったら呼び手が `Err` を返す。
pub(crate) fn raise(scene: &AlertScene, suppressed: bool) {
    let (title, body) = alert_text(scene);
    tracing::error!(
        event = "alert",
        scene = ?scene,
        title = title.as_str(),
        body = body.as_str(),
        suppressed,
        "[alert] 利用者へ告げます"
    );
    if suppressed {
        return;
    }
    let (title, body) = (HSTRING::from(title), HSTRING::from(body));
    // SAFETY: `title`／`body` は終端 0 付きで呼び出しの間ずっと生存し、他の引数は None と定数。
    let result = unsafe { MessageBoxW(None, &body, &title, MB_OK | MB_ICONERROR) };
    // 0 は出せなかった（対話デスクトップが無い・資源不足等）。押されたボタンは OK だけなので
    // 0 以外は見ない。出せなくても `alert` の記録は残っているので、原因を別の event で足す。
    if result.0 == 0 {
        // GetLastError を読む（windows-core 0.62 で from_win32 は from_thread に改名）。
        let e = windows::core::Error::from_thread();
        tracing::error!(
            event = "alert_box_failed",
            error = %e,
            "[alert] MessageBoxW failed: the alert was only logged"
        );
    }
}

/// はい／いいえの答え（ghost-install 要件 4.4・4.7・4.8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum YesNo {
    Yes,
    No,
    /// 抑止されていて出さなかった。
    Suppressed,
    /// 出せなかった。
    Unavailable,
}

/// はい／いいえの 2 択の画面を出して答えを返す（閉じるボタンは効かない）。
/// `info!(event = "ask")` を必ず 1 件残し、`suppressed` が真なら画面を出さない。持ち主の窓は
/// 渡さない。抑止と失敗を拒否へ倒すのは呼び手。
pub(crate) fn ask_yes_no(title: &str, body: &str, suppressed: bool) -> YesNo {
    tracing::info!(event = "ask", title, suppressed, "[alert] 利用者に尋ねます");
    if suppressed {
        return YesNo::Suppressed;
    }
    let (title, body) = (HSTRING::from(title), HSTRING::from(body));
    // SAFETY: `title`／`body` は終端 0 付きで呼び出しの間ずっと生存し、他の引数は None と定数。
    let result = unsafe {
        MessageBoxW(
            None,
            &body,
            &title,
            MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };
    match result {
        IDYES => YesNo::Yes,
        IDNO => YesNo::No,
        // 0 は出せなかった。`MB_YESNO` は閉じるボタンが効かないので、他の値は来ない。
        _ => {
            let e = windows::core::Error::from_thread();
            tracing::error!(
                event = "ask_box_failed",
                error = %e,
                "[alert] MessageBoxW failed: the question was not shown"
            );
            YesNo::Unavailable
        }
    }
}

/// OK だけの警告の画面を出す（インストールの警告・2026-10-17 開発者の例外）。
/// `warn!(event = "install_warning_dialog")` を必ず 1 件残し、`suppressed` が真なら画面を出さない。
/// 出せなかったら `error!(event = "warning_box_failed")` を 1 件足す。持ち主の窓は渡さない。
pub(crate) fn warn_ok(title: &str, body: &str, suppressed: bool) {
    tracing::warn!(
        event = "install_warning_dialog",
        title,
        body,
        suppressed,
        "[alert] 利用者へ警告します"
    );
    if suppressed {
        return;
    }
    let (title, body) = (HSTRING::from(title), HSTRING::from(body));
    // SAFETY: `title`／`body` は終端 0 付きで呼び出しの間ずっと生存し、他の引数は None と定数。
    let result = unsafe {
        MessageBoxW(
            None,
            &body,
            &title,
            MB_OK | MB_ICONWARNING | MB_SETFOREGROUND | MB_TOPMOST,
        )
    };
    // 0 は出せなかった。押されたボタンは OK だけなので 0 以外は見ない。
    if result.0 == 0 {
        let e = windows::core::Error::from_thread();
        tracing::error!(
            event = "warning_box_failed",
            error = %e,
            "[alert] MessageBoxW failed: the warning was only logged"
        );
    }
}

#[cfg(test)]
#[path = "alert_tests.rs"]
mod alert_tests;

#[cfg(test)]
#[path = "alert_memory_write_tests.rs"]
mod alert_memory_write_tests;

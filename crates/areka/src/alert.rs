//! 無いときの告知（areka-P0-baseware-root-layout 要件 6）。
//!
//! 起動を止める 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）と
//! SHIORI が動かなくなった場面（areka-P0-shiori-fault-notice 要件 1・5）を
//! [`AlertScene`] 1 つの型で数え、利用者向けのメッセージボックス（`MessageBoxW`）と
//! 同じ内容の `error!` を出す。文面は [`alert_text`] が純粋に組む（テストで固定する）。
//! 告知は環境変数 `AREKA_NO_ALERT` で抑えられる（自動テストでモーダルが止まらない）。
//! 終了コードは呼び手（`main`）の責務で、ここは告げるだけ。
//!
//! この module の `unsafe` は [`raise`] の `MessageBoxW` 呼び出し 1 か所に閉じる。

use std::path::PathBuf;

use areka_kanade::{ShioriFault, ShioriFaultKind};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::HSTRING;

use crate::boot_config::{RootError, RootSource};

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
        ghost_store: PathBuf,
        argv: Option<PathBuf>,
    },
    /// バルーンが無い（要件 5.8）。
    BalloonMissing { balloon_store: PathBuf },
    /// 起動窓を開けない（要件 6.4）。`reason` は失敗の内容（`PlacementError` の表示）。
    StartupWindow { reason: String },
    /// SHIORI が動かなくなった（起動時か会話中かは載せない）。
    ShioriFault {
        ghost_name: Option<String>,
        ghost_root: PathBuf,
        fault: ShioriFault,
    },
}

/// 題名（起動を止める 4 場面で共通）。
const TITLE: &str = "areka を起動できません";
/// SHIORI が動かなくなった場面の題名（smoke の目印でもある）。
const SHIORI_FAULT_TITLE: &str = "SHIORI が動かなくなりました";

/// 根を渡す道（根の場所そのものが綴れないとき）。
const ROOT_BY_ENV: &str =
    "置く場所: 環境変数 AREKA_ROOT に根のフォルダの絶対パスを設定してください。";
const ROOT_SHAPE: &str = "置くもの: ghost フォルダと balloon フォルダを持つフォルダ";
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

/// 題名と本文（純粋・テストで文面を固定する）。題名は場面ごと。本文は「何が無いか」
/// 「置くべき場所の絶対パス」「置くものの形」の 3 行構成（起動窓の場面は「何が起きたか」
/// 「失敗の内容」「確かめること」、SHIORI の場面は「どのゴーストか」「失敗の種類」「理由」）。
pub(crate) fn alert_text(scene: &AlertScene) -> (String, String) {
    let lines: [String; 3] = match scene {
        AlertScene::RootMissing(RootError::ExeLocationUnavailable) => [
            "根が決まりません: 実行ファイルの場所が取れません。".to_owned(),
            ROOT_BY_ENV.to_owned(),
            ROOT_SHAPE.to_owned(),
        ],
        // 空の AREKA_ROOT は絶対化できない唯一の値。空のパスは綴らない。
        AlertScene::RootMissing(RootError::NotADirectory {
            dir,
            source: RootSource::EnvVar,
        }) if dir.as_os_str().is_empty() => [
            "根が決まりません: 環境変数 AREKA_ROOT が空です。".to_owned(),
            ROOT_BY_ENV.to_owned(),
            ROOT_SHAPE.to_owned(),
        ],
        AlertScene::RootMissing(RootError::NotADirectory { dir, source }) => {
            let what = match source {
                RootSource::EnvVar => "環境変数 AREKA_ROOT の指すフォルダがありません。",
                RootSource::ExeDir => "実行ファイルのあるフォルダが見つかりません。",
            };
            [
                format!("根が決まりません: {what}"),
                format!("置く場所: {}", dir.display()),
                ROOT_SHAPE.to_owned(),
            ]
        }
        AlertScene::GhostMissing { ghost_store, argv } => [
            match argv {
                None => "ゴーストが見つかりません。".to_owned(),
                Some(given) => format!(
                    "ゴーストが見つかりません: 渡されたフォルダ {} はゴーストではありません。",
                    given.display()
                ),
            },
            format!("置く場所: {}", ghost_store.display()),
            GHOST_SHAPE.to_owned(),
        ],
        AlertScene::BalloonMissing { balloon_store } => [
            "バルーンが見つかりません。".to_owned(),
            format!("置く場所: {}", balloon_store.display()),
            BALLOON_SHAPE.to_owned(),
        ],
        AlertScene::StartupWindow { reason } => [
            "起動窓を開けません。".to_owned(),
            format!("失敗の内容: {reason}"),
            "モニタの接続とゴーストのファイルを確かめてください。".to_owned(),
        ],
        AlertScene::ShioriFault {
            ghost_name,
            ghost_root,
            fault,
        } => [
            match ghost_name {
                Some(name) => format!("ゴースト: {name}（{}）", ghost_root.display()),
                None => format!("ゴースト: {}", ghost_root.display()),
            },
            format!("失敗の種類: {}", fault_kind_text(fault.kind)),
            format!("理由: {}", fault.reason),
        ],
    };
    let title = match scene {
        AlertScene::ShioriFault { .. } => SHIORI_FAULT_TITLE,
        AlertScene::RootMissing(_)
        | AlertScene::GhostMissing { .. }
        | AlertScene::BalloonMissing { .. }
        | AlertScene::StartupWindow { .. } => TITLE,
    };
    (title.to_owned(), lines.join("\n"))
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

#[cfg(test)]
#[path = "alert_tests.rs"]
mod alert_tests;

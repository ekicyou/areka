//! 無いときの告知（areka-P0-baseware-root-layout 要件 6）。
//!
//! 起動を止める 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）と
//! SHIORI が動かなくなった場面（areka-P0-shiori-fault-notice 要件 1・5）と
//! 記憶を書けない場面（areka-P0-write-failure-notice 要件 2.4・2.7・6.2）を
//! [`AlertScene`] 1 つの型で数え、利用者向けのタスクダイアログ（`TaskDialogIndirect`）と
//! 同じ内容の `error!` を出す。文面は [`alert_text`] が純粋に組む（テストで固定する）。
//! 告知は環境変数 `AREKA_NO_ALERT` で抑えられる（自動テストでモーダルが止まらない）。
//! 終わりの告知（[`raise`]）は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS` が立つ走行でも抑える。
//! 終了コードは呼び手（`main`）の責務で、ここは告げるだけ。
//!
//! はい／いいえを尋ねる口 [`ask_yes_no`]（areka-P0-ghost-install 要件 4.4）と、
//! OK だけの警告の口 [`warn_ok`]（areka-P0-emily-ghost-verification 要件 1.12）も置く。
//!
//! 3 つの口はどれも [`show_box`] 1 つの道で画面を出し、出す間だけスレッドの DPI の扱いを
//! モニタごとの v2 にする（alert-dialog-per-monitor-dpi）。
//!
//! この module の `unsafe` は [`set_thread_dpi_context`]・[`show_box`] の中の `TaskDialogIndirect`・
//! [`bring_to_front`] の 3 か所に閉じる。

use std::path::PathBuf;

use areka_kanade::{ShioriFault, ShioriFaultKind};
use areka_sylphya::{PersistFailure, PersistFailureCause, PersistKey, PersistScope};
use windows::Win32::Foundation::{HWND, LPARAM, S_OK, WPARAM};
use windows::Win32::UI::Controls::{
    TASKDIALOG_FLAGS, TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TD_ERROR_ICON,
    TD_INFORMATION_ICON, TD_WARNING_ICON, TDCBF_NO_BUTTON, TDCBF_OK_BUTTON, TDCBF_YES_BUTTON,
    TDF_ALLOW_DIALOG_CANCELLATION, TDN_CREATED, TaskDialogIndirect,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetThreadDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOPMOST, IDNO, IDYES, MESSAGEBOX_RESULT, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetWindowPos,
};
use windows::core::{HRESULT, HSTRING, PCWSTR};

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

/// `error!(event = "alert")` を必ず 1 件残し、`suppressed` が偽ならエラーの画面を出す
/// （`hwnd` 無し＝`WinApp` の有無を問わない）。戻ったら呼び手が `Err` を返す。
/// 有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）が立つ走行も抑える: 人が見ていない走行で箱が
/// 閉じられるのを待つと、プロセスが終わらず走らせた側が失敗に気づけない（2026-10-31）。
pub(crate) fn raise(scene: &AlertScene, suppressed: bool) {
    let suppressed = suppressed || crate::smoke_exit::smoke_exit_ms().is_some();
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
    // 出せなかった（対話デスクトップが無い・資源不足等）ときだけ見る。押されたボタンは見ない。
    // 出せなくても `alert` の記録は残っているので、原因を別の event で足す。
    if let Err(e) = show_box(Dialog::Alert, &title, &body) {
        tracing::error!(
            event = "alert_box_failed",
            error = %e,
            "[alert] TaskDialogIndirect failed: the alert was only logged"
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
    let shown = show_box(Dialog::Ask, title, body);
    let answer = yes_no_answer(&shown);
    if answer == YesNo::Unavailable {
        let error = match &shown {
            Err(e) => e.to_string(),
            Ok(button) => format!("unexpected button {}", button.0),
        };
        tracing::error!(
            event = "ask_box_failed",
            error = %error,
            "[alert] TaskDialogIndirect failed: the question was not shown"
        );
    }
    answer
}

/// はい／いいえの画面の結果を答えへ写す（純粋）。「はい」と読むのは `IDYES` だけ
/// （はいはインストールの同意なので、思わぬ値を同意に倒さない）。
fn yes_no_answer(shown: &windows::core::Result<MESSAGEBOX_RESULT>) -> YesNo {
    match shown {
        Ok(IDYES) => YesNo::Yes,
        Ok(IDNO) => YesNo::No,
        _ => YesNo::Unavailable,
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
    // 出せなかったときだけ見る。押されたボタンは見ない。
    if let Err(e) = show_box(Dialog::Warning, title, body) {
        tracing::error!(
            event = "warning_box_failed",
            error = %e,
            "[alert] TaskDialogIndirect failed: the warning was only logged"
        );
    }
}

/// 画面を出す間だけ、呼んだスレッドの DPI の扱いをモニタごとの v2 に替えて、出し終えたら戻す。
/// 切り替えは引数で受ける（本番は [`set_thread_dpi_context`]・テストは偽物）。前の扱いが無効なら
/// 替えられなかったとして warn `alert_dpi_context_unchanged` を 1 件残し、今の扱いのまま出して戻さない。
/// `show` はどちらの場合もちょうど 1 回走る。
fn with_thread_per_monitor<T>(
    dialog: &'static str,
    mut switch: impl FnMut(DPI_AWARENESS_CONTEXT) -> DPI_AWARENESS_CONTEXT,
    show: impl FnOnce() -> T,
) -> T {
    let previous = switch(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    if previous.is_invalid() {
        tracing::warn!(
            event = "alert_dpi_context_unchanged",
            dialog,
            "[alert] 画面の DPI の扱いを替えられなかった（今の扱いのまま出す）"
        );
        return show();
    }
    let out = show();
    switch(previous);
    out
}

/// 呼んだスレッドの DPI の扱いを替え、前の扱いを返す（替えられなければ無効な値）。
fn set_thread_dpi_context(context: DPI_AWARENESS_CONTEXT) -> DPI_AWARENESS_CONTEXT {
    // SAFETY: 引数は OS の定数か、この関数が前に返した扱いの値。不正な値なら OS は無効な値を返すだけ。
    unsafe { SetThreadDpiAwarenessContext(context) }
}

/// 告知の画面の種類。種類からボタン・アイコン・閉じるボタンの効き・前面の要否が決まる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialog {
    /// エラーの画面（[`raise`]）。
    Alert,
    /// はい／いいえの画面（[`ask_yes_no`]）。
    Ask,
    /// OK だけの警告の画面（[`warn_ok`]）。
    Warning,
}

impl Dialog {
    /// 記録の欄 `dialog` の名（その口の記録の語と同じ）。
    fn name(self) -> &'static str {
        match self {
            Dialog::Alert => "alert",
            Dialog::Ask => "ask",
            Dialog::Warning => "install_warning_dialog",
        }
    }
}

/// 告知の画面を 1 つ出す（持ち主の窓は渡さない）。出す間だけスレッドをモニタごとの v2 にする。
/// 押されたボタン（IDOK・IDYES・IDNO・閉じたときの IDCANCEL）か、出せなかった HRESULT を返す。
fn show_box(dialog: Dialog, title: &str, body: &str) -> windows::core::Result<MESSAGEBOX_RESULT> {
    let (title, body) = (HSTRING::from(title), HSTRING::from(body));
    // OK だけの画面は閉じるボタンと Esc で閉じられるようにする（取り消しのボタンが無いと既定で効かない）。
    // はい／いいえの画面は閉じるボタンを効かなくする。最前面と前面は問いと警告の画面だけ。
    let (buttons, icon, flags, front) = match dialog {
        Dialog::Alert => (
            TDCBF_OK_BUTTON,
            TD_ERROR_ICON,
            TDF_ALLOW_DIALOG_CANCELLATION,
            false,
        ),
        Dialog::Ask => (
            TDCBF_YES_BUTTON | TDCBF_NO_BUTTON,
            TD_INFORMATION_ICON,
            TASKDIALOG_FLAGS(0),
            true,
        ),
        Dialog::Warning => (
            TDCBF_OK_BUTTON,
            TD_WARNING_ICON,
            TDF_ALLOW_DIALOG_CANCELLATION,
            true,
        ),
    };
    let mut config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: flags,
        dwCommonButtons: buttons,
        pszWindowTitle: PCWSTR(title.as_ptr()),
        pszContent: PCWSTR(body.as_ptr()),
        pfCallback: if front { Some(bring_to_front) } else { None },
        ..Default::default()
    };
    config.Anonymous1.pszMainIcon = icon;
    with_thread_per_monitor(dialog.name(), set_thread_dpi_context, || {
        let mut button = 0i32;
        // SAFETY: `config` が指す `title`／`body` は終端 0 付きでこの呼び出しの間ずっと生存し、
        // `button` はこの呼び出しの間生きている局所の値。他の欄は空か OS の定数。
        unsafe { TaskDialogIndirect(&config, Some(&mut button as *mut i32), None, None) }
            .map(|()| MESSAGEBOX_RESULT(button))
    })
}

/// タスクダイアログの呼び戻し。画面ができた知らせ（TDN_CREATED）で窓を最前面にし、前面へ出す。
/// 前面にできなかったことは記録しない（OS の前面の決まりでも起きる）。ここでは panic しない。
unsafe extern "system" fn bring_to_front(
    hwnd: HWND,
    msg: TASKDIALOG_NOTIFICATIONS,
    _: WPARAM,
    _: LPARAM,
    _: isize,
) -> HRESULT {
    if msg == TDN_CREATED {
        // SAFETY: `hwnd` は OS がこの知らせに渡した、いま生きている画面の窓。他の引数は OS の定数。
        let topmost = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            )
        };
        if let Err(e) = topmost {
            tracing::warn!(
                event = "alert_topmost_failed",
                error = %e,
                "[alert] 画面を最前面にできなかった（画面はそのまま出す）"
            );
        }
        // SAFETY: 同じく OS が渡した生きている窓だけを使う。
        let _ = unsafe { SetForegroundWindow(hwnd) };
    }
    S_OK
}

#[cfg(test)]
#[path = "alert_tests.rs"]
mod alert_tests;

#[cfg(test)]
#[path = "alert_memory_write_tests.rs"]
mod alert_memory_write_tests;

#[cfg(test)]
#[path = "alert_dpi_tests.rs"]
mod alert_dpi_tests;

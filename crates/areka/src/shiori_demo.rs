//! 実走デモドライバ（要件 6）。
//!
//! `reference_brain::shiori_create` で脳を取得し、`ShioriSession` で activate →
//! 即時／遅延の数往復 request → `poll_completions` 待ち合わせ → Raise 観測 → unload
//! までを駆動し、各経路を `tracing::info!` で観測する。フラグ／環境変数ゲートで
//! 起動を制御する（要件 6.8）。
//!
//! ## 本タスク（3.1）の範囲: 駆動＋観測（happy path）＋[`DemoError`] 型定義
//! `run_demo` は `shiori_create`→`ShioriSession::activate`→即時／遅延+Complete／Raise／
//! unload を 1 ループで駆動し、各経路を構造化 `tracing::info!`（`logging.md` 準拠）で観測可能
//! にする（design.md §System Flows → デモ駆動シーケンス・要件 4.3/5.3/6.1〜6.5/9.5）。視覚 UX・
//! 会話描画には依存しない（要件 6.7）。フラグ／環境変数ゲートと失敗時クリーンアップ規律は
//! 後続タスク 3.2、`main.rs` への配線は task 4.1 が担う（本ファイルでは扱わない）。

// task 4.1 が `run_demo` を `main.rs` へ配線し、3.2 がゲートを追加するまで `run_demo` は
// このクレート内から呼ばれない（shiori_host.rs / shiori_session.rs の確立済みパターンに倣う）。
#![allow(dead_code)]

use core::ptr;

use shiori_abi::interface::IShiori;
use windows_core::{AsImpl, HRESULT, HSTRING, Interface};

use crate::reference_brain::{ReferenceBrain, shiori_create};
use crate::shiori_host::HostMessage;
use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// デモドライバ固有のエラー（駆動経路の失敗を型化する）。
///
/// 本タスク（3.1）の happy path では実際には返らないが、失敗報告／クリーンアップ規律を担う
/// 後続タスク（3.2）と遅延完了タイムアウト経路（5.1）が消費する。`Timeout` は 3.2/5.1 用に
/// 先行定義しており、3.1 の happy path では未使用。
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // `Timeout` は task 3.2/5.1 が消費する先行定義（3.1 happy path では未使用）。
pub enum DemoError {
    /// `shiori_create` が失敗 HRESULT を返した（生成入口の失敗・要件 9.3/9.4）。
    #[error("shiori_create failed: 0x{:08X}", .0.0)]
    Create(HRESULT),
    /// `ShioriSession` 操作（activate/request/unload）が失敗した（利用規律 or HRESULT 由来）。
    #[error(transparent)]
    Session(#[from] SessionError),
    /// 遅延完了がタイムアウトした（task 3.2/5.1 が消費する先行定義）。
    #[error("deferred completion timed out")]
    Timeout,
}

/// OnBoot 形の不透明リクエスト content（固定）。誰にも解析されない不透明 HSTRING。
///
/// content 不透明性を保つため、この文字列は脳のエコーで往復するだけで、誰も解析・分割・
/// 意味づけしない（要件 1.4/8.1）。
const ONBOOT_CONTENT: &str = "\\0\\h\\s[0]OnBoot\\e";

/// 遅延完了で脳→host へ送らせる固定応答 content（不透明）。
const DEFERRED_RESPONSE: &str = "\\0\\h\\s[0]deferred-onboot-reply\\e";

/// 能動通知（Raise）で脳→host へ送らせる固定スクリプト content（不透明）。
const RAISE_SCRIPT: &str = "\\h\\s[0]active-notification\\e";

/// in-proc リファレンス脳を取得し、即時・遅延+Complete・Raise・unload の各経路を駆動して
/// `tracing::info!` で観測可能にする（design.md §System Flows → デモ駆動シーケンス）。
///
/// 既存セッション規律（単一 in-flight・相関トークン突合・タイムアウト）に従い、遅延完了は
/// 同一ループで [`ShioriSession::poll_completions`] を drain して待ち合わせる。完了後 `unload`
/// で後始末し、保持していた `IShiori` 参照を Release する（要件 9.5）。
///
/// 視覚 UX・バルーン・さくらスクリプト描画には依存しない（要件 6.7）。各経路の疎通結果は
/// 構造化 `tracing::info!`（フィールド `path` を含む）で出力する（`logging.md` 準拠）。
pub fn run_demo() -> Result<(), DemoError> {
    // 1. 生成入口: `shiori_create` で refcount 1 の IShiori を取得・所有する（要件 9.x）。
    let mut out: *mut core::ffi::c_void = ptr::null_mut();
    // Safety: `out` は有効な書込先スタックスロット。成功時 refcount 1 の IShiori が書き込まれる。
    let hr = unsafe { shiori_create(&mut out) };
    if hr.is_err() {
        return Err(DemoError::Create(hr));
    }
    // Safety: 成功 HRESULT のため `out` は refcount 1 の有効な IShiori。from_raw は AddRef せず adopt。
    let brain: IShiori = unsafe { IShiori::from_raw(out) };
    tracing::info!(path = "create", "[shiori-demo] shiori_create succeeded, IShiori adopted");

    // 2. 脳実体への型付きハンドルを保つため CLONE（AddRef）する。`activate` は IShiori を MOVE
    //    するため、遅延完了／Raise の発火に必要な ReferenceBrain への到達手段を別途確保する
    //    （design.md シーケンス「Demo->>Brain: trigger deferred / trigger raise」）。
    let brain_handle = brain.clone();
    // Safety: `brain_handle` は `shiori_create`（= ReferenceBrain）から得た IShiori であり、
    // 実装実体が ReferenceBrain であることが保証される。as_impl は借用ビューを返す。
    let ref_brain = unsafe { AsImpl::<ReferenceBrain>::as_impl(&brain_handle) };

    // 3. アクティベーション: brain を MOVE して in-proc Load（sink 受け渡し・要件 5.1/6.2）。
    let mut session = ShioriSession::activate(brain).map_err(DemoError::Session)?;
    tracing::info!(path = "activate", "[shiori-demo] session activated (Load delivered sink)");

    let onboot = HSTRING::from(ONBOOT_CONTENT);

    // 4. 即時応答経路: 脳が S_OK＋応答 HSTRING を返す（content はエコーで不透明往復）。
    let immediate = session.request(&onboot).map_err(DemoError::Session)?;
    let SessionRequest::Immediate(immediate_response) = immediate else {
        // happy path では即時のはず。万一遅延が返ったら利用規律違反として扱う。
        return Err(DemoError::Session(SessionError::RequestInFlight));
    };
    // content 不透明性: 即時応答は受信 content のエコー（厳密一致）であることを観測する。
    debug_assert_eq!(immediate_response, onboot, "即時応答は OnBoot content のエコー");
    tracing::info!(
        path = "immediate",
        response_len = immediate_response.len(),
        "[shiori-demo] immediate response received"
    );

    // 5. 遅延応答＋Complete 経路: 次 request を遅延武装し、PENDING＋トークンを受ける（単一 in-flight）。
    ref_brain.arm_defer_next();
    let deferred = session.request(&onboot).map_err(DemoError::Session)?;
    let SessionRequest::Deferred(token) = deferred else {
        // 遅延武装したのに即時が返ったら経路不成立。
        return Err(DemoError::Session(SessionError::RequestInFlight));
    };
    tracing::info!(
        path = "deferred",
        token = token.0,
        "[shiori-demo] deferred pending issued (single in-flight)"
    );

    // 脳→host へ Complete(token, response) を発火する（突合枠は session が request 時にセット済み）。
    let deferred_response = HSTRING::from(DEFERRED_RESPONSE);
    let hr_complete = ref_brain.complete_pending(&deferred_response);
    if hr_complete.is_err() {
        return Err(DemoError::Create(hr_complete));
    }
    // 同一ループで poll_completions を drain して遅延完了を待ち合わせる（要件 6.4 経路）。
    let drained = session.poll_completions();
    let completed = drained.iter().find_map(|m| match m {
        HostMessage::Completed { token: t, response } if *t == token => Some(response.clone()),
        _ => None,
    });
    let completed = completed.ok_or(DemoError::Timeout)?;
    debug_assert!(!session.is_pending(), "Complete 受領で保留が解除されていること");
    tracing::info!(
        path = "complete",
        token = token.0,
        response_len = completed.len(),
        "[shiori-demo] deferred completion drained"
    );

    // 6. 能動通知（Raise）経路: 脳→host へ Raise(script) を発火し、メールボックスから drain する。
    let raise_script = HSTRING::from(RAISE_SCRIPT);
    let hr_raise = ref_brain.fire_raise(&raise_script);
    if hr_raise.is_err() {
        return Err(DemoError::Create(hr_raise));
    }
    let drained = session.poll_completions();
    let raised = drained.iter().find_map(|m| match m {
        HostMessage::Raised(script) => Some(script.clone()),
        _ => None,
    });
    let raised = raised.ok_or(DemoError::Timeout)?;
    tracing::info!(
        path = "raise",
        script_len = raised.len(),
        "[shiori-demo] active notification drained"
    );

    // 7. アンロード: 脳を Unload し host を Release する（teardown 順序・要件 2.2）。
    session.unload().map_err(DemoError::Session)?;
    tracing::info!(path = "unload", "[shiori-demo] session unloaded, brain released");

    // 8. 残った参照を Release して IShiori を解放する（session が brain を、handle が clone を保持）。
    drop(session);
    drop(brain_handle);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    /// info イベントのフィールドを文字列化して捕捉する最小 Layer。
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut buf = String::new();
            struct V<'a>(&'a mut String);
            impl Visit for V<'_> {
                fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                    use std::fmt::Write;
                    let _ = write!(self.0, " {}={:?}", f.name(), v);
                }
            }
            ev.record(&mut V(&mut buf));
            self.0.lock().unwrap().push(buf);
        }
    }

    /// デモ駆動で即時・遅延+Complete・Raise・unload の各経路が tracing info ログとして
    /// 出力され、全体が `Ok(())` で完走すること（観測基準・要件 6.x）。
    #[test]
    fn demo_drives_all_paths_and_emits_info_logs() {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let sub = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(sub, || {
            super::run_demo().expect("demo ok");
        });
        let all = logs.lock().unwrap().join("\n");
        assert!(all.contains("path=\"immediate\""), "immediate path logged: {all}");
        assert!(all.contains("path=\"deferred\""), "deferred path logged: {all}");
        assert!(all.contains("path=\"complete\""), "complete path logged: {all}");
        assert!(all.contains("path=\"raise\""), "raise path logged: {all}");
        assert!(all.contains("path=\"unload\""), "unload path logged: {all}");
    }
}

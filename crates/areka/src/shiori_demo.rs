//! 実走デモドライバ（要件 12.1）。
//!
//! `reference_brain::shiori_factory` で factory を取得し、`ShioriSession` で activate → 即時 get →
//! 遅延 get+complete → raise → notify → drop teardown までを駆動し、各経路を `tracing::info!` で
//! 観測する。フラグ／環境変数ゲートで起動を制御する（design.md §ConsumerFollowup・要件 12.1）。
//!
//! ## 駆動モデル（session 越し＋脳ハンドル併用）
//! [`ShioriSession`] は `IShiori` を factory 経由で生成・保持するが脳アクセサを公開しない。遅延武装
//! （`arm_defer_next`）・能動通知（`fire_raise`）・遅延完了（`complete_pending`）といった **脳側の
//! 駆動** には [`ReferenceBrain`] 実体への到達が要る。そこで factory から **1 つの脳を create し、
//! その脳（`IShiori`）で session を、その脳実体（`ReferenceBrain`）で脳駆動を** 行う——同一インス
//! タンスなので session の突合枠と脳の採番トークンが整合する。session と同じ host（sink）を共有する
//! ことで、脳→host の complete/raise が session の `poll_completions` で観測できる。

use core::ptr;

use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost};
use windows_core::{AsImpl, HRESULT, HSTRING, Interface};

use crate::reference_brain::{ReferenceBrain, shiori_factory};
use crate::shiori_host::HostMessage;
use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// デモドライバ固有のエラー（駆動経路の失敗を型化する）。
#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // `Timeout` は遅延タイムアウト経路用の先行定義（happy path では未使用）。
pub enum DemoError {
    /// `shiori_factory` が失敗 HRESULT を返した（生成入口の失敗）。
    #[error("shiori_factory failed: 0x{:08X}", .0.0)]
    Create(HRESULT),
    /// `ShioriSession` 操作（activate/get/notify）が失敗した（利用規律 or HRESULT 由来）。
    #[error(transparent)]
    Session(#[from] SessionError),
    /// ホスト呼び出し（`complete_pending`/`fire_raise`）が失敗した（HRESULT 由来）。
    #[error(transparent)]
    Shiori(#[from] shiori_abi::error::ShioriError),
    /// 遅延完了がタイムアウトした（先行定義）。
    #[error("deferred completion timed out")]
    Timeout,
}

/// OnBoot 形の不透明リクエスト content（固定）。誰にも解析されない不透明 HSTRING。
const ONBOOT_CONTENT: &str = "\\0\\h\\s[0]OnBoot\\e";
/// 遅延完了で脳→host へ送らせる固定応答 content（不透明）。
const DEFERRED_RESPONSE: &str = "\\0\\h\\s[0]deferred-onboot-reply\\e";
/// 能動通知（raise）で脳→host へ送らせる固定スクリプト content（不透明）。
const RAISE_SCRIPT: &str = "\\h\\s[0]active-notification\\e";
/// 片道通知（notify）で脳へ送る固定 content（不透明）。
const NOTIFY_CONTENT: &str = "NOTIFY SHIORI/3.0 OnFirstBoot";

/// factory を取得し、即時→遅延+complete→raise→notify→drop teardown の各経路を駆動して
/// `tracing::info!` で観測可能にする（design.md §ConsumerFollowup）。
pub fn run_demo() -> Result<(), DemoError> {
    // 1. 生成入口: `shiori_factory` で refcount 1 の IShioriFactory を取得・所有する。
    let mut out: *mut core::ffi::c_void = ptr::null_mut();
    // Safety: `out` は有効な書込先スタックスロット。成功時 refcount 1 の IShioriFactory が書き込まれる。
    let hr = unsafe { shiori_factory(&mut out) };
    if hr.is_err() {
        tracing::error!(hr = format!("0x{:08X}", hr.0), "[shiori-demo] shiori_factory failed");
        return Err(DemoError::Create(hr));
    }
    // Safety: 成功 HRESULT のため `out` は refcount 1 の有効な IShioriFactory。from_raw は AddRef せず adopt。
    let factory: IShioriFactory = unsafe { IShioriFactory::from_raw(out) };
    tracing::info!(path = "create", "[shiori-demo] shiori_factory succeeded, IShioriFactory adopted");

    // 2. host（sink）を生成し、その host を共有する脳を factory 経由で create する。
    //    脳（IShiori）で session を、脳実体（ReferenceBrain）で脳駆動を行う（同一インスタンス）。
    //    プロパティ応答は App スコープのみの sylphya アクターへ委譲する（第 2 ストア撤去・R7.2/R7.3・
    //    design.md §bin（ShioriHostSink 統合））。session 固有の `AskerId` は demo の load_dir 相当文字列
    //    から構築する。`_sylphya_handle` は非 RAII（detach）で、アクターは sink 内包 publisher が生存
    //    する限り生き続けるため run_demo の生存期間だけ保持すれば足りる（join は要さない）。
    let asker = areka_sylphya::AskerId::new("C:/ghost/master");
    let (sink, _sylphya_handle) = crate::shiori_host::spawn_app_sylphya_sink(asker);
    let host: IShioriHost = sink.into();
    let brain: IShiori = match factory.create(
        &HSTRING::from("C:/ghost/master"),
        &HSTRING::from("reference"),
        &host,
    ) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "[shiori-demo] brain create failed");
            return Err(DemoError::Session(SessionError::Shiori(e)));
        }
    };
    // 脳駆動用ハンドル（session が brain を move するため、駆動用に AddRef した参照を確保）。
    let brain_handle = brain.clone();
    // Safety: `brain_handle` は `factory.create`（= ReferenceBrain）から得た IShiori。
    let ref_brain = unsafe { AsImpl::<ReferenceBrain>::as_impl(&brain_handle) };

    // 3. session を「同じ脳＋同じ host」で確立する。session は脳を move で保持する。
    //    ここでは activate ではなく、既に create 済みの brain/host で session を組み立てるため、
    //    session の内部生成をバイパスして同一インスタンスを共有する専用コンストラクタを用いる。
    let mut session = ShioriSession::from_parts(brain, host.clone());
    tracing::info!(path = "activate", "[shiori-demo] session established (shared brain + sink)");

    // 4. 駆動本体（即時→遅延+complete→raise→notify）。失敗しても drop teardown へ進む。
    let result = drive_paths(&mut session, ref_brain);
    if let Err(ref e) = result {
        tracing::error!(error = %e, "[shiori-demo] demo driving failed; will drop for teardown");
    }

    // 5. drop teardown: session を drop（保留取消→brain drop）し、駆動ハンドル・host も drop する。
    drop(session);
    drop(brain_handle);
    drop(host);
    if result.is_ok() {
        tracing::info!(path = "unload", "[shiori-demo] session dropped (teardown)");
    }

    result
}

/// 即時／遅延+complete／raise／notify の各経路を session 越し＋脳ハンドル併用で駆動する。
fn drive_paths(session: &mut ShioriSession, ref_brain: &ReferenceBrain) -> Result<(), DemoError> {
    let onboot = HSTRING::from(ONBOOT_CONTENT);

    // 即時応答経路: session 越しの get が S_OK＋応答 HSTRING（エコー）を返す。
    let immediate = session.get(&onboot).map_err(DemoError::Session)?;
    let SessionRequest::Immediate(immediate_response) = immediate else {
        return Err(DemoError::Session(SessionError::RequestInFlight));
    };
    debug_assert_eq!(immediate_response, onboot, "即時応答は OnBoot content のエコー");
    tracing::info!(
        path = "immediate",
        response_len = immediate_response.len(),
        "[shiori-demo] immediate response received"
    );

    // 遅延応答＋complete 経路: 脳を遅延武装 → session get で Deferred(token) を受ける（単一 in-flight）。
    ref_brain.arm_defer_next();
    let deferred = session.get(&onboot).map_err(DemoError::Session)?;
    let SessionRequest::Deferred(token) = deferred else {
        return Err(DemoError::Session(SessionError::RequestInFlight));
    };
    tracing::info!(
        path = "deferred",
        token = token.0,
        "[shiori-demo] deferred pending issued (single in-flight)"
    );

    // 脳→host complete を safe メソッドで発火（突合枠は session が get 時にセット済み）。
    let deferred_response = HSTRING::from(DEFERRED_RESPONSE);
    ref_brain.complete_pending(&deferred_response).map_err(DemoError::Shiori)?;
    // 同一ループで poll_completions を drain して遅延完了を待ち合わせる。
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

    // 能動通知（raise）経路: 脳→host raise を safe メソッドで発火し、メールボックスから drain する。
    let raise_script = HSTRING::from(RAISE_SCRIPT);
    ref_brain.fire_raise(&raise_script).map_err(DemoError::Shiori)?;
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

    // 片道通知（notify）経路: session 越しに notify を発行する（応答なし・受領ログへ記録される）。
    session
        .notify(&HSTRING::from(NOTIFY_CONTENT))
        .map_err(DemoError::Session)?;
    tracing::info!(path = "notify", "[shiori-demo] one-way notify delivered");

    Ok(())
}

/// デモ駆動を有効化する環境変数名（design.md §ConsumerFollowup Implementation Notes）。
const DEMO_ENV: &str = "AREKA_SHIORI_DEMO";

/// 与えられた値からデモ有効化を判定する純粋ヘルパ（env アクセスなし・単体テスト可能）。
///
/// 空・`0`・`false`（大小無視）は無効、それ以外の非空値は有効とする。
fn demo_enabled_from(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        Some(v) => !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"),
        None => false,
    }
}

/// 環境変数 [`DEMO_ENV`] からデモ有効化を判定する。
pub fn demo_enabled() -> bool {
    demo_enabled_from(std::env::var(DEMO_ENV).ok().as_deref())
}

/// 有効フラグを入力に取り、有効時のみ [`run_demo`] を駆動する純粋入力コア（単体テスト可能）。
fn run_demo_if_enabled_with(enabled: bool) -> Result<(), DemoError> {
    if !enabled {
        tracing::debug!(
            env = DEMO_ENV,
            "[shiori-demo] demo disabled by default; set the env var to enable; skipping"
        );
        return Ok(());
    }
    run_demo()
}

/// 環境変数ゲートを評価し、明示有効化時のみデモを駆動する。
pub fn run_demo_if_enabled() -> Result<(), DemoError> {
    run_demo_if_enabled_with(demo_enabled())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing_subscriber::prelude::*;

    /// イベントのレベル＋フィールドを文字列化して捕捉する最小 Layer。
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(
            &self,
            ev: &tracing::Event<'_>,
            _: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut buf = format!("level={}", ev.metadata().level());
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

    /// デモ駆動で即時・遅延+complete・raise・notify・teardown の各経路が tracing info ログとして
    /// 出力され、全体が `Ok(())` で完走すること（観測基準）。
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
        assert!(all.contains("path=\"notify\""), "notify path logged: {all}");
        assert!(all.contains("path=\"unload\""), "teardown (unload) path logged: {all}");
    }

    /// ゲート無効時はデモが起動しないこと（観測基準）。
    #[test]
    fn gate_disabled_does_not_drive() {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let sub = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(sub, || {
            super::run_demo_if_enabled_with(false).expect("disabled gate returns Ok");
        });
        let all = logs.lock().unwrap().join("\n");
        assert!(
            !all.contains("path=\"immediate\""),
            "disabled gate must not drive demo paths: {all}"
        );
    }

    /// ゲート解析（純粋）の境界値。
    #[test]
    fn gate_parsing_pure() {
        assert!(!super::demo_enabled_from(None));
        assert!(!super::demo_enabled_from(Some("")));
        assert!(!super::demo_enabled_from(Some("0")));
        assert!(!super::demo_enabled_from(Some("false")));
        assert!(!super::demo_enabled_from(Some("FALSE")));
        assert!(!super::demo_enabled_from(Some("  ")));
        assert!(super::demo_enabled_from(Some("1")));
        assert!(super::demo_enabled_from(Some("true")));
    }

    /// ゲート有効時はデモが駆動され、各経路ログが出ること。
    #[test]
    fn gate_enabled_drives() {
        let cap = Capture::default();
        let logs = cap.0.clone();
        let sub = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(sub, || {
            super::run_demo_if_enabled_with(true).expect("enabled gate drives demo ok");
        });
        let all = logs.lock().unwrap().join("\n");
        assert!(all.contains("path=\"immediate\""), "enabled gate drives immediate: {all}");
        assert!(all.contains("path=\"unload\""), "enabled gate drives teardown: {all}");
    }
}

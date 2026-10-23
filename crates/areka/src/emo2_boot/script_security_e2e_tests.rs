//! 偽の SHIORI で起動の道を通す、台本の出どころの通しのテスト（areka-P0-script-security-level
//! 要件 1.4・3.1・4.2・4.3・design「E2E（x64 の偽の SHIORI）」）。
//!
//! 親（`runtime_boot_tests.rs`）と同じ組み方（`ShioriWiring::Custom` に偽の受け手・12 本の受け口）で
//! 起こし、`OnBoot` の台本 `\![enter,nouserbreakmode]…` の応答に `SecurityLevel` を添える。
//! kanade → 台本の起動 → 配送 → 中断を禁じる受け口の運ぶ道が既定の値に落ちていなければ、
//! `external` では線に `Enter` が出ず、`local`／無しでは今どおり出る。

use super::*;
use crate::emo2_boot::ghost_switch_test_support::standard_script;
use crate::emo2_boot::spine::wait_until;
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_kanade::{MonotonicMs, ShioriUnblock};
use shiori_host32_host::{
    ExitKind, HelperStatus, RequestError, RequestHeaders, ShutdownError, ValueReply,
};

/// 注入する Tick 1 回で台詞の時計を進める幅（合成の ms）。
const TICK_STEP_MS: u64 = 100;

/// 区間に入って出るだけの起動の台本（`Leave` は出どころによらず届く＝読み終わりの目印）。
const ON_BOOT: &str = r"\0\![enter,nouserbreakmode]\![leave,nouserbreakmode]\e";

/// 台本つきの偽の受け手を包み、`OnBoot` の応答にだけ `SecurityLevel` を添える
/// （共有の偽物 `spine.rs` は触らない・ヘッダつきの GET だけを上書きする）。
struct LeveledBackend {
    inner: ScriptedShioriBackend,
    on_boot_level: Option<&'static str>,
}

impl ShioriBackend for LeveledBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.inner.get(id, references, status)
    }
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.inner.notify(id, references, status)
    }
    fn get_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        _headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        let level = (id == "OnBoot").then_some(self.on_boot_level).flatten();
        Ok(self
            .inner
            .get(id, references, status)?
            .map(|value| ValueReply {
                value,
                security_level: level.map(str::to_owned),
            }))
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.inner.unload()
    }
    fn status(&mut self) -> HelperStatus {
        self.inner.status()
    }
    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        self.inner.unblock_handle()
    }
}

/// emo2 検体を偽の SHIORI で起こし、`OnBoot` のトークが中断を禁じる線へ送った合図を
/// `Leave` まで読んで返す（読み終えたら降ろす）。
fn boot_signals(on_boot_level: Option<&'static str>) -> Vec<NoUserBreakSignal> {
    com();
    let rig = rig();
    let sample = acquire_emo2();
    let (inner, handle) = standard_script(ON_BOOT).build();
    let backend = LeveledBackend {
        inner,
        on_boot_level,
    };
    let mut world = World::new();
    let inputs = Emo2BootInputs {
        ghost_root: sample.folder().to_path_buf(),
        balloon_root: sample
            .balloon("emo2-kakukaku")
            .expect("emo2 の同梱バルーン")
            .to_path_buf(),
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        ticker: TickerMode::Disabled,
        app_profile_dir: None,
        boot_origin: BootOrigin::Plain,
    };
    let runtime = boot_runtime(&mut world, inputs, &rig.sources, mpsc::channel().0, None)
        .expect("偽の SHIORI と emo2 検体で実行系が起きる");

    let wiring = world
        .get_non_send::<UserBreakWiring>()
        .expect("成功で中断の持ち物が据わる");
    // 足場は `TickerMode::Disabled` なので、台詞の時計は dispatcher へ Tick を注入して進める。
    let mut signals = Vec::new();
    let mut clock_ms = 0;
    let reached = wait_until("OnBoot の中断の合図", Progress::Unknown, || {
        clock_ms += TICK_STEP_MS;
        let _ = runtime.dispatcher().send(DispatcherMsg::Tick {
            now: MonotonicMs(clock_ms),
        });
        signals.extend(wiring.flag_rx().try_iter());
        signals.contains(&NoUserBreakSignal::Leave)
    });
    assert!(
        reached.is_ok(),
        "Leave まで届かない（届いた分 {signals:?}）: {reached:?}"
    );

    let calls = move || handle.call_count();
    run_bounded_watching(
        "script_security ghost shutdown",
        Progress::Count(&calls),
        move || {
            let _ = runtime.shutdown(CloseReason::User { scope: 0 });
        },
    );
    drop(sample);
    // 同じ読みで後ろの `TalkEnded` まで取れることがある（届く時刻しだい）ので `Leave` までで切る。
    signals.truncate(
        signals
            .iter()
            .position(|s| *s == NoUserBreakSignal::Leave)
            .map_or(signals.len(), |i| i + 1),
    );
    signals
}

/// 応答の `SecurityLevel: external` で起動のトークが外部扱いになり、線に `Enter` が出ない（4.2）。
/// 運ぶ道のどこかが既定の値（ゴースト自身＝`local`）に落ちると `Enter` が出て赤になる。
#[test]
fn external_on_boot_reply_keeps_enter_off_the_line() {
    assert_eq!(
        boot_signals(Some("external")),
        vec![NoUserBreakSignal::TalkStarted, NoUserBreakSignal::Leave],
    );
}

/// `local` または応答のレベル無しでは今どおり `Enter` が出る（1.4・3.1・4.3）。
#[test]
fn local_or_absent_on_boot_reply_lets_enter_through() {
    for level in [Some("local"), None] {
        assert_eq!(
            boot_signals(level),
            vec![
                NoUserBreakSignal::TalkStarted,
                NoUserBreakSignal::Enter,
                NoUserBreakSignal::Leave,
            ],
            "応答のレベル {level:?}",
        );
    }
}

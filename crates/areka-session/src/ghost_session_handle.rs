//! `GhostSession` の窓口（名前・実行系・送り手の取り出しと降ろす手順）。

use super::*;

impl GhostSession {
    /// 告知の場面が使うゴースト名（fallback の起動に失敗して実行系が無ければ `None`）。
    pub fn ghost_name(&self) -> Option<String> {
        self.names().and_then(|n| n.name.clone())
    }

    /// ゴーストの名前情報（descript の `name`・`sakura.name` ほか）。実行系が無ければ `None`。
    pub fn names(&self) -> Option<&GhostNames> {
        self.ghost.as_ref().map(|r| &r.mount().names)
    }

    /// ゴーストの実行系（定常到達の記憶を、そのゴーストの記憶の書き手へ投函する）。無ければ `None`。
    pub fn runtime(&self) -> Option<&areka_ghost::GhostRuntime> {
        self.ghost.as_ref()
    }

    /// 窓への結線が成立せず LogSink の起動へ倒れた単位か（印の判定の材料）。
    pub fn logsink_fallback(&self) -> bool {
        self.logsink_fallback
    }

    /// kanade への送出端（切替の要求を送る）。実行系が無ければ `None`。
    pub fn kanade(&self) -> Option<&Sender<KanadeMsg>> {
        self.kanade.as_ref()
    }

    /// seriko の送り手の複製（シェル・バルーンの差し替えを頼む）。結線ありの腕でなければ `None`。
    pub fn seriko_sink(&self) -> Option<&areka_seriko::SerikoSink> {
        self.seriko_sink.as_ref()
    }

    /// 実行系の記憶の書き手（差し替えの後始末が `LastShell`／`LastBalloon` を投函する）。
    /// 実行系が無ければ `None`（テスト用の組み立ては [`GhostSession::with_memory_publisher`] の値）。
    pub(crate) fn memory_publisher(&self) -> Option<&areka_sylphya::SylphyaPublisher> {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(publisher) = &self.memory_publisher_for_test {
            return Some(publisher);
        }
        self.ghost.as_ref().map(|r| r.sylphya_publisher())
    }

    /// 今のシェルのフォルダ名（実行系のマウントの末尾）。実行系が無ければ `None`。
    pub fn current_shell_folder(&self) -> Option<String> {
        let dir = &self.ghost.as_ref()?.mount().shell.dir;
        dir.file_name().map(|n| n.to_string_lossy().into_owned())
    }

    /// 今のシェルのフォルダだけを書き換える（実行系へ委ねる）。実行系が無ければ偽。
    pub fn set_shell_dir(&mut self, dir: PathBuf) -> bool {
        match self.ghost.as_mut() {
            Some(runtime) => {
                runtime.set_shell_dir(dir);
                true
            }
            None => false,
        }
    }

    /// sylphya のバルーンの層を付け替える（実行系へ委ねる）。実行系が無ければ偽。
    pub fn set_balloon_dir(&self, dir: &Path) -> bool {
        match self.ghost.as_ref() {
            Some(runtime) => {
                runtime.set_balloon_dir(dir);
                true
            }
            None => false,
        }
    }

    /// 実行系だけを外す（窓の側＝seriko・ticker・送り手・束は残る）。
    /// kanade の送出端も外す。実行系が無ければ `None`。
    pub fn split_runtime(&mut self) -> Option<areka_ghost::GhostRuntime> {
        self.kanade = None;
        self.ghost.take()
    }

    /// 新しい実行系を付け、kanade の送出端を写す。付いていた実行系は降ろさずに
    /// 落ちるので、先に [`GhostSession::split_runtime`] で外しておくこと（外し忘れは `error!`）。
    pub fn attach_runtime(&mut self, runtime: areka_ghost::GhostRuntime) {
        if self.ghost.is_some() {
            tracing::error!(
                ghost_dir = %self.ghost_dir.display(),
                "実行系を外さずに付け直した（前の実行系は降ろさずに落ちる）"
            );
        }
        self.kanade = Some(runtime.kanade().clone());
        self.ghost = Some(runtime);
    }

    /// 受け口の送出端の束（結線ありの腕だけ `Some`）。
    pub fn sink_sources(&self) -> Option<&CueSinkSources> {
        self.sinks.as_ref()
    }

    /// ゴーストの名前を替える（実行系へ委ねる）。実行系が無ければ偽。
    pub fn set_names(&mut self, names: GhostNames) -> bool {
        match self.ghost.as_mut() {
            Some(runtime) => {
                runtime.set_names(names);
                true
            }
            None => false,
        }
    }

    /// テスト用の組み立てに記憶の書き手を持たせる（差し替えの後始末の記憶を観測するため）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_memory_publisher(mut self, publisher: areka_sylphya::SylphyaPublisher) -> Self {
        self.memory_publisher_for_test = Some(publisher);
        self
    }

    /// テスト用の組み立てに seriko の送り手を持たせる（切替の入口の文脈の判定を通すため）。
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_seriko_sink(mut self, sink: areka_seriko::SerikoSink) -> Self {
        self.seriko_sink = Some(sink);
        self
    }

    /// 実行系を持たない置き場の中身（テスト用）: kanade への送出端と根だけを持つ。
    /// 降ろすもの（実行系・seriko・ticker）が無いので `shutdown` は何もせず `Ok` を返す。
    #[cfg(any(test, feature = "test-support"))]
    pub fn for_test(kanade: Option<Sender<KanadeMsg>>, ghost_dir: PathBuf) -> Self {
        Self {
            ghost: None,
            seriko: None,
            loop_ticker: None,
            seriko_sink: None,
            sinks: None,
            kanade,
            ghost_dir,
            logsink_fallback: false,
            memory_publisher_for_test: None,
        }
    }

    /// 台詞を進める Tick の注入口（テスト用・時計を止めて起こした実行系の dispatcher）。
    /// 実行系が無ければ `None`。
    #[cfg(any(test, feature = "test-support"))]
    pub fn dispatcher(&self) -> Option<&Sender<areka_ghost::dispatcher::DispatcherMsg>> {
        self.ghost.as_ref().map(|r| r.dispatcher())
    }

    /// 起こしたゴーストの根（実行系が無くても起動に渡した根を返す）。
    pub fn ghost_dir(&self) -> &Path {
        &self.ghost_dir
    }

    /// 降ろす: ① loop ticker の停止（seriko の送り手の複製と束も落とす）→ ② ゴースト実行系の
    /// 終了（`reason` を渡す）→ ③ seriko の join。無い段は飛ばし、②③ の失敗は `error!` の上で
    /// `Err` を返して以降を飛ばす。perf の最終報告はプロセスに 1 回なので呼び手が後で行う。
    pub fn shutdown(self, reason: areka_kanade::CloseReason) -> windows::core::Result<()> {
        self.shutdown_impl(reason, None).0
    }

    /// 期限つきで降ろす（OS のセッションの終了の受け手だけが呼ぶ）。手順は
    /// [`GhostSession::shutdown`] と同じで、② だけを見張りつきの `GhostRuntime::shutdown_within` に
    /// 替える。戻りの 2 つ目は上限で SHIORI の待ちを打ち切ったときだけ `Some`（実行系が無ければ `None`）。
    pub(crate) fn shutdown_within(
        self,
        reason: areka_kanade::CloseReason,
        budget: areka_kanade::WaitBudget,
    ) -> (windows::core::Result<()>, Option<areka_kanade::ShioriCut>) {
        self.shutdown_impl(reason, Some(budget))
    }

    /// 降ろす手順の本体（`budget` が在れば ② を見張りつきで走らせる）。
    fn shutdown_impl(
        self,
        reason: areka_kanade::CloseReason,
        budget: Option<areka_kanade::WaitBudget>,
    ) -> (windows::core::Result<()>, Option<areka_kanade::ShioriCut>) {
        // ① loop ticker Close: ticker の worker スレッドは closure 内に `SerikoSink` の複製（tick_sink）を
        // 握る。先に停止させないと seriko inbox が ticker 経由で生き続け、③ の join が全 Sender の drop を
        // 永遠に待って hang する。呼び手は ticker の JoinHandle を持たず直接 join できないが、③ の join が
        // 全 Sender drop まで block するので ticker worker の終端を待つ形になる。失敗（既に終端済み）は
        // shutdown の期待事象なので `debug!`。
        if let Some(ticker) = self.loop_ticker {
            match ticker.send(areka_ghost::ticker::TickerMsg::Close) {
                Ok(()) => tracing::info!(
                    "seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止）"
                ),
                Err(_) => tracing::debug!(
                    "seriko: loop ticker は既に終端済み（Close 送信先なし・shutdown 期待事象）"
                ),
            }
            // 送信の成否に依らず停止端 Sender をここで drop し、確実に制御チャンネルを disconnected にする。
            drop(ticker);
        }
        // セッションが持つ seriko の送り手の複製（差し替えの口）も ① で落とす。残すと self の残りの欄
        // として関数の終わりまで生き、③ の join が「全 Sender drop」を待ち続けて hang する。
        drop(self.seriko_sink);
        // 受け口の送出端の束も同じ理由で ① で落とす（束の seriko の送り手）。
        drop(self.sinks);

        // ② 終了握手: boot 済み（`Some`）のときのみ `shutdown` を呼ぶ。終了理由は呼び手が渡す（`fn main` は
        // `CloseReason::User { scope: 0 }`＝全窓 close funnel はユーザ操作起点）。OnClose 応答の再生完了待ちは
        // kanade の `ForceQuit` 終了系列内で処理される。失敗は `error!` の上で呼び手へ `Err` を返す
        // （genuine な失敗を黙って exit 0 にしない）。
        let mut cut = None;
        if let Some(runtime) = self.ghost {
            let result = match budget {
                Some(budget) => {
                    let (result, fired) = runtime.shutdown_within(reason, budget);
                    cut = fired;
                    result
                }
                None => runtime.shutdown(reason),
            };
            if let Err(err) = result {
                tracing::error!(error = %err, "ghost 結線層の終了統括に失敗しました");
                return (
                    Err(windows::core::Error::from_hresult(
                        windows::Win32::Foundation::E_FAIL,
                    )),
                    cut,
                );
            }
        }

        // ③ seriko アクターの join。seriko inbox への送信端は 4 本ある: (a) ghost 側の `SerikoSink`（② の
        // `shutdown` が drop）、(b) loop ticker closure の `tick_sink`（① の Close で drop）、(c) セッションの
        // 複製（`seriko_sink`・① で drop）、(d) 受け口の送出端の束 `sinks` の送り手（① で drop）。
        // ①② で全端が drop されて inbox が切断され seriko worker は自然終了するので、この join は
        // 速やかに戻る（① で ticker を先に Close し複製を落とすのが hang 回避の要）。
        // join 失敗（worker panic）は握り潰さず `error!`＋`Err` を伝播する。
        if let Some(seriko) = self.seriko {
            if let Err(err) = seriko.join() {
                tracing::error!(error = %err, "seriko アクターの join に失敗しました");
                return (
                    Err(windows::core::Error::from_hresult(
                        windows::Win32::Foundation::E_FAIL,
                    )),
                    cut,
                );
            }
        }

        (Ok(()), cut)
    }
}

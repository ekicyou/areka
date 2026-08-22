//! areka 本体側 `IShioriHost` 実装（単一 sink・突合枠・メールボックス投函＋sylphya 委譲）。
//!
//! 脳（`IShiori` 実装）が [`IShioriFactory::create`](shiori_abi::interface::IShioriFactory) 時に
//! 受け取る単一 sink を areka 側で `#[implement(IShioriHost)]` により実装する
//! （design.md §ShioriHostSink, requirements.md 10.2/10.3/10.4/12.4/7.2/7.3）。
//!
//! ## 役割（design.md §ShioriHostSink・§bin（ShioriHostSink 統合））
//! - **単一 sink** が能動通知（[`Raise`](shiori_abi::interface::IShioriHost::Raise)）・
//!   遅延応答（[`Complete`](shiori_abi::interface::IShioriHost::Complete)）・
//!   プロパティアクセス（[`GetProperty`](shiori_abi::interface::IShioriHost::GetProperty)/
//!   [`SetProperty`](shiori_abi::interface::IShioriHost::SetProperty)）の全てを受ける（要件 10.1）。
//! - **突合枠 `Option<CorrelationToken>`**（単一 in-flight・議題3）を thread-safe に所有する。
//!   areka は遅延 request 発行時に [`ShioriHostSink::set_pending_token`] でトークンをセットし、
//!   `Complete` は保持中トークンと突き合わせて応答をメールボックスへ投函する（design.md §State）。
//! - `Raise`/`Complete` は脳の任意スレッドから来うる前提（議題3）で **thread-safe にメールボックスへ
//!   投函して即返す**。突合不能/stale/未知トークンは [`shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN`] を返す。
//! - **プロパティ応答は sylphya へ委譲**（第 2 ストア撤去・R7.2/R7.3・design.md §bin（ShioriHostSink 統合））。
//!   独立した `Mutex<HashMap<String, HSTRING>>` は撤去し、代わりに統一プロパティシステム sylphya の
//!   読み口 [`SylphyaReader`]＋変異投函 [`SylphyaPublisher`]＋自セッションの [`AskerId`] を保持する
//!   （[`ShioriHostSink::with_sylphya`] コンストラクタ）。`GetProperty` は reader の逐次同期解決
//!   （[`SylphyaReader::resolve_dotted_str`]）で **同期即答** し（mailbox 投函で代替しない・要件 10.3）、
//!   欠落 key（[`DottedResolution::NotFound`]）は [`shiori_abi::error::SHIORI_E_PROPERTY_NOT_FOUND`] を
//!   返す（暗黙の空値で続行しない）。`SetProperty` は publisher へ投函して即 `Ok(())`（裁定 §10.4・
//!   read-your-write は有界ラグ）。任意スレッドから呼出可能。
//! - `[in]` HSTRING は借用（呼び出し中のみ有効）。保持/投函する内容は host 側で clone/String 化する
//!   （要件 10.4/12.4）。HSTRING⇄String 橋は本 sink 境界に閉じる（sylphya は String のみ扱う）。
//! - **非循環所有**: host struct は脳（`IShiori`）へ強参照を持たない（脳→host の一方向）。
//!
//! ## 再入規約（契約として doc 固定・design.md §確定設計判断 (b)・要件 10.3/7.2）
//! areka は [`IShiori::Get`](shiori_abi::interface::IShiori::Get)/
//! [`IShiori::Notify`](shiori_abi::interface::IShiori::Notify) 呼出中にプロパティ解決でブロッキング
//! 照会・大域ロック取得を行わない。→ 脳が `Get` 処理中に同一スレッドで `get_property` を呼び戻しても
//! デッドロックしない。これは reader の逐次解決が **鏡像スナップショットの read lock 内 `Arc` clone のみ**
//! （他スレッド／他アクターへの pull 照会をしない・最小区間ロック）で完結する性質により構造的に
//! 担保される（design.md §「同期読み・非同期供給の掲示板」）。
//!
//! ## メールボックスの範囲（design.md §Boundary Commitments）
//! メールボックスは「受け皿」までを本仕様が所有する（thread-safe queue の最小実装）。
//! ECS/bevy への実際の配送は本仕様スコープ外であり、ここでは取り出して検証できる最小形に留める。

use std::collections::VecDeque;
use std::sync::Mutex;

use areka_sylphya::{AskerContext, AskerId, DottedResolution, SylphyaPublisher, SylphyaReader};
use shiori_abi::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{HSTRING, Result, implement};

/// メールボックスへ投函される host 受信メッセージ（能動通知 / 遅延完了）。
///
/// `Raise`/`Complete` が受領内容を clone してこの enum で投函する。ECS/上位への配送は
/// 本仕様外のため、テスト/上位が取り出して解釈する最小表現に留める（design.md §Boundary）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostMessage {
    /// 能動通知（`Raise`）。さくらスクリプト相当の不透明 HSTRING を内包する（要件 10.1）。
    Raised(HSTRING),
    /// 遅延完了応答（`Complete`）。突合に成功したトークンと応答 HSTRING を内包する（要件 10.1）。
    Completed {
        token: CorrelationToken,
        response: HSTRING,
    },
}

/// areka 本体側の `IShioriHost` 実装（単一 sink）。
///
/// 突合枠・メールボックスを thread-safe に所有し、プロパティ応答は統一プロパティシステム sylphya の
/// 読み口 [`SylphyaReader`]＋変異投函 [`SylphyaPublisher`]＋自セッションの [`AskerId`] へ委譲する
/// （第 2 ストア撤去・R7.2/R7.3・design.md §bin（ShioriHostSink 統合））。脳へ強参照を持たない
/// （非循環所有・design.md §ShioriHostSink）。
#[implement(shiori_abi::interface::IShioriHost)]
pub struct ShioriHostSink {
    /// 唯一の保留枠（単一 in-flight・議題3）。areka が遅延 request 発行時にセットし、
    /// `Complete` が突き合わせる。突合成功時にクリアして stale/重複 `Complete` を弾く。
    pending: Mutex<Option<CorrelationToken>>,
    /// 能動通知・遅延完了の受け皿（thread-safe queue の最小実装）。
    mailbox: Mutex<VecDeque<HostMessage>>,
    /// sylphya の同期・無待機の読み口（`GetProperty` の逐次同期解決に使用・R7.2）。
    ///
    /// 逐次解決は鏡像スナップショットの read lock 内 `Arc` clone のみで完結し、他スレッド／他アクターへの
    /// pull 照会をしない（大域ロックにならない・再入規約を構造的に担保・要件 7・design.md 掲示板モデル）。
    reader: SylphyaReader,
    /// sylphya の変異投函の送信端（`SetProperty`／`set_property_value` の投函先・R7.2）。
    ///
    /// 投函のみで鏡像を直接触らない（single-writer はアクターが担保）。アクター死亡後の投函は
    /// panic せず縮退する（fire-and-forget は warn＋破棄・design.md §Error Handling）。
    publisher: SylphyaPublisher,
    /// 自セッション固有の問い合わせ元 ID（フラット/dotted の per-asker 着地先・R7.2）。
    ///
    /// `GetProperty` は本 asker から [`AskerContext`] を導いて逐次解決し、`SetProperty`／
    /// `set_property_value` は本 asker を投函に添える（同一セッションの読み書きが一致する）。
    asker: AskerId,
}

impl ShioriHostSink {
    /// sylphya の読み口・投函端・自セッション `AskerId` を与えて sink を生成する（R7.2/R7.3）。
    ///
    /// 突合枠・メールボックスは空で初期化する。プロパティ応答（`GetProperty`/`SetProperty`）は
    /// 与えられた `reader`/`publisher` へ委譲し、独立した第 2 ストアは持たない（design.md
    /// §bin（ShioriHostSink 統合））。`reader`/`publisher` は同一 sylphya アクターに由来し、
    /// `asker` はそのセッション固有の問い合わせ元 ID である（呼び出し側＝sink 組立各所が保証する）。
    pub fn with_sylphya(
        reader: SylphyaReader,
        publisher: SylphyaPublisher,
        asker: AskerId,
    ) -> Self {
        Self {
            pending: Mutex::new(None),
            mailbox: Mutex::new(VecDeque::new()),
            reader,
            publisher,
            asker,
        }
    }

    /// 遅延 request 発行時に areka が突合枠へ相関トークンをセットする（単一 in-flight）。
    ///
    /// 既存の保留トークンは上書きされる（単一 in-flight 前提では発行前に解消済みのはず）。
    pub fn set_pending_token(&self, token: CorrelationToken) {
        *self.pending.lock().expect("pending mutex poisoned") = Some(token);
    }

    /// 現在の突合枠の内容を返す（観測用）。
    pub fn pending_token(&self) -> Option<CorrelationToken> {
        *self.pending.lock().expect("pending mutex poisoned")
    }

    /// 突合枠を空にする（保留取消・タイムアウト放棄時に areka が呼ぶ）。
    ///
    /// 突合枠が空になるため、以降に遅れて来る `Complete`（タイムアウト後・teardown 後の stale）は
    /// トークン不一致で [`SHIORI_E_UNKNOWN_TOKEN`] により弾かれる（議題3）。
    pub fn clear_pending_token(&self) {
        *self.pending.lock().expect("pending mutex poisoned") = None;
    }

    /// メールボックスから先頭メッセージを取り出す（FIFO・受け皿からの取り出し）。
    pub fn try_recv(&self) -> Option<HostMessage> {
        self.mailbox
            .lock()
            .expect("mailbox mutex poisoned")
            .pop_front()
    }

    /// メールボックスに滞留しているメッセージ数（観測用）。
    pub fn mailbox_len(&self) -> usize {
        self.mailbox.lock().expect("mailbox mutex poisoned").len()
    }

    /// areka 側からプロパティ値を充填する（`AsImpl` 経由・要件 10.5・R7.2）。
    ///
    /// 第 2 ストアへの直接書込ではなく sylphya publisher への **投函の薄いラッパ** として存続する
    /// （`SetProperty` と同じ着地・design.md §bin（ShioriHostSink 統合））。key は SSP プロパティ
    /// システムの dotted パス（要件 10.2）。任意スレッドから呼出可能（投函のみ）。反映は sylphya
    /// アクターが担うため read-your-write は有界ラグ（研究 §12-4・テストは barrier で反映を待つ）。
    pub fn set_property_value(&self, key: &str, value: HSTRING) {
        // HSTRING⇄String 橋は sink 境界に閉じる（sylphya は String のみ）。投函して即返る。
        self.publisher
            .set(self.asker.clone(), key.to_string(), value.to_string());
    }

    /// sylphya の反映フェンスへ委譲する（テスト用・design.md §bin（ShioriHostSink 統合）
    /// 「テスト用に `barrier()` 委譲も公開」）。
    ///
    /// 復帰時、それ以前に本 sink の `SetProperty`／`set_property_value` から投函した全変異が
    /// sylphya 鏡像へ反映済み（mpsc FIFO＋直列処理）。決定論テストは `set → barrier → get` の列で
    /// read-your-write の有界ラグ（研究 §12-4）をレースなく畳む。アクター死亡時は
    /// [`areka_actor::ReplyError`] を返す（送信端で死亡観測可能・永久ブロックしない・R6.7）。
    pub fn barrier(&self) -> std::result::Result<(), areka_actor::ReplyError> {
        self.publisher.barrier()
    }

    /// メールボックスへ thread-safe に投函する内部ヘルパ。
    fn enqueue(&self, msg: HostMessage) {
        self.mailbox
            .lock()
            .expect("mailbox mutex poisoned")
            .push_back(msg);
    }
}

/// App スコープのみの sylphya アクターを起動し、その reader/publisher と与えた session `AskerId` で
/// sink を構築する（sink 組立各所〔shiori_session 系〕共通の App-root-only spawn・R7.3・design.md
/// §bin（ShioriHostSink 統合）「spawn_sylphya〔App root のみ・ghost/shell root なし〕」）。
///
/// - App root は bin の [`crate::default_app_profile_dir`]（env `AREKA_PROFILE_DIR` 優先・既定は
///   実行ファイル隣接 `profile/areka/`・R8.2）を供給する。ghost/shell/balloon root は付けない
///   （bin の serving surface は App スコープのみを据える・ghost/shell スコープは ghost boot 系列が
///   `<shiori.dir>`／`<shell.dir>` から導く・design.md §boot 系列）。
/// - 本番 IO は [`areka_sylphya::persist::FsPersistIo`]、運行 sink は未配線（M1・`None`）。
///
/// 返る [`areka_actor::ActorHandle`] は **非 RAII（drop で detach・areka-actor 規約）**。アクター
/// スレッドは inbox の送信端（sink が内包する publisher）が生存する限り生き続けるため、返した
/// sink が生きている間はアクターも生存する（publisher の投函は失敗しない）。呼び出し側は join して
/// panic を観測したい場合のみ handle を保持すればよく、本経路（in-proc serving surface）は join を
/// 要さないため handle を drop（detach）してよい。
pub(crate) fn spawn_app_sylphya_sink(asker: AskerId) -> (ShioriHostSink, areka_actor::ActorHandle) {
    let parts = areka_sylphya::spawn_sylphya(areka_sylphya::SylphyaInit {
        roots: areka_sylphya::ScopeRoots {
            app: Some(crate::default_app_profile_dir()),
            ghost: None,
            shell: None,
            balloon: None,
        },
        io: Box::new(areka_sylphya::persist::FsPersistIo),
        runtime_sink: None,
    });
    let sink = ShioriHostSink::with_sylphya(parts.reader, parts.publisher, asker);
    (sink, parts.handle)
}

// windows-core 0.62: `#[implement]` 生成の `*_Impl` 型に対し pub vtable メソッドを実装する。
// 型付き引数（`&HSTRING` 借用／`&mut HSTRING` move-out）により本体から raw ポインタ操作を排する。
// `[in]` HSTRING（`&HSTRING`）は借用——呼び出し中のみ参照可で解放せず、投函/保持する内容は
// clone して所有する（要件 10.4/12.4・interface.rs §HSTRING 所有権規約）。
//
// 再入規約（要件 10.3/7.2）: `GetProperty` は reader の逐次同期解決（鏡像スナップショットの read
// lock 内 `Arc` clone のみ・他スレッド/他アクターへの pull 照会なし）で完結し、`SetProperty` は
// publisher への投函のみで返るため、`Get`/`Notify` 呼出中の同一スレッド呼び戻しでもデッドロックしない。
impl shiori_abi::interface::IShioriHost_Impl for ShioriHostSink_Impl {
    unsafe fn Raise(&self, script: &HSTRING) -> Result<()> {
        // `[in]` 借用: 保持するため clone して所有する（要件 10.4）。
        self.enqueue(HostMessage::Raised(script.clone()));
        Ok(())
    }

    unsafe fn Complete(&self, token: u64, response: &HSTRING) -> Result<()> {
        let token = CorrelationToken(token);
        // 突合枠と照合（単一 in-flight・議題3）。一致時のみ受理してクリアし、
        // stale/重複 `Complete` を以降トークン不一致で弾けるようにする。
        {
            let mut pending = self.pending.lock().expect("pending mutex poisoned");
            match *pending {
                Some(expected) if expected == token => {
                    *pending = None;
                }
                // 突合枠が空 or トークン不一致（未知/stale）= 突合不能（議題3）。
                _ => return Err(SHIORI_E_UNKNOWN_TOKEN.into()),
            }
        }
        // `[in]` 借用: 保持するため clone して所有する（要件 10.4）。
        self.enqueue(HostMessage::Completed {
            token,
            response: response.clone(),
        });
        Ok(())
    }

    unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> Result<()> {
        // sylphya reader の逐次同期解決で同期即答する（mailbox 投函で代替しない・要件 10.3/7.2）。
        // 解決は鏡像スナップショットの read lock 内 `Arc` clone のみで完結し、`Get`/`Notify` 呼出中の
        // 同一スレッド呼び戻しでもデッドロックしない（再入規約）。
        // HSTRING⇄String 橋は本 sink 境界に閉じる（key: HSTRING→String〔UTF-16→UTF-8〕）。
        let key = key.to_string();
        let ctx = AskerContext {
            asker: self.asker.clone(),
        };
        match self.reader.resolve_dotted_str(&ctx, &key) {
            // 実在時: 値を out へ move-out（callee 確保・caller 解放・String→HSTRING）。
            DottedResolution::Value(v) => {
                *out_value = HSTRING::from(v);
                Ok(())
            }
            // 欠落 key は PROPERTY_NOT_FOUND（out_value 未書込・暗黙の空値で続行しない・R7.2）。
            DottedResolution::NotFound => Err(SHIORI_E_PROPERTY_NOT_FOUND.into()),
        }
    }

    unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> Result<()> {
        // sylphya publisher へ投函して即 `Ok(())`（裁定 §10.4・要件 10.1/7.2）。分類・中継・host 区画
        // 書込はアクターの領分。`[in]` 借用ゆえ key/value を String 化して所有を移す（HSTRING⇄String
        // 橋は本 sink 境界に閉じる）。read-your-write は有界ラグ（研究 §12-4）。
        self.publisher
            .set(self.asker.clone(), key.to_string(), value.to_string());
        Ok(())
    }
}

/// テスト専用: 偽 IO（[`areka_sylphya::persist::FakePersistIo`]・実 FS/実時計/実 SHIORI なし）で
/// sylphya アクターを起動し、既知 asker で sink を組む共通ヘルパ（Task 9.3・design.md §bin
/// （ShioriHostSink 統合）Validation）。
///
/// 本 crate の各 `#[cfg(test)]` モジュール（`shiori_host`／`reference_brain`／`shiori_*_e2e_tests`）が
/// `ShioriHostSink` を hermetic かつ決定論的に構築するために共有する（実 profile ディレクトリに触れ
/// ない偽境界）。返り値は `sink` に加え、同一 asker/同一 inbox の [`SylphyaPublisher`]・[`AskerId`]・
/// join ハンドルも渡す——別スレッド set＋`barrier()` の檻（`sink` 実体は `!Sync` のため共有せず、
/// `Clone + Send` の publisher を跨がせる）や `set → barrier → get` の決定論列を組める。
///
/// アクタースレッドは `sink` が内包する publisher が生存する限り生き続けるため、返した
/// `handle`（非 RAII・drop で detach）は保持不要なら drop してよい（`spawn_app_sylphya_sink` と同規約）。
#[cfg(test)]
pub(crate) struct TestSylphyaSink {
    /// sylphya 委譲済みの被試験 sink。
    pub sink: ShioriHostSink,
    /// `sink` と同一 inbox を指す送信端（別スレッド set の檻用・`Clone + Send`）。
    pub publisher: SylphyaPublisher,
    /// `sink` が読み書きに用いるのと同一の問い合わせ元 ID（per-asker 着地先の一致を保証）。
    pub asker: AskerId,
    /// アクタースレッドの join ハンドル（非 RAII・保持不要なら drop で detach）。
    ///
    /// アクター生存は `sink` 内包 publisher が担保するため、本 handle を読む必要はない
    /// （join して panic を観測したいテストのみ保持すればよい）。
    #[allow(dead_code)]
    pub handle: areka_actor::ActorHandle,
}

/// [`TestSylphyaSink`] を組む（偽 IO・全 root `None`＝永続不要の寛容縮退・既知 asker）。
#[cfg(test)]
pub(crate) fn spawn_test_sylphya_sink() -> TestSylphyaSink {
    let asker = AskerId::new("test-asker");
    let parts = areka_sylphya::spawn_sylphya(areka_sylphya::SylphyaInit {
        // 全 root None: 永続は本テスト群の関心外——起動時ロードは空へ寛容縮退する（実 FS 非依存）。
        roots: areka_sylphya::ScopeRoots::default(),
        io: Box::new(areka_sylphya::persist::FakePersistIo::new()),
        runtime_sink: None,
    });
    // publisher は Clone: sink へ渡す 1 本と、別スレッド set の檻へ渡す 1 本を分ける。
    let publisher = parts.publisher.clone();
    let sink = ShioriHostSink::with_sylphya(parts.reader, parts.publisher, asker.clone());
    TestSylphyaSink {
        sink,
        publisher,
        asker,
        handle: parts.handle,
    }
}

#[cfg(test)]
mod tests {
    //! areka 側 `IShioriHost` 実装の結合テスト（要件 10.2/10.3/10.4/12.4・design.md §System Flows）。
    //!
    //! host を立て、**COM ポインタ（`IShioriHost`）経由の安全面メソッド**（`raise`/`complete`/
    //! `get_property`/`set_property`）で駆動する。pub メソッド化により vtable 直呼びヘルパは不要。
    //!
    //! 観測:
    //! - (a) トークンをセット→`complete(token, resp)` でメールボックスに `Completed` が入り Ok。
    //! - (b) 未知トークンの `complete` は `UnknownToken` を返し投函しない。
    //! - (c) `raise(script)` でメールボックスに `Raised` が入り Ok。
    //! - (d) `set_property`→`barrier`→`get_property` の決定論列で set 値が可視・欠落 key→`PropertyNotFound`・
    //!   別スレッド set＋`barrier`（sink 実体は `!Sync` ゆえ Clone+Send の publisher を跨がせる）。
    //! - (e) `Get` 実装内からの `get_property` 再入同期応答（デッドロックしない・要件 10.3）。
    //!
    //! sylphya 委譲後、`SetProperty`／`set_property_value` は publisher 投函で即返るため read-your-write は
    //! 有界ラグ（研究 §12-4）。決定論テストは `set → barrier() → get` の列で反映をレースなく畳む
    //! （`thread::sleep` を用いない・design.md §Get/Set 図）。sink 構築は hermetic な
    //! [`super::spawn_test_sylphya_sink`]（偽 IO・既知 asker）を共有する。

    use super::*;
    use shiori_abi::error::ShioriError;
    use shiori_abi::interface::IShioriHost;
    use windows_core::AsImpl;

    /// (a) 突合枠にトークンをセット → 一致 `complete` でメールボックスに応答が入り Ok（要件 10.1）。
    #[test]
    fn complete_with_matching_token_enqueues_response_and_returns_ok() {
        let sink = spawn_test_sylphya_sink().sink;
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("deferred-response-body");
        host.complete(CorrelationToken(7), &response)
            .expect("一致トークンの complete は Ok");

        // COM ポインタが包む実体を取り出して観測する。
        let observed = host_inner(&host);
        assert_eq!(
            observed.try_recv(),
            Some(HostMessage::Completed {
                token: CorrelationToken(7),
                response: HSTRING::from("deferred-response-body"),
            }),
            "メールボックスに完了応答が投函されていること"
        );
        assert_eq!(
            observed.pending_token(),
            None,
            "突合成功で突合枠がクリアされること（stale/重複防御）"
        );
    }

    /// (b) 未知/stale トークンの `complete` は `UnknownToken` を返し投函しないこと（議題3）。
    #[test]
    fn complete_with_unknown_token_returns_unknown_token_error() {
        let sink = spawn_test_sylphya_sink().sink;
        sink.set_pending_token(CorrelationToken(7));
        let host: IShioriHost = sink.into();

        // 突合枠(7)と不一致のトークン(999)。
        let response = HSTRING::from("stale");
        let err = host
            .complete(CorrelationToken(999), &response)
            .expect_err("未知トークンの complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "未知トークンの complete は UnknownToken を返すこと, got {err:?}"
        );

        let observed = host_inner(&host);
        assert_eq!(observed.mailbox_len(), 0, "未知トークンでは投函しないこと");
        assert_eq!(
            observed.pending_token(),
            Some(CorrelationToken(7)),
            "突合枠は不一致では消費されないこと"
        );
    }

    /// 突合枠が空のときの `complete` も `UnknownToken`（議題3・空枠）。
    #[test]
    fn complete_with_empty_slot_returns_unknown_token_error() {
        let sink = spawn_test_sylphya_sink().sink;
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("orphan");
        let err = host
            .complete(CorrelationToken(1), &response)
            .expect_err("空枠の complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "空の突合枠での complete は UnknownToken を返すこと, got {err:?}"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 0);
    }

    /// (c) `raise(script)` でメールボックスに通知が入り Ok（要件 10.1/10.4）。
    #[test]
    fn raise_enqueues_script_and_returns_ok() {
        let sink = spawn_test_sylphya_sink().sink;
        let host: IShioriHost = sink.into();
        let script = HSTRING::from("\\h\\s[0]hello");
        host.raise(&script).expect("raise は Ok");

        let observed = host_inner(&host);
        assert_eq!(
            observed.try_recv(),
            Some(HostMessage::Raised(HSTRING::from("\\h\\s[0]hello"))),
            "メールボックスに能動通知が投函されていること"
        );
    }

    /// `complete` の重複呼び出し: 1 回目で枠をクリアするため 2 回目は弾かれること（stale 防御・議題3）。
    #[test]
    fn duplicate_complete_is_rejected_after_first_consumes_slot() {
        let sink = spawn_test_sylphya_sink().sink;
        sink.set_pending_token(CorrelationToken(42));
        let host: IShioriHost = sink.into();
        let response = HSTRING::from("once");

        host.complete(CorrelationToken(42), &response)
            .expect("初回 complete は Ok");
        let err = host
            .complete(CorrelationToken(42), &response)
            .expect_err("枠消費後の重複 complete は Err");
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "枠消費後の重複 complete は UnknownToken を返すこと, got {err:?}"
        );
        assert_eq!(host_inner(&host).mailbox_len(), 1, "投函は初回の 1 件のみ");
    }

    /// (d) `set_property`→`barrier`→`get_property` の決定論列で set 値が可視・欠落 key→`PropertyNotFound`
    /// （要件 10.2/10.3/7.2）。
    ///
    /// sylphya 委譲後、`SetProperty` は publisher 投函で即返り、`GetProperty` は reader の同期即答。
    /// read-your-write は有界ラグ（研究 §12-4）ゆえ、`barrier()`（sylphya 反映フェンス）で set の反映を
    /// 待ってから get する決定論列で畳む（`thread::sleep` を用いない・design.md §Get/Set 図）。
    /// `barrier` 復帰後の `GetProperty` は同期即答（mailbox 投函で代替しない・要件 10.3）。
    /// key は自由 dotted（[`areka_sylphya::classify_set`] StoreWrite）で per-asker host 区画へ着地する。
    #[test]
    fn set_then_barrier_then_get_property_is_visible_and_missing_key_errors() {
        let t = spawn_test_sylphya_sink();
        let host: IShioriHost = t.sink.into();

        // 自由 dotted key（葉 `key` は汎用プロパティ名でなく・根 `path` は点付きルートでない → StoreWrite）。
        let key = HSTRING::from("path.to.key");
        let value = HSTRING::from("some-value");
        host.set_property(&key, &value).expect("set_property は Ok");

        // read-your-write の有界ラグを barrier で畳む（決定論・sleep なし）。
        host_inner(&host)
            .barrier()
            .expect("barrier は Ok（アクター生存）");

        // barrier 復帰後は set した値が同期即答で move-out される。
        let got = host.get_property(&key).expect("get_property は Ok");
        assert_eq!(
            got, value,
            "set→barrier 後の値が同期即答で move-out されること"
        );

        // 欠落 key は PropertyNotFound（暗黙の空値で続行しない・barrier 後も未 set は不在）。
        let err = host
            .get_property(&HSTRING::from("no.such.key"))
            .expect_err("欠落 key の get_property は Err");
        assert!(
            matches!(err, ShioriError::PropertyNotFound),
            "欠落 key は PropertyNotFound を返すこと, got {err:?}"
        );
    }

    /// areka 側充填 API `set_property_value`（`AsImpl` 経由）で充填した値が `barrier` 後に同期即答される
    /// こと（要件 10.5/7.2）。`set_property_value` は publisher 投函の薄いラッパゆえ read-your-write は
    /// 有界ラグ——`barrier()` で反映を待つ決定論列で畳む（sleep なし）。
    #[test]
    fn set_property_value_fills_store_observable_via_get_property() {
        let t = spawn_test_sylphya_sink();
        let host: IShioriHost = t.sink.into();
        // areka 側の充填口（AsImpl 経由）で自由 dotted key（StoreWrite）を埋める。
        host_inner(&host).set_property_value("myghost.customflag", HSTRING::from("reference"));

        // 投函の反映を barrier で待ってから読む（決定論・sleep なし）。
        host_inner(&host)
            .barrier()
            .expect("barrier は Ok（アクター生存）");

        let got = host
            .get_property(&HSTRING::from("myghost.customflag"))
            .expect("充填済み key の get_property は Ok");
        assert_eq!(
            got,
            HSTRING::from("reference"),
            "充填値が barrier 後に同期即答されること"
        );
    }

    /// (e) 別スレッドからの set＋`barrier` が sink 経由の `get_property` で可視になること（別スレッド
    /// set の檻・要件 7.2）。
    ///
    /// sylphya 委譲後、`ShioriHostSink` は `SylphyaPublisher`（内包 `mpsc::Sender`）を保持するため
    /// `Send` だが `!Sync`——旧テストの `Arc<Sink>` 跨ぎ（`Sync` 要）は成立しない。「別スレッド set」の
    /// 意図は、`Clone + Send` の publisher（sink と同一 inbox・同一 asker）を別スレッドへ跨がせて
    /// `set` させることで保つ。join で set の投函完了を確定し（happens-before）、`barrier()` で反映を
    /// 待ってから（決定論・sleep なし）sink 経由で読む。全供給者が同一 inbox を FIFO 共有するため、
    /// join 後に投函する barrier は別スレッド set の後段に必ず並び、反映を漏れなく畳む。
    #[test]
    fn cross_thread_set_then_barrier_is_visible_via_get_property() {
        let t = spawn_test_sylphya_sink();
        // 別スレッドへ跨がせるのは Clone+Send の publisher と asker（sink 実体は !Sync ゆえ跨がせない）。
        let publisher = t.publisher.clone();
        let asker = t.asker.clone();

        let handle = std::thread::spawn(move || {
            // 別スレッドから同一 inbox・同一 asker へ set（任意スレッド可・投函のみ）。
            publisher.set(asker, "threaded.key".to_string(), "from-thread".to_string());
        });
        handle.join().expect("スレッド join");

        // barrier で別スレッド set の反映を待つ（決定論・sleep なし）。
        t.sink.barrier().expect("barrier は Ok（アクター生存）");

        // 同スレッドから COM 面 `get_property` で読めること（別スレッド set の反映）。
        let host: IShioriHost = t.sink.into();
        let got = host
            .get_property(&HSTRING::from("threaded.key"))
            .expect("別スレッド set の値を get できること");
        assert_eq!(
            got,
            HSTRING::from("from-thread"),
            "別スレッド set＋barrier が反映されること"
        );
    }

    /// `Get` 実装内から `get_property` を同一スレッドで呼び戻してもデッドロックしないこと（再入規約・要件 10.3）。
    ///
    /// mock 脳の `Get` 実装が host の `get_property` を呼び戻す。areka はプロパティストアの
    /// ロックを `Get` 呼出を跨いで保持しないため、同一スレッド呼び戻しでデッドロックしない。
    #[test]
    fn get_property_reentrant_call_from_brain_get_does_not_deadlock() {
        use shiori_abi::interface::{IShiori, IShiori_Impl};
        use windows_core::{HRESULT, Result as ComResult, implement};

        /// `Get` 内で保持 host の `get_property` を呼び戻す再入 mock 脳。
        #[allow(non_snake_case)]
        #[implement(IShiori)]
        struct ReentrantBrain {
            host: IShioriHost,
        }

        impl IShiori_Impl for ReentrantBrain_Impl {
            unsafe fn Get(
                &self,
                _input: &HSTRING,
                out_response: &mut HSTRING,
                _out_token: &mut u64,
            ) -> HRESULT {
                // Get 処理中に同一スレッドで get_property を呼び戻す（再入）。
                let v = self
                    .host
                    .get_property(&HSTRING::from("reentrant.key"))
                    .expect("再入 get_property は Ok（デッドロックしない）");
                *out_response = v;
                HRESULT(0) // S_OK
            }

            unsafe fn Notify(&self, _input: &HSTRING) -> ComResult<()> {
                Ok(())
            }
        }

        let sink = spawn_test_sylphya_sink().sink;
        let host: IShioriHost = sink.into();
        host_inner(&host).set_property_value("reentrant.key", HSTRING::from("reentrant-value"));
        // 充填の反映を barrier で待ってから再入経路を起動する（決定論・sleep なし）。
        host_inner(&host)
            .barrier()
            .expect("barrier は Ok（アクター生存）");

        let brain: IShiori = ReentrantBrain { host: host.clone() }.into();

        // brain.get が内部で host.get_property を呼び戻す。デッドロックせず値を得られること。
        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("再入する get は Ok");
        assert_eq!(
            outcome,
            shiori_abi::outcome::GetOutcome::Immediate(HSTRING::from("reentrant-value")),
            "Get 内から呼び戻した get_property の値が即時応答として返ること（再入・非デッドロック）"
        );
    }

    /// COM ポインタ `IShioriHost` が包む `ShioriHostSink` 実体への参照を取り出す。
    ///
    /// windows-core 0.62 の `#[implement]` は `AsImpl<ShioriHostSink>` を生成するため、
    /// `host.as_impl()` で実体参照を借りられる（テスト専用の観測）。
    fn host_inner(host: &IShioriHost) -> &ShioriHostSink {
        // Safety: `host` は本テストで `ShioriHostSink` から構築した COM ポインタであり、
        // 実装実体が `ShioriHostSink` であることが保証される。
        unsafe { AsImpl::<ShioriHostSink>::as_impl(host) }
    }
}

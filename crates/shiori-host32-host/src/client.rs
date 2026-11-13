//! `Shiori3Client`（要件 4.1〜4.8 / 5.1〜5.4・design.md §Shiori3Client）: host-32 SHIORI
//! 出口 API。「イベント（ID＋References）→ Value」の単一 request 経路を提供し、codec /
//! wire / HGLOBAL の内部型を一切露出しない。
//!
//! # 責務
//! - [`Shiori3Client::get`] — 応答を要するイベント。前半（`encode_and_note`）→ `send_request`
//!   → 後半（`parse_and_note`）を結線し、200 は `Some(Value)`／204 は `None`／400・500・
//!   `ErrorLevel` は `Err(RequestError::Shiori)` を返す（要件 4.1/4.3/4.7）。
//! - [`Shiori3Client::notify`] — 片道イベント。前半だけを通して**同期 `request()` 往復**で
//!   送出し、返却 response（例 204）を破棄して `Ok(())` を返す（片道 IPC 化しない・要件 4.8）。
//!
//! # 文字コードの交渉（要件 3.6/5.1/5.3）
//! 交渉の判断は 1 往復の前半 `encode_and_note`（現在の文字コードで符号化し置換を記録）と
//! 後半 `parse_and_note`（方針で解析しヘッダと置換有無を記録）の 2 つに閉じており、規則
//! そのものは [`CharsetNegotiator`] が持つ。前半は GET／NOTIFY の双方が、後半は **GET だけ**が
//! 通る（片道イベントの応答からは採用しない・要件 5.3）。交渉状態は本 client が持たず借りる
//! ——本 client は 1 往復ごとに作り捨てられるためで、状態は接続の値が持ち続ける（要件 5.1）。
//! 符号化したバイト列は helper・IPC へそのまま渡す（解釈を挟まない・要件 3.6）。
//!
//! # GET/NOTIFY の合流（要件 4.7）
//! GET と NOTIFY は host32 wire 上で単一 request 経路（同一 codec build ＋ 同一
//! `send_request(MsgTag::Request, ..)`）へ合流する。相違は **request line**（`GET` /
//! `NOTIFY SHIORI/3.0`＝`Method`）と、**応答 `Value` を呼び手へ返すか否か**のみ。wire tag は
//! 両者とも [`MsgTag::Request`] であり、NOTIFY 専用の wire tag は設けない。
//!
//! # スレッド前提（要件 4.4/4.5）
//! 本 client は [`ParentMessageWindow`] を `&'a` で借用する。`ParentMessageWindow`（内部に
//! `windows` の窓ハンドルを持つ）は `!Send` ゆえ client 自身も `!Send` だが、これは意図通りで
//! ある: client は窓を所有する専用スレッド上で駆動する。一方、出口 API の**引数・戻り値**は
//! すべて `Send` な所有データ（`&str`／`&[String]` in、`Option<String>`／`RequestError` out）で
//! あり（要件 4.5）、下流 `kanade` が channel で結果を別スレッドへ渡せる。同期ブロッキングは
//! 許容される（`SMTO_ABORTIFHUNG` ＋ 実効 timeout で有限復帰・要件 4.4）。
//!
//! # 型シーム（実装しない・要件 4.6）
//! IShiori（`IShiori::Get(HSTRING)→HSTRING`）への写像点は doc ＋ コメント署名で示すに留め、
//! 本仕様では実装しない。`SHIORI_S_PENDING`（遅延応答）も塞がず型シームとする。HSTRING⇄バイト
//! 変換はプロセス境界を跨がない（HGLOBAL=32bit ローカル・HSTRING=x64 ローカル・各プロセスが
//! 自前通貨を持つ）。詳細は [`Shiori3Client`] の型シーム節を参照。
//!
//! # 失敗経路のログ規律（steering: areka-log-first-no-silent-failure）
//! 失敗は握り潰さず必ず `Err(RequestError::..)` として surface する。transport／parse の
//! 失敗で `unwrap()`／`panic!` しない。SHIORI エラー応答（400/500/ErrorLevel）も panic せず
//! `Err(RequestError::Shiori(..))` を返す。

use std::time::Duration;

use shiori_host32_ipc::MsgTag;

use crate::charset::CharsetNegotiator;
use crate::error::{RequestError, ShioriError};
use crate::parent_window::{ParentMessageWindow, SendError};
use crate::process_host::request_timeout_from_env;
use crate::request_headers::{RequestHeaders, ValueReply, request_of};
use crate::shiori2::{self, ShioriVersion, VersionVerdict};
use crate::shiori3::{Method, ParsedResponse, ShioriRequest, build_request, parse_response};

/// 送出ヘッダ `Sender` の既定値（design.md §送出ヘッダ最小集合）。
///
/// 単一差替点: areka は SSP を詐称せず、自身を `"areka"` として正直に名乗る。値の差替は
/// [`Shiori3Client::with_sender`] のみで行い、それ以外の箇所にハードコードした `"SSP"` 等を
/// 撒かない。
const DEFAULT_SENDER: &str = "areka";

/// host-32 SHIORI 出口 API（downstream `kanade` が消費・要件 4.1〜4.8）。
///
/// 「イベント→Value」の単一 request 出口 API であり、codec / wire / HGLOBAL の内部型を露出
/// しない（要件 4.5）。GET＝応答待ち・NOTIFY＝投げきり（同期往復・応答破棄）。
///
/// # 借用と `!Send`
/// `&'a ParentMessageWindow` を借用する。窓は `!Send` ゆえ client も `!Send`（専用スレッド駆動
/// 前提・要件 4.4）。ただし `get`／`notify` の引数・戻り値はすべて `Send`（要件 4.5）。
///
/// # 型シーム（実装しない・要件 4.6）
/// IShiori への写像点は下記コメント署名で示すのみ（実装しない）:
/// ```text
/// // --- 型シーム（実装しない・要件 4.6）: IShiori::Get への写像点を型で示す ---
/// // fn onto_ishiori_get(input: &HSTRING) -> Result<GetOutcome, HRESULT>;  // seam only
/// //   - HSTRING⇄バイト変換はプロセス境界を跨がない（HGLOBAL=32bit ローカル・HSTRING=x64 ローカル）。
/// //   - SHIORI_S_PENDING（遅延応答）は塞がず型シームに留める。
/// ```
pub struct Shiori3Client<'a> {
    /// HELLO ハンドシェイク済みの親メッセージ窓（送信経路）。窓所有スレッド上で借用する。
    window: &'a ParentMessageWindow,
    /// `Sender` ヘッダ値（単一差替点・既定 [`DEFAULT_SENDER`]＝`"areka"`）。
    sender: &'a str,
    /// 文字コードの交渉状態（接続が所有・1 往復ごとに可変借用する・要件 5.1）。
    ///
    /// 本 client は 1 往復ごとに作り捨てられるため、交渉状態を**持たず借りる**。採用結果が
    /// イベントをまたいで保たれるのは、接続の値（`areka-kanade` の `ShioriConnection`）が
    /// フィールドとして 1 つだけ持ち続けるからである。
    negotiator: &'a mut CharsetNegotiator,
    /// 相手の SHIORI の版（接続が持つ値の写し・既定は 3.0）。
    version: ShioriVersion,
    /// この往復が資源の問い合わせか（2.x では `GET String` で送る・3.0 では何も変えない）。
    is_resource: bool,
}

impl<'a> Shiori3Client<'a> {
    /// `Sender` を既定（`"areka"`）とする client を構築する（要件 4.x・5.1）。
    ///
    /// 文字コードは `negotiator` が決める（本 client は規則を持たない）。`window` は
    /// ハンドシェイク完了済みであることを前提とする（未準備ガードは設けない・design.md
    /// §Preconditions: Load は Request に構造的に先立つ）。
    #[must_use]
    pub fn new(window: &'a ParentMessageWindow, negotiator: &'a mut CharsetNegotiator) -> Self {
        Self::with_sender(window, negotiator, DEFAULT_SENDER)
    }

    /// `Sender` を明示指定して client を構築する（単一差替点・design.md §送出ヘッダ最小集合）。
    ///
    /// 通常は [`Shiori3Client::new`]（`"areka"` 既定）で足りる。互換ベースウェアの名乗りを
    /// 差し替えたい場合のみ本関数を使う。
    #[must_use]
    pub fn with_sender(
        window: &'a ParentMessageWindow,
        negotiator: &'a mut CharsetNegotiator,
        sender: &'a str,
    ) -> Self {
        Self {
            window,
            sender,
            negotiator,
            version: ShioriVersion::Shiori3,
            is_resource: false,
        }
    }

    /// 版を指定する（既定は [`Shiori3Client::new`]／[`Shiori3Client::with_sender`] の 3.0）。
    #[must_use]
    pub fn with_version(self, version: ShioriVersion) -> Self {
        Self { version, ..self }
    }

    /// この往復が資源の問い合わせであると印を付ける（2.x では `GET String` で送る）。
    /// 3.0 では何も変えない。既定は印なし。
    #[must_use]
    pub fn for_resource(self) -> Self {
        Self {
            is_resource: true,
            ..self
        }
    }

    /// `GET Version SHIORI/2.6` を 1 本送り、答えから版を決める（要件 2.1・2.5）。
    /// 符号化は今の文字コード。答えは文字コードの採用の根拠にしない（要件 6.1）。
    pub fn probe_version(&mut self) -> VersionVerdict {
        let req = shiori2::version_probe_request(self.negotiator.current(), self.sender);
        let timeout = self.effective_timeout();
        verdict_of_probe(self.window.send_request(MsgTag::Request, &req, timeout))
    }

    /// 応答を要するイベントを送り、Value を取り出す（要件 4.1/4.3/4.7・design.md §GET 往復）。
    ///
    /// build（`GET SHIORI/3.0`＋ID＋References＋任意 `Status`）→ `send_request(Request, bytes, 実効 timeout)` →
    /// `parse_response` を結線する。結果は [`map_get_result`] の意味論に従い:
    /// - 200（＋その他の非エラー status）→ `Ok(Some(Value))`／Value 欠落なら `Ok(None)`
    /// - 204 → `Ok(None)`（成功・スクリプト無し）
    /// - 400 / 500 / `ErrorLevel` あり → `Err(RequestError::Shiori(ShioriError::Status{..}))`
    /// - 311 / 312 等の許容 status → `Ok(value)`（drop せず区別可能・要件 2.7）
    ///
    /// `status` は `Status` ヘッダの wire 値（`None`⇒行を出さない）であり、codec へ verbatim に透過する
    /// （中身は解釈しない・語彙は kanade が所有・DD-IT-6）。
    ///
    /// # Errors
    /// - wire timeout → [`RequestError::Timeout`]（要件 5.1）
    /// - transport 送出失敗（helper 応答不能）→ [`RequestError::Ipc`]（要件 5.3）
    /// - 未ハンドシェイク → [`RequestError::Handshake`]（構造上通常起きない・要件 3.3）
    /// - malformed 応答 → [`RequestError::Shiori`]（`ShioriError::Parse`）
    /// - SHIORI エラー応答（400/500/ErrorLevel）→ [`RequestError::Shiori`]（`ShioriError::Status`・要件 5.2）
    pub fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let headers = RequestHeaders::internal(self.sender);
        Ok(self
            .get_with_headers(id, references, status, &headers)?
            .map(|reply| reply.value))
    }

    /// [`Shiori3Client::get`] の追加のヘッダつき版。値と、応答の `SecurityLevel` の生の値を返す
    /// （areka-P0-script-security-level 要件 2.1・3.5）。結果の判定と誤りは `get` と同じ。
    ///
    /// # Errors
    /// [`Shiori3Client::get`] と同じ。
    pub fn get_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        let timeout = self.effective_timeout();
        // 現在の文字コードを先に取り出す（交渉状態の可変借用と読みを同時に持たない）。
        let charset = self.negotiator.current();
        let req = request_of(Method::Get, id, references, status, *headers, charset);
        // 版で分かれるのは符号化と読みの 2 か所だけ。2.x で送らない要求は送らずに値なしで返す。
        let bytes = match self.version {
            ShioriVersion::Shiori3 => encode_and_note(self.negotiator, &req),
            ShioriVersion::Shiori2 => {
                match encode2_and_note(self.negotiator, &req, self.is_resource) {
                    Some(bytes) => bytes,
                    None => return Ok(None),
                }
            }
        };
        // GET/NOTIFY は wire tag = MsgTag::Request で合流（要件 4.7）。符号化したバイト列は
        // helper・IPC へ**そのまま**渡す（解釈を挟まない・要件 3.6）。
        let resp = self
            .window
            .send_request(MsgTag::Request, &bytes, timeout)
            .map_err(map_send_error)?;
        // parse の malformed（ShioriError::Parse）は #[from] で RequestError::Shiori へ持ち上がる。
        // 採用と記録は `map_get_result` より前（応答が 204／400／500 でも `Charset` ヘッダは
        // 採用対象・design.md §交渉の一周）。
        let parsed = match self.version {
            ShioriVersion::Shiori3 => parse_and_note(self.negotiator, &resp)?,
            ShioriVersion::Shiori2 => {
                parse2_and_note(self.negotiator, &req, &resp, self.is_resource)?
            }
        };
        map_get_reply(parsed)
    }

    /// 片道イベントを送る（同期 `request()` 往復・応答破棄・要件 4.2/4.8・design.md §GET 往復）。
    ///
    /// NOTIFY も GET と同一の `send_request(Request, ..)` 同期往復で送出する（**片道 IPC 化しない**・
    /// 要件 4.8）。DLL は NOTIFY でも常に response（例 204）を返し、その応答 HGLOBAL の caller-free は
    /// helper 側で処理済ゆえ、本 API は bytes を受け取って**破棄**するだけでよい（解析も surface も
    /// しない）。片道化すると応答 HGLOBAL の解放漏れを招くため、GET と同一契約に統一する。
    ///
    /// `status` は GET と同じく `Status` ヘッダの wire 値を codec へ verbatim に透過する（中身は解釈しない・
    /// `None`⇒行を出さない・DD-IT-6）。
    ///
    /// # Errors
    /// transport 失敗の写像は [`Shiori3Client::get`] と同一（[`map_send_error`] 経由）。SHIORI 応答の
    /// status は破棄するため、NOTIFY はエラー status を `Err` にしない（応答を surface しない・要件 4.8）。
    pub fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        let headers = RequestHeaders::internal(self.sender);
        self.notify_with_headers(id, references, status, &headers)
    }

    /// [`Shiori3Client::notify`] の追加のヘッダつき版（areka-P0-script-security-level 要件 2.1）。
    /// 応答は `notify` と同じく破棄する。
    ///
    /// # Errors
    /// [`Shiori3Client::notify`] と同じ。
    pub fn notify_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<(), RequestError> {
        let timeout = self.effective_timeout();
        // 現在の文字コードを先に取り出す（交渉状態の可変借用と読みを同時に持たない）。
        let charset = self.negotiator.current();
        let req = request_of(Method::Notify, id, references, status, *headers, charset);
        // 版で分かれるのは符号化だけ。2.x で送らないイベントは送らずに成功で返す。
        let bytes = match self.version {
            ShioriVersion::Shiori3 => encode_and_note(self.negotiator, &req),
            ShioriVersion::Shiori2 => {
                match encode2_and_note(self.negotiator, &req, self.is_resource) {
                    Some(bytes) => bytes,
                    None => return Ok(()),
                }
            }
        };
        // 同期往復（要件 4.8）: wire tag は GET と同じ MsgTag::Request（要件 4.7）。
        let _discarded = self
            .window
            .send_request(MsgTag::Request, &bytes, timeout)
            .map_err(map_send_error)?;
        // 返却 response（例 204）は破棄する（要件 4.8・5.3）。解析も採用も surface もしない
        // ——片道イベントの応答は採用の根拠に用いないため、1 往復の後半をここでは通らない。
        Ok(())
    }

    /// env シーム（`AREKA_SHIORI_REQUEST_TIMEOUT_MS`）から実効 timeout を解決する（要件 4.3/5.1）。
    ///
    /// [`request_timeout_from_env`] の `Option<Duration>` を `send_request` の `Duration` 引数へ写す:
    /// - env 不在 → `Some(REQUEST_TIMEOUT)`＝既定 60s（有限 timeout）。
    /// - env `"0"` → `None`＝**無限待ち**（デバッグ opt-in）を [`Duration::MAX`] に写す。凍結 ipc の
    ///   `timeout_millis` が `Duration::MAX` を `u32::MAX` ミリ秒（≈49.7 日 ≈ 実質無限）へクランプ
    ///   するため、無限待ちが有限復帰機構を壊さずに表現できる。凍結 ipc の timeout 機構自体は変更しない。
    fn effective_timeout(&self) -> Duration {
        match request_timeout_from_env() {
            Some(d) => d,
            None => Duration::MAX,
        }
    }

    // --- 型シーム（実装しない・要件 4.6）: IShiori::Get への写像点を型で示す ---
    // fn onto_ishiori_get(input: &HSTRING) -> Result<GetOutcome, HRESULT>;  // seam only
    //   - `IShiori::Get(HSTRING)→HSTRING` を本 client の get/notify へ橋渡しする写像点。
    //   - HSTRING⇄バイト変換はプロセス境界を跨がない（HGLOBAL=32bit ローカル・HSTRING=x64 ローカル・
    //     各プロセスが自前通貨を持ち、跨ぐのは生バイト列のみ）。
    //   - SHIORI_S_PENDING（遅延応答）は塞がず型シームに留める（本仕様では実装しない）。
}

/// 1 往復の**前半**: 現在の文字コードで要求を符号化し、置換を記録する（要件 3.1/3.5/5.1）。
///
/// GET／NOTIFY の双方が通る（同じ交渉状態＝同じ文字コードで送る・要件 5.1）。戻り値は
/// そのまま helper・IPC へ渡すバイト列で、ここから先はバイト列に触れない（要件 3.6）。
fn encode_and_note(negotiator: &mut CharsetNegotiator, req: &ShioriRequest<'_>) -> Vec<u8> {
    let encoded = build_request(req);
    negotiator.note_request(req.id, encoded.replaced);
    encoded.bytes
}

/// 1 往復の**後半**: 応答を交渉の方針で解析し、応答ヘッダと置換の有無を記録する（要件 4.2〜4.7）。
///
/// **GET だけが呼ぶ**（片道イベントの応答からは採用しない・要件 5.3）。記録は `map_get_result`
/// より前に行う——応答の status が 204／400／500 であっても `Charset` ヘッダは採用対象である
/// （design.md §交渉の一周）。`note_response` が記録する文字コード名は「採用後の現在値」であり、
/// `parse_response` が実際に復号に使った文字コードと一致する（`Negotiate` では解決できた
/// ヘッダの値、`Force` では強制中の値がそれぞれ両者に共通するため）。
fn parse_and_note(
    negotiator: &mut CharsetNegotiator,
    resp: &[u8],
) -> Result<ParsedResponse, ShioriError> {
    let parsed = parse_response(resp, negotiator.policy())?;
    negotiator.note_response(parsed.charset_header.as_deref(), parsed.decode_had_errors);
    Ok(parsed)
}

/// 2.x の 1 往復の**前半**: 2.x の要求を組んで置換を記録する（要件 3.5・3.6）。
///
/// `None` は「2.x の相手へは送らない」で、そのとき DEBUG を 1 行出す（要件 3.5）。
fn encode2_and_note(
    negotiator: &mut CharsetNegotiator,
    req: &ShioriRequest<'_>,
    is_resource: bool,
) -> Option<Vec<u8>> {
    let Some(encoded) = shiori2::build_request(req, is_resource) else {
        tracing::debug!(
            target: "shiori2",
            event = "shiori2_event_not_sent",
            id = req.id,
            "SHIORI/2.x の相手へは送らないイベント"
        );
        return None;
    };
    negotiator.note_request(req.id, encoded.replaced);
    Some(encoded.bytes)
}

/// 2.x の 1 往復の**後半**: 2.x の答えを読み、応答ヘッダと置換の有無を記録する（要件 4.4〜4.6）。
///
/// 3.0 の後半と同じく **GET だけが呼ぶ**。値の見出しは要求の種類と資源の印で選ぶ。
fn parse2_and_note(
    negotiator: &mut CharsetNegotiator,
    req: &ShioriRequest<'_>,
    resp: &[u8],
    is_resource: bool,
) -> Result<ParsedResponse, ShioriError> {
    let parsed =
        shiori2::parse_response(resp, negotiator.policy(), req.method, req.id, is_resource)?;
    negotiator.note_response(parsed.charset_header.as_deref(), parsed.decode_had_errors);
    Ok(parsed)
}

/// 確かめの往復の**後半**: 送受の結果から版を決める。往復の失敗は決められない（理由は失敗の表示）。
fn verdict_of_probe(result: Result<Vec<u8>, SendError>) -> VersionVerdict {
    match result {
        Ok(reply) => shiori2::decide_version(&reply),
        Err(err) => VersionVerdict::Undecided {
            reason: err.to_string(),
        },
    }
}

/// [`SendError`]（transport/handshake）を [`RequestError`]（統合語彙）へ写像する純関数（要件 5.1〜5.4）。
///
/// **load-bearing な区別**: [`shiori_host32_ipc::IpcError::Timeout`] は [`RequestError::Timeout`] へ
/// 振り分け、`RequestError::Ipc` へは**含めない**（要件 5.1/5.4）。`RequestError::Ipc` が意図的に
/// `#[from]` を持たないのはこのためで、素の `IpcError` を自動変換で一律 `Ipc` へ流すと timeout 経路が
/// 潰れる。ここで手動で振り分ける:
/// - `SendError::Handshake(h)` → `RequestError::Handshake(h)`（未ハンドシェイク・要件 3.3）
/// - `SendError::Ipc(IpcError::Timeout)` → `RequestError::Timeout`（wire timeout・要件 5.1）
/// - `SendError::Ipc(其他)`（`SendFailed`／`CorruptFrame`）→ `RequestError::Ipc`（helper 死活の一態様・要件 5.3）
///
/// 窓に依存しない純関数ゆえ単体テスト可能（本タスクの stated Observable）。
fn map_send_error(err: SendError) -> RequestError {
    match err {
        // Handshake は #[from] を持つが、写像点の網羅を明示するため明示 variant で写す。
        SendError::Handshake(h) => RequestError::Handshake(h),
        // CRITICAL（要件 5.1/5.4）: Timeout は Timeout へ。Ipc へ潰さない。
        SendError::Ipc(shiori_host32_ipc::IpcError::Timeout) => RequestError::Timeout,
        // SendFailed / CorruptFrame は transport 送出失敗として Ipc へ（要件 5.3）。
        SendError::Ipc(other) => RequestError::Ipc(other),
    }
}

/// [`ParsedResponse`] を GET の結果 `Result<Option<String>, RequestError>` へ写す純関数（要件 4.1/5.2）。
///
/// 意味論（design.md §エラー分類・順序が load-bearing）:
/// 1. **エラー条件を先に判定**: `status == 400 || status == 500 || error_level.is_some()` なら
///    [`RequestError::Shiori`]（`ShioriError::Status{..}`）。400/500/`ErrorLevel` は SHIORI エラーで
///    あり区別保持する（要件 5.2）。`error_level` を先に見るのは、200 でも `ErrorLevel` 付きなら
///    エラーとして扱うため。
/// 2. `status == 204` → `Ok(None)`（成功・スクリプト無し・要件 2.2）。
/// 3. それ以外（200・311・312・その他の許容 status）→ `Ok(parsed.value)`（200→Some(Value) or None・
///    要件 2.1／311・312 は drop せず区別可能・要件 2.7）。
///
/// 窓に依存しない純関数ゆえ単体テスト可能。
fn map_get_result(parsed: ParsedResponse) -> Result<Option<String>, RequestError> {
    // ① エラー条件を先に判定（400/500/ErrorLevel は SHIORI エラー・要件 5.2）。
    if parsed.status == 400 || parsed.status == 500 || parsed.error_level.is_some() {
        return Err(RequestError::Shiori(ShioriError::Status {
            status: parsed.status,
            error_level: parsed.error_level,
            error_description: parsed.error_description,
        }));
    }
    // ② 204 は成功・スクリプト無し（要件 2.2）。
    if parsed.status == 204 {
        return Ok(None);
    }
    // ③ 200・311・312・その他の許容 status は Value をそのまま返す（要件 2.1/2.7）。
    Ok(parsed.value)
}

/// [`map_get_result`] の値に、応答の `SecurityLevel` の生の値を添える（判定の順は変えない・要件 3.5）。
///
/// 同じプロセス内の SHIORI の道（`areka-ghost` の `shiori_inproc`）も応答の読みにこれを使い、
/// 2 つの道で答えの扱いを 1 つにする。
pub fn map_get_reply(parsed: ParsedResponse) -> Result<Option<ValueReply>, RequestError> {
    let security_level = parsed.security_level.clone();
    Ok(map_get_result(parsed)?.map(|value| ValueReply {
        value,
        security_level,
    }))
}

#[cfg(test)]
#[path = "client_shiori2_tests.rs"]
mod shiori2_tests;

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;

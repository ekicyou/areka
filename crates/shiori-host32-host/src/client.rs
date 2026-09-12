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
}

impl<'a> Shiori3Client<'a> {
    /// `Sender` を既定（`"areka"`）とする client を構築する（要件 4.x・5.1）。
    ///
    /// 文字コードは `negotiator` が決める（本 client は規則を持たない）。`window` は
    /// ハンドシェイク完了済みであることを前提とする（未準備ガードは設けない・design.md
    /// §Preconditions: Load は Request に構造的に先立つ）。
    #[must_use]
    pub fn new(window: &'a ParentMessageWindow, negotiator: &'a mut CharsetNegotiator) -> Self {
        Self {
            window,
            sender: DEFAULT_SENDER,
            negotiator,
        }
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
        }
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
        let timeout = self.effective_timeout();
        // 現在の文字コードを先に取り出す（交渉状態の可変借用と読みを同時に持たない）。
        let charset = self.negotiator.current();
        let bytes = encode_and_note(
            self.negotiator,
            &ShioriRequest {
                method: Method::Get,
                id,
                references,
                sender: self.sender,
                status,
                charset,
            },
        );
        // GET/NOTIFY は wire tag = MsgTag::Request で合流（要件 4.7）。符号化したバイト列は
        // helper・IPC へ**そのまま**渡す（解釈を挟まない・要件 3.6）。
        let resp = self
            .window
            .send_request(MsgTag::Request, &bytes, timeout)
            .map_err(map_send_error)?;
        // parse の malformed（ShioriError::Parse）は #[from] で RequestError::Shiori へ持ち上がる。
        // 採用と記録は `map_get_result` より前（応答が 204／400／500 でも `Charset` ヘッダは
        // 採用対象・design.md §交渉の一周）。
        let parsed = parse_and_note(self.negotiator, &resp)?;
        map_get_result(parsed)
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
        let timeout = self.effective_timeout();
        // 現在の文字コードを先に取り出す（交渉状態の可変借用と読みを同時に持たない）。
        let charset = self.negotiator.current();
        let bytes = encode_and_note(
            self.negotiator,
            &ShioriRequest {
                method: Method::Notify,
                id,
                references,
                sender: self.sender,
                status,
                charset,
            },
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use shiori_host32_ipc::{FramingError, IpcError};

    use crate::charset::Charset;

    use crate::error::HandshakeError;

    // --- map_send_error（THE core observable・要件 5.1〜5.4）-------------------

    /// CRITICAL（要件 5.1/5.4）: `IpcError::Timeout` は `RequestError::Timeout` へ写り、
    /// **`RequestError::Ipc` には含まれない**（timeout を単一不透明失敗へ潰さない）。
    #[test]
    fn map_send_error_timeout_routes_to_timeout_not_ipc() {
        let mapped = map_send_error(SendError::Ipc(IpcError::Timeout));
        assert!(
            matches!(mapped, RequestError::Timeout),
            "IpcError::Timeout は RequestError::Timeout へ（要件 5.1）: got {mapped:?}"
        );
        // Ipc へ流れていないことを明示（区別保持・要件 5.4）。
        assert!(
            !matches!(mapped, RequestError::Ipc(_)),
            "Timeout を Ipc へ潰してはならない（要件 5.4）: got {mapped:?}"
        );
    }

    /// `IpcError::SendFailed`（送出失敗＝helper 応答不能の一態様）は `RequestError::Ipc` へ（要件 5.3）。
    #[test]
    fn map_send_error_send_failed_routes_to_ipc() {
        let mapped = map_send_error(SendError::Ipc(IpcError::SendFailed));
        assert!(
            matches!(mapped, RequestError::Ipc(IpcError::SendFailed)),
            "SendFailed は RequestError::Ipc(SendFailed) へ（要件 5.3）: got {mapped:?}"
        );
    }

    /// `IpcError::CorruptFrame`（framing 破損）も transport 失敗として `RequestError::Ipc` へ（要件 5.3）。
    #[test]
    fn map_send_error_corrupt_frame_routes_to_ipc() {
        let mapped = map_send_error(SendError::Ipc(IpcError::CorruptFrame(
            FramingError::UnknownTag(0xFF),
        )));
        assert!(
            matches!(
                mapped,
                RequestError::Ipc(IpcError::CorruptFrame(FramingError::UnknownTag(0xFF)))
            ),
            "CorruptFrame は RequestError::Ipc へ（要件 5.3）: got {mapped:?}"
        );
    }

    /// `SendError::Handshake(Incomplete)` は `RequestError::Handshake` へ（要件 3.3・区別保持）。
    #[test]
    fn map_send_error_handshake_incomplete_routes_to_handshake() {
        let mapped = map_send_error(SendError::Handshake(HandshakeError::Incomplete));
        assert!(
            matches!(mapped, RequestError::Handshake(HandshakeError::Incomplete)),
            "Handshake(Incomplete) は RequestError::Handshake へ: got {mapped:?}"
        );
    }

    /// `SendError::Handshake(Timeout)`（ハンドシェイク timeout）も `RequestError::Handshake` へ。
    ///
    /// 注意: これは **ハンドシェイク** timeout（HandshakeError::Timeout）であり、**wire** timeout
    /// （IpcError::Timeout→RequestError::Timeout）とは別語彙である（要件 5.4 の区別保持）。
    #[test]
    fn map_send_error_handshake_timeout_routes_to_handshake_not_wire_timeout() {
        let mapped = map_send_error(SendError::Handshake(HandshakeError::Timeout));
        assert!(
            matches!(mapped, RequestError::Handshake(HandshakeError::Timeout)),
            "Handshake(Timeout) は RequestError::Handshake へ: got {mapped:?}"
        );
        // wire timeout（RequestError::Timeout）へ混同していないこと。
        assert!(
            !matches!(mapped, RequestError::Timeout),
            "ハンドシェイク timeout を wire timeout へ混同しない（要件 5.4）: got {mapped:?}"
        );
    }

    // --- map_get_result（要件 4.1/2.1/2.2/2.7/5.2）---------------------------

    /// 200 ＋ Value → `Ok(Some(v))`（要件 2.1）。
    #[test]
    fn map_get_result_200_with_value_is_some() {
        let parsed = ParsedResponse {
            status: 200,
            value: Some(r"\s[0]hi\e".to_string()),
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        // RequestError は PartialEq を持たない（error.rs は本タスクで不変）ため、Ok 側を取り出して比較する。
        let ok = map_get_result(parsed).expect("200+value は Ok");
        assert_eq!(ok, Some(r"\s[0]hi\e".to_string()));
    }

    /// 200 で Value 欠落 → `Ok(None)`（要件 2.1・Value は 200 でも欠落し得る）。
    #[test]
    fn map_get_result_200_without_value_is_none() {
        let parsed = ParsedResponse {
            status: 200,
            value: None,
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let ok = map_get_result(parsed).expect("200 (Value 欠落) は Ok");
        assert_eq!(ok, None);
    }

    /// 204 → `Ok(None)`（成功・スクリプト無し・要件 2.2）。
    #[test]
    fn map_get_result_204_is_none() {
        let parsed = ParsedResponse {
            status: 204,
            value: None,
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let ok = map_get_result(parsed).expect("204 は Ok");
        assert_eq!(ok, None);
    }

    /// 400 → `Err(RequestError::Shiori(ShioriError::Status{status:400,..}))`（要件 5.2）。
    #[test]
    fn map_get_result_400_is_shiori_status_error() {
        let parsed = ParsedResponse {
            status: 400,
            value: None,
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let err = map_get_result(parsed).expect_err("400 は SHIORI エラー");
        assert!(
            matches!(
                err,
                RequestError::Shiori(ShioriError::Status { status: 400, .. })
            ),
            "400 は Shiori(Status{{status:400}}) へ: got {err:?}"
        );
    }

    /// 500 → `Err(RequestError::Shiori(ShioriError::Status{status:500,..}))`（要件 5.2）。
    #[test]
    fn map_get_result_500_is_shiori_status_error() {
        let parsed = ParsedResponse {
            status: 500,
            value: None,
            error_level: Some("critical".to_string()),
            error_description: Some("boom".to_string()),
            charset_header: None,
            decode_had_errors: false,
        };
        let err = map_get_result(parsed).expect_err("500 は SHIORI エラー");
        assert!(
            matches!(
                err,
                RequestError::Shiori(ShioriError::Status {
                    status: 500,
                    error_level: Some(_),
                    error_description: Some(_),
                })
            ),
            "500 は Shiori(Status{{status:500, error_level:Some, error_description:Some}}) へ: got {err:?}"
        );
    }

    /// 200 だが `ErrorLevel` 付き → エラー扱い（要件 5.2・error_level 先読み）。
    #[test]
    fn map_get_result_200_with_error_level_is_error() {
        let parsed = ParsedResponse {
            status: 200,
            value: Some("ignored".to_string()),
            error_level: Some("warning".to_string()),
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let err = map_get_result(parsed).expect_err("ErrorLevel 付きはエラー");
        assert!(
            matches!(
                err,
                RequestError::Shiori(ShioriError::Status {
                    status: 200,
                    error_level: Some(_),
                    ..
                })
            ),
            "ErrorLevel 付きは status に関わらず Shiori エラー: got {err:?}"
        );
    }

    /// 311 → `Ok(value)`（OnTeach 系・drop せず区別可能・要件 2.7）。
    #[test]
    fn map_get_result_311_tolerated_returns_value() {
        let parsed = ParsedResponse {
            status: 311,
            value: Some("teach".to_string()),
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let ok = map_get_result(parsed).expect("311 は許容 status で Ok");
        assert_eq!(ok, Some("teach".to_string()));
    }

    /// 312 → `Ok(value)`（OnTeach 系・要件 2.7）。
    #[test]
    fn map_get_result_312_tolerated_returns_value() {
        let parsed = ParsedResponse {
            status: 312,
            value: None,
            error_level: None,
            error_description: None,
            charset_header: None,
            decode_had_errors: false,
        };
        let ok = map_get_result(parsed).expect("312 は許容 status で Ok");
        assert_eq!(ok, None);
    }

    // --- 構築（既定 sender・単一差替点）--------------------------------------

    /// 既定 sender が `"areka"` であることを固定する（design.md §送出ヘッダ最小集合）。
    #[test]
    fn default_sender_is_areka() {
        assert_eq!(DEFAULT_SENDER, "areka");
    }

    // --- 交渉状態の結線（タスク 3.3・要件 5.1/5.3）----------------------------
    //
    // 1 往復の前半（`encode_and_note`）と後半（`parse_and_note`）は `get`／`notify` が
    // 実際に呼ぶ本番関数そのものである。窓も 32bit 成果物も要さずに結線を検査できるのは、
    // 交渉に関わる判断がこの 2 つに閉じているため（要件 9.8）。

    /// 「あ」の EUC-JP バイト列（符号化器から導かない定数）。
    const A_EUC_JP: &[u8] = &[0xA4, 0xA2];

    /// `Charset: EUC-JP` を名乗り、Value に「あ」の EUC-JP バイト列を載せた 200 応答。
    fn euc_jp_response() -> Vec<u8> {
        let mut bytes = b"SHIORI/3.0 200 OK\r\nCharset: EUC-JP\r\nValue: ".to_vec();
        bytes.extend_from_slice(A_EUC_JP);
        bytes.extend_from_slice(b"\r\n\r\n");
        bytes
    }

    /// `Charset: EUC-JP` を名乗る、Value を持たない応答（ステータス行だけを差し替えて使う）。
    ///
    /// 200 以外の応答でも `Charset` ヘッダが採用対象であることを見るための固定物。
    fn euc_jp_response_with_status(status_line: &str) -> Vec<u8> {
        let mut bytes = status_line.as_bytes().to_vec();
        bytes.extend_from_slice(b"\r\nCharset: EUC-JP\r\n\r\n");
        bytes
    }

    /// 結線が組み立てる要求と同形の [`ShioriRequest`]（`sender` は既定・References なし）。
    fn a_request(method: Method, id: &str, charset: Charset) -> ShioriRequest<'_> {
        ShioriRequest {
            method,
            id,
            references: &[],
            sender: DEFAULT_SENDER,
            status: None,
            charset,
        }
    }

    /// 応答の `Charset` ヘッダから採用が起きるのは **後半を通ったときだけ**であり、
    /// 片道イベントが通る前半だけでは採用が起きない（要件 5.3 の零）。
    #[test]
    fn only_the_response_half_of_the_chain_adopts_a_charset() {
        let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");

        // 応答待ちのイベントが通る後半: 解析と記録で EUC-JP を採用する。
        let mut answered = CharsetNegotiator::new(Charset::UTF_8, false);
        let parsed =
            parse_and_note(&mut answered, &euc_jp_response()).expect("200 応答は解析できる");
        assert_eq!(
            parsed.value.as_deref(),
            Some("あ"),
            "宣言された文字コードで復号されていない"
        );
        assert_eq!(
            answered.current(),
            euc_jp,
            "応答の Charset ヘッダが採用されていない（要件 4.2）"
        );

        // 片道イベントが通る前半だけでは、同じ応答が来ても現在の文字コードは動かない。
        let mut one_way = CharsetNegotiator::new(Charset::UTF_8, false);
        let charset = one_way.current();
        let bytes = encode_and_note(
            &mut one_way,
            &a_request(Method::Notify, "OnSecondChange", charset),
        );
        assert!(
            bytes.starts_with(b"NOTIFY SHIORI/3.0\r\nCharset: UTF-8\r\n"),
            "片道イベントの要求が現在の文字コードで出ていない"
        );
        assert_eq!(
            one_way.current(),
            Charset::UTF_8,
            "片道イベントの経路で採用が起きてはならない（要件 5.3）"
        );
    }

    /// 片道イベントの本体が採用の経路を一切踏まないことを、本ファイルの本文で機械的に
    /// 確かめる（要件 5.3 の零＝「NOTIFY 応答からの採用 0」を件数で数え直す）。
    ///
    /// 呼ばれていないことは実行では観測できない（呼び出しが無い＝出来事が無い）ため、
    /// 本文そのものを判定材料にする。採用の経路を後から片道側へ繋ぐと赤になる。関数の本文は
    /// コメントも含めて検査するので、片道側の説明では採用の経路を名指ししない（名指しすると
    /// 赤になる——検査を緩めるのではなく説明の方を言い換えること）。
    #[test]
    fn the_one_way_event_body_contains_no_adoption_path() {
        const SELF_SOURCE: &str = include_str!("client.rs");
        // 改行の綴り（CRLF／LF）に依存しないよう CR を落としてから切り出す。
        let source = SELF_SOURCE.replace('\r', "");
        let after = source
            .split_once("    pub fn notify(")
            .expect("notify の定義行が見つからない")
            .1;
        let body = after
            .split_once("\n    }\n")
            .expect("notify の閉じ括弧が見つからない")
            .0;
        assert!(
            body.contains("encode_and_note("),
            "片道イベントは前半（符号化と置換の記録）を通るべき（要件 3.5/5.1）"
        );
        for forbidden in ["parse_and_note", "note_response", "parse_response"] {
            assert!(
                !body.contains(forbidden),
                "片道イベントの本体に採用の経路 `{forbidden}` がある（要件 5.3 の零が破れている）"
            );
        }
    }

    /// 交渉状態は結線より長生きし、応答待ちのイベントで採用した文字コードが**次の片道
    /// イベント**の要求バイト列に現れる（要件 5.1＝双方のイベントが同じ文字コードを使う）。
    #[test]
    fn adoption_in_an_answered_event_reaches_the_next_one_way_request() {
        let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);

        let charset = negotiator.current();
        let first = encode_and_note(&mut negotiator, &a_request(Method::Get, "OnBoot", charset));
        assert!(
            first.starts_with(b"GET SHIORI/3.0\r\nCharset: UTF-8\r\n"),
            "最初の要求は初期の文字コードで出る"
        );

        parse_and_note(&mut negotiator, &euc_jp_response()).expect("200 応答は解析できる");

        let charset = negotiator.current();
        let second = encode_and_note(
            &mut negotiator,
            &a_request(Method::Notify, "OnSecondChange", charset),
        );
        assert!(
            second.starts_with(b"NOTIFY SHIORI/3.0\r\nCharset: EUC-JP\r\n"),
            "採用した文字コードが次の片道イベントの要求に現れていない（要件 5.1）"
        );
    }

    /// **200 以外の応答でも** `Charset` ヘッダは採用対象である（要件 4.2）。
    ///
    /// design.md §System Flows「交渉の一周（GET）」の流れの決め事 1 つめ——「`note_response` は
    /// `map_get_result` より前。応答の status が 204・400・500 であっても `Charset` ヘッダは
    /// 採用対象（4.2）。emo2 の最初の応答待ちイベント `username` 照会が 204 でも UTF-8 を
    /// 採用する」——が本タスクの結線の契約そのものであり、その順序をここで押さえる。
    ///
    /// `note_response` が status を引数に取らないのは構造上の備えであって検査ではない。後半を
    /// 「200 のときだけ採用する」へ退化させると本テストが赤になる（204・400 の 2 系統）。
    #[test]
    fn a_non_200_response_still_adopts_its_charset_header() {
        let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");

        // 204（スクリプト無しの成功）——emo2 の `username` 照会がこの形。
        let mut after_204 = CharsetNegotiator::new(Charset::UTF_8, false);
        let parsed = parse_and_note(
            &mut after_204,
            &euc_jp_response_with_status("SHIORI/3.0 204 No Content"),
        )
        .expect("204 応答は解析できる");
        assert_eq!(
            parsed.status, 204,
            "固定物の status 行が 204 になっていない"
        );
        assert_eq!(
            after_204.current(),
            euc_jp,
            "204 応答の Charset ヘッダが採用されていない（要件 4.2・design §交渉の一周）"
        );

        // 400（SHIORI エラー応答）——結果の写しはエラーでも、採用は先に済んでいる。
        let mut after_400 = CharsetNegotiator::new(Charset::UTF_8, false);
        let parsed = parse_and_note(
            &mut after_400,
            &euc_jp_response_with_status("SHIORI/3.0 400 Bad Request"),
        )
        .expect("400 応答も（SHIORI エラーではあるが）解析自体は成功する");
        assert_eq!(
            parsed.status, 400,
            "固定物の status 行が 400 になっていない"
        );
        assert_eq!(
            after_400.current(),
            euc_jp,
            "400 応答の Charset ヘッダが採用されていない（要件 4.2・design §交渉の一周）"
        );
        // 結果の写しは 400 をエラーにする（採用が起きたことと両立する・要件 5.2）。
        assert!(
            map_get_result(parsed).is_err(),
            "400 は SHIORI エラーとして写るべき（採用の有無とは別の主張）"
        );
    }

    /// 復号で代替文字への置換が起きたという事実が、後半の結線を通って記録まで届く
    /// （要件 4.7・7.1＝記録なしに握り潰さない）。
    ///
    /// 解析側（置換の検出）と交渉状態側（記録の発火）はそれぞれ別の決定論テストが押さえて
    /// いるが、その 2 つを繋ぐ 1 語——後半が `parse_response` の返した置換の有無をそのまま
    /// `note_response` へ渡すこと——はここでしか見ていない。渡す値を `false` に固定すると
    /// 本テストだけが赤になる。
    #[test]
    fn the_decode_replacement_fact_reaches_the_record() {
        // `Charset: EUC-JP` を名乗りながら、EUC-JP として不正なバイトを値に持つ応答。
        // 0x80 は EUC-JP の先行バイト域（0xA1〜0xFE）の外にあり、代替文字へ吸収される。
        let mut bytes = b"SHIORI/3.0 200 OK\r\nCharset: EUC-JP\r\nValue: ".to_vec();
        bytes.push(0x80);
        bytes.extend_from_slice(b"\r\n\r\n");

        let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
        let (parsed, events) = log_capture_kit::capture(|| parse_and_note(&mut negotiator, &bytes));
        let parsed = parsed.expect("不正な並びでも解析は成功する（要件 4.7）");
        assert!(
            parsed.decode_had_errors,
            "解析側が置換を検出していない（固定物が不正な並びになっていない）"
        );

        let recorded: Vec<_> = events
            .iter()
            .filter(|e| e.field_str("event") == Some("charset_invalid_bytes_replaced"))
            .collect();
        assert_eq!(
            recorded.len(),
            1,
            "置換の事実が記録まで届いていない（要件 4.7・7.1）"
        );
    }
}

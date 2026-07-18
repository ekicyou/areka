//! `Shiori3Client`（要件 4.1〜4.8 / 5.1〜5.4・design.md §Shiori3Client）: host-32 SHIORI
//! 出口 API。「イベント（ID＋References）→ Value」の単一 request 経路を提供し、codec /
//! wire / HGLOBAL の内部型を一切露出しない。
//!
//! # 責務
//! - [`Shiori3Client::get`] — 応答を要するイベント。`build_request` → `send_request` →
//!   `parse_response` を結線し、200 は `Some(Value)`／204 は `None`／400・500・`ErrorLevel`
//!   は `Err(RequestError::Shiori)` を返す（要件 4.1/4.3/4.7）。
//! - [`Shiori3Client::notify`] — 片道イベント。**同期 `request()` 往復**で送出し、返却
//!   response（例 204）を破棄して `Ok(())` を返す（片道 IPC 化しない・要件 4.8）。
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

use crate::error::{RequestError, ShioriError};
use crate::parent_window::{ParentMessageWindow, SendError};
use crate::process_host::request_timeout_from_env;
use crate::shiori3::{Charset, Method, ParsedResponse, ShioriRequest, build_request, parse_response};

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
}

impl<'a> Shiori3Client<'a> {
    /// `Sender` を既定（`"areka"`）とする client を構築する（要件 4.x）。
    ///
    /// charset は本仕様では [`Charset::Utf8`] 固定。`window` はハンドシェイク完了済みである
    /// ことを前提とする（未準備ガードは設けない・design.md §Preconditions: Load は Request に
    /// 構造的に先立つ）。
    #[must_use]
    pub fn new(window: &'a ParentMessageWindow) -> Self {
        Self {
            window,
            sender: DEFAULT_SENDER,
        }
    }

    /// `Sender` を明示指定して client を構築する（単一差替点・design.md §送出ヘッダ最小集合）。
    ///
    /// 通常は [`Shiori3Client::new`]（`"areka"` 既定）で足りる。互換ベースウェアの名乗りを
    /// 差し替えたい場合のみ本関数を使う。
    #[must_use]
    pub fn with_sender(window: &'a ParentMessageWindow, sender: &'a str) -> Self {
        Self { window, sender }
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
        &self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let bytes = build_request(&ShioriRequest {
            method: Method::Get,
            id,
            references,
            sender: self.sender,
            status,
            charset: Charset::Utf8,
        });
        // GET/NOTIFY は wire tag = MsgTag::Request で合流（要件 4.7）。
        let resp = self
            .window
            .send_request(MsgTag::Request, &bytes, self.effective_timeout())
            .map_err(map_send_error)?;
        // parse の malformed（ShioriError::Parse）は #[from] で RequestError::Shiori へ持ち上がる。
        let parsed = parse_response(&resp, Charset::Utf8)?;
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
        &self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        let bytes = build_request(&ShioriRequest {
            method: Method::Notify,
            id,
            references,
            sender: self.sender,
            status,
            charset: Charset::Utf8,
        });
        // 同期往復（要件 4.8）: wire tag は GET と同じ MsgTag::Request（要件 4.7）。
        let _discarded = self
            .window
            .send_request(MsgTag::Request, &bytes, self.effective_timeout())
            .map_err(map_send_error)?;
        // 返却 response（例 204）は破棄する（要件 4.8・解析も surface もしない）。
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
}

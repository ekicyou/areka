# Implementation Plan

- [ ] 1. Foundation: 環境検証・timeout基盤・エラー型骨格
- [x] 1.1 (P) vendors/pasta submodule 展開確認と pasta request 署名照合の記録参照
  - `git submodule status` で `vendors/pasta` が populated であることを確認する（未展開なら `git submodule update --init` で展開する）
  - research.md §7.2 記載のバイト照合結果（commit `048d646`・`RequestFn` 署名一致）を参照し、実装前提として記録する
  - Observable: `vendors/pasta/Cargo.toml` 等のファイルが存在し、submodule のコミットハッシュが記録値と一致することを確認できる
  - _Requirements: 7.5_

- [x] 1.2 (P) REQUEST_TIMEOUT 定数 + AREKA_SHIORI_REQUEST_TIMEOUT_MS env seam 追加
  - `LOAD_ACK_TIMEOUT=30s` とは別建てで `REQUEST_TIMEOUT` 定数（既定 60 秒）を `process_host.rs` に追加する
  - env `AREKA_SHIORI_REQUEST_TIMEOUT_MS` で上書き可能にし、`"0"` は無限待ちとして扱う
  - Observable: 単体テストで既定 60 秒・env 上書き・`"0"`=無限待ちの3ケースが期待どおりの値を返すことを確認できる
  - _Requirements: 4.3, 5.1_
  - _Boundary: process_host.rs (shiori-host32-host)_

- [x] 1.3 (P) RequestError / ShioriError 型定義
  - `ShioriError`（`Parse` ／ `Status{status, error_level, error_description}`）を `error.rs` に定義する
  - `RequestError`（`Handshake` ／ `Timeout` ／ `Ipc(IpcError)` ／ `Shiori(ShioriError)`、`thiserror` 使用）を定義する
  - Observable: 各 variant の `Display` 文字列を単体テストで確認できる
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Boundary: error.rs (shiori-host32-host)_

- [ ] 2. Core: host x64 SHIORI/3.0 codec（純関数）
- [ ] 2.1 request 組立（build_request）実装
  - `GET`/`NOTIFY SHIORI/3.0` の request line、`Reference0..N` 連番、`Charset`（UTF-8）／`Sender`（areka）／`ID`／`SecurityLevel`（local）ヘッダ、CRLF 区切り、空行終端を組み立てる
  - イベント名は汎用の `ID` 値として受け取り、特定イベントに固有の分岐や既定 Reference を埋め込まない
  - Observable: 単体テストで GET/NOTIFY それぞれのバイト列出力を検証し、末尾が二重 CRLF であることを確認できる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_
  - _Boundary: shiori3 codec (shiori-host32-host)_

- [ ] 2.2 response 解析（parse_response）実装
  - status（200/204/311/312/400/500）分岐、`Value` 抽出、`ErrorLevel`/`ErrorDescription` 保持、`Charset` 省略時の継承、未知ヘッダ寛容、malformed は `Err` を返す解析を実装する
  - Observable: 各 status 別・malformed 別に単体テストで期待される `ParsedResponse`/`Err` が得られることを確認できる
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_
  - _Depends: 1.3_

- [ ] 3. Shiori3Client（get/notify 出口API）実装と公開 re-export 統合
  - `get(id, refs)`: request 組立 → `send_request(REQUEST_TIMEOUT)` → response 解析 → 200 は `Some(Value)`、204 は `None`、400/500/`ErrorLevel` は `Err(RequestError::Shiori)` を返す
  - `notify(id, refs)`: request 組立 → 同期 `request()` 往復 → 応答を破棄 → `Ok(())` を返す（片道 IPC 化しない）
  - `IShiori::Get` への写像は型シームとして doc コメントのみで示し、実装しない
  - `SendError` から `RequestError` への写像で `IpcError::Timeout` が必ず `RequestError::Timeout` へ振り分けられ `RequestError::Ipc` には含まれないことを単体テストで固定する
  - `lib.rs` に `shiori3`／`client`／`RequestError`／`ShioriError`／`ParsedResponse`／`REQUEST_TIMEOUT` を公開 re-export する
  - Observable: 単体テストで `SendError` の各パターン（Handshake/Timeout/SendFailed）が `RequestError` の正しい variant へ写像されることを確認できる
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.1, 5.2, 5.3, 5.4_
  - _Depends: 1.2, 1.3, 2.1, 2.2_
  - _Boundary: Shiori3Client, lib.rs (shiori-host32-host)_

- [ ] 4. Core: helper i686 request 実呼出
- [ ] 4.1 (P) ShioriByteProxy::request メソッド実装（HGLOBAL 非対称契約）
  - `global_alloc_copy` で入力を `GMEM_FIXED` 化し `request(hreq, &mut len)` を呼び出す。入力 HGLOBAL は自ら解放しない（callee-free）
  - 応答 HGLOBAL から `*len` バイトを copy した後 `GlobalFree` する（caller-free）
  - Observable: testdll 越し loopback テストで固定応答往復・入力 callee-free／応答 caller-free が panic 無く完了することを確認できる
  - _Requirements: 3.2, 3.3, 3.4, 3.6, 7.5, 7.6_
  - _Boundary: ShioriByteProxy (shiori-host32-helper)_

- [ ] 4.2 Reply アーム置換（main.rs, handle_message 側）
  - `classify_inbound` は純関数を維持する（proxy に到達しない）。proxy 駆動は `handle_message` の Reply アームで実施する
  - proxy 確立済みなら `proxy.request(payload)` の結果を RESPONSE 返送、未確立なら明示エラーバイト列を返送する
  - RefCell 再入規律を守る: `proxy.borrow()` を `send_copydata` 越しに保持しない（LOAD アームと同型）
  - 新 `MsgTag`・framing 変更なし、既存 RESPONSE 経路（`MsgTag::Response`）をそのまま使用する
  - Observable: 結合テストで proxy 確立済み時に request 駆動結果が RESPONSE 経路で返送されることを確認できる
  - _Requirements: 3.1, 3.5, 3.7, 4.7, 4.8, 7.1_
  - _Depends: 4.1_
  - _Boundary: helper WndProc Reply arm (shiori-host32-helper)_

- [ ] 5. (P) Core: testdll request fixture 拡張
  - `request` stub（null 返却）を「受領 request line/`ID` を検証し固定 SHIORI/3.0 応答を返す」実装へ拡張する
  - テスト GET ID → `200 OK`+`Value`、テスト NOTIFY ID → `204 No Content` を `GlobalAlloc(GMEM_FIXED)` で確保し返却する
  - 入力 HGLOBAL を callee 側で `GlobalFree` する（受領検証後）
  - `crates/pilot` へ依存しない（葉ノード隔離を維持する）
  - Observable: fixture 単体テストで request line/`ID` の assert が機能し、GET/NOTIFY 双方で正しい固定応答が返ることを確認できる
  - _Requirements: 6.1, 6.2, 6.4, 6.8, 6.9_
  - _Boundary: testdll request fixture (shiori-host32-testdll)_

- [ ] 6. Integration: E2E 結線とテスト
- [ ] 6.1 決定的 request E2E テスト
  - `shiori_load_e2e.rs` の骨格を踏襲する（`resolve_helper_exe`/`resolve_testdll`、`HelperGuard`、env→target 解決＋silent skip 禁止 panic）
  - helper 越し fixture へテスト GET を送出し `Value` 抽出を assert、テスト NOTIFY を送出し 204 破棄を assert する
  - 所有権規約（callee-free／caller-free）無違反、request line/`ID` の assert 面で正しく組み立てられ届いたことを裏付ける
  - Observable: `cargo test`（x64、事前に i686 helper/testdll を PowerShell でビルド済み）で GET/NOTIFY 両 E2E が green になることを確認できる
  - _Requirements: 6.3, 6.4, 6.9, 7.2, 7.3_
  - _Depends: 3, 4.2, 5_

- [ ] 6.2 env-gated 実 pasta.dll OnBoot 追験テスト
  - env `HOST32_PASTA_DLL` 設定時のみ OnBoot request 送出→`Value` 受領を検証する
  - 指定 DLL 不在は明示的に失敗させ、未設定時は silent skip とする（CI 必須ゲートにしない）
  - Observable: env 設定時に OnBoot `Value` が受領されテストが green になり、DLL 不在時は明示的な失敗メッセージで落ちることを確認できる
  - _Requirements: 6.5, 6.6, 6.7_
  - _Depends: 6.1_

- [ ] 7. Validation: 横断規律の最終検証
  - `shiori-host32-ipc`（凍結境界）への差分が無いことを確認する（git diff 等）
  - `crates/pilot` への inbound 依存が本仕様の新規/変更コードに無いことを確認する（grep）
  - i686 成果物（helper/testdll）が PowerShell 経由で `cargo build`/`cargo test --target i686-pc-windows-msvc` に成功することを確認する
  - Observable: 上記3点のチェックがすべて pass する（diff 無し・grep 結果0件・i686 ビルド/テスト green）ことを確認できる
  - _Requirements: 7.1, 7.2, 7.3, 7.4_
  - _Depends: 6.2_

## Implementation Notes

- 1.1: `vendors/pasta` submodule 展開済み（commit `048d646c`・research.md §7.2 記録値と一致）。`vendors/pasta/crates/pasta_shiori/src/windows.rs:76` の `request` 実署名 = `pub extern "C" fn request(req: HGLOBAL, len: &mut usize) -> HGLOBAL`。helper 既存 `RequestFn = unsafe extern "cdecl" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL` と ABI バイト一致（i686 で `extern "C"`≡`cdecl`・`&mut usize`≡`*mut usize`）＝**helper 側 RequestFn 型は変更不要**（R7.5 充足）。

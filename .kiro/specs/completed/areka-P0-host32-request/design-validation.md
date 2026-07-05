# 設計バリデーションレポート: areka-P0-host32-request

> 検証日: 2026-07-03 / 対象: 確定済み design.md（凍結）＋ requirements.md（7 要件・凍結）＋ research.md ＋ steering
> 種別: 非対話バリデーション（design-review.md プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO）
> 検証観点: 7 要件の忠実・完全被覆／ディスカッション確定不変条件／凍結 wire 不接触／HGLOBAL 非対称所有権契約／①-C 最小実装スコープ／pilot 非依存

## 設計レビュー要約

design.md は 7 要件を Requirements Traceability 表・Components/Interfaces・System Flows で漏れなく写し、ディスカッションで確定した不変条件（Load-before-Request 無ガード・GET/NOTIFY 合流・NOTIFY 同期往復 caller-free・testdll 固定応答）を Key Decisions と Data Models へ正確に落としている。凍結境界 `shiori-host32-ipc`（`MsgTag::Request=3/Response=4`・`send_request`）はコード実測で改変対象に含まれず不透明バイト列として利用のみ、`RequestFn` 署名は helper 実コードと一致、pilot 非依存は host/testdll の実コード grep で確認済み。実装準備は十分整っており、指摘は微細な明確化に留まる。

## クリティカルイシュー（≤3）

本バリデーションではブロッキングなクリティカルイシューは検出しなかった。以下は GO を妨げない軽微な明確化候補（design 凍結ゆえ実装/タスク時の留意点として記録）。

### 軽微 1: `RequestError::Ipc(IpcError)` と `SendError` 経路の写像の 1 対 1 対応が図と型で微妙にズレる
- **内容**: エラー分類 flow（design「エラー分類」）は `send_request` の結果を `Handshake`/`Ipc Timeout`/`Ipc SendFailed` の 3 系へ分岐するが、State Management の `RequestError` は `Handshake(#[from] HandshakeError)`・`Timeout`・`Ipc(IpcError)`・`Shiori` の 4 variant。上流 `send_request` は `Result<Vec<u8>, SendError>`（`SendError{Handshake, Ipc}`）を返すため、`SendError::Ipc(IpcError::Timeout)`→`RequestError::Timeout`／`SendError::Ipc(其他)`→`RequestError::Ipc` の分解は本文（State model 行）に記述はあるが、`RequestError::Ipc(IpcError)` が `Timeout` を含み得ない不変（Timeout は別 variant へ吸い上げ済み）を型では強制していない。
- **影響**: 区別保持（R5.4）は満たすが、実装時に `IpcError::Timeout` を `RequestError::Ipc` へ誤って包む余地が型上残る（テストで捕捉可能・設計上の欠陥ではない）。
- **提案**: タスク/実装時に「`SendError`→`RequestError` 写像は `IpcError::Timeout` を必ず `Timeout` variant へ振り分ける」単体テスト（design Testing Strategy に既記）で不変を固定すれば足りる。design 変更不要。
- **Traceability**: R5.1, R5.4
- **Evidence**: design.md「エラー分類（R5.1〜5.4）」flow ／「エラー語彙 RequestError / ShioriError」State Management

### 軽微 2: `classify_inbound` 純関数と proxy 駆動の責務境界が File Structure Plan で暗黙
- **内容**: helper 実コードでは `classify_inbound` が純関数（bytes のみ受領・`InboundAction::Reply(respond(payload))` を算出）で proxy に到達できない。design は「`main.rs` の `Reply` アームを proxy.request 駆動へ置換」と正しく `main.rs`（WndProc/`handle_message` 側）を変更先に指定しており responsibilities も ②-A に整合するが、`classify_inbound` を純に保つか proxy 参照へ変えるかの線引きが Modified Files の記述からは一意に読み取りづらい。
- **影響**: 実装者が誤って `classify_inbound` へ proxy 依存を注入すると純粋性（単体テスト可能性）が崩れる。research §7.3 は ②-A で「`classify_inbound` の純粋性維持」を含意するが design 本文では明示されていない。
- **提案**: タスク時に「proxy 駆動は WndProc（`handle_message`）アームで行い `classify_inbound` は純関数を維持」を実装制約として記す。design の意図（unsafe 一点集約・RefCell 再入規律）とは既に整合。
- **Traceability**: R3.1, R7（unsafe 隔離）
- **Evidence**: design.md「Modified Files」`main.rs` 行 ／「ShioriByteProxy::request ＋ Reply アーム」Responsibilities

### 軽微 3: testdll fixture の Req カバレッジ表記が R6.9 の NOTIFY コードパスを Requirements 欄に含めていない
- **内容**: 「testdll request fixture」コンポーネントの Requirements 欄は「6.1, 6.2, 6.4, 6.9」と記載され R6.9（GET/NOTIFY 両 request line）は含むが、Responsibilities の固定応答本文は GET 200+Value／NOTIFY 204 を両方明記しており実体は完全。一方 Requirements Traceability 表の 6 行は「6.1–6.9」と範囲表記で E2E コンポーネント（6.3/6.5/6.6/6.7/7.2/7.3）と重複被覆され、fixture 単体と E2E の担当境界（6.3 は E2E 観測・6.9 は fixture 応答生成）が二重表記でやや冗長。
- **影響**: 被覆漏れではない（全 6.x が少なくとも 1 コンポーネントに割当済）。読み手が fixture/E2E の責務分界を追う際に軽い曖昧さが残るのみ。
- **提案**: 実質影響なし。必要なら tasks 生成時に fixture=応答生成＋所有権実体化、E2E=往復観測＋env-gated 実 pasta と役割を明記。
- **Traceability**: R6.1–6.9
- **Evidence**: design.md「Requirements Traceability」6 行 ／「i686 fixture — testdll request」Requirements 欄

## 設計の強み

- **凍結境界と非対称所有権契約が全境界で一次情報に接地**。`shiori-host32-ipc` の `MsgTag::Request=3/Response=4`・`send_request` はコード実測で不接触（不透明バイト列利用のみ）を確認、`RequestFn = unsafe extern "cdecl" fn(HGLOBAL, *mut usize) -> HGLOBAL` は helper 実コードと一致（§7.2 バイト照合）。HGLOBAL 契約は Data Models の 2 行表で「入力=helper alloc/DLL free（callee-free）」「応答=DLL alloc/helper free（caller-free）」と helper・fixture の双方向で対称に固定され、`spec_dll` 正典に接地している。
- **①-C 最小実装スコープが要件不変条件と精密に一致**。IShiori 写像は型シームのみ（`onto_ishiori_get` を doc＋署名で示し実装しない）・`SHIORI_S_PENDING` 非塞ぎ・Load-before-Request 無ガード（IShioriFactory 融合 create+load の構造的必然）・GET/NOTIFY 合流かつ NOTIFY も同期往復で応答 HGLOBAL を caller-free 解放（R4.8 の解放漏れ回避）が Non-Goals・Key Decisions・Preconditions に一貫して現れ、過小でも過剰でもないスコープに収まっている。pilot は host/testdll の grep で inbound 依存ゼロを確認（host README が依存グラフ非出現を明文で保証）。

## 最終評価

### 判定: **GO**

### 根拠
7 要件はトレーサビリティ表とコンポーネント契約で忠実・完全に被覆され、ディスカッション確定の 5 不変条件（Load-before-Request 無ガード／GET-NOTIFY 合流／NOTIFY 同期往復 caller-free／testdll 固定応答／型シームのみの ①-C）が設計全体に一貫実装され、凍結 wire 不接触・HGLOBAL 非対称契約・pilot 非依存はいずれもコード実測で裏付けられている。検出したのはブロッキングでない明確化 3 点のみで、実装/タスク段階のテストと制約明記で吸収可能。

### 次ステップ
- design.md は凍結・approved 待ち。`/kiro-spec-tasks areka-P0-host32-request` でタスク生成へ進む。
- タスク生成時の留意点（design 変更不要）: (1) `SendError`→`RequestError` 写像の `IpcError::Timeout` 振り分け単体テスト、(2) `classify_inbound` 純関数維持・proxy 駆動は WndProc アーム、(3) fixture=応答生成/E2E=往復観測の責務分界明記。
- 実装前提: `vendors/pasta` submodule 展開（署名バイト照合済 commit `048d646`）と i686 helper/testdll の PowerShell 事前ビルド（Git Bash 不可）。

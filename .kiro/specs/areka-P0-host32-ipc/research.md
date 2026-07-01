# ギャップ分析 (Gap Analysis): areka-P0-host32-ipc

> 本書は `kiro-validate-gap` による**要件⇄既存コードベースのギャップ分析**である。実装方針の**決定ではなく、選択肢と論点の提示**を目的とする（最終判断は要件ディスカッション／design フェーズに委ねる）。分析日: 2026-07-01。

## 0. スコープ確認

本ユニット `areka-P0-host32-ipc` は M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラックの**先頭ユニット**。責務は **bytes-over-wire transport 層まで**（「request bytes を送り、response bytes を受ける」seam）。SHIORI/3.0 の build/parse・`LoadLibraryW pasta.dll`・常駐 lifecycle は**明示的に除外**（下流ユニットの領分）。

- **上流アンカー（参照専用・go 済 2026-07-01）**: `crates/pilot/examples/shiori-host-32/`（README/REPORT が一次記録・正本）
- **接続先（残す基盤）**: `crates/shiori-abi/`（x64 `IShiori`/`IShioriHost` COM）
- **規律**: pilot コードは**コピペ donor 禁止・クリーン再掘**（two-tunnel.md 掘り直し禁止・要件 8.4）

---

## 1. 現状調査 (Current State Investigation)

### 1.1 既存資産マップ

| 資産 | 場所 | 本ユニットとの関係 |
|------|------|---------------------|
| pilot transport 実装 | `crates/pilot/examples/shiori-host-32/{ipc,parent_window,helper_window,process_host,helper,main}.rs` | **参照専用**（コピペ禁止）。実証済みの構造と規約を「知見」として移植 |
| pilot バイト proxy | 同 `shiori_proxy.rs` | **本ユニット対象外**（下流 `host32-shiori-load`）。ただし本ユニットの seam の下流境界を規定 |
| pilot SHIORI3 codec | 同 `shiori3.rs` | **本ユニット対象外**（下流 `host32-request`）。往復 echo テストでは SHIORI3 セマンティクス不要 |
| SHIORI ABI | `crates/shiori-abi/`（`IShiori`/`IShioriHost` COM・HSTRING） | **接続先**。本ユニットは bytes transport のみ提供、ABI 実装本体は下流と design で結線 |
| pilot クレート構造 | `crates/pilot/`（空 lib ＋ `examples/` のみ） | 葉ノード隔離の規範。本ユニットは**別の production クレート**として新設 |
| ワークスペース定義 | `/Cargo.toml`（`members = ["crates/*"]`） | 新クレートは `crates/*` glob で自動メンバー化。`workspace.dependencies` に既に `wintf-winmsg-executor=0.0.5` / `event-listener=5` / `windows`(0.62.2) / `windows-core` あり |

### 1.2 pilot が実証した構造（本ユニットが「知見」として担ぐもの）

pilot README の「学び」節（正本）から、本ユニットが transport 層として再実装すべき確定事項:

1. **WM_COPYDATA 一方向トランスポート（`ipc.rs`）**
   - `MsgTag`（`#[repr(u32)]`・`dwData` 低 32bit に載る・跨ビットネス安全）: Hello=1/Load=2/Request=3/Response=4/Unload=5
   - ペイロード = 生バイト列（`cbData`=長さ・固定ヘッダ 0・ポインタ/HANDLE/struct 禁止）
   - HWND は **u32 LE 4 バイト**でワイヤ化（`encode_hwnd_le`/`decode_hwnd_le`/`hwnd_from_u32`）
   - 送出は `SendMessageTimeoutW(SMTO_ABORTIFHUNG, timeout_ms)`（ハングしない）

2. **再入 RESPONSE 受信 = デッドロック回避の核（`parent_window.rs` + `ipc.rs::ResponseSlot`）**
   - 親が `SendMessageTimeout` でブロック中、helper の RESPONSE(2nd WM_COPYDATA) が親 WndProc へ**再入配送**され `ResponseSlot`（`RefCell<Option<Vec<u8>>>`）へ格納 → 復帰後 `take()`
   - 応答 WndProc は payload 格納後**即 return**（それ以上跨プロセス SendMessage しない）＝ single-in-flight・厳密ネスト・循環待ちなし

3. **プロセス spawn / 非ブロッキング生存監視（`process_host.rs`）**
   - `std::process::Command` で helper 起動（`windows` クレート不要の std-only）
   - `try_wait()` ベースの非ブロッキング `poll_exit` / `poll_exit_kind`
   - `ExitKind` 分類: `Clean`(0) / `Abnormal(i32)`(非0) / `Terminated`(コードなし＝シグナル等)
   - 親 HWND は u32 ワイヤ値で arg/env（`PARENT_HWND`/`GHOSTDIR`/`HELPER_EXE`）経由で子へ渡す

4. **helper 側 message-only 窓 + ループ（`helper_window.rs`）**
   - `wintf-winmsg-executor` 0.0.5 の `Window<S>`（`WindowType::MessageOnly`）＋ `MessageLoop::run`
   - 起動時 HELLO 送出（自 HWND を u32 LE）→ WndProc で REQUEST 受領 → RESPONSE 1 通返送
   - `HelperState` 状態機械（Started→Pumping→Unloading→CleanExit）
   - GetMessage を無入力でも起こす**ハートビート**（別スレッドから WM_NULL を PostMessage）で bounded なループ制御

5. **HELLO ハンドシェイク（`parent_window.rs::pump_until_hello_or`）**
   - 親が helper HWND 受領まで（または timeout まで）ループを pump・bounded・ハングしない

### 1.3 命名・レイヤ規約（structure.md / tech.md）

- ファイル `snake_case.rs`・型 `PascalCase`・関数 `snake_case`・定数 `SCREAMING_SNAKE_CASE`
- エラーは `thiserror` の構造化 enum（全クレート共通規約）。pilot は素の `enum`（`IpcError`/`ExitKind`/`ProxyError`）ゆえ本ユニットは `thiserror` 化を検討
- `unsafe` は Win32 境界に集約し安全 API を上位へ（pilot の `copydata_payload`/`send_copydata` が該当）
- テスト: in-source `#[cfg(test)]` ＋ ドメイン別 `tests/`。examples はテストの代替でない
- **重要な既知トラップ（i686）**: `usize`=32bit ゆえ `(x as usize) >> 32` は overflow lint でコンパイルエラー → `u64` cast で評価（pilot `ipc.rs` テストが実践）

### 1.4 統合面（Integration Surfaces）

- **上流 seam（helper 側の下流境界）**: 本ユニットの helper は「REQUEST payload を受けて RESPONSE payload を返す」ところまで。実際の pasta 駆動（`ShioriByteProxy`）は下流 `host32-shiori-load`。往復 echo テストでは **helper が受信 payload をそのまま echo で返す** stub で足りる。
- **下流 seam（x64 側の上位境界）**: 本ユニットの x64 side は「request bytes を送り response bytes を受ける」API を公開。上に載る `IShiori` ABI 実装本体は下流と design で結線（本ユニットは `shiori-abi` に**依存しなくてもよい**設計余地あり＝bytes seam の純度）。

---

## 2. 要件実現可能性分析 (Requirements Feasibility)

要件（EARS）→ 技術的必要事項 → ギャップ（Missing / Unknown / Constraint）のマップ。

| 要件 | 技術的必要事項 | pilot 対応箇所 | ギャップ判定 |
|------|----------------|----------------|--------------|
| R1 spawn / 生存監視 | `Command::spawn`・`try_wait` 非ブロッキング・ExitKind 分類 | `process_host.rs`（完全）| **Missing**（クリーン再実装。構造は既知＝低リスク）|
| R2 WM_COPYDATA framing | MsgTag(u32低32bit)・u32 LE HWND・cbData 境界・生バイト規約・不正フレーム検出 | `ipc.rs` §1–3（完全）| **Missing**（同上）。不正フレーム検出（R2.5）は pilot では `try_from_u32` の `Err` 経路・payload 長検査が該当 |
| R3 HELLO ハンドシェイク | helper→親 HELLO（u32 LE HWND）・親記録・完了まで往復開始せず・timeout | `helper_window.rs::create` + `parent_window.rs::pump_until_hello_or` | **Missing**（構造既知）。R3.3「完了まで往復開始しない」の順序保証は明示的なゲート化を要検討 |
| R4 再入 RESPONSE / デッドロック回避 | single-in-flight ブロック送信・helper 即 return・厳密ネスト・デッドロックなし | `ipc.rs::send_request`+`ResponseSlot` / `parent_window.rs` WndProc | **Missing**（**設計の核**・pilot で go 実証済＝方向確定）|
| R5 timeout / wedge 検出 | `SendMessageTimeout` 上限時間・Timeout 打ち切り・`SMTO_ABORTIFHUNG` | `ipc.rs::send_copydata`/`send_request`（`IpcError::Timeout`）| **Missing**（実証済＝低リスク）|
| R6 往復 echo 観測 | request bytes 送出→同一（照合可能な）response bytes 受領・無 crash・無デッドロック | pilot は **SHIORI3 往復**（echo でなく実 pasta）| **Constraint/Unknown**（下記 §2.1）|
| R7 i686 ビルド / 32bit 可搬性 | i686 target ビルド・shift overflow 回避・生バイトのみ跨ぐ | `[[example]]` 宣言 + `helper.rs` + README ビルド手順 | **Missing/Constraint**（クレート構成が pilot の example とは異なる＝§3 の論点）|
| R8 責務境界 | SHIORI3/DLL load/lifecycle を持たない・pilot コピペしない | — | **Constraint**（negative requirement・レビュー規律）|

### 2.1 R6「往復 echo」の重要ギャップ（Research/Design Flag）

pilot は往復を**実 pasta.dll の OnBoot 応答**（SHIORI3 セマンティクス込み）で実証した。しかし本ユニットの R6 は「**request bytes → 同一内容の response bytes**」という**意味を持たない生バイトの echo**を要求する（SHIORI3・pasta を除外＝R8）。

したがって本ユニットの往復 echo テストでは、helper 側に **pasta 駆動の代わりに「受信 payload をそのまま RESPONSE として返す echo stub」**が必要になる。これは pilot の `helper_window.rs::handle_message`（REQUEST→`ShioriByteProxy::shiori_request`→RESPONSE）から **pasta 依存を外し echo に置換**する形。

- **論点**: この echo helper は本ユニットの production 成果物か、テスト専用の fixture か。往復 echo は「M1 のゲート指標」（要件 6・観測者視点）ゆえ、**恒久的な統合テストハーネス**として位置づけるのが自然だが、下流が pasta を挿すと echo helper は不要になる。→ helper の REQUEST ハンドラを**差し替え可能なシーム**（trait / コールバック）として設計し、本ユニットでは echo 実装、下流で pasta 実装を注入する構成が候補（§3 Option 参照）。

### 2.2 複雑性シグナル

- 外部統合（Win32 IPC・跨プロセス・跨ビットネス）＝**high**。ただし **pilot が go 実証済み**ゆえ「未知の実現可能性」リスクは解消。残るは「クリーンな production 品質での再実装」＝中程度。
- ワークフロー（spawn→handshake→往復→timeout→生存監視）＝状態機械あり・中程度。
- アルゴリズム的難所は少ない（framing は単純な符号化）。核心は**並行制御（再入・デッドロック回避）**で、これは pilot の構造を忠実に踏襲すれば足りる。

---

## 3. 実装アプローチの選択肢 (Implementation Approaches)

brief は「pilot は `examples/` 隔離ゆえ本坑は `crates/` 直下の新クレート＝x64 host lib ＋ i686 helper bin のペア想定」とし「配置と構成を依頼者へ提示して確認」を求めている。以下 3 案。**配置は design 議題**。

### Option A: 既存クレート拡張（`shiori-abi` 等へ相乗り）

**検討根拠**: 本ユニットは `shiori-abi` の下流ホストゆえ同居も理屈上は可能。

- **却下寄り**: `shiori-abi` は「UI 基盤非依存の最小依存 ABI 定義クレート」で `windows-core`/`windows`(Com)/`thiserror` のみ。IPC transport（`wintf-winmsg-executor`・i686 helper bin・spawn）を混ぜると最小依存の純度が壊れる。structure.md の「shiori-abi は wintf に依存させない／下流 32bit ホストが同 ABI を共有」という設計意図に反する。
- **トレードオフ**: ❌ 責務混濁・❌ ABI クレートの依存肥大・✅ 新クレート数が増えない（利点小）。

### Option B: 新クレート新設（推奨候補・単一 or ペア）

**検討根拠**: transport は独立した lifecycle・依存（`wintf-winmsg-executor`・i686 target）を持つ。structure.md 末尾「Workspace 構成により別クレート追加が容易」と整合。

- **B-1: 単一クレート（x64 lib ＋ i686 helper を `[[bin]]` で内包）**
  - 1 つの `crates/host32-ipc/`（仮名）に、共有 `ipc` モジュール（両 target ビルド）＋ x64 側 lib（`process_host`/`parent_window`）＋ helper を `[[bin]]`（i686 独立ビルド）で持つ。
  - pilot の `#[path = "ipc.rs"] mod ipc;` 物理共有パターンを、**通常の `mod` 共有**（同一クレート内）へ格上げできる。
  - ✅ 共有規約が単一クレート内で自然に共有・✅ ワークスペースメンバー 1 つ・❌ x64 lib と i686 bin を 1 クレートで両 target ビルドする際の feature/target 条件分岐が要設計（§4 Unknown）。
- **B-2: ペアクレート（`host32-ipc`(x64 lib) ＋ `host32-ipc-helper`(i686 bin) ＋ 共有 `host32-ipc-proto`）**
  - 共有規約を独立クレート化し、x64 lib と helper bin が両方依存。
  - ✅ target ごとの依存が明確・✅ 各クレート単責務・❌ ワークスペースメンバー 3 つに増える・❌ 過剰分割の懸念（M1 の最小実装方針と要相談）。
- **トレードオフ（B 全般）**: ✅ 責務分離・✅ 単体テスト容易・✅ pilot の葉ノード隔離を production の clean 構造へ昇格・❌ target 混在ビルドの CI/コマンド設計が新規。

### Option C: ハイブリッド（proto 共有クレート ＋ x64 lib に helper を内包）

- 共有規約のみ独立クレート（`host32-ipc-proto`）、x64 transport lib（`host32-ipc`）が helper を `[[bin]]` として内包し proto へ依存。
- B-1 と B-2 の中間。共有規約の再利用性（下流ユニットも proto を参照しうる）を確保しつつクレート数を抑える。
- ✅ 共有規約が下流（`host32-shiori-load` 等）からも参照可能・✅ helper との target 分離は `[[bin]]`＋target 指定で・❌ 設計判断が最も多い。

> **注**: いずれの Option でも「helper の REQUEST ハンドラを差し替え可能なシーム（echo↔pasta）」（§2.1）をどこに置くかが交差論点。本ユニットは echo 実装を持ち、下流が pasta 実装を注入できる境界（trait object / ジェネリック / コールバック）を design で確定する。

---

## 4. Research Needed（design フェーズへ持ち越す不確実事項）

1. **[R6/クレート構成] echo helper の位置づけ**: 往復 echo helper を production 成果物（差し替えシーム付き）とするか、テスト fixture とするか。下流で pasta を挿す際の注入境界（trait / callback）の形。→ design 議題。
2. **[B-1/C] 単一クレートでの x64 lib ＋ i686 helper bin の両 target ビルド構成**: `[[bin]]` の target 指定・`#[cfg(target_pointer_width)]` 分岐・`cargo build --target i686-pc-windows-msvc` を production クレートで回す際の `windows` feature 差分（pilot は `Win32_System_DataExchange`/`Memory`/`Globalization` を追加）。本ユニットは DataExchange のみで足りるか（Memory/Globalization は pasta proxy 用＝下流）を要確認。
3. **[R4] 再入受信の production 品質での再現**: pilot は `RefCell`/`Cell`（UI スレッド固定・single-in-flight）で足りた。production で同一前提を型・API で保証する形（`ResponseSlot` の公開 API・window state 共有パターン）。`wintf-winmsg-executor` の `Window<S>` state 共有（`Pin<&S>`）を踏襲するか、独自ラッパを立てるか。
4. **[R7] i686 ビルドの CI/コマンド規律**: PowerShell 必須（Git Bash link.exe トラップ）。production クレートでの i686 target 追加ビルドの手順を README/steering にどう固定するか。`rustup target add i686-pc-windows-msvc` は導入済（pilot 実証）。
5. **[R3.3] ハンドシェイク完了前の往復抑止の明示化**: pilot は手順順序（pump_until_hello の後に send_request）で担保。production では状態ゲート（未ハンドシェイク時の send を型/実行時で拒否）にするか。
6. **[エラー型] `thiserror` 化**: pilot の `IpcError`/`ExitKind` を全クレート共通規約の `thiserror` enum へ。境界（Timeout/SendFailed/PeerGone/HandshakeTimeout/spawn 失敗）の網羅。
7. **[shiori-abi 依存の要否]** bytes transport seam の純度を保つため、本ユニットが `shiori-abi` に依存しない設計が可能か（依存すると ABI 実装が本ユニットへ滲む懸念）。→ 依存させず、下流で結線が seam 原則と整合。
8. **[HWND ライフタイム/正当性]** 相手窓消滅時（PeerGone）の観測。pilot は `SendMessageTimeout` の 0 返りを一律 Timeout 扱い（GetLastError で区別可能と注記）。production で PeerGone を分離観測するか。

---

## 5. 工数・リスク見積り (Effort & Risk)

| 観点 | 見積り | 根拠 |
|------|--------|------|
| **Effort** | **M（3–7 日）** | 構造は pilot で完全に実証済＝新規発明ゼロ。ただし「クリーン再掘（コピペ禁止）」＋ production 品質（thiserror・API 設計・両 target ビルド構成・echo シーム）＋テスト整備で S を超える。新パターン（production クレートでの i686 helper bin 同居）が数点。 |
| **Risk** | **Low〜Medium** | 実現可能性リスクは pilot go で**解消済み**（最大の耐力壁は突破）。残余リスク: ① 両 target ビルド構成の新規設計（Medium）② 再入受信の production 品質再現（Low・構造既知）③ echo↔pasta シームの下流整合（Medium・design で確定要）。並走性リスクは低い（別プロセス＝天然のアクター境界・parser/wintf と非衝突）。 |

---

## 6. design フェーズへの推奨事項

- **推奨アプローチ**: **Option B-1（単一クレート・helper を `[[bin]]` 内包）** または **Option C（proto 共有＋helper 内包）** を軸に検討。`shiori-abi` 相乗り（Option A）は最小依存純度を壊すため非推奨。クレート数（B-1=1 / C=2 / B-2=3）は M1 の最小実装方針と照らして依頼者確認。
- **鍵となる設計判断**:
  1. クレート配置・構成（B-1/B-2/C）と命名（`host32-ipc` 系・仮）
  2. echo helper の差し替えシーム（本ユニット echo／下流 pasta 注入の境界形）
  3. 両 target（x64 lib + i686 bin）ビルド構成と `windows` feature 最小セット
  4. `shiori-abi` 非依存で bytes seam を保つ方針の確認
  5. エラー型の `thiserror` 化とバリアント網羅
  6. ハンドシェイク完了ゲート（R3.3）の明示化手段
- **参照規律**: 実装は pilot README/REPORT の**検証結果を参照**し、コードは**一から掘る**（two-tunnel.md 掘り直し禁止・要件 8.4）。design は README 検証結果を二重化しない（No Hidden Shared Ownership）。
- **持ち越す Research 項目**: §4 の 1–8。

---

## 7. まとめ (Analysis Summary)

- **既存パターン**: pilot `crates/pilot/examples/shiori-host-32/` が transport（WM_COPYDATA framing・u32 LE HWND・再入 RESPONSE/ResponseSlot・非ブロッキング spawn 監視・HELLO handshake・timeout/wedge）を**go 判定済みで完全実証**。構造はそのまま「知見」として本ユニットへ移植可能（コピペは禁止）。
- **欠落能力**: production クレートとしての transport 実装が丸ごと Missing（ただし方向は確定）。加えて R6 の「意味を持たない生バイト echo」用の helper REQUEST ハンドラ（pilot の pasta 駆動を echo へ置換＋下流 pasta 注入シーム）が新規論点。
- **候補アプローチ**: Option B（新クレート・単一 or ペア）／Option C（proto 共有＋helper 内包）が本命。Option A（`shiori-abi` 拡張）は依存純度を壊すため非推奨。配置は design 議題。
- **Research フラグ**: 両 target ビルド構成・echo↔pasta シーム・`shiori-abi` 非依存方針・i686 ビルド規律・thiserror 化・R3.3 ゲート化（§4）。
- **見積り**: Effort M（3–7 日）／ Risk Low〜Medium（実現可能性は pilot go で解消済・残余は production 化と構成設計）。

---

## 8. 設計フェーズ・ディスカバリと統合 (Design Discovery & Synthesis) — 2026-07-01

> `kiro-spec-design` により追記。ディスカバリ種別 = **light（Extension / Complex Integration・pilot が feasibility を go 実証済ゆえ external web research 不要）**。全依存はピン済（`windows` 0.62.2 / `wintf-winmsg-executor` 0.0.5 / `event-listener` 5 / `thiserror` 2）で pilot 実行時検証済。ディスカバリは pilot 実装（`ipc.rs` / `process_host.rs` / `parent_window.rs` / `helper_window.rs`）と既存ワークスペース規約（structure.md / tech.md / `/Cargo.toml`）の精読に限定。

### 8.1 統合レンズ (Synthesis)

**Generalization**:
- R2〜R7 は「x64↔i686 の生バイト WM_COPYDATA トランスポート」という単一問題の変奏。共有 `ipc` モジュール（`MsgTag` / u32 LE HWND / `cbData` framing / `ResponseSlot` / 送信規約）を x64/i686 双方から共有する単一ソースとして設計し、跨ビットネス規約のズレを構造的に排除する。凍結する cross-unit seam は **WM_COPYDATA ワイヤ形式**（MsgTag ＋ framing）であって responder 実装ではない。

**Build vs. Adopt**:
- **Adopt**: `wintf-winmsg-executor` 0.0.5（message-only 窓・`MessageLoop::run`・WndProc dispatch。i686 実行時 GO を pilot が実証＝raw Win32 fallback 不要）。`SendMessageTimeout`（`SMTO_ABORTIFHUNG`）は Win32 ネイティブでハング回避を担う既存 API。`std::process`（spawn/try_wait）で生存監視。`thiserror` でエラー型。
- **Build**: production transport 一式（pilot は examples 隔離ゆえ丸ごと Missing）＋ **pasta 非依存の echo responder**（pilot の REQUEST は `ShioriByteProxy::shiori_request` を hardwire し selftest は entries=None で実往復しないため、echo 往復は genuinely 新規）。named pipe は不要（pilot が WM_COPYDATA 再入で go 実証・後退トリガのみ記録）。

**Simplification**:
- **`trait RequestHandler` を設けない（YAGNI）**。helper の REQUEST ブランチは平関数 `respond(&[u8]) -> Vec<u8>`（echo）とし、下流 `host32-shiori-load` が同クレートを編集して echo 行を pasta 駆動へ差し替える。差し替え点は「responder 実装」であって seam ではない（seam はワイヤ形式）。research §2.1 が候補とした trait/callback 注入は要件ディスカッションで YAGNI と確定済のため design では採らない。
- **distinct PeerGone を設けない**。送信失敗と timeout は `IpcError`（Timeout/SendFailed）に一様化。peer の生死は R1 の `poll_exit`（Clean/Abnormal/Terminated）で**別系統**に観測（R5 と R1 の関心分離）。research §4-8 の「PeerGone 分離観測」は不採用と確定。
- **常駐 lifecycle 状態機械を最小化**。helper の状態は echo 実証に必要な最小に留め、常駐 msg loop / OnSecondChange / unload 監視は Out of Boundary（下流 `host32-lifecycle`）。

### 8.2 設計判断 (Design Decisions)

| # | 判断 | 決定 | 根拠 / トレース |
|---|------|------|-----------------|
| D1 | クレート配置・構成 | **Option B-1（単一クレート・helper を `[[bin]]` 内包）を推奨**。ただし brief 明示要求により **design discussion で依頼者確認**（design.md Open Questions §1） | research §3・§6。`shiori-abi` 相乗り（A）は最小依存純度を壊す |
| D2 | `shiori-abi` 依存 | **非依存**（bytes seam の純度維持・ABI 実装は下流で結線） | research §4-7・§6 |
| D3 | REQUEST responder 差し替え | 平関数 `respond`（echo）を helper 窓層に置き、下流が編集差し替え。trait 抽象なし | 要件ディスカッション確定（YAGNI） |
| D4 | 送信失敗の表現 | Timeout/SendFailed に一様化・distinct PeerGone なし。生死は ExitKind で別系統 | 要件ディスカッション確定 |
| D5 | エラー型 | pilot 素 enum を `thiserror` へ昇格（IpcError / SpawnError / HandshakeError） | research §4-6・structure.md 規約 |
| D6 | `windows` feature | 本ユニットは `Win32_System_DataExchange`（＋ WindowsAndMessaging / Foundation）で足りる想定（Memory/Globalization は下流 pasta proxy 用ゆえ不要）。ビルド配線はタスクで確定 | research §4-2 |
| D7 | ハンドシェイクゲート（3.3） | 完了前の往復を型/実行時で拒否（`HandshakeIncomplete`）。具体形はタスクで確定 | research §4-5 |
| D8 | 32bit 可搬 | dwData/HWND shift 評価は `u64` cast（i686 `usize=32bit` overflow 回避）。i686 ビルド/テストは PowerShell 必須 | README 学び 3・brief |

### 8.3 設計レビューゲート結果

- **合格（1 パス・修復なし）**。機械チェック: 全 31 要件 ID が traceability に存在／Boundary 4 セクション充足（Owns5・Out3・Allowed5・Triggers3+）／File Structure Plan に具体パス／orphan コンポーネントなし／placeholder なし。判断レビュー: 要件網羅・アーキ準備性・境界明確性・実行可能性いずれも充足。要件レベルの gap/矛盾なし。唯一の真の未決事項（クレート配置）は brief 要求どおり Open Question として surface（papering over せず）。

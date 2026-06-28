# ギャップ分析（areka-P0-shiori-reference）

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5（SHIORI ホスティング）。
> 上流（完成）: `areka-P0-shiori-com`（`IShiori`/`IShioriHost` ABI・`shiori-abi` クレート）。
> 本書は要件（WHAT）と既存コードベースの実装ギャップを整理し、設計フェーズの判断材料を提供する（決定はしない）。

## 1. 分析サマリ（概観）

- **ABI・ホスト受け皿は完成済み**。欠落しているのは「非テストの `IShiori` 実装（リファレンス脳）」と、それを `main.rs` から挿して数往復ドライブする「実走デモ配線」の 2 点のみ。脳の振る舞い自体は既存テストモック（`MockBrain`/`DeferringBrain`/`StatefulBrain`）に完成形のテンプレートがある。
- **`shiori-abi` の公開 API は脳実装に十分**。`#[implement(IShiori)]` ＋ `IShiori_Impl`（raw vtable 実装）、`ShioriExt`（load/unload/request の安全変換）、`RequestOutcome`/`CorrelationToken`/`SHIORI_S_PENDING`/`SHIORI_E_NOT_LOADED` が揃い、リファレンス脳は ABI 面を一切変更せず実装できる（要件 1.3/8.3 と整合）。
- **areka 側のセッション規律も完成済み**。`ShioriSession`（単一 in-flight・遅延完了タイムアウト・`Unload` 保留取消）と `ShioriHostSink`（突合枠＋メールボックス）が非テストの製品コードとして存在し、`activate`→`request`→`poll_completions`→`unload` の利用面が確立している（要件 6.5 が要求する既存セッション規律はそのまま使える）。
- **最大の設計判断は「デモ配線をどこに置くか」**。`main.rs` は `WinThreadMgr` のブロッキングメッセージループ＋`bevy_ecs` World＋非同期 `CommandSender` で動く。リファレンス脳の `IShiori`（`#[implement]` は `!Send`/`!Sync` の COM オブジェクト）をどのスレッド／どのタイミングで駆動し、`poll_completions` をどう回すかが核心。観測手段（要件 6.4）は `tracing`（logging.md 準拠）が既定線。
- **リサーチフラグ**: (a) COM オブジェクトのスレッドアフィニティと bevy/async 実行系の往来、(b) 遅延完了をデモで「待ち合わせる」具体機構（ポンプ駆動 or タイマ）、(c) `windows_subsystem="windows"`（release）でコンソール出力が出ない点と観測手段の両立。いずれも設計フェーズで詰める。

## 2. 現状調査（既存資産）

### 2.1 `shiori-abi` クレート（上流・完成、変更不可の前提）

| 要素 | 場所 | 内容 |
|------|------|------|
| `IShiori` / `IShiori_Impl` | `crates/shiori-abi/src/interface.rs` | 脳が実装する唯一の COM 境界。`Load(host: *mut c_void)` / `Unload()` / `Request(input, out_response, out_token) -> HRESULT`。HSTRING 所有権規約・Request の 3 分岐（`S_OK`即時／`SHIORI_S_PENDING`遅延／error）を doc で固定。 |
| `IShioriHost` / `IShioriHost_Impl` | 同上 | areka が実装し脳へ渡す単一 sink。`Raise(script)` / `Complete(token, response)`。 |
| `ShioriExt`（load/unload/request） | `crates/shiori-abi/src/ergonomic.rs` | raw `unsafe fn -> HRESULT` を `Result<RequestOutcome, ShioriError>` へ写す安全層。areka はこの面越しに脳を操作する。 |
| `RequestOutcome` / `CorrelationToken` / `CorrelationTokenAllocator` | `crates/shiori-abi/src/outcome.rs` | 即時/遅延の内部表現と単調増加トークン採番。リファレンス脳が遅延トークンを発行する際に `CorrelationTokenAllocator` を再利用できる。 |
| HRESULT 定数 / `ShioriError` | `crates/shiori-abi/src/error.rs` | `SHIORI_S_PENDING`(0x20A1_0001)・`SHIORI_E_NOT_LOADED`・`SHIORI_E_UNKNOWN_TOKEN`。脳が遅延/未ロード拒否を返す際の語彙。 |

**重要**: `interface.rs` の IID は「dev 用・流動契約 D7（リリース時に凍結）」と明記。要件 8.3 の「流動契約への整合」は、リファレンス脳が ABI 変更時に in-tree 実装者として追従更新する想定と一致する。

### 2.2 areka 側の既存受け皿（製品コード・非テスト）

| 要素 | 場所 | 役割 |
|------|------|------|
| `ShioriHostSink`（`#[implement(IShioriHost)]`） | `crates/areka/src/shiori_host.rs` | 単一 sink。突合枠 `Mutex<Option<CorrelationToken>>` ＋ メールボックス `Mutex<VecDeque<HostMessage>>`。`Raise`/`Complete` を thread-safe に受けて投函。`HostMessage::{Raised, Completed}` を取り出して観測できる最小形。 |
| `ShioriSession` | `crates/areka/src/shiori_session.rs` | `activate`(=Load で sink 受け渡し)／`request`（単一 in-flight）／`poll_completions`（Complete/Raise 取り出し・保留解除）／`expire_if_elapsed`（決定的タイムアウト）／`unload`（保留取消）。`activate` が内部で `ShioriHostSink::new().into()` を生成し `ShioriExt::load` で脳へ渡す。 |

これらは `main.rs` で `mod shiori_host; mod shiori_session;` として宣言済み（`#![allow(dead_code)]` 付き＝現状は結合テストからのみ利用）。**デモ配線が入れば dead_code 警告は解消する。**

### 2.3 既存テストモック脳（リファレンス脳のテンプレート）

非テストのリファレンス脳が満たすべき各経路は、**既にテストモックとして実装済みで、製品コードへ昇格させる雛形になる**:

- 即時応答: `interface.rs`/`ergonomic.rs` の `MockBrain`（`out_response` に HSTRING を `core::ptr::write` で move-out して `S_OK`）。
- 遅延＋Complete／Raise: `shiori_e2e_tests.rs` の `DeferringBrain`（`Load` で host を `from_raw_borrowed`＋`cloned()` で AddRef 保持、`SHIORI_S_PENDING`＋token を返し、後から保持 host へ vtable 直呼びで `Complete`/`Raise`）。
- ライフサイクル＋未ロード拒否: `shiori_lifecycle_e2e_tests.rs` の `StatefulBrain`（`AtomicBool` でロード状態を保持し未ロード時 `SHIORI_E_NOT_LOADED`）。

**脳→host の呼び出しは raw メソッドが ABI 定義モジュール private のため `(Interface::vtable(host).Complete)(host.as_raw(), ..)` 形の vtable 直呼びが必須**（tasks 由来の Implementation Note）。リファレンス脳でも同じ技法を踏襲する。

### 2.4 areka 本体の実行モデル（デモ配線の着地先）

`crates/areka/src/main.rs`:
- `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` — release ビルドではコンソールが付かない（観測手段に影響・後述リスク）。
- `WinThreadMgr::new()?` → `mgr.world()`（`bevy_ecs` World）→ `world.borrow().spawn(|tx| async move { ... })` で非同期 UI 構築 → `mgr.run()?`（ブロッキングメッセージループ）。
- 非同期タスクは `CommandSender`（`tx.send(Box<dyn FnOnce(&mut World)>)`）で World を操作する。`async-io`/`bevy_tasks` 基盤（`doc/COMPAT_ARCHITECTURE.md` §6: 「OnSecondChange クロックと request 非同期圧送の土台」）。

デモ脳の `IShiori`（`#[implement]` 生成 COM オブジェクト）は `Send`/`Sync` を満たさない可能性が高く、**どのスレッドで生成・駆動し、`poll_completions` をどのタイミング（メッセージループ／World system／async タスク）で回すか**が配線の主論点。

## 3. 要件 → 資産マッピング（ギャップ表）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ判定 |
|------|----------------|----------|--------------|
| R1 非テストのリファレンス脳 | `#[implement(IShiori)]` 製品コード・固定/エコー応答 | `MockBrain` 等は `#[cfg(test)]` のみ | **Missing**（非テスト実装が無い。雛形はある） |
| R2 ライフサイクル（load/unload） | `Load` で host 保持・`Unload` で Release・未ロード拒否 | `StatefulBrain`/`DeferringBrain`（test） | **Missing**（製品コード化が必要） |
| R3 即時応答 | `out_response` move-out＋`S_OK`・content 不透明取り回し | `MockBrain`（test） | **Missing**（昇格） |
| R4 遅延応答（pending＋Complete） | `SHIORI_S_PENDING`＋token、保持 host へ `Complete`、トークン保持 | `DeferringBrain`（test）＋ `CorrelationTokenAllocator` | **Missing**（昇格）。トークン採番は既存アロケータ再利用可 |
| R5 能動通知（Raise） | 保持 host へ `Raise`、固定/既知文字列 | `DeferringBrain::fire_raise`（test） | **Missing**（昇格） |
| R6 実走デモ経路 | `main.rs` から activate→数往復 drive→unload、各経路の観測、セッション規律遵守 | `ShioriSession` 利用面・`tracing` 基盤・async/World | **Missing（配線）＋Constraint**（実行モデル整合が主論点） |
| R7 リファレンスとしての doc 化 | 各経路の正解見本説明・content 不透明方針・下流位置づけ | `doc/COMPAT_ARCHITECTURE.md` §5、各ソース doc コメント | **Missing（文書）**（置き場所は要決定：doc/ か module doc か） |
| R8 content 不透明性・スコープ境界 | UTF-16 HSTRING を不透明取り回し・正準プロトコル/DLL/pasta/conformance を実装しない・x64 前提 | ABI doc が不透明性を既に固定、Cargo は x64 ターゲット前提 | **Constraint**（境界の遵守。新規実装なし、規律として担保） |

ギャップの本質は **「テスト専用に存在する脳の振る舞いを、製品コードのリファレンス脳として 1 つに統合し、`main.rs` から実走させる」** こと。新規アルゴリズムや外部依存は不要で、既存パターンの再構成が中心。

## 4. 実装アプローチ案（A/B/C）

### Option A: 既存モジュールへ最小追加（脳を areka 内に新規 module、デモ配線を main.rs へ）

- リファレンス脳を `crates/areka/src/` の新規 module（例 `reference_brain.rs`）として `#[implement(IShiori)]` の製品コードで実装。即時/遅延/Raise/未ロード拒否を 1 つの脳に統合（既存 3 モックの和集合）。
- `main.rs` に小さなデモ関数（`activate`→数往復 `request`→遅延完了待ち→`Raise`→`unload`）を追加し、起動時に一度駆動。観測は `tracing::info!`。
- **トレードオフ**: ✅ 最小ファイル・既存 `ShioriSession` をそのまま利用・パターン踏襲で速い。✅ shiori-abi/host/session に手を入れない。❌ `main.rs` の実行モデル（COM スレッドアフィニティ・遅延の待ち合わせ）と噛み合わせる配線判断が残る。❌ 脳とデモが areka に同居し、下流が「見本」として参照する際にテスト/デモ用と製品用の線引きが要る。

### Option B: 脳とデモを独立コンポーネント化（脳 module＋デモ driver module を分離、必要なら example も）

- リファレンス脳 module（純粋な `IShiori` 実装・観測しやすい固定挙動）と、デモ driver module（セッション規律の駆動・各経路の観測ログ）を責務分離。`main.rs` は driver を呼ぶだけ。
- 手動検証用に `crates/areka/examples/` か `crates/wintf/examples/` 流儀の example バイナリでデモを切り出す案も含む（structure.md は examples を「手動検証用サンプル」と位置づけ）。
- **トレードオフ**: ✅ 関心の分離が明確で、下流の「見本」参照点として脳 module 単体が読みやすい。✅ デモを main 起動から切り離せば UI（シェル/バルーン）と干渉しにくい。❌ ファイル数増・driver と main の接続インターフェイス設計が必要。❌ example 化すると `windows_subsystem` のコンソール問題は回避できるが「実アプリ（areka 本体）で動く証明」という要件 R6 の狙いとずれる懸念（証明の場が本体か example か、は設計判断）。

### Option C: ハイブリッド（脳は独立 module、デモは main.rs に最小フック＋観測は tracing、待ち合わせは既存ポンプに相乗り）

- 脳は独立 module（Option B の脳分離）として製品コード化し、デモ駆動は `main.rs` の起動経路へ最小フックで載せる（Option A のデモ配線）。遅延完了の「待ち合わせ」は既存の非同期/メッセージループのティックに `poll_completions` を相乗りさせる（毎フレーム or タイマで drain）。
- **トレードオフ**: ✅ 脳の見本性（分離）とデモの本体実走性（main 駆動）を両取り。✅ 既存セッション規律・観測基盤を最大活用。❌ 待ち合わせ機構を実行モデルへ載せる箇所の設計（World system / async / メッセージタイマ）を確定する必要があり、配線の複雑さは A より高い。

> いずれの案でも shiori-abi（上流・完成）と areka 側 `ShioriHostSink`/`ShioriSession` は変更しないのが前提（要件 1.3/8.3）。差異は「脳の置き場所」と「デモ駆動・待ち合わせの実装モデル」に集約される。

## 5. 工数・リスク評価

| 項目 | 評価 | 根拠 |
|------|------|------|
| **工数** | **S（1〜3日）** | 新規アルゴリズム・外部依存なし。既存テストモックを製品コードへ昇格＋既存 `ShioriSession` 利用面で配線するのみ。最も時間を要するのは「実行モデルへの待ち合わせの載せ方」確定とデモの観測整備。 |
| **リスク** | **Low〜Medium** | Low 要因: ABI/host/session が完成・確立パターンの再利用・スコープ明確。Medium 要因: COM オブジェクトのスレッドアフィニティと bevy/async 実行系の往来、release ビルドのコンソール非表示と観測手段の両立、遅延完了の待ち合わせ機構の設計が未確定。 |

## 6. 設計フェーズへ持ち越すリサーチ項目（Research Needed）

1. **COM スレッドアフィニティ**: `#[implement(IShiori)]` 生成オブジェクト（`!Send`/`!Sync` 想定）を、areka の UI スレッド／`mgr.run()` メッセージループ／async タスクのどこで生成・駆動するか。in-proc 直 vtable は呼び出しスレッド上実行（interface.rs doc）なので、デモ駆動スレッドと `poll_completions` ドレイン箇所を同一規律に収める必要がある。
2. **遅延完了の待ち合わせ機構**: デモが遅延 request 後に `Complete` を待つ具体手段。リファレンス脳が「いつ」`Complete` を発火するか（同期直後・別ティック・タイマ）と、areka 側 `poll_completions` の駆動契機（毎フレーム system・async ループ・OnSecondChange 相当のティック）。`expire_if_elapsed` の決定的タイムアウトをデモで使うか。
3. **観測手段とビルド構成**: release で `windows_subsystem="windows"` によりコンソールが無い。`tracing`（logging.md 準拠・既定 info）で各経路（load/即時/遅延/Raise/unload）の疎通を info ログとして出すのが既定線。要件 6.4「利用者または開発者が観測可能」を満たす出力形式（ログのみ／UI への反映有無）を確定する。
4. **doc の置き場所（要件 7）**: 「正解見本」文書を `doc/`（COMPAT_ARCHITECTURE と並ぶ独立文書）に置くか、リファレンス脳 module の module-level doc に集約するか、両方か。下流（host-32／pasta）が参照しやすい単一の参照点を決める。
5. **デモの起動形態**: areka 本体起動時に常に走らせるか、フラグ/環境変数でデモを有効化するか、example バイナリに切り出すか。R6 の「実アプリ上で動く証明」と通常起動時の UI 体験（シェル/バルーン）の両立。
6. **トークン採番**: 遅延トークンを固定値にするか `CorrelationTokenAllocator`（既存）で採番するか。リファレンスとしては採番を見せる方が見本価値が高い可能性。
7. **コンストラクタ取得機構（要件 9）**: 純粋Cコンストラクタ `shiori_create`（`extern "system"`＝COM 標準 stdcall・`#[unsafe(no_mangle)]`・`HRESULT shiori_create(IShiori** out)`）を、(A) リファレンスを実 `cdylib` 化し areka が `LoadLibraryW`＋`GetProcAddress("shiori_create")` で実 DLL 境界を渡って取得するか、(B) in-tree シンボル直呼びへ縮退するか。「DLL 契約の正解見本」を名乗るなら (A) が忠実だが、デモ配線（§6-1/2）の複雑さとのトレードオフで決定。シンボル可視性・エクスポート設定（`.def`／`cdylib` crate-type）も併せて確定する。
8. **ARM64 ビルド（要件 8.3）**: 対象が x64 ＋ ARM64 へ拡張。content 不透明・呼出規約一意（x86 除外）のため実装差は無い想定だが、ARM64 ターゲットのビルド/CI 確認とクロスビルド経路を設計フェーズで点検する。

## 7. 設計フェーズへの推奨

- **推奨アプローチの起点**: Option C（脳は独立 module で見本性を担保しつつ、デモは areka 本体の起動経路で実走させる）を軸に、§6 のリサーチ 1〜2（スレッドアフィニティ・待ち合わせ機構）の結論次第で A（最小・main 同居）へ縮退する余地を残す。
- **変更しない境界の明文化**: shiori-abi・`ShioriHostSink`・`ShioriSession` は不変。リファレンス脳は ABI 面を変えず、既存セッション規律を利用する（要件 1.3/6.5/8.3）。
- **content 不透明性の徹底**: 応答は固定文字列 or 受信 content のエコーのみ。パース・スキーマ・意味づけを一切持ち込まない（要件 1.4/8.1）。正準プロトコル・DLL ホスト・pasta・conformance は本仕様で実装しない（要件 8.2）。
- **持ち越すリサーチ**: §6 の 6 項目を design.md の Discovery/技術調査で確定する。

---

# 設計フェーズ追記（Discovery: Light / Extension）

> 上記ギャップ分析（§1〜§7）に対し、設計フェーズで §6 の Research Needed 8 項目を決定した記録。既存コードの実シグネチャ確認（`shiori-abi`・`shiori_host.rs`・`shiori_session.rs`・`main.rs`・既存テストモック）に基づく。

## Discovery サマリ（設計フェーズ）
- **Discovery Scope**: Extension（light discovery）。新規アルゴリズム・外部依存なし。確定済み ABI（`shiori-abi`）と既存受け皿（`ShioriHostSink`/`ShioriSession`）の再構成が中心。
- **検証済み実シグネチャ**: `IShiori::{Load,Unload,Request}`（`Request(input:*const HSTRING, out_response:*mut HSTRING, out_token:*mut u64)->HRESULT`）、`IShioriHost::{Raise,Complete}`、`ShioriExt::{load,unload,request}->Result<RequestOutcome,ShioriError>`、`CorrelationTokenAllocator::next()->CorrelationToken`、`ShioriSession::{activate,request,poll_completions,expire_if_elapsed,unload}`、`HostMessage::{Raised,Completed}`、HRESULT 定数（`SHIORI_S_PENDING`=0x20A1_0001 / `SHIORI_E_NOT_LOADED`=0xA0A1_0002 / `SHIORI_E_UNKNOWN_TOKEN`=0xA0A1_0003）。
- **昇格元テンプレ**: `MockBrain`（即時 move-out）/`DeferringBrain`（host を `from_raw_borrowed`＋`cloned()` で AddRef 保持、vtable 直呼びで `Complete`/`Raise`）/`StatefulBrain`（`AtomicBool` ＋未ロード拒否）の和集合を 1 つの製品コード脳へ統合。

## 設計判断（§6 Research Needed の決定）

### Decision 1: デモ駆動スレッドと遅延待ち合わせ（§6-1 / §6-2）
- **Context**: `#[implement(IShiori)]` 生成 COM オブジェクトは `!Send`/`!Sync` 想定。`main.rs` は `WinThreadMgr` ブロッキングメッセージループ＋`bevy_ecs` World＋async `CommandSender` で動く。どのスレッドで脳を生成・駆動し `poll_completions` を回すか。
- **Alternatives**: (A) bevy World system／async タスクに `poll_completions` を相乗り（毎フレーム drain）。(B) `mgr.run()` 前に main スレッドで同期完結するデモ関数を一度だけ駆動。
- **Selected**: (B)。デモ脳・`ShioriSession`・`poll_completions` を **すべて main スレッド上**で、`mgr.run()`（メッセージループ）に入る前に同期完結させる。リファレンス脳は遅延 request に対し `Request` 内で即トークンを発行し、`Complete` をデモドライバが明示トリガするタイミング（request 直後の同一ループ反復）で同期発火する。`poll_completions` を同ループで drain して突き合わせる。
- **Rationale**: COM スレッドアフィニティ問題（COM オブジェクトと sink・session が同一スレッドに収まる）と遅延待ち合わせ問題（タイマ／クロスタスク不要・決定的）を同時に解消。`ShioriSession` の単一 in-flight・`poll_completions`・`expire_if_elapsed` 利用面をそのまま使う。実時間 sleep 非依存。
- **Trade-offs**: ✅ 配線最小・決定的・スレッド規律単純。❌ 「非同期実行系での遅延圧送」の実演にはならない（本仕様の狙いは各経路の疎通証明であり、非同期圧送は下流／別仕様の範疇のため許容）。
- **Follow-up**: メッセージループ前駆動が UI 立ち上げ（シェル/バルーン）を阻害しないこと（同期完結のため短時間）を実装時に確認。

### Decision 2: コンストラクタ取得機構 `shiori_create`（§6-7）
- **Context**: R9 が「`IShiori` を生成する唯一の純粋 C コンストラクタ `shiori_create`」を要求。実 DLL 境界（cdylib＋`LoadLibraryW`）越しに取得するか、in-tree シンボル直呼びか。
- **Alternatives**: (A) リファレンスを `cdylib` 化し areka が `LoadLibraryW`＋`GetProcAddress("shiori_create")` で実 DLL 境界を渡る。(B) in-tree モジュールが `extern "system"` `#[unsafe(no_mangle)]` で `shiori_create` を公開し、areka が直接シンボル呼び出し。
- **Selected**: (B)。リファレンス脳は areka 内部モジュール（`reference_brain.rs`）として `pub unsafe extern "system" fn shiori_create(out: *mut *mut c_void) -> HRESULT` を `#[unsafe(no_mangle)]` で公開し、デモドライバは in-tree でこの関数を直接呼ぶ。呼出規約は COM/STDAPI 標準の stdcall（`extern "system"`・x64/ARM64 で `extern "C"` と同一 ABI）、C リンケージ（非マングル）は `#[unsafe(no_mangle)]`。署名・所有権（参照カウント 1 を out 引数で move-out、失敗時 out 未書込・失敗 HRESULT）は R9.2〜R9.4 を厳密に満たす純粋 C 入口形を採る。設計ディスカッションで開発者が (B) in-tree 直呼びを確認（実 DLL ロード／cdylib＋LoadLibrary は host-32 の責務へ委譲・R9.7）。
- **Rationale**: R9.7 が本コンストラクタ契約の対象を **COM（x64/ARM64・in-proc）経路の生成入口に限定**し、32bit DLL ホスティング固有の DLL 境界は明示的に `areka-P0-shiori-host-32` の責務。cdylib 化は §6-1/2 のスレッド・配線複雑さを増やすだけで R6 の「実アプリ（areka 本体）で動く証明」に寄与しない。純粋 C 署名そのものが下流の「正解見本」であり、DLL ロード経路は host-32 が本見本を参照して実装する。
- **Trade-offs**: ✅ 配線最小・スレッド規律単純・署名は忠実な見本。❌ 実 DLL ロード経路は本仕様で実走しない（host-32 の責務に正しく委譲）。
- **Follow-up**: `extern "system"` シンボルが in-tree 直呼びでも `#[unsafe(no_mangle)]` 名で解決できること、将来 host-32 が同シンボルを `GetProcAddress` で引ける署名であることを doc で明記。

### Decision 3: 観測手段とビルド構成（§6-3）
- **Selected**: 各経路（load/即時/遅延/Raise/unload/shiori_create）を `tracing::info!` で出力（`logging.md` 準拠・スコーププレフィックス `[ref-brain]`/`[shiori-demo]`・構造化フィールド優先）。デモは debug ビルド（`windows_subsystem="windows"` 非適用＝コンソール有）または `RUST_LOG` 制御で観測。
- **Rationale**: R6.4 は「開発者が構造化 tracing ログで観測可能」を要求し視覚 UX 非依存。release のコンソール非表示は debug 実行／`RUST_LOG` で回避。

### Decision 4: doc の置き場所（§6-4）
- **Selected**: 単一の参照点をリファレンス脳モジュール（`reference_brain.rs`）の module-level doc（`//!`）に集約し、各経路（ロード／アンロード・即時・遅延・Raise・`shiori_create`）の正解見本説明・content 不透明方針・下流位置づけを記述。`doc/COMPAT_ARCHITECTURE.md` §5 からの参照リンクのみ追加（内容複製はしない）。
- **Rationale**: R7 が「下流が参照しやすい単一の参照点」を要求。content 語彙の二重定義禁止（R8.1・正準プロトコルは `areka-P0-shiori-protocol`／`doc/shiori/fragments/`）に整合させ、module doc は content スキーマを持たず不透明方針のみ記す。

### Decision 5: トークン採番（§6-6）
- **Selected**: 遅延トークンは `CorrelationTokenAllocator`（既存・`shiori-abi`）を脳内に保持して `next()` で採番。固定値は使わない。
- **Rationale**: リファレンスとして「単調増加トークン採番を見せる」見本価値が高い。既存アロケータ再利用で新規実装ゼロ。

### Decision 6: ARM64 ビルド（§6-8）
- **Selected**: x64 ＋ ARM64（CPU ネイティブ前提・x86 除外）。正準呼出規約は COM 標準 `extern "system"`（stdcall）で、対象各プラットフォームでは `extern "C"` と同一 ABI（R9.2）。実装差は無く、ビルド／CI 上の ARM64 ターゲット確認のみ。
- **Rationale**: content 不透明・呼出規約一意のため、ソース上の分岐は不要。

## Build-vs-Adopt / Simplification（Synthesis 結果）
- **Generalization**: 即時／遅延／Raise／未ロード拒否は「1 つの `IShiori` 実装の各経路」であり、3 テストモックの和集合を **単一リファレンス脳**へ統合（インターフェイス＝`IShiori` のまま、実装は最小）。脳とデモドライバの 2 責務に分離（脳＝見本性、ドライバ＝本体実走性）。
- **Adopt**: `shiori-abi`（`IShiori`/`ShioriExt`/`CorrelationTokenAllocator`/HRESULT 定数）・areka `ShioriHostSink`/`ShioriSession` を不変で採用。新規アルゴリズム・外部依存なし。
- **Simplification**: cdylib／LoadLibrary 経路を排除（host-32 へ委譲）。非同期圧送・タイマ・クロススレッド待ち合わせを排除（同期 main-thread 駆動）。`example` バイナリ案は不採用（R6 の「areka 本体で動く証明」を満たすため main フック＋フラグゲートを採用）。

## Risks & Mitigations（設計フェーズ）
- **COM スレッドアフィニティ** — main スレッド同期完結で回避（Decision 1）。
- **release コンソール非表示と観測** — debug 実行／`RUST_LOG` で回避（Decision 3）。
- **`#![allow(dead_code)]` 解消** — デモ配線が入ると `shiori_host`/`shiori_session` の dead_code が解消（ギャップ §2.2）。実装時に `main.rs` の allow 属性整理を確認。
- **境界逸脱（content 語彙の混入）** — 応答は固定文字列 or エコーのみ。パース／スキーマ／意味づけ禁止（R8.1）。doc は不透明方針のみ（Decision 4）。

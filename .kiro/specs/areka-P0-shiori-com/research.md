# ギャップ分析: areka-P0-shiori-com

> 本書はギャップ分析（情報提供）であり、最終的な実装方針の決定ではない。設計フェーズ（`/kiro-spec-design`）の入力として用いる。
> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。

## 0. 分析サマリー（概要）

- **既存パターン**: areka ワークスペースは windows-rs（`windows` 0.62.2 / `windows-core` 0.62.2）で **既存 COM インターフェイスを「消費」または `#[implement]` で「実装」**する流儀は確立しているが、**自前の GUID/vtable を持つカスタム COM インターフェイスを `#[interface]` で「定義」した実績はゼロ**。本仕様は「COM インターフェイスを新規定義する」という、このコードベースにとって初の領域に踏み込む。
- **不足している能力**: `IShiori` / `IShioriHost` の **インターフェイス定義そのもの**、それらを置くモジュール/クレート、HSTRING を引数・戻り値とする COM メソッド契約、in-proc アクティベーション経路、push(sink) 受け渡し機構 — いずれも未実装。SHIORI/脳に関するコードは現在コードベースに一切存在しない（areka は main.rs + tests.rs のみの試作バイナリ）。
- **候補アプローチ**: (A) 既存 `crates/wintf/src/com/` に同居、(B) 新規クレート `crates/areka-shiori`（または `crates/shiori-abi`）として独立、(C) ハイブリッド（ABI 定義は新規軽量クレート、in-proc アクティベーション配線は areka 本体）。本仕様のスコープ（ABI 定義 + ネイティブ in-proc 経路）と「`areka-P0-shiori-host-32` が同 `IShiori` を別実装する」という下流要件を踏まえると **(B) または (C) が有力**。
- **リサーチフラグ**: windows-rs `#[interface]` マクロの正確な利用形・派生制約（`IUnknown` 継承、IID 指定、`#[implement]` との組み合わせ）、HSTRING を ABI 引数に置いたときの `#[implement]` 自動マーシャリング非発生の確証、`IShioriHost` を脳へ渡す際の参照カウント/ライフタイム設計、`load` 時の sink 受け渡しシグネチャ — これらは設計フェーズで詰める「Research Needed」。
- **規模/リスク見立て**: ABI 定義 + 最小のネイティブ in-proc 経路で **Effort = M（3〜7日）／Risk = Medium**。新パターン（自前 COM 定義）導入と HSTRING/マーシャリング不変条件の検証が主リスク。

---

## 1. 現状調査（Current State）

### 1.1 ワークスペース構成と配置候補

- マルチクレート構成 `crates/*`（`wintf` / `dola` / `areka`）+ ベンダリング `vendors/pasta`（git submodule, `[patch.crates-io]`）。
- COM ラッパー層は `crates/wintf/src/com/`（`dcomp.rs`, `d3d11.rs`, `dwrite.rs`, `dxgi`(=`mod`), `wic.rs`, `ulw.rs`, `animation.rs`, `d2d/`）。`mod.rs` で束ねる。
- `crates/areka/src/` は `main.rs` と `tests.rs` のみ。**SHIORI/脳/ghost に相当する既存資産は存在しない**（`shiori`/`brain`/`ghost` の grep ヒットは cue registry・pointer dispatch test の無関係語）。
- レイヤー分離規約（steering structure.md）: **COM → ECS → Message Handling** の依存方向を厳守。`unsafe` は COM ラッパー層へ集約。

### 1.2 COM の使われ方（重要な発見）

- **消費**: windows-rs が生成する既存 COM 型（`IDCompositionDevice`, `ID2D1DeviceContext`, `IDWriteFactory2` 等）を多数利用。
- **実装**: `crates/wintf/src/com/d2d/command_sink.rs` が **唯一の `#[implement]` 実例** —
  ```rust
  #[implement(ID2D1CommandSink5)]
  pub struct RecCommandSink { ... }
  impl ID2D1CommandSink_Impl for RecCommandSink_Impl { ... }
  ```
  ただしこれは **既存の windows-rs 定義済みインターフェイスを Rust 側で実装**するもの。
- **定義（=本仕様で必要なもの）**: 自前の GUID/vtable を持つ **新規 COM インターフェイス定義（`windows_core::interface` / `#[interface(...)]`）の使用実績は皆無**。`grep` で `#[interface`・`com_interface`・`GUID(`・`core::interface` のいずれもヒットなし。
- 含意: 本仕様は「windows-rs で**カスタム COM インターフェイスを新規定義する**」という、このプロジェクト初の技法を導入する。設計フェーズで windows-rs の `interface` マクロ仕様を確定する必要がある。

### 1.3 文字列・依存・features

- HSTRING は既に wintf 内で広く使用（`window_system.rs`, `process_singleton.rs`, テキスト系, テスト等 10+ ファイル）。`windows-core` の `HSTRING` は純 Rust 実装で WinRT 非依存（steering/正本ドキュメント §5 で明記）— 本仕様の R4「WinRT 非依存」前提と整合。
- ルート `Cargo.toml` の `windows` features には `Win32_System_Com` が既に含まれる（COM 基盤は有効）。`windows-core` は workspace dependency として 0.62.2 固定。
- エラー規約（steering tech.md）: Windows API 境界は `windows::core::Result` を使用、内部エラーは `thiserror` enum + `#[from]`。R7（COM HRESULT に沿ったエラー報告）と素直に整合。

### 1.4 命名・規約の整合先

- 完了済み `com-resource-naming-unification` は **ECS コンポーネント命名**（GPU=`XxxGraphics` / CPU=`XxxResource`）と **COM アクセスメソッド命名**（`target()`→`Option<&IDCompositionTarget>` 等）を定める。R7-3 が参照する「既存 COM 命名規約」は主にこれ。**COM インターフェイス名の規約（`IXxx`）は明文化されていない**が、`IShiori`/`IShioriHost` という `I` プレフィックスは windows-rs/COM 慣習および既存消費型と整合する。

---

## 2. 要件 → 既存資産マップ（ギャップ tag: Missing / Unknown / Constraint）

| 要件 | 技術的ニーズ | 既存資産 | ギャップ |
| --- | --- | --- | --- |
| R1 内部唯一 ABI（同一視） | `IShiori` を唯一の境界として定義し分岐をアクティベーション経路へ局所化 | なし | **Missing**（ABI 定義そのもの） |
| R2 ライフサイクル load/unload | load/unload メソッド、未ロード時の request 拒否 | なし | **Missing** + **Unknown**（メソッドシグネチャ・状態管理方式） |
| R3 リクエスト処理 hrequest | 文字列 in → 文字列 out の request メソッド | HSTRING 取り回し実績あり | **Missing**（メソッド定義） |
| R4 文字列 HSTRING/UTF-16・WinRT 非依存 | 全引数/戻り値を HSTRING、OOP 自動マーシャリング非要求 | HSTRING 純 Rust 実装、WinRT 非依存は確認済み | **Constraint**（不変条件の維持）+ **Unknown**（`#[implement]` で HSTRING 引数がマーシャリングを誘発しない確証） |
| R5 ネイティブ in-proc アクティベーション | 同一プロセス内で `IShiori` 実装へ到達する経路、x64 前提 | windows-rs COM、in-proc 利用は通常 | **Missing**（アクティベーション API）+ **Constraint**（x86 除外） |
| R6 push 経路（IShioriHost sink） | areka 実装の sink を load 時に脳へ渡し、`Raise(HSTRING)` で能動通知 | `#[implement]` 実装パターン（command_sink）あり | **Missing**（sink 定義 + 受け渡し機構）+ **Unknown**（ライフタイム/参照カウント） |
| R7 エラー報告規約 | HRESULT に沿った成否報告、命名規約整合 | `windows::core::Result`/`thiserror` 規約あり | **Constraint**（既存規約に合わせる）— ギャップ小 |

**複雑性シグナル**: 単純 CRUD ではなく「外部統合（COM ABI 境界）+ ライフサイクル状態機械 + 非同期 wakeup 経路」。アルゴリズム的難所は少ないが、ABI 契約の正確さ（後続の 32bit ホスト/pasta が同契約を実装する）が要。

---

## 3. 実装アプローチ案（A / B / C）

### Option A: 既存 wintf COM ラッパー層に同居（Extend）
- **対象**: `crates/wintf/src/com/` 配下に `shiori.rs`（または `shiori/` ディレクトリ）を追加し `mod.rs` で束ねる。
- **トレードオフ**:
  - ✅ 新規クレート不要・既存の COM/unsafe 集約方針に乗る。
  - ✅ HSTRING・windows-core の既存依存をそのまま使える。
  - ❌ `wintf` は「Windows **UI** 基盤」であり、SHIORI/脳 ABI はドメインが異なる。責務がにじむ。
  - ❌ 下流 `areka-P0-shiori-host-32`（i686 別バイナリ）が同 `IShiori` 定義を共有する必要があるが、wintf 全体を 32bit 随伴バイナリへ引き込むのは過剰結合。

### Option B: 独立 ABI クレートを新設（New）
- **対象**: `crates/areka-shiori`（または `crates/shiori-abi`）を新規作成。`IShiori`/`IShioriHost` の **インターフェイス定義のみ**を最小依存（`windows-core` 等）で持つ。
- **トレードオフ**:
  - ✅ 責務が明快。ABI が独立クレートとして再利用可能。
  - ✅ **下流の 32bit ホスト/pasta が同一クレートに依存して同 `IShiori` を実装**できる（呼び出し側の同一視という R1 の核と最も整合）。
  - ✅ 依存最小化で 32bit ターゲットにも乗せやすい。
  - ❌ クレート分割の初期コスト（Cargo 設定・workspace members は `crates/*` グロブなので追加は容易）。
  - ❌ areka 本体との配線（in-proc アクティベーション）はこのクレート外に置く設計判断が要る。

### Option C: ハイブリッド（ABI 定義クレート + areka 本体に配線）（Hybrid）
- **対象**: ABI 定義は Option B の軽量クレート、**in-proc アクティベーション経路と `IShioriHost` の areka 側実装**は areka 本体（または別モジュール）に配置。
- **トレードオフ**:
  - ✅ 「ABI（安定契約）」と「アクティベーション（実装種別差の局所化点・R1-5）」を物理的に分離でき、R1 の「差異をアクティベーション経路のみへ局所化」を構造で表現できる。
  - ✅ 下流（32bit ホスト/pasta）は ABI クレートだけに依存し、areka 本体の配線には依存しない。
  - ❌ 分割境界の設計が要る（どこまでが ABI で、どこからが配線か）。計画コストが Option A/B より高い。

> 本仕様スコープ（ABI 定義 + ネイティブ in-proc 経路）と下流の同一視要件を重視するなら **B/C 系**が素直。最終決定は設計フェーズ。

---

## 4. Research Needed（設計フェーズへ繰り越す未決事項）

1. **windows-rs カスタム COM インターフェイス定義の正確な技法**: `#[interface("<IID-GUID>")]`（`windows_core::interface`）の利用形、`IUnknown` 継承の書き方、IID の採番方針、`#[implement]` との併用可否。0.62 系での API 形を確認すること。
2. **HSTRING を ABI 引数に置いたときのマーシャリング非発生の確証**: in-proc（同一プロセス・vtable 直叩き）であれば自動マーシャリングは発生しない想定だが、`#[implement]` 生成コード経由で HSTRING 引数/戻り値が WinRT マーシャリングを誘発しないことを R4-3 の不変条件として実証する方法。
3. **`IShioriHost` の受け渡しシグネチャとライフタイム**: `load` メソッドが sink を引数に取るか、別メソッドで set するか。脳側が保持する間の参照カウント/循環参照（脳⇄host）の回避策。
4. **ライフサイクル状態管理**（R2-4 未ロード時 request 拒否）: 状態を ABI 側で持つか呼び出し側（areka）で持つか。COM 的には `RPC_E_*` / 専用 HRESULT のどちらで拒否を表すか。
5. **エラー HRESULT 設計**（R7）: 成功/失敗の HRESULT マッピング、独自 FACILITY/カスタム HRESULT を定義するか既存標準コードに乗せるか。
6. **クレート分割の最終判断**（§3 A/B/C）と 32bit ホストとの共有方法（同クレートを i686 ターゲットでもビルド可能にする feature/依存設計）。

---

## 5. 規模・リスク評価

- **Effort: M（3〜7日）** — ABI 定義 + 最小のネイティブ in-proc アクティベーション + sink 受け渡しの土台まで。既存 HSTRING/COM 資産を活用できるため XL ではないが、自前 COM 定義という新パターン導入で S には収まらない。
- **Risk: Medium** —
  - 新パターン（カスタム COM インターフェイス定義）導入だが、windows-rs にガイド/マクロが存在し、`#[implement]` の前例（command_sink）もあるため High ではない。
  - 主リスクは (a) HSTRING/マーシャリング不変条件（R4）の実証、(b) ABI 契約の安定性（後続の 32bit ホスト/pasta が同契約を実装するため後戻りコストが高い）、(c) sink ライフタイム/循環参照。
  - x86 はスコープ外（R5-3）のため bitness 連鎖リスクは本仕様では回避済み（32bit は `areka-P0-shiori-host-32` 側へ分離）。

---

## 6. 設計フェーズへの推奨

- **推奨アプローチ**: Option B または C（独立 ABI クレート）を起点に検討。理由は R1 の「呼び出し側同一視」と下流 `areka-P0-shiori-host-32`／pasta が同 `IShiori` を実装する要件を、クレート境界で素直に満たせるため。
- **先に確定すべき設計判断**:
  1. ABI の置き場所（新規クレート vs wintf/com 同居）。
  2. windows-rs `#[interface]` による `IShiori`/`IShioriHost` の具体メソッド面（load/unload/request/Raise）と HSTRING シグネチャ。
  3. IID 採番と命名（`IShiori`/`IShioriHost`、既存 COM 慣習との整合）。
  4. sink 受け渡しとライフサイクル状態の所有者。
- **繰り越すリサーチ項目**: §4 の 1〜6。

---

## 7. 要件ディスカッション由来の追加設計判断（議題1: 同期/遅延リクエスト）

> 議題1（request の同期性と非同期応答）で確定した要件変更に伴い、設計フェーズで詰める HOW を追記する。要件側の決定は requirements.md（R3 同期＋遅延、R6 遅延完了経路）に反映済み。COM ABI レベルでは `async fn` を公開できないため、「同期呼び出し＋遅延コールバック」でモデル化する点が前提。

- **D1. request 結果の表現方式**: 「即時応答／遅延（HTTP 204 相当）／失敗」を COM ABI でどう区別するか。候補: (a) 成功 HRESULT を分ける（`S_OK`=即時応答あり、カスタム成功コード `SHIORI_S_PENDING`=遅延、失敗=error HRESULT）＋応答 HSTRING の out-param、(b) `#[repr(i32)]` の `ResCode` enum を out-param に追加。§4-5（エラー HRESULT 設計）の拡張。
- **D2. 相関トークンの表現**: トークンの型・サイズ（例: u64 / GUID）、発行・寿命・再利用ポリシー。in-proc 単一脳前提での最小実装でよい。
- **D3. 遅延完了メソッドのシグネチャ**: `IShioriHost` に `Raise` とは別に設ける完了メソッド（相関トークン＋応答 HSTRING を受け取る）の COM シグネチャ。§4-3（sink 受け渡し／ライフタイム）に併合して検討。
- **D4. 2層構造（ABI＋エルゴノミック変換トレイト）**: 生 `#[interface]`（`unsafe fn -> HRESULT`）の上に、Rust 風の `Result<RequestOutcome, ShioriError>` を返す変換層を手書きで被せる方針。`windows` クレートが自動生成する人間向けラッパーを、自前インターフェイスでは手で再現する。`RequestOutcome { Immediate(HSTRING), Deferred(Token) }` のようなデータ enum は ABI 非公開（Rust 内部のみ）とする。§4-1（`#[interface]` 技法）に関連。

### 議題2由来（単一正準 content プロトコル）

- **D5. 正準 content プロトコルの具体形**: `IShiori` 境界の単一正準プロトコル（要件 R1-6 で不変条件化）の具体形を設計で確定する。候補: (a) 独自 JSON スキーマ、(b) **json-rpc 2.0 そのものを採用する**（開発者案）——議題1で決めた「即時／遅延／失敗」と相関トークンが json-rpc の `id`／`result`／`error` 構造に素直に乗るため相性が良い（遅延＝`id` のみ先行返却し `result` は後続の sink 完了で配送）。中に入るさくらスクリプトは不透明文字列のまま（解釈は別仕様）。プロトコルのバージョニング方針は議題3の結論に従う。
- **D6. 高レート通信対応**: ネイティブ SHIORI（pasta）は従来 SHIORI（`OnSecondChange` 等で数 Hz）より**高頻度の通信にも対応し得る**前提を置き、正準プロトコルの効率（通知＝応答不要メッセージの扱い、バッチ、パース負荷）を設計時に考慮する。これは json-rpc 採用（通知＝`id` なし、バッチ要求のサポート）を後押しする要因の一つ。注: 通常の SHIORI 通信頻度ではパースコストは無視できる水準であり、本項は将来の高レート用途を見据えた設計余地の確保が目的。

### 議題3由来（契約の安定化方針）

- **D7. プレリリース流動契約 → リリース時凍結**: 本仕様の `IShiori`/`IShioriHost` は**リリースまで流動的な契約**とし、後方互換保証・明示的バージョニング機構は持たない（要件 Out of scope に明記）。作りこみ段階のインターフェイス変動を許容し、変更時は in-tree の全実装者（areka 本体・`areka-P0-shiori-host-32`・pasta）を **lockstep で再ビルド・更新**する。32bit 互換ホストは別バイナリ（プロセス／bitness 境界）だが同一リポジトリで共にビルド・出荷されるため lockstep が成立する。**リリース時点**（または第三者製ネイティブ脳が公開 ABI へ独立実装する段階）で、COM 標準の進化規律（公開インターフェイス不変＋新 IID の `IShiori2` 追加＋`QueryInterface`）と、json-rpc 採用時は protocol version を導入する——これは本 P0 ではなくリリース前マイルストーン／別仕様の責務。これにより §0・§5 で挙げた「ABI 後戻りコスト」リスクは*凍結*ではなく*プロセス（lockstep 再ビルド）*で緩和される。

# ギャップ分析: areka-P0-shiori-protocol

> 本書はギャップ分析（情報提供）であり、最終的な実装方針の決定ではない。設計フェーズ（`/kiro-spec-design`）および要件ディスカッションの入力として用いる。
> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。互換契約の典拠: ukadoc。沈黙/曖昧箇所は COMPAT §2 の沈黙ルールに従い areka 裁量＋対応表へ明記。
> 隣接（完了）: `areka-P0-shiori-com`（content を不透明 HSTRING で運ぶ ABI。本仕様はその content の中身＝正準プロトコルを定義し、ABI 面は変更しない）。

## 0. 分析サマリー（概要）

- **本仕様は「コード」ではなく「契約ドキュメント＋データスキーマ」が主成果物**: 成果物の中核は (a) ukadoc 全イベントカタログ、(b) フィールドスキーマ、(c) 意味名⇔`ReferenceN` の唯一の正本対応表、(d) json-rpc 封筒マッピングである。これらは規範的契約（文書／スキーマ）であり、`areka-P0-shiori-com` が確定した `IShiori`/`IShioriHost` ABI 面には一切手を入れない（content は引き続き不透明 HSTRING）。実装コード（投影機構・パーサ・翻訳器）は HOW として明示的にスコープ外（→ 設計／下流仕様）。
- **既存パターン**: 隣接 `shiori-abi` クレートが ABI 封筒（`Request`/`Complete`/`Raise`、相関トークン `u64`、HRESULT 3 分岐＝即時/遅延/失敗）を既に実装済み。本仕様の json-rpc 封筒マッピング（R4）は、この**既存 ABI セマンティクスへ json-rpc の `id`/`result`/`error`/notification を被せるだけ**で、新しい通信機構は不要。`SHIORI_S_PENDING`＝遅延＝`id` 先行返却、`Complete`＝後続 `result` 配送、`Raise`＝notification（`id` なし）という対応は `areka-P0-shiori-com` 設計（D5/§Data Models）で既に方針宣言済み。
- **不足している能力（Missing）**: ukadoc イベントカタログそのもの、フィールドスキーマ表、意味名⇔`ReferenceN` 対応表、json-rpc メソッド/params 投影規約、エンコーディング規約 — コードベース・doc いずれにも未存在（grep で `ukadoc`/`Reference0`/`OnSecondChange` 等のヒットは brief/COMPAT/隣接 spec の言及のみで、データ資産は皆無）。これらは本仕様で新規に「定義」する。
- **候補アプローチ**: 契約の置き場所として (A) doc 配下に Markdown 台帳のみ、(B) `shiori-abi` クレート内に機械可読スキーマ＋生成コード、(C) ハイブリッド（doc 正本＋ machine-readable 種データを軽量に同梱、投影機構は設計/後続へ）。本仕様スコープ（WHAT のみ・投影機構は HOW で除外）を厳守するなら **A または C（doc/スキーマ中心、コードは最小）**が素直。
- **リサーチフラグ**: (1) ukadoc 全イベントの網羅的棚卸し（一次資料の取得・GET/NOTIFY 分類の確定）、(2) 予約 SHIORI ヘッダの確定集合（R6）、(3) json-rpc 2.0 Rust クレート採否（型定義のみ adopt するか手書きか・設計フェーズ）、(4) Charset/エンコーディング規約と HSTRING(UTF-16) 運搬の整合（R8）。
- **規模/リスク見立て**: 契約定義＋対応表整備が主体で **Effort = M〜L（イベント網羅の量に依存）／Risk = Medium**。アルゴリズム的難所はないが、ukadoc 網羅の完全性と「1スキーマ→2表示」の正本性担保が主リスク。Q1（native wire のエイリアス併載・要件 R10 で未裁定）は**要件ディスカッションでの裁定が前提**。

---

## 1. 現状調査（Current State）

### 1.1 ワークスペース構成と既存資産

- マルチクレート `crates/*`（`wintf` / `dola` / `areka` / **`shiori-abi`**）＋ ベンダリング `vendors/pasta`。
- 隣接 `shiori-abi`（`areka-P0-shiori-com` で完成）は ABI 封筒を実装済み:
  - `crates/shiori-abi/src/interface.rs` — `IShiori`（`Load`/`Unload`/`Request`）・`IShioriHost`（`Raise`/`Complete`）の raw COM 定義。**content は `HSTRING`（不透明）**。
  - `outcome.rs` — `RequestOutcome { Immediate(HSTRING) / Deferred(CorrelationToken) }`、`CorrelationToken(u64)`、単調増加採番アロケータ。
  - `error.rs` — `SHIORI_S_PENDING`（成功コード＝遅延）・`SHIORI_E_NOT_LOADED`・`SHIORI_E_UNKNOWN_TOKEN`、HRESULT⇄`ShioriError` 変換。
  - `ergonomic.rs` — `ShioriExt`（`load`/`unload`/`request`）。HRESULT を即時/遅延/失敗へ振り分け。
- **本仕様が扱う「content の中身」に相当する既存資産は皆無**: イベントカタログ・フィールドスキーマ・対応表・json-rpc 投影は未存在。`shiori-abi` は content を一貫して不透明 HSTRING として運ぶのみ（`Request(input: *const HSTRING, ...)`）。

### 1.2 json-rpc 封筒の「半分」は既に ABI 側に存在する（重要な発見）

`areka-P0-shiori-com` 設計（design.md §Data Models / research D5）が、ABI セマンティクスと json-rpc 構造の対応を既に宣言している:

| ABI セマンティクス（既存・shiori-com） | json-rpc 2.0 構造（本仕様で確定） | 要件 |
| --- | --- | --- |
| `Request(input)` の即時応答 = `S_OK` ＋ `out_response` | `id` 付き request → 同 `id` の `result` | R4-1, R4-2 |
| 遅延 = `SHIORI_S_PENDING` ＋ `out_token`（相関トークン） | `id` を先行して確定（result は後送り） | R4-3 |
| `IShioriHost::Complete(token, response)` | 先行 `id` に対応する `result` を後続配送 | R4-3 |
| `Request` 失敗 = error HRESULT | 同 `id` の `error` | R4-2 |
| `IShioriHost::Raise(script)`（能動通知） | `id` なし notification | R4-4 |
| 相関トークン `u64`（`CorrelationToken`） | json-rpc `id` と一意対応 | R4-5 |

→ **本仕様の R4（封筒マッピング）は新規通信機構の導入ではなく、既存 ABI の意味論を json-rpc の語彙で正準化・成文化する作業**。リスクが大きいのはむしろ R1〜R3（イベントカタログ／フィールドスキーマ／対応表）の網羅と正本性。

### 1.3 依存・features の現状（json/serde の利用可否）

- ルート `Cargo.toml` の `[workspace.dependencies]` には **`serde`/`serde_json`/json-rpc クレートは無い**。`serde`/`serde_json` は `dola` クレートが個別に依存しているのみ（`tech.md` の dola 依存節）。`shiori-abi` の依存は `windows-core` / `windows`(`Win32_System_Com`) / `thiserror` の最小構成（下流 32bit ホストが UI 基盤を引き込まないための分割。`structure.md` 明記）。
- 含意: 仮に本仕様の成果として **機械可読スキーマや json-rpc 型を `shiori-abi` に同梱**する判断になった場合、`serde`/`serde_json`（または json-rpc クレート）を `shiori-abi` の依存へ追加する是非が論点になる（最小依存・32bit ビルド可搬性の維持と要バランス）。ただし**投影機構・パーサの実装は本仕様のスコープ外（HOW）**であるため、本仕様の必達範囲では依存追加は不要にできる（契約は doc/スキーマで表現可能）。

### 1.4 命名・規約の整合先

- 予約 SHIORI ヘッダ（R6: `ID`/`Sender`/`Charset`/`SecurityLevel`/`Status`/`Reference*` 等）は ukadoc/SHIORI 規約由来であり、コードベースには定義が無い（本仕様で確定集合を定義する）。
- 既存 COM/エラー命名規約（`structure.md`・`com-resource-naming-unification`）は ABI 面の規約であり、本仕様の「意味名（content フィールド名）」とは別レイヤ。意味名の命名規約（予約ヘッダ非衝突・R6）は本仕様が新規に定める。

---

## 2. 要件 → 既存資産マップ（ギャップ tag: Missing / Unknown / Constraint）

| 要件 | 技術的ニーズ | 既存資産 | ギャップ |
| --- | --- | --- | --- |
| R1 正準イベントカタログ＋GET/NOTIFY 分類 | ukadoc 全イベント列挙、各イベントの応答期待分類、沈黙裁定 | なし（ukadoc 一次資料が外部） | **Missing**（カタログ本体）＋ **Unknown**（ukadoc 網羅・沈黙箇所の裁定） |
| R2 正準語彙（意味名 params） | 各イベントの意味名・型・必須/任意・`ReferenceN` 位置・応答意味 | なし | **Missing**（フィールドスキーマ表） |
| R3 単一正本対応表＋エイリアス従属 | 意味名⇔`ReferenceN` の唯一の対応表、canonical 優先規則 | なし | **Missing**（対応表）＋ **Constraint**（正本1枚・分散禁止） |
| R4 json-rpc 封筒マッピング | request/応答/遅延/Raise → `id`/`result`/`error`/notification | `shiori-abi` ABI 封筒・相関トークン `u64`・`SHIORI_S_PENDING`/`Complete`/`Raise` 既存（§1.2） | **Constraint**（既存 ABI 意味論へ整合）＋ 小 **Missing**（メソッド名=イベント ID・params=意味名 の投影規約） |
| R5 レガシー wire 放出方針＋キルスイッチ | `ReferenceN` 必須放出、意味名併載の per-DLL 切替、`Reference0/1/2…` 不消去/不改名の不変条件 | なし（放出は host-32 が実装。契約は本仕様） | **Missing**（放出契約の成文化）＋ **Constraint**（旧 DLL が `ReferenceN` を要求） |
| R6 予約 SHIORI ヘッダ非衝突 | 予約ヘッダ確定集合、意味名の非衝突保証と是正プロセス | なし（予約集合の定義が無い） | **Missing**（予約集合）＋ **Constraint**（既存予約解釈を壊さない） |
| R7 沈黙ルールの裁定追跡 | 各裁定の典拠（ukadoc 有無・SSP 二次参照・areka 裁量）の記録形式 | COMPAT §2 沈黙ルール（方針のみ） | **Missing**（追跡フォーマット）＋ **Constraint**（COMPAT §2 準拠） |
| R8 content エンコーディング規約＋不透明性 | content 文字列の charset 規約、`Charset` ヘッダ対応、不解釈の保証 | `shiori-abi` は content を不透明 HSTRING（UTF-16）で運ぶ | **Constraint**（HSTRING/UTF-16 運搬・不解釈）＋ **Unknown**（`Charset` ヘッダと UTF-16 運搬の対応規約） |
| R9 バージョニング方針宣言 | D7（プレリリース流動→リリース凍結）整合、D6（高レート余地）非阻害宣言 | `areka-P0-shiori-com` D7/D6（既存方針） | **Constraint**（既存 D7/D6 と整合）— ギャップ小 |
| R10 native wire エイリアス併載（未決・Q1） | native（COM-SHIORI）wire に `ReferenceN` 併載か pristine か | なし（要件内で未裁定・OPEN QUESTION） | **Unknown / 要裁定**（要件ディスカッションで決定） |

**複雑性シグナル**: 単純 CRUD ではなく「外部仕様（ukadoc）の網羅的棚卸し＋規範的対応表の設計＋既存 ABI 意味論への整合」。アルゴリズム的難所は少ないが、(a) ukadoc 網羅の**完全性**、(b) 「1スキーマ→2表示」の**正本性**（意味名と `ReferenceN` が機械投影で一致する保証）、(c) 沈黙裁定の**追跡可能性**、が要。

---

## 3. 実装アプローチ案（A / B / C）

> 本仕様の成果物は「契約（文書／スキーマ）」であり、投影機構・パーサ・翻訳器の実装は HOW としてスコープ外。以下は**契約の表現形式・置き場所**の選択肢。

### Option A: doc 配下に Markdown 台帳として定義（Document-only / Extend doc）
- **対象**: `doc/`（例: `doc/SHIORI_PROTOCOL.md` または COMPAT 台帳の節）に、イベントカタログ・フィールドスキーマ・対応表・封筒マッピング・沈黙裁定追跡を Markdown 表として記述。`shiori-abi` クレートには手を入れない。
- **トレードオフ**:
  - ✅ COMPAT §5 と同じ「設計台帳」流儀に乗る。依存追加ゼロ・32bit 可搬性に無影響。
  - ✅ WHAT のみを宣言し HOW（投影機構）に踏み込まない、という本仕様スコープに最も忠実。
  - ✅ 下流（host-32/reference/pasta）は同一 doc を典拠に話せる。
  - ❌ 「1スキーマ→2表示」の機械投影が人手保証になり、意味名と `ReferenceN` の整合がドリフトしうる（レビュー/将来のコード化で補完）。
  - ❌ 機械可読でないため、後続の投影機構（コード生成等）は別途スキーマ起こしが要る。

### Option B: `shiori-abi` 内に機械可読スキーマ＋生成物として定義（New in crate）
- **対象**: `shiori-abi`（または新規 `shiori-protocol` クレート）に、対応表を機械可読データ（例: 定数テーブル／`serde` 構造／ビルドスクリプト入力）として持ち、意味名・`ReferenceN` 双方をそこから投影。
- **トレードオフ**:
  - ✅ 正本1枚から2表示を**機械投影**でき、R3-2 の核（ドリフト不能）を構造で担保。
  - ✅ 下流が同クレート依存で対応表を直接参照できる。
  - ❌ **投影機構の実装は本仕様スコープ外（HOW）**であり、ここまで踏み込むと要件境界を越える懸念。
  - ❌ `serde`/json-rpc クレートを `shiori-abi` 最小依存へ持ち込むと 32bit 可搬性・最小依存方針（structure.md）と緊張。
  - ❌ 量（ukadoc 全イベント）が確定する前にコード構造を固めると後戻りコスト大。

### Option C: ハイブリッド（doc 正本＋軽量 machine-readable 種、機構は後続）（Hybrid）
- **対象**: 正本は Option A の doc 台帳（人間可読・レビュー可能・契約の所有者）。加えて、後続の投影機構が起こしやすいよう**軽量な機械可読種データ**（例: 対応表の CSV/JSON/TOML を doc 同梱）を「参考添付」として置く。投影機構・コード生成は設計／後続仕様。
- **トレードオフ**:
  - ✅ 契約の正本性（doc）と将来の機械投影（種データ）を両立。スコープは WHAT に留まる。
  - ✅ 依存追加なし（種データは静的ファイル）。
  - ❌ 正本（doc）と種データの二重管理リスク（どちらが正かを明記して回避）。
  - ❌ 種データのフォーマット選定に追加の設計判断。

> 本仕様スコープ（WHAT のみ・投影機構は HOW で除外）と「正本は対応表スキーマ1枚」の要求を重視するなら **A または C** が素直。B は投影機構へ踏み込むため要件境界に注意。最終決定は設計フェーズ。

---

## 4. Research Needed（設計フェーズ／要件ディスカッションへ繰り越す未決事項）

1. **ukadoc 全イベントの網羅的棚卸し**: ukadoc（正典）の一次資料から全 SHIORI イベントを列挙し、GET（応答期待）/NOTIFY（通知のみ）を分類する。ukadoc が応答期待を沈黙するイベントは COMPAT §2 沈黙ルールで裁定し対応表へ明記（R1-3, R7）。**網羅の完全性が本仕様の品質の中核**。一次資料の取得経路（ukadoc サイト/ローカルコピー）の確保が前提。
2. **予約 SHIORI ヘッダの確定集合**（R6）: `ID`/`Sender`/`Charset`/`SecurityLevel`/`Status`/`Reference*` 以外に予約すべきヘッダ（`BaseID`/`Event`/`Type`/`Status` 等）を ukadoc から確定し、意味名の非衝突保証の対象集合を固定する。
3. **json-rpc 2.0 の表現形式と Rust クレート採否**（設計フェーズ・R4）: 封筒を doc で規定するに留めるか、機械可読型として表現するか。後者なら候補クレート: `jsonrpc-types`（1.0/2.0・notification/batch 対応）、`jrpc`/`jrpc-types`（serde ベースの型定義）、`roboplc-rpc`（超軽量・transport 非依存）。**ただし本仕様は採用判断のみで実装は別仕様**（D5 が json-rpc 2.0 採用を既に確定済み）。`shiori-abi` 最小依存方針との緊張を要評価。
4. **Charset/エンコーディング規約**（R8）: content を HSTRING(UTF-16) で運ぶ（shiori-com）一方、レガシー wire は `Charset` ヘッダで符号化（host-32 が byte 化）。本仕様は「content 文字列の charset 規約と `Charset` ヘッダ対応」をどう成文化するか（UTF-16 内部表現と wire charset の対応規則）。
5. **「1スキーマ→2表示」の正本性の担保方法**（R3-2・スコープ境界に注意）: 機械投影の機構自体は HOW（設計/後続）だが、契約として「意味名と `ReferenceN` が単一スキーマの2投影である」ことをどう**検証可能**に宣言するか（doc 上の対応表が唯一の正本である旨の明記＋将来のコード化フック）。
6. **Q1 / R10 の裁定**（要件ディスカッション）: native（COM-SHIORI）wire に `ReferenceN` エイリアスを併載するか pristine（意味名のみ）か。要件 R10 は未裁定（OPEN QUESTION）。暫定推奨＝native は pristine、レガシー wire のみ併載（R5）。**この裁定が確定するまで R10 の `shall` が空欄**。

---

## 5. 規模・リスク評価

- **Effort: M〜L（ukadoc イベント網羅の量に依存）** — 封筒マッピング（R4）は既存 ABI 意味論への整合で軽量（S 相当）。一方、イベントカタログ（R1）・フィールドスキーマ（R2）・対応表（R3）の網羅的整備は ukadoc 全イベント数に比例し、量が嵩めば L。コード実装（投影機構）はスコープ外のため XL には至らない。
- **Risk: Medium** —
  - 既存 ABI 封筒（shiori-com）が json-rpc 構造に素直に対応する設計を済ませているため、R4 の技術的不確実性は低い。
  - 主リスクは (a) ukadoc 網羅の**完全性・正確性**（一次資料依存・沈黙箇所の裁定品質）、(b) 「正本1枚→2表示」の**ドリフト防止**（Option A だと人手保証）、(c) 沈黙裁定の**追跡可能性**（R7 のフォーマット設計）、(d) Q1 未裁定（R10）が要件ディスカッション待ち。
  - x86/bitness 連鎖・OOP マーシャリング等の ABI 物理リスクは `areka-P0-shiori-com`/`host-32` 側に分離済みで本仕様には無い（content は不透明のまま）。

---

## 6. 設計フェーズへの推奨

- **推奨アプローチ**: Option A または C（doc 正本台帳中心、投影機構は HOW として設計/後続へ）。理由は本仕様スコープが「WHAT（契約）」に限定され、投影機構・パーサ・翻訳器を明示的に除外しているため。`shiori-abi` の最小依存・32bit 可搬性を崩さずに契約を表現できる。
- **既存 ABI への整合を最優先**: R4 封筒マッピングは `areka-P0-shiori-com` の `Request`/`Complete`/`Raise`・相関トークン `u64`・`SHIORI_S_PENDING` 意味論（§1.2 表）と一意対応させる。ABI 面は変更しない（content の中身のみ定義）。
- **先に確定すべき設計判断**:
  1. 契約の表現形式・置き場所（doc 台帳 / 機械可読スキーマ / ハイブリッド）と「正本は1枚」の明記方法。
  2. ukadoc 全イベントカタログの網羅と GET/NOTIFY 分類（沈黙裁定の COMPAT §2 適用）。
  3. フィールドスキーマと意味名⇔`ReferenceN` 対応表の表フォーマット（型・必須/任意・応答意味・典拠列）。
  4. json-rpc 封筒の投影規約（メソッド名=イベント ID、params=意味名、`id`/`result`/`error`/notification 対応）。
  5. 沈黙裁定追跡（R7）の記録列（ukadoc 条項有無 / SSP 二次参照 / areka 裁量）。
- **要件ディスカッションで先に裁定すべき項目**: Q1 / R10（native wire のエイリアス併載 vs pristine）。これが確定しないと R10 の `shall` が空欄のまま。
- **繰り越すリサーチ項目**: §4 の 1〜6。

---

## 7. スコープ境界の注意（HOW と取り違えないために）

本仕様で**定義する（WHAT）**: イベントカタログ／フィールドスキーマ／唯一の正本対応表／json-rpc 封筒マッピング／エンコーディング規約／沈黙裁定追跡／バージョニング方針宣言／（R10 裁定後）native wire 表現契約。

本仕様で**定義しない（HOW・別仕様）**:
- 「1スキーマ→2表示」の投影機構（対応表レジストリ／コード生成／実行時変換）— 設計/後続。
- キルスイッチ設定の置き場・データ構造 — 設計。
- host-32 のレガシー wire 翻訳モジュールの実装 — `areka-P0-shiori-host-32`。
- さくらスクリプト／SAORI 本文の解釈・実行 — `areka-P0-sakura-script` ほか（content は不透明）。
- COM ABI 面（`IShiori`/`IShioriHost`）・トランスポート（HSTRING 取り回し）— `areka-P0-shiori-com`（完了・変更しない）。

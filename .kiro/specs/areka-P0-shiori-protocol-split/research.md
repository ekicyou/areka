# ギャップ分析（Gap Analysis）: areka-P0-shiori-protocol-split

> 対象: 完了仕様 `areka-P0-shiori-protocol` の確定成果物 `doc/shiori/shiori_protocol.toml`（10,685 行・単一正本）を、契約内容を一切変えずに Event/Resource 単位フラグメント群＋keyed/inline 符号化形へ再編する非破壊の物理リファクタ。
> 言語: ja（spec.json）。本書は情報提供（options）であり最終決定ではない。決定は要件ディスカッション／設計フェーズへ送る。

## 0. 分析サマリ（3–5 行）

- **ほぼグリーンフィールド**: リポジトリ全体で `shiori_protocol.toml` を parse/読込する実装上の consumer・codegen・build.rs・TOML ツーリングは **存在しない**（grep ヒットは仕様 doc と当該 TOML 本体のみ）。ギャップは「新規ツーリング契約＋データ移行＋要件改訂」に集約され、既存コードへの破壊的影響は皆無。
- **既存パターンの援用先は dola の 600 行リファクタ規律**（`structure.md`：肥大ファイルを `{module}/mod.rs`＋サブモジュールへ割る方針）。ただし本件は **Rust コード分割ではなくデータファイル分割**であり、援用できるのは「サイズ閾値＋意味境界で割る」哲学のみ。決定的再構成・同値ゲートに相当する既存資産は無い（= 新規定義が要る）。
- **本仕様の成果物は契約・受け入れ基準まで**（フラグメント群・共有フラグメント・マニフェスト/順序・同値ゲートの判定基準・要件改訂・README 改訂）。再構成器/バリデータ/doc-Web 生成器/Rust codegen の **実装コードはスコープ外**（後続）。したがって「機構の実装をどう作るか」ではなく「機構が満たすべき契約をどう規定するか」を裁定する分析である。
- **最大の設計判断は 3 点**: (a) 決定的 merge と意味的同値ゲートの**判定基準の規定方法**、(b) monolith→fragments の**無損失移行の実施・検証手順**、(c) Q1（`shiori_protocol.toml` を非権威生成物として残置 vs 削除）。
- **リサーチ要否は低い**。TOML v1.0.0（inline table 単一行・quoted key）と既存データ実測（446 entry / 802 field / 9 silence_ruling / dot・asterisk 混じり id 実在）で判断材料は揃う。外部依存リサーチは不要（標準 TOML 仕様内で完結）。

## 1. 現状調査（Current State）

### 1.1 正本 TOML の実測構造（grep で実測・検証済み）
- 総行数 **10,685**。`[[entry]]` **446**（`kind="event"` **287** ＋ `kind="resource"` **159**）。`[[entry.field]]` **802**。`[[silence_ruling]]` **9**。`silence_ref` 参照は **44** 箇所。`reference_variadic` を持つ field **32**。
- 共有テーブルは各 1: `[meta]` / `[mapping]` / `[envelope]` / `[reserved_headers]`。いずれも `description` をデータとして保持し、`[mapping]` は `canonical_key="name"` / `alias_key="reference"` / `alias_variadic_key="reference_variadic"` 等を符号化（要件 4.6 の改訂対象）。
- entry は **36 カテゴリ順**で整列済み（`lifecycle`→…→`tooltip`）。`category =` 行の出現位置を実測すると、`shortcut_key` 等の大カテゴリは entry が連続して 600 行を容易に超える（93 件・40 件級カテゴリが存在）。
- 現行 entry の符号化形は `[[entry]]`（配列）＋ `id=`/`kind=`/`category=`/`dispatch=`/`response=`/`provenance=`/`description=` 行、field は `[[entry.field]]`（配列）＋ `name=`/`reference=`/`type=`/`required=`/`provenance=`/`description=` 行。1 field 当たり約 6–7 行。
- id に **ドット・アスタリスク混じり**が実在（`OnUpdate.OnDownloadBegin`・`property.get`・`sakura.defaultx`・`char*.defaultx` 等）。→ keyed table 化では **キー常時 quote 必須**（要件 4.4）。
- `silence_ruling` は OS 状態系 8 entry を 1 裁定が束ねる等、**カテゴリ横断**で参照される（要件 5.2/5.3 の共有フラグメント集約の根拠）。
- TOML 先頭バナーは「SKELETON 段階」の旧記述が残存（task 1.1 等）。本仕様で残置する場合はバナー自体も「fragments から生成・編集禁止」へ要更新（要件 7.3）。

### 1.2 consumer / ツーリング探索（グリーンフィールド確認）
- `shiori_protocol` / `shiori/` の全リポジトリ grep ヒットは **8 ファイルのみ**: 本仕様の requirements/brief、当該 TOML、`doc/shiori/README.md`、completed 仕様 3 本（tasks/design-validation/design）、隣接 `areka-P0-shiori-com/research.md`。**実コード（crates/*）からの参照はゼロ**。
- `crates/` 配下の Cargo.toml は 4 本（areka/dola/shiori-abi/wintf）。**`build.rs` は無し**、TOML を parse して codegen する仕掛けも無し。`shiori-abi` は COM ABI 定義クレートで、本契約データを読まない。
- → **下流の lockstep 改修対象が現時点で実体として存在しない**。要件 boundary が言う下流（host-32・reference・pasta native・doc/Web 生成器・Rust codegen）はいずれも未実装の後続フェーズ。本仕様完了時点で壊れる実装は無く、移行は純粋にデータ／ドキュメント／要件層で閉じる。

### 1.3 既存の規約・援用可能パターン
- **600 行リファクタ規律**（`structure.md` dola `runtime/` の前例）: 「肥大化したら閾値＋意味境界で分割」哲学は本件のサイズ不変条件（要件 2.1: ≤600 行）と精神が一致。ただし対象が `.rs` ではなくデータ TOML な点が差分。
- **SSOT 哲学**（completed 要件 3/11・`doc/shiori/README.md`）: 「正本 1 枚・二重定義禁止・description はコメントでなくデータ・provenance 保持・派生は正本と同値/冪等」。本仕様はこの**精神を維持しつつ「論理 SSOT＝フラグメント結合結果」へ literal を改訂**する（要件 8）。
- **Revalidation Trigger**（completed design.md §Revalidation Triggers）: 「TOML 正本の table 階層・必須キー・型規約の変更」は下流再検証トリガと明記済み。本仕様の符号化形刷新（`[[entry]]`→keyed・`[[entry.field]]`→inline）はまさに DP1 の `array of entry` 規定の改訂であり、**意図的な設計判断として畳み込む**必要がある（要件 4・8）。

## 2. 要件→資産マップ（Requirement-to-Asset Map／gap タグ）

| 要件 | 必要能力 | 既存資産 | gap タグ |
|---|---|---|---|
| R1 フラグメント正本化・二重定義禁止 | keyed table によるパーサ機械担保 | SSOT 哲学（completed）／既存 id 一意性（実測で重複無し） | **Missing**（フラグメント物理レイアウト・二重検出ゲートは新規） |
| R2 ≤600 行・カテゴリ純度・サブ分割 | サイズ駆動分割規則 | dola 600 行リファクタ哲学 | **Constraint**（大カテゴリの entry 境界サブ分割規則を新規定義） |
| R3 決定的・冪等再構成＋意味的同値ゲート | merge 順序固定＋構造比較基準 | なし | **Missing**（最重要・新規契約） |
| R4 keyed/inline 符号化＋一意性パーサ担保 | TOML v1.0.0 連想表・inline table | TOML 0.8 を dola で使用（serde/toml）／実測の dot・asterisk id | **Missing**（符号化スキーマの新規規定）＋ **Constraint**（quote 必須・inline 単一行） |
| R5 共有テーブル集約 | `_shared` フラグメント＋silence 集約 | 共有テーブル 4＋silence 9 が実在・横断参照 44 | **Missing**（集約レイアウト新規）／参照解決は **Constraint** |
| R6 description/provenance 無損失 | 全データ保持・典拠参照整合 | 全要素が既に description/provenance をデータ保持 | **Constraint**（移行時の無損失を保証する手順が要る） |
| R7 旧単一ファイルの降格（＋Q1） | 派生地位・banner・同値 | 現 README が「正本＝1 枚」を宣言 | **Unknown（Q1）**＋ **Missing**（banner データ・派生扱い） |
| R8 completed 要件 3/11 改訂・README 改訂 | 系譜を残す改訂継承 | completed/ は不変原則・README 現行宣言 | **Constraint**（completed/ 履歴不変のまま本仕様側で系譜追跡） |
| R9 非破壊保証（セマンティクス不変） | 同値ゲートによる証明 | なし | **Missing**（ゲート＝R3 と同根） |

## 3. 実装アプローチの選択肢

本仕様の成果物は**契約・受け入れ基準まで**（機構の実装は後続）。以下は「成果物をどの形で着地させるか」の選択肢であり、Extend/New/Hybrid を成果物レイアウトと検証戦略の観点で提示する。

### 3.A 決定的 merge ＋ 意味的同値ゲートの「判定基準」の規定方法

- **A-1（明示マニフェスト方式）**: `_manifest.toml` にフラグメント結合順を列挙して固定。順序は人間が明示管理。
  - 利点: 順序が一望・レビュー容易・サブ分割（`shortcut_key.01/02`）の順序を曖昧さなく固定。 欠点: フラグメント追加時にマニフェスト更新を忘れるとドリフト（運用規律が要る）。
- **A-2（ファイル名数値接頭辞方式）**: `NN.category.toml` の辞書順＝結合順。マニフェスト不要。
  - 利点: 単一の真実源（ファイル名）・追加が機械的。 欠点: 大量採番のリナンバリングコスト／カテゴリ 36＋サブ分割で接頭辞が窮屈。
- **A-3（ハイブリッド: マニフェスト＋接頭辞の二重固定）**: 接頭辞で既定順、マニフェストで上書き/検証。
  - 利点: 冗長な決定性（要件 3.1 は「マニフェストまたは接頭辞」を許容）。 欠点: 二重管理の不整合リスク。
- **同値ゲートの判定基準**（いずれの方式でも共通・要件 3.4/9.4）: `parse(現行 toml)` と `parse(merge(fragments))` を**順序非依存の構造**として比較し、(i) entry 集合、(ii) 各 entry の field 集合、(iii) 共有テーブル、(iv) silence_ruling、(v) 全 description、(vi) 全 provenance、(vii) 封筒マッピング、(viii) 予約ヘッダ集合 が過不足なく一致することを条件と規定。keyed 化により map 等価比較が自然に成立する点が要点。**本仕様は判定基準（何を比較対象とし何をもって同値とするか）を確定し、比較器コードは後続**。
  - 留意: 符号化形が `array(reference 順)` → `inline(reference キー)` へ変わるため、同値比較は **reference 値で正規化**してから集合比較する（配列インデックス依存を排除＝要件 4.5）。`reference` と `reference_variadic` の両保持 field（固定開始＋可変長末尾）の正規化規則を判定基準に明記する必要がある。

### 3.B monolith→fragments の無損失移行の実施・検証

- **B-1（機械変換スクリプト 1 回・成果物は出力データ）**: 現行 TOML を parse → カテゴリ別に keyed/inline へ機械変換 → フラグメント書き出し。スクリプトは使い捨て（成果物はフラグメント群、スクリプトは本仕様スコープ外でも検証用に許容）。
  - 利点: 802 field の手作業を排し転記ミスをゼロ化・description 無損失を機械保証。 欠点: 変換器の正しさ自体を同値ゲート（3.A）で別途担保する必要（自己検証ループ）。
- **B-2（手作業分割）**: 人手でカテゴリごとに切り出し・keyed 化。
  - 利点: ツール不要。 欠点: 802 field・10,685 行で現実的でない／無損失リスク大。**非推奨**。
- **推奨観点**: B-1 を採り、**移行の正しさは 3.A の同値ゲートで事後証明**する設計（移行手段は問わず、ゲート合格が無損失の唯一の証拠）。要件は「ゲート＝非破壊の証拠」を既に規定（9.4）しているので、本仕様は**ゲートの判定基準を成果物の受け入れ基準として固定**し、変換手段自体は HOW として後続/補助に置ける。

### 3.C Q1: `shiori_protocol.toml` の処遇（要件 7.4 が設計フェーズ裁定を明示）

- **C-1（非権威の生成物として残置・暫定推奨）**: フラグメントから再構成した結果を `shiori_protocol.toml` として書き出し、先頭に「fragments から生成・直接編集禁止」banner をデータで明示。
  - 利点: 下流（未実装だが将来 consumer）が単一ファイル入力のまま移行不要・移行期の安全網。 欠点: 派生物を tree にコミットする＝再生成忘れによる派生ドリフト risk（同値ゲートを CI 的に回す前提が要る／ただし生成器は後続）。
- **C-2（tree から削除・オンデマンド結合に一本化）**: 単一ファイルを廃し、必要時に merge で正準ビューを得る。
  - 利点: 派生ドリフトの構造的排除・正本が fragments のみで一意。 欠点: 単一ファイル入力を前提する将来 consumer が出た時に merge 手段（後続実装）への依存が必須化。現時点で consumer がゼロな点は C-2 を後押しする材料。
- **判断材料**: 1.2 の通り**現在 consumer はゼロ**。よって C-2 の「移行期互換が要らない」前提が成立しやすい一方、completed 仕様の README が単一ファイル前提で書かれてきた連続性は C-1 を後押しする。**この対立を要件ディスカッション→設計で裁定**。要件 7 は処遇に依らず満たすべき不変条件（正本でない・残す場合は同値・編集禁止表示）を規定済みなので、Q1 はあくまで「残す/消す」の二択に閉じている。

### 3.D 成果物レイアウト（ディレクトリ構成）の選択肢

- **D-1（フラット）**: `doc/shiori/fragments/NN.category.toml` ＋ `_shared.toml` ＋ `_manifest.toml`。 利点: 単純。 欠点: event/resource の視覚的分離なし。
- **D-2（kind 別ディレクトリ）**: `fragments/events/` ＋ `fragments/resources/` ＋ `_shared.toml`。 利点: 287/159 の物理分離が直感的・捜索が速い。 欠点: entry 形式は kind 非依存単一（要件 1.3）なのでディレクトリ分離は純粋に整理目的（マニフェスト順序の管理が 2 系統に）。
  - 注: 要件 1.3 は「kind に依らず単一フラグメント形式」を要求するが、これは**フラグメントの内部スキーマ形式**の話であり、ディレクトリ物理配置の分離を禁じない。brief も「必要なら events/・resources/ 分離は設計フェーズで判断」と保留。

## 4. 工数・リスク（成果物＝契約/データ/要件改訂の範囲）

| 作業塊 | 工数 | リスク | 一行根拠 |
|---|---|---|---|
| 符号化スキーマ＋フラグメントレイアウトの契約規定 | S–M | Low | TOML v1.0.0 内で完結・実測データで形状確定済み |
| 決定的 merge ＋ 同値ゲートの**判定基準**規定 | M | Medium | reference/variadic 正規化・順序非依存比較の基準化が肝（実装は後続だが基準は厳密に） |
| monolith→fragments データ移行（無損失）＋ゲート通過の実証 | M | Medium | 802 field の機械変換＋同値証明。手段は機械化必須（B-1） |
| completed 要件 3/11 改訂・README 改訂・banner | S | Low | 文書改訂のみ・completed/ 履歴は不変で系譜追跡 |
| Q1 裁定の畳み込み（残置 or 削除） | S | Medium | 技術的には軽いが下流連続性 vs ドリフト排除の方針判断 |

- 全体: 契約・データ・要件改訂に閉じるため **M（数日〜1 週間級）／総合リスク Medium**。Medium の主因は同値ゲートの判定基準を緩く書くと「契約を変えていない」証明が崩れる点（要件 9.4 が不合格条件を握っている）。

## 5. リサーチ要否（Research Needed）

- **R-N1（任意・低優先）**: TOML v1.0.0 で「inline table 単一行・長文 description を含む 1 行」が現実的に許容範囲か（最長 description は共有テーブルの 1,132–1,601 字だが、これは field でなくテーブル＝inline 化対象外。field 側 description は p90 146 字で 1 行化に支障なし）。→ 実測で**支障なしの見込み**、設計で確認のみ。
- **R-N2**: reference と reference_variadic を**両保持**する field（固定開始 N＋可変長末尾）の正規化・同値比較規則の厳密定義。実測 variadic 32／reference 無し 6 の少数ケースを判定基準でどう扱うか。→ **設計フェーズで判定基準として確定要**。
- **R-N3**: 同値ゲートを将来 CI で回す前提か（C-1 残置時の派生ドリフト防止）。生成器/比較器は後続スコープだが、「いつ・何が・どうゲートを回すか」の運用前提を要件/設計で明記するか。→ 設計判断。
- 外部依存（クレート互換・バージョン制約）リサーチは **不要**（標準 TOML 仕様内で完結、新規外部依存を導入しない）。

## 6. 設計フェーズへの引き継ぎ（要件ディスカッションへ供する設計判断項目）

1. **DD-1 merge 順序の決定方式**: 明示マニフェスト（A-1）／数値接頭辞（A-2）／ハイブリッド（A-3）のいずれを正本順序源とするか。要件 3.1 は両許容だが、二重管理リスクを避けるため**単一の真実源を選ぶ**べき。
2. **DD-2 同値ゲートの判定基準の厳密化**: 比較対象 8 要素（§3.A）の正規化規則、とりわけ reference/reference_variadic 両保持 field と reference 無し field の正規化（R-N2）を受け入れ基準としてどこまで明文化するか。
3. **DD-3 移行手段の位置づけ**: 機械変換（B-1）を本仕様の補助とするか純粋に後続 HOW とするか。いずれにせよ**無損失の証拠は同値ゲートに一本化**する方針の確認。
4. **DD-4 Q1（残置 vs 削除）**: C-1（非権威生成物として残置・暫定推奨）か C-2（削除・オンデマンド結合）か。**現 consumer ゼロ**という材料を踏まえた裁定。残置なら banner データ・再生成運用（R-N3）も併せて確定。
5. **DD-5 フラグメント物理レイアウト**: フラット（D-1）か kind 別ディレクトリ（D-2）か。命名規約（`NN.category.toml`／サブ分割 `category.01.toml`）と `_shared.toml` の位置を確定。
6. **DD-6 completed 要件 3/11 改訂の文言と系譜の残し方**: completed/ 履歴を不変に保ちつつ本仕様側で「論理 SSOT＝フラグメント結合結果」へ改訂継承する具体表現（要件 8.1–8.3）。あわせて completed design.md DP1（`array of entry`）・Revalidation Trigger との整合を改訂理由として明記するか。
7. **DD-7 README/典拠参照の整合**: `doc/shiori/README.md` の「正本＝1 枚」宣言を「SSOT＝fragments／`shiori_protocol.toml`＝派生 or 廃止」へ改訂し、ukadoc ピン留めスナップショット参照（要件 6.3）の整合を保つ。

---
（本書は kiro-validate-gap により生成。情報と選択肢の提示であり最終決定ではない。決定は要件ディスカッション・設計フェーズで行う。）

# ギャップ分析: areka-P0-bindoption-exclusivity

**分析日**: 2026-08-11（validate-gap フェーズ・file-slimming PR#103 マージ後の現物実測）
**入力**: requirements.md（確定済）・brief.md（2026-08-11 実測追記込み）・steering・現行コードベース

## 分析サマリ

- **ギャップは小さく局在している**。是正の骨格（排他置換 `apply_bind_exclusive`・共通後段 `commit_bind`・カテゴリ ID 収集 `category_ids`・Changed=info/Unchanged=debug のログ流儀）は mayuna-compose 実装で**既に存在し、無改変で再利用できる**。欠けているのは (a) parsers が `multiple` 宣言を破棄している情報欠落、(b) `+` 区切り複数オプションの正典適合、(c) actor.rs:367 の 2 値分岐述語、の 3 点のみ。
- **本質的な変更は「述語の反転」**——「mustselect **である**なら排他」から「multiple と**明示宣言されていない**なら排他」へ。コード差分は parsers 1 関数＋モデル 2 フィールド＋BindResolver 1 述語＋actor 1 行が核。
- **搬送形（3 値をどう運ぶか）が最大の設計判断**。集合 2 本追加の最小形（A）・3 値 enum ポリシー表（B）・署名変更耐性のある構造体引数（C）の 3 案を比較した。いずれも `BindResolver::empty()` の**署名不変**（W6 並走無干渉の前提条件）は保てる。
- **広い作業は文書・テスト文言の一掃**（R6.1／R4.5）。「非 mustselect＝加算」「multiple／非宣言は収録しない（R4.5・D11）」前提の doc コメント・テスト名・テスト期待値が parsers／seriko／areka の 3 クレートに分布する（棚卸り一覧を後述）。
- **工数 M・リスク 低〜中**。判定は全て純関数で決定論檻が容易。リスク源は「既定の意味が反転する」ことによる既存テストへの波及だが、実測では同一カテゴリ複数 bind を非 mustselect で検証するテストは存在しない（檻 `bind_non_mustselect_accumulates_via_actor` は**異カテゴリ**加算＝正典後も有効・要改名のみ）。

## 1. 現状調査（Requirement → 既存資産マップ）

### 1.1 変更の主対象（現物・2026-08-11 実測一致）

| 資産 | 場所 | 現状 | ギャップ |
|---|---|---|---|
| bindoption 読み取り | `crates/areka-parsers/src/package/resolve.rs:172-188`（走査）＋`parse_bindoption_mustselect` :196-205 | `option == "mustselect"` の**完全一致**のみ収録・`multiple` 破棄 | **Missing**: multiple 収録（R1.1）・`+` 区切り分解（R1.3）・未知語読み流し（R1.4） |
| 転記モデル | `crates/areka-parsers/src/package/model.rs:49-64` `BindGroupDefaults`（`#[non_exhaustive]`・`sakura_mustselect`/`kero_mustselect: Vec<String>` :61-63） | multiple 用フィールドなし | **Missing**: スコープ別 multiple 保持（追加は additive・`#[non_exhaustive]` ゆえ下流破壊なし） |
| 排他判定述語 | `crates/areka-seriko/src/bind.rs:121-127` `BindResolver::is_mustselect` | mustselect 集合の所属判定（未知カテゴリ＝false＝加算） | **Missing**: 「排他か」の 3 値正典述語（非宣言→true へ**意味反転**） |
| 適用分岐 | `crates/areka-seriko/src/actor.rs:367-372` | `if on && is_mustselect(...)` → 排他、それ以外→加算 | **Missing**: 述語差し替え（実質 1 行）。off 経路は現行どおり `apply_bind`（R2.2/R3.2） |
| 資産構築 | `crates/areka/src/emo2_boot/assets.rs:253-267`（`BindResolver::new` 本番唯一の呼出 :267・現署名 4 引数） | mustselect 2 集合のみ構築 | **Missing**: multiple 集合（または 3 値表）の構築・搬送 |

### 1.2 無改変で再利用できる既存資産（回帰の錨）

| 資産 | 場所 | 再利用内容 |
|---|---|---|
| 排他置換の実体 | `crates/areka-seriko/src/state.rs:342-360` `apply_bind_exclusive` | `(現在集合 − category_ids) ∪ {target}`。**カテゴリ非依存の汎用形**——非宣言カテゴリにもそのまま使える |
| 共通後段 | `state.rs:374-402` `commit_bind` | 冪等ガード（Unchanged）・状態更新・発行判定（Shown/Hidden/未知）＝R2.4〜R2.6 の既存流儀そのもの |
| カテゴリ全 ID 収集 | `bind.rs:134-146` `category_ids` | `.name` 宣言表から昇順 dedup で収集。**bindoption 非宣言でも `.name` 宣言があれば引ける**（emo2 まばたき 1400-1403 は :50-53 で `.name` 宣言済み＝本件のケースを直接カバー） |
| 加算/除去 | `bind.rs:287-299` `accumulate`＋`state.rs:322-327` `apply_bind` | multiple 明示・off 経路の従来挙動（R3.3） |
| ログ流儀 | `actor.rs:374-396` | Changed=info（grep マーカー「seriko: bind 適用」）／Unchanged・StateOnly=debug——R2.4/R2.5 は**既に充足済みの流儀を維持するだけ** |
| ルーパー bind ゲート | `looper.rs:215` | `current_binds(scope).contains(anim.id)` の読み取りのみ＝**本増分で無変更**。排他置換が集合を正せば発火も自然に正常化（R5.2 の観測点） |
| 実機サインオフ流儀 | steering（有界 auto-exit＋ログ grep・絶対パス）＋`target/bindopt-debug-observation.log` 採取実績 | R5 の判定手順は確立済み（`AREKA_APP_SMOKE_EXIT_MS` 有界・`animation_id=140x` 共存 grep・飽和パターン不在確認） |

### 1.3 atomic 追随の呼出元台帳（コンパイル結合・実測全数）

`BindResolver::new`（署名変更時に全数追随・計 8 箇所）:

- 本番: `crates/areka/src/emo2_boot/assets.rs:267`（唯一）
- seriko in-crate テスト: `bind.rs:356`・`bind.rs:370`・`actor_bind_loop_tests.rs:64`・`:76`・`:202`
- seriko tests/: `tests/bind_e2e.rs:123`・`:244`

`BindResolver::empty()`（**署名不変が W6 並走無干渉の前提**・areka 側監視 4 箇所は現物一致）:

- areka 側: `input_events/balloon_test_support.rs:73`・`emo2_boot/frame_test_support.rs:71`・`emo2_boot/mod.rs:374`・`spine.rs:671`
- seriko 側: actor_dispatch_tests／actor_bind_loop_tests／tests/{regression,loop_integration,cue_sequence,balloon_face_e2e}.rs 等に多数（引数なしのため署名不変なら全て無傷）

**注意（意味論の反転が `empty()` に及ぶ）**: 3 値化後、`empty()` は「multiple 宣言ゼロ＝全カテゴリ排他既定」を意味するようになる。ただし `empty()` は名前表も空＝`resolve()` が常に `None` で bind cue が適用まで届かないため、**挙動への実害はない**。一方 `empty()` の doc（bind.rs:85-89「mustselect も空ゆえ排他判定は常に非排他」）は旧前提の文言であり R6.1 の一掃対象。

### 1.4 旧 2 値前提の文書・テストの棚卸り（R6.1／R4.5 の作業面・実測）

| 種別 | 場所 | 内容 |
|---|---|---|
| テスト名・対比文言 | `crates/areka-seriko/src/actor_bind_loop_tests.rs:192-239` `bind_non_mustselect_accumulates_via_actor` | 検証実体は**異カテゴリ**（腕/肩）加算＝正典後も有効。名前と「非 mustselect カテゴリは従来どおり加算」文言を新語彙へ（実体保持・R4.5） |
| テスト期待値の反転 | `crates/areka-parsers/src/package/resolve.rs:582-592`（in-file テスト）「`multiple` オプションは mustselect ではない＝取り込まない（既定＝非排他・R4.5）」 | multiple **収録**へ期待値ごと更新 |
| doc コメント（parsers） | `resolve.rs:120-121`・:172-176・:190-195／`model.rs:59-63`（「multiple／非宣言は既定＝非排他ゆえ収録しない」） | 3 値正典の記述へ |
| doc コメント（seriko） | `bind.rs:53-60`・:69-70・:85-89・:116-120（「multiple・非宣言（既定＝非排他）は false」）／`actor.rs:170`・:364-366／`state.rs:329-339`（apply_bind_exclusive の「mustselect（排他選択）カテゴリの」限定文言） | 述語の意味と併せ全面更新。mayuna-compose「D11・R4.5」引用は本 spec の要件 ID 引用へ差し替え |
| doc コメント（areka） | `assets.rs:261-262`（「Req 4.5・D11」引用） | 同上 |
| モデルのテスト | `model.rs` bindgroup_name_tests（:282-314 の is_mustselect 系・「非宣言カテゴリ（紅＝multiple/既定）は偽」等） | 文言更新＋multiple フィールドのテスト追加 |
| 適合スコープ文書 | `doc/emo2-conformance-scope.md`（bindoption 言及あり） | 3 値適合の記述整合を確認（設計フェーズで実測） |

補足: `BindGroupDefaults::is_mustselect`（model.rs:138-）は現状 parsers 内テスト専用で本番消費者なし（本番は assets.rs が Vec フィールドを直接読む）。3 値化の際にこのアクセサをどう扱うか（拡張／退役）は設計判断。

### 1.5 正典再確認（ukadoc MCP・2026-08-11 本分析で逐語再取得）

`sakura/kero/char*.bindoption*.group,カテゴリ名,オプション`（ukadoc descript_shell）:
「mustselectでパーツを必ず1つ選択、multipleで複数のパーツを選択可能。**オプションは+区切りで複数可**。」既定値「**選択解除可能、複数選択不可**」——requirements の引用と完全一致。

追加の観察: 正典には **`char*.bindoption*.group`（3 人目以降）** も存在するが、現行 parsers は `sakura.`/`kero.` 接頭辞のみ走査し、`scope_namespace`（bind.rs:158-164）も "0"/"1" のみ写像（char2+ は M1 未取込・M-dual シーム・D7）。**本 spec でも既存縮退の維持が整合的**（新規の乖離ではない・語彙として設計文書に明記推奨）。

## 2. 要件フィージビリティ

| 要件 | 実現性 | 根拠 |
|---|---|---|
| R1（3 値読み取り） | 高 | `parse_bindoption_mustselect` の置換 1 関数＋モデル 2 フィールド。寛容パース（R1.4/R1.5）は既存の `None` 読み流し流儀の延長。決定論（R1.7）は `parse_kv` の BTreeMap 反復順で既に成立 |
| R2（非宣言の排他置換） | 高 | `apply_bind_exclusive`＋`category_ids` が汎用形で既存。述語の反転と actor 1 行で成立。R2.4〜R2.7 は既存流儀の維持 |
| R3（回帰の錨） | 高 | mustselect 経路・multiple 経路・異カテゴリ共存とも既存テストが実体を持つ（actor_bind_loop_tests:135-190・:195-239 等） |
| R4（決定論檻） | 高 | ①採取②判定は純関数（steering 規律どおり全網羅可能）。最小再現（brief 記載）は actor 経路の in-crate テストで固定可能 |
| R5（実機サインオフ） | 高 | 流儀・判定 grep・採取実績（bindopt-debug-observation.log）あり。**Unknown**: 是正後ログで「Unchanged 沈黙」と「正しい冪等 Unchanged」の判別基準（R5.4）の具体化は設計で定義要 |
| R6（文書整合・登記） | 中〜高 | 棚卸り（§1.4）で作業面は確定。R6.4 の mustselect off 裁定は要件ディスカッション事項 |

## 3. 実装アプローチ（複数案・トレードオフ）

### Option A: 既存パターンの最小延長（集合 2 本追加）

`BindGroupDefaults` へ `sakura_multiple`/`kero_multiple: Vec<String>` を追加し、`BindResolver` へ multiple の `BTreeSet` 2 本を追加。述語 `is_exclusive(ns, category) = !multiple.contains(category)` を新設し actor.rs:367 を差し替え。`is_mustselect` は R6.4 の帰趨（mustselect 解除不可の将来シーム）に応じ保持または退役。

- ✅ 既存の mustselect 搬送（Vec→BTreeSet→述語）と完全同型＝レビュー容易・差分最小
- ✅ `#[non_exhaustive]`＋`..Default::default()` ゆえモデル追加は既存テスト無傷
- ❌ `BindResolver::new` が 4→6 引数へ肥大（同順型引数の取り違えリスク・呼出元 8 箇所追随）
- ❌ 「mustselect 集合」「multiple 集合」の 2 集合並置は 3 値意味論を暗黙にしか表現しない（第 4 の状態「両方宣言」の扱いが読み手に非自明）

### Option B: 3 値ポリシーの型付け（enum＋表）

`BindChoicePolicy { MustSelect, Default, Multiple }`（`+` 併記 `mustselect+multiple` の表現を含めるなら flags 型）を導入し、parsers は宣言を忠実転記（転写層原則を守るなら Vec<(String, オプション語彙)> のまま）、seriko 側 `BindResolver` が `BTreeMap<String, Policy>` ×2 を持ち `policy(ns, category)` 1 アクセサで引く。

- ✅ 3 値（＋併記）が型で明示され、R6.4（mustselect 解除不可）を将来拾う際の拡張点が自然
- ✅ 述語が 1 本化され、doc の語彙一掃（R6.1）と実装語彙が一致する
- ❌ 変更面が広い（モデル・resolver・構築・テスト fixture 全て）。parsers 転写層原則との整合（enum は「解釈」か「転記」か）の裁定が要る
- ❌ 差分レビュー量が A の 2〜3 倍

### Option C: A の搬送を構造体引数で包む（署名変更耐性）

実装意味論は A と同じだが、`BindResolver::new(tables: BindTables)` のような単一構造体（または builder）へ署名を変え、以後のオプション追加（R6.4 拾い・char2+ 等）で**再びの全呼出元追随を不要化**する。

- ✅ 今回 1 回の atomic 追随（8 箇所）で署名churn を打ち止めにできる。名前付きフィールドで引数取り違えを排除
- ✅ `empty()` は据え置き＝W6 前提条件に触れない
- ❌ A より初期差分がやや大きい。「必要になってから」原則との緊張（YAGNI）——ただし本件自体が 2 度目の署名変更（mayuna-compose 4 引数化に続く）で、R6.4 裁定次第で 3 度目が見えている

**組合せ**: A＋C は両立（意味論は A、搬送形のみ C）。B は A/C と排他。

### 適用分岐の形（全案共通の確認事項）

新分岐は `if on && is_exclusive(ns, category) { apply_bind_exclusive } else { apply_bind }`。

- off は常に `apply_bind`（除去）＝R2.2/R3.2（mustselect off 素通し維持を含む）
- `mustselect+multiple` 併記は「排他か」の判定で **multiple が優先**（複数可）と解するのが正典文言（「multipleで複数のパーツを選択可能」）に整合——設計判断 #4 として明示裁定を推奨
- 未知カテゴリ（名前解決済みだが bindoption 非宣言）→ 排他＝正典既定。`category_ids` は `.name` 表由来ゆえ解決済みカテゴリなら必ず引ける（解決不能は :351-362 で先に error!+skip 済み＝到達しない）

## 4. 工数・リスク

- **工数: M（3〜7 日）**——コア差分は小（parsers 1 関数＋2 フィールド・seriko 述語＋1 行・assets 数行）だが、3 クレート跨ぎの atomic 追随（8 箇所）＋文書/テスト文言の一掃（§1.4 の 7 群）＋決定論檻の網羅追加（R4.2/R4.3 のマトリクス）＋実機サインオフ（実走・記録文書）を含むため。
- **リスク: 低〜中**——判定は全て純関数で檻が容易（低）。中に寄せる要因は (a) 既定の意味反転が「multiple 集合が空の非空 resolver」を使う全既存テストの挙動を変え得ること（実測では同一カテゴリ複数 bind を前提とするテストは無し＝影響は文言のみ、ただし設計時に全数再監査が必要）、(b) 実機サインオフが必達で、檻が緑でも実機で覆る前歴が 2 度あるドメインであること。

## 5. 設計フェーズへの推奨と Research Needed

**推奨**: Option A（＋必要なら C の構造体引数）を基線とし、B は R6.4 を本 spec で拾う裁定になった場合の代替として再評価する。①採取（parsers）②判定（seriko 純関数）③結線（actor/assets）④実機観測、の brief 境界 4 分割は現物と整合しており設計の骨格にそのまま使える。

**Research Needed（設計フェーズへ持ち越し）**:

1. **R5.4 の判定基準の具体化**——是正後も「正しい冪等 Unchanged」（同一パーツ再 on）は debug に出る。飽和パターン「不在」の grep 判定式（例: Changed の継続性・カテゴリ別 Changed/Unchanged 比）を設計で定義する。
2. **既存テスト全数の意味反転監査**——「multiple 集合が空の非空 resolver」を使うテスト（actor_bind_loop_tests・bind_e2e・actor_dispatch_tests 等）を全数列挙し、同一カテゴリ複数 on を暗黙前提にしていないことをタスク化して確認する（本分析のサンプル監査では該当なし）。
3. **`doc/emo2-conformance-scope.md` の bindoption 記述**の現物確認と追随要否。
4. **SSP 実挙動の `mustselect+multiple` 併記時の解釈**——正典文言からは multiple 優先（複数可）が読めるが、SSP 実機の追験可否を設計で判断（追験不能なら正典文言解釈で確定し設計に根拠を明記）。

## 6. 設計判断事項（要件ディスカッションへ供する・番号付き・**全件裁定済み→§8**）

1. **【裁定済 2026-08-11・要件ディスカッション議題 1】mustselect「解除不可」適合＝本 spec で拾う**——開発者裁定（GO）。off 指示は bind 集合を変えず読み流し、ログに痕跡を残す（requirements R3.2 改訂済・旧 R6.4 の先送り登記条項は解消）。帰結: Option B（3 値型付け）の相対優位が上がる（#2 の判断材料）。emo2 実害なし（休眠中）は実測済みのため実機リスク増なし。
2. **3 値の搬送形**——Option A（集合 2 本・最小同型）／B（enum ポリシー表・型で明示）／C（A＋構造体引数で署名 churn 打ち止め）。判断軸: R6.4 の帰趨・レビュー差分量・3 度目の署名変更を許容するか。
3. **`BindResolver::new` の署名戦略**——6 引数直列（同順型の取り違えリスク）か名前付き構造体か。いずれでも `empty()` 署名不変（W6 並走無干渉の前提）は維持できるが、明示の不変条件としてテスト/設計に固定するか。
4. **`mustselect+multiple` 併記時の排他判定**——multiple 優先（複数可・非排他）で確定してよいか。R1.3 は「各オプションを個別に収録」までを定めており、適用時の優先則は設計の領分。
5. **`is_mustselect` 語彙の去就**——排他述語の 1 本化後、mustselect 集合と述語を（R6.4 シームとして）保持するか、退役させ R6.1 の一掃対象に含めるか。`BindGroupDefaults::is_mustselect`（本番消費者なし・テスト専用）の扱いも同枠。
6. **旧語彙テストの改名方針**——`bind_non_mustselect_accumulates_via_actor`（実体＝異カテゴリ加算・保持）の新名と、resolve.rs in-file テストの期待値反転の粒度（R4.5/R6.1 の適用単位）。
7. **`char*.bindoption*.group`（char2+）の登記**——正典に実在・現行未走査（M-dual シーム・D7 整合）。本 spec の設計文書で「既存縮退の維持」として語彙明記するか（新規乖離ではないため要件追加は不要という整理でよいか）。
8. **mayuna-compose 引用の差し替え様式**——コード内の「D11・R4.5」引用を本 spec の要件 ID へ置換する際の引用規約（R6.1/R6.2 の実施形。completed 文書は不改変・roadmap 追記で追跡＝R6.3）。

---

# 設計フェーズ追記（2026-08-11・kiro-spec-design）

## 7. 設計ディスカバリ（light・現物再検証）

Extension（既存システムの是正）ゆえ light discovery を主コンテキストで実施。§1 の全アンカーを Grep/Read で再照合し**全件現物一致**を確認した（`BindResolver::new` 8 箇所・`empty()` areka 側 4 箇所＋seriko 側多数・actor.rs:367 分岐・resolve.rs:172-205・state.rs:342-402・assets.rs:253-267・looper.rs:215）。外部依存の新規調査は不要（新規 crates.io 依存なし・正典は §1.5 で逐語保全済み）。

**Research Needed（§5）の消化**:

1. **R5.4 判定基準** → design.md「Real Machine Sign-off」で確定。J1（各まばたき発火の id が直前のまばたき Changed の id と一致＝任意時点で発火し得るまばたき id 高々 1 種・是正前ログで必ず赤になる既知ケース較正付き）・J2（|Changed(まばたき)−Changed(目)| ≤ 2 かつ最終まばたき Changed が最終目 Changed の 120 秒以内＝観測済み飽和形の直接否定）・J3（目視）。
2. **既存テスト全数の意味反転監査** → 実施済み（下記 §7.1）。同一カテゴリ複数 on を非 multiple カテゴリで前提とするテストは**ゼロ**。
3. **`doc/emo2-conformance-scope.md`** → 現物確認済み（:43）。bindoption 言及は descript キーの列挙のみで「非宣言＝非排他」等の旧前提の主張を**含まない**＝追随不要。
4. **SSP 実機の併記解釈追験** → 行わない裁定（design D4）。正典文言「multipleで複数のパーツを選択可能」が一意に読め、emo2 に併記宣言が存在せず観測対象にならないため。正典文言解釈（multiple 優先）で確定し design に根拠明記。

### 7.1 意味反転監査台帳（「multiple 集合が空の非空 resolver」全数・2026-08-11 実測）

> **【設計時点の台帳・実装後は §11 が最新】** 本節は design 生成時（実装前）の実測である。実装（タスク 2〜6.1）で resolver・テストが増減し行番号も動いたため、**現物と突合した最新の台帳はタスク 6.2 の §11**（結論は同じだが本節には**漏れが 2 件**あった）。

| resolver | 場所 | 非宣言カテゴリの構成 | 反転影響 |
|---|---|---|---|
| `tiny_resolver` | bind.rs:351 | 腕 1 パーツ／kero 脚 1 パーツ | なし（1 パーツ/カテゴリ＝排他と加算が集合同値） |
| `mustselect_resolver` | bind.rs:362 | 紅 1 パーツ（目は mustselect） | なし |
| `arm_bind_resolver` | actor_bind_loop_tests.rs:61 | 腕 1 パーツ | なし |
| `eye_mustselect_resolver` | actor_bind_loop_tests.rs:69 | （非宣言側なし） | なし |
| 檻内 2 カテゴリ表 | actor_bind_loop_tests.rs:198-202 | 腕 1・肩 1（**異カテゴリ**加算檻） | なし（3.4 で正典後も有効・改名のみ D6） |
| `test_bind_resolver` | tests/bind_e2e.rs:118 | 腕 1・頬 1 | なし |
| `mustselect_bind_resolver` | tests/bind_e2e.rs:236 | 紅 1（目は mustselect） | なし |
| `empty()` 系全数 | actor_dispatch_tests／tests/{regression,loop_integration,cue_sequence,balloon_face_e2e} ほか | 名前表空＝resolve 常時 None | なし（適用に到達しない） |

結論: 既存テストの**期待値変更はゼロ**。変更は語彙（名前・コメント・アサーションメッセージ）と `new()` 署名追随のみ。実装フェーズで本監査を再実行して緑を確認する（R3.5）。

## 8. 設計判断の裁定（§6 → design.md D1〜D8）

| §6 | 裁定 | design |
|---|---|---|
| #1 mustselect 解除不可 | 本 spec で実装（開発者裁定 GO）。off は集合不変＋`warn!`（レベル根拠: steering logging の「無効なパラメーター」区分・debug では info 実走で不可視＝無言の握り潰しの再来ゆえ不採用） | D1 |
| #2 搬送形 | **合成形**: A の格納（parsers は Vec 追加のみ・転写層原則維持）＋B の語彙を seriko 限定で採用（`BindChoicePolicy`＋`policy()` 単一アクセサ——#1 裁定で off 経路に MustSelect/Default 判別が必須となり bool 述語 2 本より型付き 3 値が一致）＋C の構造体引数。純 B（parsers に enum）は転写層原則違反で棄却。純 A（6 引数直列）は同型 `BTreeSet<String>` 4 本の取り違えリスクで棄却 | D2 |
| #3 new 署名戦略 | `new(sakura, kero, options: BindOptionDecls)`（named-field＋`Default`）。`empty()` は署名不変（W6 前提・専用テスト不要＝外部呼出 10 箇所超のコンパイル結合が検証）。意味反転（empty＝全カテゴリ Default）は resolve 常時 None ゆえ実害なし——この根拠を檻に固定 | D3 |
| #4 併記の優先則 | multiple 優先（複数可）。導出順＝multiple 所属 → mustselect 所属 → Default。parsers は両集合へ転記し情報を落とさない（将来の解釈変更に parsers 無改変で耐える） | D4 |
| #5 is_mustselect の去就 | seriko 側は**退役**（policy() へ一本化・2 値語彙の残存を許さない）。parsers 側 `BindGroupDefaults::is_mustselect` は転写所属照会として**保持**＋対称の `is_multiple` 追加（テスト可読性・解釈を含まない） | D5 |
| #6 テスト改名 | `bind_non_mustselect_accumulates_via_actor` → `bind_cross_category_accumulates_via_actor`（実体保持）。resolve.rs `multiple_option_not_ingested` は期待値ごと反転。新語彙＝排他置換/既定（高々 1 個・解除可）/複数可 | D6 |
| #7 char2+ の登記 | 既存縮退の維持を design に語彙明記（M-dual シーム・新規乖離でないため要件追加不要という整理を採用） | D7 |
| #8 引用差し替え規約 | 「D11・R4.5」（mayuna-compose）→ `bindopt D1〜D8`／`bindopt N.M` 形式（spec 略号を冠し裸の ID を残さない）。completed 不改変・覆しの記録は design.md 専用節＋完了時 roadmap 追記 | D8 |

## 9. 統合（synthesis）の記録

- **一般化**: 「排他か」と「解除不可か」の 2 述語は同一の 3 値ポリシーの射影——`policy()` 1 アクセサへ一般化（インターフェースを一般化し実装は現要件の範囲に留める）。`apply_bind_exclusive`／`category_ids` は元よりカテゴリ非依存の汎用形で、非宣言カテゴリへ**無改変**で適用可能（§1.2 の再確認）。
- **build vs adopt**: 全面 adopt（既存資産再利用）。新設は型 2 つ（`BindChoicePolicy`・`BindOptionDecls`）と関数 2 つ（`policy`・`parse_bindoption_options`）のみ。新規依存なし。
- **簡素化**: seriko `is_mustselect` 退役（述語 1 本化）。char2+ の投機的配線なし（D7＝縮退維持のみ）。`BindOptionDecls` は「3 度目の署名 churn を打ち止める」現実の反復実績（mayuna-compose 4 引数化→本件）に基づく採用で、投機ではない。

## 10. 設計レビューゲート結果（2026-08-11）

- 機械検査: 全 34 要件 ID（1.1-1.7／2.1-2.7／3.1-3.5／4.1-4.5／5.1-5.6／6.1-6.4）がトレーサビリティ表に存在・境界 4 節充足・File Structure Plan 具体パス（new 8 呼出元台帳含む）・境界と file plan の整合・孤児コンポーネントなし——**合格**。
- 判定レビュー: 要件被覆に穴なし（Research Needed 4 件は §7 で全消化）・分岐は (on×policy) 直積表で網羅・実装タスクへ分割可能（採取→判定→結線→檻→実機の brief 境界 4 分割と一致）——**合格（修復パス 0 回）**。

---

# 実装完了後の再監査（2026-08-11・タスク 6.2）

## 11. 意味反転監査の再実行（全数・現物実測）

**実施日**: 2026-08-11（タスク 1〜6.1 コミット済み・HEAD `e1335ee`・作業ツリークリーン）
**根拠**: workspace 全体を `BindResolver::new` / `BindResolver::empty` で grep して構築点を全数列挙し、各構築点を消費する全テストのテスト本体（名前表構成・投入する cue 列・期待集合）を逐一読んで判定した。コード・テストは 1 行も変更していない。
**結論**: **design.md §テスト影響監査 の結論（「同一カテゴリ複数 on を前提とするテストは存在しない」）は現物と一致する**。既定の意味反転で期待値が変わるテストはゼロ。ただし**設計時点の台帳（§7.1）には漏れが 2 件**あり（下記 §11.4）、いずれも結論を変えない。

### 11.1 判定軸

排他置換 `apply_bind_exclusive` は「現在集合 − `category_ids(カテゴリ)` ∪ {対象 ID}」ゆえ、加算と結果が食い違うのは次の 2 条件のいずれかが成り立つときに限る。

- **(a)** 同一カテゴリの**別パーツ**へ 2 回以上 on が流れる
- **(b)** 対象カテゴリの**別パーツ ID が事前に集合へ載っている**（静的既定 bind 集合 `static_binds` 経由を含む）

したがって各テストについて「1 カテゴリあたりの宣言パーツ数」「同一カテゴリへの on 回数」「静的既定集合と対象カテゴリ ID 集合の交わり」の 3 点を実測した。

### 11.2 `multiple` 集合が空の非空判定器（＝反転の影響を受け得る全数）

| # | 判定器 | 構築 file:line | 非宣言（Default）カテゴリのパーツ数 | 消費テスト（file:line・fn） | 同一カテゴリへの on 回数 | 静的既定との交わり | 判定 |
|---|---|---|---|---|---|---|---|
| 1 | `tiny_resolver` | `crates/areka-seriko/src/bind.rs:404` | sakura 腕 1／kero 脚 1 | bind.rs:458 `policy_empty_or_absent_is_default`／:681 `resolve_declared_returns_id_per_namespace`／:697 `resolve_is_namespace_isolated`／:713 `resolve_unknown_returns_none` | 0（純関数アクセサのみ・適用経路を通らない） | なし | **影響なし** |
| 2 | `mustselect_resolver` | `bind.rs:415` | 紅 1（目は mustselect 3 パーツ） | bind.rs:432 `policy_mustselect_only_declared_category_per_namespace`／:655 `category_ids_collects_category_members_ascending` | 0（純関数アクセサのみ） | なし | **影響なし** |
| 3 | `arm_bind_resolver` | `crates/areka-seriko/src/actor_bind_loop_tests.rs:64` | 腕 1（1302） | :117 `bind_apply_on_shown_emits_show_and_info_marker`／:645 `bind_apply_on_hidden_scope_state_only_no_emit`／:676 `bind_toggle_form_warns_no_emit`／:717 `bind_category_wide_form_warns_no_emit`／:752 `bind_malformed_errors_no_emit`／:796 `bind_scope_unmapped_warns_no_emit`／:866 `bind_name_gate_other_name_is_benign_debug_no_emit`／:912 `bind_noncanonical_addressee_severity_split` | 高々 1（残りは Toggle/CategoryWide/Malformed/宛名違い＝適用に到達しない縮退枝） | static `{1100,1207}` ∩ `category_ids(腕)={1302}` = ∅ | **影響なし**（1 パーツ/カテゴリ＝排他と加算が集合同値） |
| 4 | `blink_default_resolver` | `actor_bind_loop_tests.rs:91` | まばたき 2（1400/1402） | :422 `bind_default_category_off_removes_part`（on×1→off×1）／:471 `bind_default_exclusive_replace_emits_show_and_info_marker`（**別パーツへ on×2＝本 spec の新規檻**）／:536 `bind_default_same_part_re_on_is_unchanged_no_emit`（**同一パーツ**へ on×2） | :471 のみ別パーツ 2 回 | static `{1100,1207}` ∩ `{1400,1402}` = ∅ | **本 spec が新規に追加した檻**（反転後の期待値で書かれている・既存テストではない） |
| 5 | `eye_mustselect_resolver` | `actor_bind_loop_tests.rs:100` | （非宣言カテゴリなし・目は mustselect 3 パーツ） | :170 `bind_mustselect_second_on_replaces_prior_part_in_category`／:271 `bind_mustselect_off_is_ignored_with_warn` | 目へ on×2（:170） | ∅ | **影響なし**（MustSelect は反転前から排他） |
| 6 | インライン（まばたき 2 パーツ） | `actor_bind_loop_tests.rs:239` | まばたき 2（1400/1402） | :233 `bind_default_category_second_on_replaces_prior_part` | on×2 | ∅ | **本 spec の最小再現檻**（タスク 4 で新規追加・既存テストではない） |
| 7 | インライン（腕 1・肩 1） | `actor_bind_loop_tests.rs:604` | 腕 1・肩 1 | :598 `bind_cross_category_accumulates_via_actor` | **異なるカテゴリ**へ 1 回ずつ | ∅ | **影響なし**（排他置換が外すのは同一カテゴリのみ＝期待値 `{1100,1207,1302,1500}` 不変） |
| 8 | `test_bind_resolver` | `crates/areka-seriko/tests/bind_e2e.rs:119` | 腕 1（1100）・頬 1（1200） | :309 `bind_off_reissues_show_with_updated_set_end_to_end`／:323 `bind_off_then_on_round_trips_and_reissues_end_to_end`／:342 `bind_off_twice_is_idempotent_no_third_show_end_to_end`／:357 `unresolvable_bind_does_not_grow_display_list_end_to_end`／:377 `bind_and_text_streams_do_not_cross_contaminate_end_to_end` | 高々 1 | static `{1100,1207}` ∩ `category_ids(腕)={1100}` = `{1100}` ——ただし**唯一のパーツ**ゆえ排他置換は「1100 を除いて 1100 を足す」＝同値 | **影響なし** |
| 9 | `mustselect_bind_resolver` | `bind_e2e.rs:241` | 紅 1（1600）（目は mustselect 3 パーツ） | :428 `mustselect_sequence_replaces_prior_part_end_to_end`（目へ on×3）／:472 `default_category_explicit_on_then_off_removes_part_end_to_end`（紅 on→off）／:498 `mustselect_replace_does_not_drop_other_category_end_to_end`（紅 on×1＋目 on×2） | 紅へは高々 1 | static `{1207}` ∩ `{1600}` = ∅ | **影響なし** |
| 10 | 実 emo2 判定器（本番構築経路） | `crates/areka/src/emo2_boot/assets.rs:272`（`build_boot_assets`） | 実 fixture: まばたき 4（1400-1403）・キラリ 2（1700/1701）・髪飾り 2（1800/1801）・紅 1（1600）／mustselect＝腕・口・眉・目 | `crates/areka/src/emo2_boot/assets_tests.rs:503` `build_boot_assets_bind_resolver_resolves_emo2_names`／:534 `build_boot_assets_bind_resolver_carries_mustselect`（いずれも `resolve`／`policy`／`category_ids` の純関数照会のみ・適用経路を通らない）／`crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:298` `spine_e2e_sakura_blink_after_bind_one_cycle_golden`（実 `boot_live` 貫通・`\![bind,まばたき,通常,1]` **1 回のみ**）／:353 `spine_e2e_sakura_blink_default_off_emits_nothing`（bind を通さない） | まばたきへ on×1 | 実 fixture の静的既定 on＝`{1100 腕, 1207 口, 1302 目, 1500 眉, 1800 髪飾り}`。`category_ids(まばたき)={1400,1401,1402,1403}` と交わらない | **影響なし** |

### 11.3 `multiple` 集合が非空の判定器（反転の対象外・対照として列挙）

| 判定器 | 構築 file:line | multiple 宣言 | 消費テスト |
|---|---|---|---|
| `policy_matrix_resolver` | `bind.rs:484` | sakura `{髪, 腕}`／kero `{尾, 羽}` | bind.rs:510／:541／:575／:604（導出マトリクス檻・純関数） |
| `hair_multiple_resolver` | `actor_bind_loop_tests.rs:75` | sakura `{髪飾り}` | :344 `bind_multiple_category_two_parts_coexist_via_actor`／:381 `bind_multiple_category_off_removes_only_that_part` |
| `multiple_bind_resolver` | `bind_e2e.rs:276` | sakura `{髪飾り}` | :526 `multiple_category_two_parts_coexist_end_to_end` |

### 11.4 `empty()` 系（名前解決が常に不成立＝適用に到達しない・別枠）

`BindResolver::empty()` は名前表が両スコープとも空ゆえ `resolve()` が常に `None` で、bind cue は `error!`＋skip の解決不能枝に落ちて適用へ到達しない（`bind.rs:629` `empty_resolver_is_default_policy_and_never_reaches_apply` がこの根拠自体を檻に固定している）。ポリシーは全カテゴリ `Default`（＝意味反転の影響下）に変わるが、**挙動への実害はゼロ**。呼出全数:

- seriko in-crate: `actor_dispatch_tests.rs:117/206/230/264/296/333/377/426/457/491/549/605/613/651/659/716/770/818`（18）・`actor_bind_loop_tests.rs:832（bind_unresolvable_errors_no_emit・解決不能枝を意図して使う）/1121/1152/1166/1206/1270/1278/1288/1298`（9）・`bind.rs:460/630/730`（3）
- seriko tests/: `regression.rs:74/135/190`・`loop_integration.rs:82`・`cue_sequence.rs:71`・`balloon_face_e2e.rs:85`（6）
- areka 側（**署名不変の監視対象 4 箇所**・W6 並走無干渉の前提）: `input_events/balloon_test_support.rs:73`・`emo2_boot/frame_test_support.rs:71`・`emo2_boot/spine.rs:671`・`emo2_boot/mod.rs:374` ——**現物一致・無改変**（`empty()` の署名が変わっていないことは、これら 4 箇所を触らずに workspace がコンパイルを通ることで検証されている）

### 11.5 設計時点の台帳（§7.1）との差分

結論は一致するが、**§7.1 には次の 2 件の漏れがあった**（いずれも判定は「影響なし」ゆえ結論は不変。設計の監査漏れとして記録する）:

1. **areka クレート側の実 emo2 判定器を列挙していなかった**——§7.1 は seriko 側の 7 fixture と `empty()` 系のみを数え、`build_boot_assets` が実 emo2 descript から組む**本番構築経路の判定器**（`assets_tests.rs` 2 本＋`spine_seriko_loop_tests.rs` の実貫通 1 本）を台帳に載せていなかった。実 fixture は非宣言カテゴリに**複数パーツ**を持つ（まばたき 4・キラリ 2・髪飾り 2）ため、seriko 側 fixture（全て 1 パーツ/カテゴリ）より条件が厳しい。実測では貫通テストが送る on はまばたきへ 1 回のみ、かつ静的既定 on 集合 `{1100,1207,1302,1500,1800}` がまばたきの ID 集合と交わらないため影響なし。
2. **判定軸 (b)（静的既定 bind 集合との交わり）を明示していなかった**——§7.1 は「1 カテゴリあたりのパーツ数」だけを見ており、「対象カテゴリの別パーツが `static_binds` 経由で事前に載っている」経路を条件として立てていない。実測では全テストで交わりが空か唯一パーツ（`test_bind_resolver` の 腕/1100）であり影響なし。

あわせて **`BindResolver::new` の呼出元は 8 → 13 箇所へ増えた**（本 spec が檻を追加したため）。実装後の全数 13: 本番 1（`assets.rs:272`）／seriko in-crate 9（構築関数 7＝`bind.rs:409/427/504`・`actor_bind_loop_tests.rs:67/85/95/111`＋テスト本体インライン 2＝`actor_bind_loop_tests.rs:239/604`）／seriko tests/ 3（`bind_e2e.rs:126/253/286`）。design.md の「全 8 呼出元」台帳は**設計時点の値**として読むこと。

### 11.6 本番挙動で新たに排他となるカテゴリ（実 emo2・記録）

反転により実 emo2 で新たに排他置換の対象となるのは**非宣言かつ複数パーツを持つカテゴリ**——**まばたき（1400/1401/1402/1403）・キラリ（1700/1701）・髪飾り（1800/1801）**の 3 つ。まばたきが本 spec の是正対象そのもの。髪飾りは `1800.default,1` ゆえ、今後 `\![bind,髪飾り,ボンボン,1]` が来ると既定オンの 1800（リボン）が自動で外れる——これが正典の既定挙動（高々 1 個）であり意図した是正である。決定論テストでこの経路を通るものは現時点で存在しない（実機サインオフ〔タスク 7.2〕はまばたきを観測対象とする）。

### 11.7 workspace 全緑の実測（要件 3.5・4.4）

前提ビルド（PowerShell・Git Bash 不可）: `cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc` → exit 0。

`cargo test --workspace` を 4 回実行:

| 回 | 結果 | 内訳 |
|---|---|---|
| 1 | **exit 0**（全緑） | 全 60 テストバイナリ ok・約 1 分 30 秒 |
| 2 | **exit 0**（全緑） | — |
| 3 | exit 101 | `actor::bind_loop_tests::bind_apply_on_shown_emits_show_and_info_marker` 1 本のみ FAILED |
| 4 | **exit 0**（全緑） | — |

第 3 回の赤は**別 spec 所有の既知間欠赤**（`areka-P0-test-cage-determinism`／W6.9 に登記済みの tracing callsite 毒化＝ログ捕捉 0 件）であり、本 spec の変更とは因果独立。追加の頻度実測として `cargo test -p areka-seriko -p areka-emo-compose --lib` を 20 回反復した内訳:

- **RED 3/20 回**（内訳: `actor::dispatch_tests::non_shell_broadcast_reception_is_benign_debug_no_warn_error` ×1・`scale::ratio_tests::mul_degradation_emits_warn_log` ×2）

観測された赤は**全て登記済みの 4 本の集合内**（`non_shell_broadcast_reception_is_benign_debug_no_warn_error`／`bind_apply_on_shown_emits_show_and_info_marker`／`wait_broadcast_reception_is_benign_debug_no_warn_error`／`mul_degradation_emits_warn_log`）に収まり、**本 spec が追加した檻は 1 度も落ちていない**。要件 3.5 の「決定論的緑」は、この 4 本を除いた全体で成立している——4 本の非決定性の解消は W6.9 の所有事項として残る（本 spec では是正しない）。

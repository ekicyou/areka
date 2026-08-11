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

## 6. 設計判断事項（要件ディスカッションへ供する・番号付き）

1. **【裁定済 2026-08-11・要件ディスカッション議題 1】mustselect「解除不可」適合＝本 spec で拾う**——開発者裁定（GO）。off 指示は bind 集合を変えず読み流し、ログに痕跡を残す（requirements R3.2 改訂済・旧 R6.4 の先送り登記条項は解消）。帰結: Option B（3 値型付け）の相対優位が上がる（#2 の判断材料）。emo2 実害なし（休眠中）は実測済みのため実機リスク増なし。
2. **3 値の搬送形**——Option A（集合 2 本・最小同型）／B（enum ポリシー表・型で明示）／C（A＋構造体引数で署名 churn 打ち止め）。判断軸: R6.4 の帰趨・レビュー差分量・3 度目の署名変更を許容するか。
3. **`BindResolver::new` の署名戦略**——6 引数直列（同順型の取り違えリスク）か名前付き構造体か。いずれでも `empty()` 署名不変（W6 並走無干渉の前提）は維持できるが、明示の不変条件としてテスト/設計に固定するか。
4. **`mustselect+multiple` 併記時の排他判定**——multiple 優先（複数可・非排他）で確定してよいか。R1.3 は「各オプションを個別に収録」までを定めており、適用時の優先則は設計の領分。
5. **`is_mustselect` 語彙の去就**——排他述語の 1 本化後、mustselect 集合と述語を（R6.4 シームとして）保持するか、退役させ R6.1 の一掃対象に含めるか。`BindGroupDefaults::is_mustselect`（本番消費者なし・テスト専用）の扱いも同枠。
6. **旧語彙テストの改名方針**——`bind_non_mustselect_accumulates_via_actor`（実体＝異カテゴリ加算・保持）の新名と、resolve.rs in-file テストの期待値反転の粒度（R4.5/R6.1 の適用単位）。
7. **`char*.bindoption*.group`（char2+）の登記**——正典に実在・現行未走査（M-dual シーム・D7 整合）。本 spec の設計文書で「既存縮退の維持」として語彙明記するか（新規乖離ではないため要件追加は不要という整理でよいか）。
8. **mayuna-compose 引用の差し替え様式**——コード内の「D11・R4.5」引用を本 spec の要件 ID へ置換する際の引用規約（R6.1/R6.2 の実施形。completed 文書は不改変・roadmap 追記で追跡＝R6.3）。

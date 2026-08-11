# Requirements Document

## Introduction

### 問題

エンドユーザ（ゴースト利用者）の実機で、**表情が非可逆に固着する**——「むらさきの目がジト目になった後、他の表情に切り替わらないように見える」（開発者の実機観測・2026-07-26）。一度固着すると以後どんな会話が来ても表情が戻らず、アプリ再起動まで復帰しない。表情はキャラクター表現の中核であり、固着すると会話内容と表情が乖離してゴーストが壊れて見える。しかも**ゴースト側スクリプトからは原理的に復帰不能**で、ゴースト作者に回避手段がない。

### 根本原因（確定・4 系統一致）

ukadoc 正典（`sakura/kero/char*.bindoption*.group,カテゴリ名,オプション`——2026-08-11 に ukadoc MCP で逐語再確認: 「mustselectでパーツを必ず1つ選択、multipleで複数のパーツを選択可能。オプションは+区切りで複数可。」既定値「選択解除可能、複数選択不可」）は着せ替えカテゴリの **3 値意味論**を規定する:

| 宣言 | 正典の意味 |
|---|---|
| `mustselect` | ちょうど 1 個（解除不可） |
| **非宣言（既定）** | **高々 1 個**（解除可・**複数選択不可**） |
| `multiple` | 複数可 |

areka の現実装は「`mustselect` か、さもなくば加算」の **2 値**で、**非宣言を `multiple` と同一視**している。descript 読み取りが `mustselect` のみを収録し `multiple` 宣言を破棄するため、下流には「明示 multiple」と「非宣言」を区別する情報が**存在しない**——これが構造的な根である。

emo2 の shell descript は `bindoption` を腕/口/眉/目の 4 カテゴリにしか宣言しておらず、**まばたきカテゴリ（1400-1403）は非宣言**。ゴーストは正典どおり on のみを送り off を送らない（ベースウェアの排他置換に依存する正しい作法）ため、まばたきのパーツが永久に積み上がる。z-order は作者意図どおり animation ID 昇順（14xx が 13xx の上）なので、積み上がったジトーまばたき（1402）の不透明最終コマが以後のすべての目・まばたき表示を覆い、表情が固着する。さらに 1402 は `interval,bind+random,4` の抽選発火アニメゆえ、会話と無関係に「唐突にジト目へ変わる」観測も必然の帰結である。

#### 根因はもう 1 段ある（2026-08-11 実機サインオフで判明・因果記述の訂正）

**当初この文書は「排他置換で積み上がりを止めれば固着は消える」と結論していたが、それは因果を一段取り違えていた。** 2026-08-11 の実機サインオフ（`real-machine-signoff.md`）で、bind 層の是正は機械判定 J1/J2 が FAIL→PASS へ反転して**実機で証明された**にもかかわらず、**目視の固着は再現した**。

取り違えの中身: emo2 の 14xx は `pattern0` を**持たない**（`shell/master/surfaces.txt:84-86`）。したがって 14xx の視覚寄与は**すべて再生状態（`PatternState`）経由**であり、bind ゲートの外側にある。**bind 集合を正典どおり高々 1 個に保っても、既に置かれた残留コマには何の影響も及ばない。**

固着の**直接の機構**は次の 2 段である:

1. **bind 集合の単調肥大**（本文書が当初から捉えていた層）——非宣言カテゴリを複数可と同一視したことによる。**要件 2 で是正済み・実機で証明済み**
2. **bind から外れたパーツの最終コマが掃除されない**（本改訂で新たに要件化する層）——`-1` 終端を持たないアニメは末尾到達後に最終コマを**保持**する仕様であり、その ID が bind 集合から外れても、保持されたコマは合成対象から取り除かれない。不透明な `surface1413`（`eyebase.png`＋`jito.png`）が下層の目を覆い続ける

**(1) だけを直しても固着は消えない**——これが実機が示した事実である。是正には (2) が必須であり、本改訂で Requirement 7 として追加する。

### 決定的証拠（直接観測・2026-08-11）

requirements 作成に先立ち、brief が義務づけた `RUST_LOG=info,areka_seriko=debug` の emo2 実機実走（実 pasta.dll・絶対パス・有界 auto-exit・7 分）を実施し、**「飽和した bind 集合がゴーストの是正指示を無言で握り潰す」ことを推論から直接観測へ格上げした**（ログ: `target/bindopt-debug-observation.log`）:

- ゴーストは**毎回の表情変更で目とまばたきをペアで送り続けている**（01:19:34 目=通常+まばたき=通常 1400、01:20:20 目=ジトー+まばたき=ジトー 1402 …）。**ゴーストは完全にシロ**。
- まばたき集合は {1403}→{1403,1400}→{1403,1400,1402} と単調に育ち（off が来ないため一度入ったパーツが外れない）、Changed（info）は全 7 分でこの **3 回のみ**。01:20:20 の飽和以降、ゴーストが送ったまばたき指示 **28 件**（1400×13・1402×4・1403×16 の一部・実走末尾 01:25:36 まで継続）は**すべて Unchanged の debug ログ**（`RUST_LOG=info` 実走では不可視）に落ちて握り潰されている。同一実走でルーパーは 1400×156・1402×182 を並行発火しており、同一カテゴリ複数パーツの同時 bind も再確認した。
- 対照的に mustselect 宣言済みの目/眉/口カテゴリは正しく排他置換され、変更時のみ info（Changed）が出る——mustselect 配線は健全。

これで根拠は 4 系統独立に一致した: (a) ukadoc 正典の既定値、(b) ソース上の 2 値分岐、(c) ルーパー発火ログでの同一カテゴリ 2 パーツ並行再生（brief 採取の 1400×70/1402×45）、(d) 本日の Unchanged 直接観測。

### 是正の方向（WHAT）

`bindoption` の 3 値意味論を実装する——descript 読み取りが `multiple` 宣言を収録して非宣言と区別可能にし、**非宣言カテゴリを「高々 1 個（解除可）」として排他置換**する。あわせて `mustselect` の**「解除不可」も正典適合**させる（off 指示は bind 集合を変えず読み流す——2026-08-11 要件ディスカッション裁定）。mustselect の着衣側排他と `multiple` 明示時の加算は不変（回帰の錨）。具体の述語設計・搬送形は設計フェーズの領分。

### アンカー再検証（2026-08-11・file-slimming PR#103 マージ後の現物実測）

brief 記載のアンカーは file-slimming（テスト分離・ファサード分割）で一部移動した。現物一致を再確認した最新値:

| 対象 | brief 記載 | 現物（2026-08-11 実測） |
|---|---|---|
| 2 値分岐（is_mustselect 判定） | actor.rs:367 | `crates/areka-seriko/src/actor.rs:367` **不変** |
| Changed=info / Unchanged=debug のログ分岐 | actor.rs:374-384 / :387-396 | 同 :374-384 / :387-396 **不変** |
| Toggle/CategoryWide 未実導出 warn | actor.rs:319-326 | 同 :319-326 **不変** |
| multiple 破棄（mustselect のみ収録） | resolve.rs:172-188 | `crates/areka-parsers/src/package/resolve.rs:172-188`＋`parse_bindoption_mustselect` :196-205 **不変** |
| ルーパー bind ゲート | looper.rs:215 | `crates/areka-seriko/src/looper.rs:215` **不変** |
| 資産構築（BindResolver::new 本番唯一の呼出） | assets.rs:267 | `crates/areka/src/emo2_boot/assets.rs:267` **不変**（現署名は sakura表・kero表・sakura mustselect・kero mustselect の 4 引数） |
| 既存檻 `bind_non_mustselect_accumulates_via_actor` | actor.rs:1546 | **移動**: `crates/areka-seriko/src/actor_bind_loop_tests.rs:195`（テスト分離・検証内容は異カテゴリ加算＝正典修正後も有効） |
| `BindResolver::empty()` 呼出元（areka 側・署名不変条件の監視対象） | balloon.rs:1482・frame.rs:1563・emo2_boot/mod.rs:374・spine.rs:671 | **2 箇所移動**: `input_events/balloon_test_support.rs:73`・`emo2_boot/frame_test_support.rs:71`（テスト分離）／`emo2_boot/mod.rs:374`・`spine.rs:671` 不変 |

また ukadoc 逐語再確認で brief 未記載の正典事実を 1 件確定した: **オプションは `+` 区切りで複数指定可**（例 `mustselect+multiple`）。現実装の読み取りは値全体と `mustselect` の完全一致判定のため、`+` 結合値は mustselect 側も含めて**全部落ちる**。3 値読み取りの実装時に同時に正典適合させる（Requirement 1）。

### ウェーブ文脈

W6 並走 4 本（balloon-visibility ∥ 本 spec ∥ zorder ∥ scope-chain-gap・col は 2026-08-05 着地済み）。kero-balloon（PR#97）マージ済み＝「ker 先着後に rebase」条件は消化済み。本 spec は W7 `emo2-conformance-e2e` 適合 #3（着せ替え表情）の前提。

## Boundary Context

- **In scope**:
  - shell descript の `bindoption*.group` を正典 3 値（`mustselect`／非宣言／`multiple`）で読み取り、非宣言と `multiple` 明示の区別が下流の bind 適用まで届くこと（`+` 区切り複数オプション値の正典適合を含む）。
  - 非宣言カテゴリの排他置換——着衣（on）で同一カテゴリの既 bind 他パーツを自動で外し、カテゴリ内の bind を高々 1 個に保つ。脱衣（off）は可（ゼロ個を許す）。
  - 上記の決定論テスト檻（GPU・実窓・実機不要）と、emo2 実機サインオフ（ログ判定＋目視）。
  - 資産構築から適用分岐までの変更を全呼出元一括で通す atomic 追随（中間 stand-in を挟まない）。
  - 旧 2 値前提のコード内 doc コメント・テスト文言の一掃と、`completed/areka-P0-mayuna-compose` R4.5/D11 を覆す旨の明記＋roadmap 追跡。
  - `mustselect` カテゴリの「解除不可」正典適合——off（脱衣）指示は bind 集合を変えずに読み流す（2026-08-11 要件ディスカッション裁定・同じ 2 値誤実装から派生した同根の正典乖離を本 spec で一括是正）。
  - **bind から外れたパーツの残留コマの掃除（Requirement 7・2026-08-11 実機サインオフ後の裁定でスコープ追加）**——bind 集合から外れた着せ替え ID の保持コマを合成対象から取り除き、再生中なら再生も止める。**これが無いと (1) の排他置換だけでは固着が消えないことが実機で確定した**。掃除の結線は `looper.rs` の再生相・状態側の発行前処理・合成計画の合流条件に及ぶ（下記 Out of scope の「無改変」条項を本改訂で解除）。
- **Out of scope**:
  - SERIKO アニメーションのループ/interval 意味論（`completed/areka-P0-seriko-loop` 領分・本件と無関係）。**ただし「bind から外れた ID の再生停止と保持コマ除去」は本 spec の Requirement 7 が担う**——最終コマ保持そのもの（`-1` 終端なしの仕様）は疑わず、外れたときの後始末だけを足す。
  - z-order／合成順の変更——14xx が 13xx の上に来るのは作者意図どおり（正常・変更は誤った治療）。
  - ゴースト fixture（emo2）の辞書修正——正典準拠の排他実装下では一過性のズレに縮退し自己修復する。fixture を直して症状を隠すのは禁じ手。
  - `\![bind]` の Toggle 形／CategoryWide 形の実導出——全実走ログで発火ゼロ＝本件と無関係（既知の別件先送り）。
  - DPI／スケール関心事（別 spec 領分）。
- **Adjacent expectations**:
  - **W6 並走の無干渉条件**: 本 spec の編集面は parsers（package 読み取り）＋seriko（bind 判定・状態・actor）＋areka 資産構築の 1 行域で、並走 3 本と実測で互いに素。`BindResolver::empty()` の**署名不変**が並走無干渉の前提条件であり、areka 側の現呼出元 4 箇所（`input_events/balloon_test_support.rs:73`・`emo2_boot/frame_test_support.rs:71`・`emo2_boot/mod.rs:374`・`spine.rs:671`）はこの条件下で無傷に保つ。
  - **資産構築の署名変更は全呼出元をコンパイル結合で巻き込む**: 本番呼出は 1 箇所（assets.rs:267）だが、seriko の in-crate／tests 配下に多数のテスト呼出元があり、atomic 追随が必要。
  - **mayuna-compose の覆し**: 本 spec は `completed/areka-P0-mayuna-compose` の R4.5／D11（「非宣言＝既定は非排他で無視」）を覆す。completed 文書は更新不可のため改変せず、本 spec 文書への明記と roadmap 追記で追跡する。同 spec は `mustselect` についても同型の誤仮定（「ゴーストが明示 off を送るはず」）を実機で反証された前例があり、本件は**同じ穴の 2 度目**である。
  - **新規 crates.io 依存を増やさない**（プロジェクト規律）。
  - **ログ無し失敗経路の禁止**（steering logging 規律）——既存の bind 経路のログ流儀（Changed=info・Unchanged/StateOnly=debug・解決不能=error）を排他置換でも維持する。

## Requirements

### Requirement 1: bindoption 3 値意味論の読み取り

**Objective:** シェル作者・保守者として、descript.txt の `bindoption` 宣言が ukadoc 正典の 3 値のまま取り込まれることを求める。これにより「明示 multiple」と「非宣言」の区別が下流で成立する。

#### Acceptance Criteria

1. When shell descript に `sakura.bindoption*.group,カテゴリ名,multiple`（kero 側も同型）が宣言されているとき、the descript 読み取り shall 当該カテゴリを multiple 宣言としてスコープ（本体／相方）別に収録し、非宣言カテゴリと区別可能にする。
2. When shell descript に `mustselect` 宣言があるとき、the descript 読み取り shall 従来どおり mustselect 宣言としてスコープ別に収録する（既存挙動不変）。
3. When オプション欄が `+` 区切りで複数のオプションを持つとき（正典「オプションは+区切りで複数可」・例 `mustselect+multiple`）、the descript 読み取り shall 各オプションを個別に解釈し、認識できるオプションをそれぞれ収録する。
4. If オプション欄に認識できない語が現れたとき、then the descript 読み取り shall 認識できるオプションのみ収録して未知語を捏造なく読み流す（寛容パース維持）。
5. If カテゴリ名が空、またはオプション欄が欠落しているとき、then the descript 読み取り shall 当該宣言を収録対象外とする（捏造しない・既存の寛容パース維持）。
6. When bindoption 宣言が 1 件も無い shell を読むとき、the descript 読み取り shall 全カテゴリ非宣言（既定）として成立させ、読み取り失敗にしない。
7. The descript 読み取り shall 同一入力に対し常に同一の収録結果を返す（決定論・既存の走査順維持）。

### Requirement 2: 非宣言カテゴリの排他置換（高々 1 個・解除可）

**Objective:** エンドユーザとして、`bindoption` 非宣言のカテゴリが正典の既定（高々 1 個・解除可・複数選択不可）で動くことを求める。これにより表情パーツが積み上がらず、表情固着が起きない。

#### Acceptance Criteria

1. When mustselect とも multiple とも宣言されていないカテゴリのパーツへ着衣指示（on）が適用されるとき、the bind 適用 shall 同一カテゴリで既に bind 済みの他パーツを自動的に外し、当該カテゴリの bind を高々 1 個に保つ（排他置換）。
2. When 非宣言カテゴリのパーツへ脱衣指示（off）が適用されるとき、the bind 適用 shall 当該パーツを外す（解除可＝正典既定・カテゴリ内ゼロ個の状態を許す）。
3. When ゴーストが off を送らず on のみで表情を切り替える正典作法で運用されるとき、the bind 集合 shall 同一カテゴリ内で単調に積み上がらず、以後の指示が既 bind ゆえ常に無変化となる飽和状態（是正指示の無言の握り潰し）に至らない。
4. When 排他置換により表示集合が変わったとき、the bind 適用 shall 従来の単一発行点から表示を発行し、実機ログで検索可能な適用ログ（scope・カテゴリ・パーツ・id・on）を保つ（既存流儀維持）。
5. When 既に当該パーツのみが bind 済みで表示集合が変わらないとき、the bind 適用 shall 表示を再発行しない（冪等・「変更時のみ発行」の既存流儀維持）。
6. While 対象 scope が非表示または未知のとき、the bind 適用 shall 既存の縮退挙動（状態のみ更新・発行なし・debug ログ）を排他置換でも維持する。
7. If (カテゴリ, パーツ) が名前解決できないとき、then the bind 適用 shall 既存どおり error ログの上読み飛ばし、状態を変えない（既存挙動不変）。

### Requirement 3: 明示宣言カテゴリの正典挙動（回帰の錨＋mustselect 解除不可適合）

**Objective:** 保守者として、`mustselect`・`multiple` 明示宣言の着衣側挙動とカテゴリ横断の挙動が本増分で変わらず、mustselect の脱衣素通しのみ正典「解除不可」へ適合することを求める（2026-08-11 要件ディスカッション裁定）。これにより是正が既存の正常系を壊さず、正典乖離を残さない。

#### Acceptance Criteria

1. When mustselect 宣言カテゴリのパーツへ着衣指示（on）が適用されるとき、the bind 適用 shall 従来どおり排他置換する（挙動不変）。
2. When mustselect 宣言カテゴリのパーツへ脱衣指示（off）が適用されるとき、the bind 適用 shall 当該指示で bind 集合を変更せず、無視した事実をログに残す（正典「解除不可」への適合・2026-08-11 裁定。ログ無し失敗経路の禁止に従い無言の握り潰しにしない。ログレベルの選定は既存流儀への整合として設計の領分）。
3. When multiple 宣言カテゴリのパーツへ着衣／脱衣指示が適用されるとき、the bind 適用 shall 従来の加算／除去を維持し、同一カテゴリ内の複数パーツ同時 bind を許す。
4. The 本増分 shall 異なるカテゴリ間の bind 共存（カテゴリを跨いだ複数カテゴリの同時 bind）を変更しない。
5. The 本増分 shall 適用後も `cargo test --workspace` を exit 0 の決定論的緑に保つ。

### Requirement 4: 決定論テストの檻

**Objective:** 開発者として、3 値判定の判断分岐が GPU・実窓・実機・実時間待機なしの決定論テストで全網羅されることを求める。これにより「檻が緑のまま実機で壊れていた」本件の再発を防ぐ。

#### Acceptance Criteria

1. The 本増分 shall brief の最小再現——非宣言カテゴリの同一カテゴリ 2 パーツへ着衣指示を 2 回流し、現在 bind 集合が後勝ち 1 個のみとなる——を決定論テストとして固定する（現状の欠陥挙動 {両方} が正典期待値 {後者のみ} へ反転したことの檻）。
2. The 決定論テスト shall descript 読み取りの 3 値分岐（mustselect／multiple／非宣言／`+` 区切り複数／未知語／不完全値／宣言ゼロ）を網羅する。
3. The 決定論テスト shall 排他か否かの判定について、mustselect 宣言・multiple 宣言・非宣言の各カテゴリ × 着衣／脱衣の組合せを網羅する（mustselect × 脱衣＝無視（解除不可）の檻を含む）。
4. The 決定論テスト shall GPU 実描画・実窓・実 DPI・実 SHIORI・sleep・実時間待機のいずれにも依存しない。
5. The 本増分 shall 旧 2 値前提を名前・文言で固定している既存テスト（例: 「非 mustselect＝加算」を謳う名前・コメント）を新正典の語彙へ更新し、検証内容が正典下でも有効なテスト（異カテゴリ加算等）は実体を保って維持する。

### Requirement 5: emo2 実機サインオフ

**Objective:** 開発者として、実機 emo2 で表情固着が解消したことをログと目視で確認できることを求める。本件は檻が緑のまま実機で壊れていた事例（同型の誤仮定の実機反証は 2 度目）であり、実機サインオフは必達である。

#### Acceptance Criteria

1. The 実機確認 shall 実 emo2 ゴースト（実 pasta.dll・辞書込みフルゴースト）を絶対パスで起動し、有界の自動終了とログ検索で決定論的に判定できる形で行う。
2. When 実機で目とまばたきの表情ペアが複数回切り替わったとき、the 実機ログ shall まばたきカテゴリの複数パーツ（1400 と 1402 等）が同一時間帯に共存して並行発火する痕跡を含まない。
3. When ジト目（目=ジトー＋まばたき=ジトー）へ切り替わった後に別の表情変更が届いたとき、the 表示 shall 次の表情へ正しく切り替わる（開発者の目視サインオフ・再起動不要）。
4. When ゴーストが表情切替を送り続けているとき、the 実機ログ shall まばたきカテゴリの適用が途中から恒久的に沈黙する飽和パターン（2026-08-11 直接観測の握り潰しパターン）を示さない。
5. The 実機確認の結果 shall 判定・実測値・実施条件を含む受け入れ記録として文書に残る。
6. If 実機確認で期待と異なる観測が得られたとき、then the 本 spec shall 不一致を受け入れ記録に残し、是正するまで完了としない。

### Requirement 6: 正典訂正の文書整合と先送りの登記

**Objective:** 保守者として、覆された誤仮定の痕跡がコード内文書から一掃され、覆しの経緯と残る正典乖離が追跡可能であることを求める。これにより次の読み手が旧前提を信じて誤った改変を行わない。

#### Acceptance Criteria

1. The 本増分 shall コード内の「非 mustselect＝加算」前提の doc コメント・設計参照（mayuna-compose R4.5／D11 を根拠に引く記述を含む）を 3 値正典の記述へ更新し、旧前提の主張を残さない。
2. The 本 spec の設計文書 shall `completed/areka-P0-mayuna-compose` の R4.5／D11 を覆す旨とその根拠（正典・実機証拠）を明記する（completed 文書自体は改変しない）。
3. When 本 spec が完了するとき、the roadmap shall mayuna-compose R4.5／D11 の覆しと本 spec による是正を追記として追跡する。
4. The 本 spec の設計文書 shall mustselect「解除不可」適合を本 spec で拾う 2026-08-11 裁定（要件ディスカッション議題 1）を記録する（旧 R6.4 の先送り登記条項は本裁定により解消）。

### Requirement 7: bind から外れたパーツの残留コマの掃除（2026-08-11 実機サインオフ後にスコープ追加）

**Objective:** エンドユーザとして、着せ替えパーツが bind から外れたら**画面からも消える**ことを求める。これにより、排他置換で bind 集合が正しくなった後も残る表情固着が解消する。

**追加の経緯:** 2026-08-11 の実機サインオフで J1・J2 が PASS へ反転しても目視の固着が再現し、根因が bind 集合の下流（残留コマの掃除欠落）にあることが `file:line` で確定した。開発者裁定によりスコープを拡大する（`real-machine-signoff.md` §4／§6）。正典（ukadoc）は `bindgroup*.default` を「1 で表示、**0 で非表示**」と規定しており、bind から外れたパーツのコマが描画され続けるのは正典違反である。

#### Acceptance Criteria

1. When 着せ替え ID が bind 集合から外れるとき、the 表示 shall 当該 ID の保持コマ（`-1` 終端を持たないアニメが末尾到達後に保つ最終コマを含む）を以後の合成対象から取り除く。
2. When 排他置換により同一カテゴリの他パーツが自動的に外れるとき、the 表示 shall 外れたパーツの残留コマを同じ発行で取り除き、新しいパーツのみを表示する。
3. When 再生途中のアニメの ID が bind 集合から外れるとき、the 再生 shall 当該 ID の再生を止め、その保持コマも取り除く（次の評価で復活しない）。
4. The 本増分 shall bind 種でないアニメ（`interval` の抽選発火など bind に属さないもの）の再生・保持コマに影響を与えない。
5. When 残留コマを取り除いたとき、the 実装 shall 取り除いた事実をログに残す（無言の状態変更を作らない・ログ無し失敗経路の禁止）。
6. The 決定論テスト shall 上記を実機・GPU 実描画・実窓・実時間待機なしで固定する（外れた ID が発行される表示指令に含まれないこと・再生中に外れた場合に次の評価で復活しないこと・bind 非所属の ID のコマが合成計画へ入らないこと）。
7. When 実機で表情がジト目へ切り替わった後に別の表情変更が届くとき、the 表示 shall 次の表情へ正しく切り替わる（要件 5.3 の J3 目視の再実施で確認する）。

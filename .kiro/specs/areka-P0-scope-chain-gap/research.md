# Gap Analysis: areka-P0-scope-chain-gap

> 作成: 2026-08-11（kiro-validate-gap）。requirements.md 確定版と現行コードベース（file-slimming 後・PR#103 マージ済の main 由来 worktree）の実測突合。

## 分析サマリ

- **是正対象は 1 関数内の 1 分岐**: 連鎖式は `crates/areka/src/placement/resolver.rs` の `resolve_placement` P2（:155-158）に単一実装で存在し、`Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w)` が brief の欠陥式そのもの。H1（隣接）確定なら是正は実質 1 行（`prev_w` → 自スコープの `w`）＋波及追随で、既存アーキテクチャへの構造変更は不要。
- **波及面は狭く、かつ全数特定済み**: 連鎖値を絶対値で固定するテストは resolver 檻 3 本＋emo2 実寸フィクスチャ 1 本のみ。spawn/follow/persist 系のテストは resolver 出力を記号的に参照する（`p.char_pos` 透過）ため自動追随し、バルーン offset の SSP 突合テストは char 位置に依存しない（R4.2 の成立を静的に確認済み）。
- **最大のギャップは SSP 実測オラクル（Missing）**: SSP のキャラ窓生矩形は未保存（kero-balloon 6.1 はバルーン offset のみ記録）で、実測ツールもリポジトリに存在しない。さらに**実測タイミングの罠を新発見**——emo2 の boot スクリプト自体が scope1 へ `\![move]` を発行する（実機ログ証跡: from_x=2754 → to_x=3005）ため、SSP 定常状態の観測値は既定配置ではない。既定連鎖規則を測るには move 前の初期矩形を捕捉する必要がある。
- **記録経路は先例が完備**: COMPAT_ARCHITECTURE §8 の kero-balloon R3.8 行（doc/COMPAT_ARCHITECTURE.md:147）が「否定した先行 AC の名指し・アーカイブ非改変・オラクルと測定値の記載」の体裁をそのまま提供する。実機受け入れも `AREKA_APP_SMOKE_EXIT_MS` 有界終了＋`merge_scope restore` ログ（persist.rs:402 が scope 別 `default_char_x`/`char_w` を出力）grep で決定論判定できる部品が揃っている。
- **推奨方向（決定ではない）**: Option C（実測先行のハイブリッド）。Step 0 の実測結果で H1 なら最小是正（Option A 相当）、H3 なら P2 構造の置換（設計分岐）とする工程設計が、R1「測ってから式を書く」と R2.7 のヘッジ（連鎖構造を保つ「場合」）に整合する。

## 1. 現状調査（既存資産・file:line 実測済み）

### 1.1 連鎖式の所在（是正対象）

`crates/areka/src/placement/resolver.rs`
- `resolve_placement`（:124-212）: 純粋関数・物理 px 一貫・wintf 非依存・panic しない契約。
- **P2 実装（:155-158）**:
  ```rust
  let base_x = match prev {
      None => work_area.right.saturating_sub(w),
      Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w),   // ← 欠陥式（前スコープの幅を引く）
  };
  ```
  `prev` は「(クランプ後 char_x, char 幅)」のタプル（:131-132）で、P4 クランプ後に `prev = Some((x, w))`（:178）で更新される。**H1 是正なら `prev_x.saturating_sub(w)`（自スコープの幅）へ変更＝タプル第 2 要素 `prev_w` は不要になり `prev: Option<i32>` へ縮小できる**。
- doc コメントの式引用: モジュール doc（:99-102）・インライン（:151-152）が `base_x(n≥1) = char_x(n−1) − w(n−1)（2.9）` を明記。是正時に「2.9」参照ごと書き換えが必要（本仕様の要件番号へ）。
- P5（:180-188）は `windowposition-limit` と共有ハンク（roadmap 干渉台帳: scg 先・直列必達）。**P5 は本仕様では触らない**——ただし同ファイル同関数のため wpl 側が本仕様の檻へ rebase する前提。

### 1.2 連鎖値に依存するテスト（波及の全数）

**絶対値・式で連鎖を固定している檻（要改修）** — `crates/areka/src/placement/resolver_resolve_tests.rs`:

| テスト | 行 | 現行期待値 | H1 是正後 | 備考 |
|---|---|---|---|---|
| `t_r2_scope_chain_defaultx_zero_stays_adjacent` | :130 | `out[1].x = x0 − w0`・`out[2].x = (x0−w0) − w1` | `x0 − w1`・`x0 − w1 − w2` | **名前が嘘をつく檻の本丸**（不等幅 400/320/200 入力で「密着（2.9）」を主張しながら実幾何は 80px 隙間）。真実の名前＋gap=0 明示 assert へ |
| `t_r2_chain_defaultx_offsets_leftward_from_base` | :175 | `out[1].x = x0 − w0 − dx1` | `x0 − w1 − dx1` | defaultx 合成の檻。式のみ追随 |
| `t_r4_free_position_feeds_scope_chain` | :524 | `out[1].x = x0 − w0` | `x0 − w1` | free 実位置が連鎖基準になる檻（T-R4 補）。brief の「:847 T-R4 補」に相当 |
| `t_r6_chain_uses_clamped_previous_position` | :363 | 両者 `wa.left`（クランプ） | **期待値不変の見込み**（`x0−w1` も左外→クランプ） | assert は不変だが :377 のコメント式は追随 |

**期待値が構造的に不変のテスト（無改修で緑が続くはず・回帰監視役）**:
- `t_r5_seam_output_identical_to_bottom`（:236）・`t_r4_free_both_unspecified_equals_bottom`（:461）・`postconditions_order_length_and_offset_identity`（:836）——比較・恒等式ベースで連鎖の絶対値に非依存。
- `spawn_follow_pipeline_tests.rs`・`spawn_assembly_tests.rs`・persist/follow 系——`resolve_placement` 出力を `p.char_pos` として記号参照するのみ（不等幅 434/278 を既に入力しており、式変更に自動追随することを確認済み）。

**emo2 実寸フィクスチャ（要期待値更新）** — `crates/areka/src/placement/placement_prepare_tests.rs`:
- `prepare_emo2_returns_two_scope_placements`（:57）: `s1.char_pos = (1052, 640)`（:80）＝ `1486 − 434`（scope0 の幅）。H1 是正後は `1486 − 336 = 1150`。連動して `s1.balloon_pos`（:84・右置き＝char 依存）も `1198 → 1296` へ。`s1.balloon_offset (146,−75)`（:95）は**不変**（offset は char 相対）。doc コメント（:38-51）の導出式も追随。
- ほか同ファイルの `prepare_emo2_scales_window_sizes_by_k0`（:370）等は寸法系で位置非依存。

**SSP 突合テスト（無改変合格が要件 R4.2）** — `crates/areka/src/placement/placement_windowposition_tests.rs`:
- `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`（:87）: assert 対象は `char_size`・`balloon_size`・`balloon_offset`・恒等式のみで **char 絶対位置を一切参照しない**。R4.2「無改変のまま合格し続ける」は静的に成立見込み。崩れたら欠陥シグナル（R4.3）。

### 1.3 doc・宣言の式引用（是正時に追随が要る記述）

- `resolver.rs` :99-102（モジュール doc P2）・:151-152（インライン）。
- `resolver_resolve_tests.rs` :127-128・:154・:164・:193・:377・:545（メッセージ内の式引用）。
- `placement_prepare_tests.rs` :38-39（導出コメント）。
- `mod.rs` :368・`windowposition.rs` :63-64 は P5（バルーン基本位置）の式引用で **P2 非依存＝追随不要**（ただし :63-64 の表は char_x の実値を含まないことを確認済み）。
- 記憶則「doc の主張は file:line で裏取り・値の意味を変えたら全下流の宣言を洗う」の適用対象一覧として design へ持ち越し。

### 1.4 SSP 実測の既存資産と欠落

- **手順の先例**: kero-balloon 6.1（completed spec tasks.md :195-210）——同一ゴースト emo2＋実 DPI 120 で SSP に表示させ「DPI aware で窓矩形を実測」。証跡は `real-run-signoff-2026-07-31.log` として spec ディレクトリへ保存する体裁。
- **欠落 1（記録）**: 当時保存されたのはバルーン offset（逆算値）と 6.1 表の一部窓矩形のみで、**キャラ窓の生矩形ペア（両 scope 同時）と work area 情報は未保存**。R1.2 の再採取必須の根拠を実地確認した。
- **欠落 2（ツール）**: 実測ツール（読み取り専用ポーリング）はリポジトリに存在しない（`GetWindowRect`/`FindWindow` の検索で pilot の自窓検証と wintf 内部のみヒット・SSP 向け外部計測スクリプトは無し）。当時はセッション限りのアドホック手段だったと推定。R1.2「証跡ログとして保存」を満たすには**再現可能な計測手段の形を決める必要**（→設計判断 #1）。
- **新発見（タイミングの罠・重要）**: emo2 の boot スクリプトは scope1 へ `\![move]` を発行する。実機証跡 = `real-run-signoff-2026-07-31.log:68`
  ```
  apply_move_directive: move 適用完了（scope→物理px移動） scope=1 base_scope=0 from_x=2754 from_y=1600 to_x=3005 to_y=1600
  ```
  （boot 約 1 秒後・scope1 が +251px 右へ＝scope0 側へ移動）。SSP でも同一スクリプトが走るため、**SSP の定常観測矩形は「既定連鎖規則＋ghost 自身の move」の合成値**になる。既定規則を単離するには move 適用前の初期矩形を捕捉する（高頻度ポーリングで時系列を取る／二時点を区別して記録する）必要がある。R1.3 の「単発観測から言える不変量の範囲を明記」に直結する運用上の要点。
- **仮説判別の条件が良好であることを確認**: emo2 shell descript（`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt:12-13`）は `sakura.defaultx,0`・`kero.defaultx,0` を**明示宣言**。H3（連鎖せず独立解釈）なら kero も右端密着＝scope0 と重なるため、H1/H2（並置）とは決定的に区別できる。H1 と H2 は gap の定数差で判別。**現行式（幅差 123px の隙間）を支持する仮説は 3 つのいずれでもない**（brief の見立てと一致）。

### 1.5 実機受け入れ（R6）の既存部品

- 有界自動終了: `AREKA_APP_SMOKE_EXIT_MS`（実機ログ :33 で稼働実証済み）＋ RUST_LOG grep の決定論判定（記憶則 areka-real-machine-signoff-bounded-auto-exit）。
- **ログから gap を算出できる観測点が既にある**: `persist.rs:402` の `merge_scope restore` ログが scope 別に `default_char_x`/`char_x`/`char_w` を出力する。実機ログ実例: scope0 `char_x=3297`、scope1 `char_x=2754 char_w=420` → gap = 3297 − (2754+420) = **123**（欠陥の実機値と厳密一致）。是正後はこの同一 grep で gap=0（または SSP 確定値）を判定できる——**追加ログ実装なしで R6.2 の決定論判定が組める**。
- SSP との突合（R6.4）は Step 0 で作る計測手段を areka 側の窓にも向ければ両者同条件で取れる（kero-balloon 6.1 の突合表と同型）。
- 実 DPI≠96 必達（R6.1/R6.3）は開発機の DPI 120 実績があり手順確立済み。プロファイル削除＝初回起動の前提も先例踏襲（real-run-signoff ログ冒頭 :4 に明記の手順）。

### 1.6 記録経路（R5）の既存資産

- `doc/COMPAT_ARCHITECTURE.md` §8（:122-153）が沈黙ルール対応表の正本。**先例行 = kero-balloon R3.8**（:147）が R5.5 の求める体裁の完全な雛形: 「実機確定（日付）＝参照実装 SSP を受理オラクルとした」・実測値と条件・「本裁定が否定した先行 AC は position-persist の R2.2…と R8.5」の名指し・「アーカイブ済み spec は非改変とし、上書きの事実を本表と現行 spec に記録する」。
- 上書き対象の先行 AC 実在確認: `completed/areka-P0-window-placement/requirements.md:41`（R2.9「scope1（相方）を scope0 のサーフェス画像幅ぶん左へずらした位置へ置く（SSP de-facto）」）。
- 「SSP de-facto 札が無検証」の証跡実在確認: 同 spec `research.md:78`（「**Unknown**: …scope 相対配置（SSP de-facto）」）→ `:122`（設計判断 #7「**要件討議#2 で確定**」＝討議で決めた値に札が付いた経緯が文書上で追える）。R5.3 の記載材料はすべて既存文書から引用可能。

### 1.7 干渉・依存（roadmap 台帳と突合）

- `scg⇄wpl` は resolver.rs 同一関数内 30 行差の**同ハンク級・直列必達（scg 先）**（roadmap.md:95）。本仕様が W6 で先行し、wpl（W6.5）が本仕様の檻へ rebase する。→ 本仕様のテスト設計は wpl が引き継ぎやすい形（連鎖規則の檻と P5 の檻を分離したまま）が望ましい。
- W6 並走 4 本（vis ∥ bind ∥ zorder ∥ scg）のうち placement/resolver を触るのは scg のみ（vis は follow 系・因果のみ）。コード面の並走衝突は台帳上なし。
- 下流: `emo2-conformance-e2e`（二体間隔の目視）・`balloon-visibility`（char 位置従属）は本仕様の是正値を前提に後続。

## 2. 要件→資産マップ（ギャップタグ）

| 要件 | 必要能力 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 SSP 実測オラクル | DPI aware 読み取り専用ポーリング・両 scope 窓矩形の生値採取・証跡ログ保存・3 仮説判別 | 手順先例（kero-balloon 6.1）・emo2 fixture・プロファイル削除手順・実 DPI 120 環境 | **Missing**: 計測ツール実体（リポジトリに無し）／**Missing**: キャラ窓生矩形の記録／**Unknown**: SSP の実規則（H1/H2/H3/その他）／**Constraint**: ghost 自身の `\![move]` が定常観測を汚染＝move 前の捕捉が必要 |
| R2 連鎖式是正 | P2 の基準式変更・DPI 不変・等幅特殊扱いなし・丸め権威維持・クランプ連鎖維持 | `resolve_placement` 純関数（:155-158 の 1 分岐）・`ScaleRatio` 権威・P4 連鎖基準の現行実装 | **Missing**: 是正式（実測待ち）。H1 なら 1 行＋`prev` タプル縮小。**H3 の場合のみ**連鎖構造の置換（設計影響大・R2.7 のヘッジ対象） |
| R3 テスト真実性 | 名前・メッセージ・内容の一致・不等幅入力・gap 明示 assert・不等幅×DPI 行列 | 不等幅入力は既に使用（400/320/200）・`DPIS=[96,120,144,192]` 行列・影響テスト全数特定済み（§1.2） | **Missing**: gap=0（確定規則）の明示 assert・真実の名前への改名・連鎖依存 3 本の期待値追随 |
| R4 フィクスチャ追随・不変量監視 | emo2 実寸期待値の更新・SSP バルーン offset 檻の無改変合格 | `prepare_emo2_returns_two_scope_placements`（更新対象 2 値特定済み）・`prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`（char 位置非参照を確認済み） | **Missing**: 期待値更新のみ（s1.char_pos / s1.balloon_pos）。オラクル由来の新規檻（543/420 実寸系）は設計判断 |
| R5 正典記録 | §8 エントリ追加・先行 AC 名指し・無検証札の事実記載・アーカイブ非改変 | §8 表・R3.8 先例行（体裁の雛形）・引用元 file:line 全確認済み（§1.6） | **Missing**: エントリ執筆のみ（構造ギャップなし） |
| R6 実機受け入れ | 実 DPI≠96・有界終了・ログ突合・SSP との間隔一致 | `AREKA_APP_SMOKE_EXIT_MS`・`merge_scope restore` ログ（gap 算出可能・追加実装不要）・DPI 120 実績 | **Missing**: 判定手順書（grep 式と合否条件の明文化）／**Constraint**: SSP 突合には Step 0 と同じ計測手段を areka 窓へも適用 |

## 3. 実装アプローチ選択肢

### Option A: 最小是正（H1 前提の直接修正）

`resolver.rs` P2 の 1 分岐を `prev_x.saturating_sub(w)`（自スコープ幅）へ変更し、`prev: Option<(i32, i32)>` を `Option<i32>` へ縮小。§1.2 の檻 3 本＋フィクスチャ 1 本＋doc 引用を追随。COMPAT §8 追記。

- ✅ 変更面が最小・純関数内で完結・既存の検証戦略（DPIS 行列・実寸フィクスチャ・実機 grep）にそのまま乗る。
- ✅ wpl への引き継ぎ面（P5・同関数）を乱さない。
- ❌ **実測前に式を確定させる工程順になり R1 の「測ってから式を書く」に反する**。H2/H3 だった場合に手戻り。
- ❌ 単独では SSP 実測・記録の工程が別立てになる。

### Option B: 連鎖規則の戦略切り出し（規則注入可能な構造化）

P2 の基準式を enum（例 `ChainRule::AdjacentOwnWidth | FixedMargin(i32) | Independent`）等へ切り出し、実測結果で規則値を選ぶ。

- ✅ H1/H2/H3 のどれに転んでも構造変更なしで追随でき、実測と実装を並行できる。
- ✅ 規則が第一級の値になり、COMPAT 記録と実装の対応が明示的。
- ❌ **確定後は 1 規則しか使わない抽象が残る**（YAGNI）。「単純な基準配置のみ」（R2.9 継承）の精神と摩擦。
- ❌ resolver の可読性・「純粋関数 1 本」の現行の簡潔さを損なう。テスト面も規則パラメタ化で複雑化。

### Option C: 実測先行ハイブリッド（工程で分岐・構造は最小のまま）【推奨の方向性・決定ではない】

**Step 0（実測）を実装より先に完了させ、確定規則に応じて実装形を選ぶ**工程設計。H1/H2（並置系）なら Option A の最小修正（H2 は定数項が 1 つ増えるだけ）、H3（独立配置）の場合のみ P2 連鎖構造の置換を設計フェーズで別途設計する。

- ✅ R1「測ってから式を書く」と R2.1（確定規則に従う）・R2.7（連鎖構造を保つ「場合」のヘッジ）に工程レベルで整合。
- ✅ 見込み（H1 最有力・emo2 は defaultx=0 明示宣言で判別条件良好）どおりなら最終形は Option A と同一＝抽象の残骸なし。
- ✅ 実測ツールと証跡の形（設計判断 #1）を design の一部として先に確定でき、R6 の SSP 突合とツールを共用できる。
- ❌ 実測が design のクリティカルパスになる（実測完了まで式の最終形を書けない）。
- ❌ H3 だった場合は design を二段階にする管理コストが生じる。

## 4. 工数・リスク

- **工数: M（3–5 日）**。コード変更自体は S 相当（1 分岐＋檻 4 本＋doc 追随＋§8 追記）だが、SSP 実測（環境準備・move 前捕捉のポーリング設計・証跡整形）と実機受け入れ（DPI≠96 実行・SSP 突合）が主工数。
- **リスク: Medium**。
  - 技術リスクは Low（純関数 1 分岐・波及全数特定済み・比較系テストが回帰網として機能）。
  - Medium 要因は実測ロジスティクス: (a) ghost 自身の `\![move]` による観測汚染（§1.4）を回避できないと規則を誤読する、(b) SSP の丸め（kero-balloon で切り捨て寄りを確認済み）により非整数 k で gap が ±1px 揺れ、H1（gap=0）と H2（微小マージン）の判別を乱しうる——DPI 96（k=1）での対照採取が判別を締める、(c) H3 だった場合の設計影響（連鎖構造の置換・R2.7/テスト設計の再構成）。

## 5. Research Needed（設計フェーズへ持ち越す調査）

1. **SSP 実測の実施と規則確定（R1・最優先）**: 実 DPI 120＋対照 DPI 96 での両 scope 窓矩形の時系列採取。move 前の初期配置を単離。H1/H2/H3 判別と不変量範囲の記録。
2. **SSP の gap 丸め挙動**: k 非整数時に gap が 0 でなく ±1px になるか（R2.3 の許容差 1px の根拠を実測側からも固める）。
3. **SSP の kero Y 配置**（付随観測・スコープ外だが記録価値）: 実測時に Y も生値が取れるため、bottom 揃えの検証データとして併記するか。

## 6. 設計判断項目（要件ディスカッションの種）

1. **SSP 実測ツールの形態と置き場**: (a) アドホック PowerShell（DPI aware プロセス＋`GetWindowRect` ポーリング・証跡ログのみ spec dir へ保存）、(b) リポジトリにコミットする計測スクリプト/Rust example（再現可能性最大・R6 の areka 窓突合と共用）、のどちらか。先例（kero-balloon）は (a) 相当でツールが残らず、本仕様の R1.2 が再採取を強いられた当の原因。証跡ログの置き場は `real-run-signoff-2026-07-31.log` 先例に倣い spec ディレクトリが自然。
2. **実測タイミングの単離方法**: emo2 boot スクリプトの `\![move]`（scope1・実機ログ :68 で +251px）より前の初期矩形をどう捕捉するか——高頻度ポーリング（起動直後から窓出現を待ち受け）か、move を含む二時点記録で既定配置時点を明示区別するか。R1.3 の「単発観測の不変量範囲」の書き方に直結。
3. **H3 だった場合の設計分岐の扱い**: Option C 採用時、H3（独立配置）判明時に design を二段階にするか、design 内で条件分岐した二案を先に書くか。R2.7 のヘッジが構造変更を許すが、その場合の free フォールバック（P3 の X 未指定→連鎖値）・クランプ連鎖（P4）の再定義範囲。
4. **連鎖檻の再設計粒度（R3.5）**: 既存 `DPIS=[96,120,144,192]`×不等幅の行列で足りるか、gap 明示 assert を独立テスト（例 `t_r2_unequal_widths_leave_no_gap`）として新設するか、既存 3 本の改修に留めるか。wpl が rebase する前提での檻の命名・分割方針。
5. **実機受け入れの判定チャネル（R6.2/R6.4)**: `merge_scope restore` ログ grep（追加実装ゼロ・gap 算出式を手順書化）を主判定とし、外部計測（設計判断 #1 のツール）を SSP 突合用の従とする二本立てで足りるか。
6. **doc 内「2.9」参照の書き換え方針**: resolver.rs・檻メッセージが引用する旧要件番号（2.9/DD3）を本仕様の要件番号へ振り直すか、「window-placement R2.9（本仕様で上書き・COMPAT §8 参照）」の形で履歴を残すか。記憶則（値の意味を変えたら全下流の宣言を洗う）の適用範囲確定。
7. **§8 エントリの粒度**: R2.9 上書き 1 行で足りるか、「SSP de-facto 札が無検証だった」事実（R5.3）を同一行に含めるか別行にするか（R3.8 先例は 1 行に全要素を畳んでいる——同型を推奨）。

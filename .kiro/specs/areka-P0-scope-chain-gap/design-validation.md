# Design Validation: areka-P0-scope-chain-gap

> 実施: 2026-08-11（kiro-validate-design・非対話モード）
> 対象: `design.md`（finalized）／照合: `requirements.md`・`research.md`・`ssp-oracle-notes.md`・steering・現行コードベース（file:line 実測）

## Review Summary

設計は「1 分岐の式是正＋`prev` 型縮小＋檻の真実性是正＋正典記録＋実機受け入れ」という最小是正（Option A）に収束しており、要件 6 本すべてが Traceability 表で実現要素（C0〜C5）へ写像されている。設計が主張する file:line・期待値・不変量を現行 tree に対して全数照合した結果、実質的な誤りは検出されなかった（詳細は下記「検証結果」）。実装への手戻りリスクを持つ残件は、従判定ツールの再利用に小改修が要る 1 点のみである。

## 検証結果（doc 主張の file:line 裏取り・記憶則適用）

| 設計の主張 | 実測結果 |
|---|---|
| P2 欠陥式 = `resolver.rs:155-158`（`Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w)`） | **一致** ✓（:157 が欠陥式そのもの） |
| `prev: Option<(i32, i32)>` = :131-132・更新 = :178 | **一致** ✓ |
| モジュール doc 式引用 :98-102・インライン :151-152 | **一致** ✓（`base_x(n≥1) = char_x(n−1) − w(n−1)（2.9）` を実記載） |
| P5 ハンク :180-188 非接触（wpl 領分） | **一致** ✓（roadmap.md:95 の干渉台帳と同一範囲表記。P5 コード実体は :180-190 だが台帳規約に整合） |
| `t_r2_scope_chain_defaultx_zero_stays_adjacent` :130・不等幅 400/320/200・現行期待 `x0−w0`／`(x0−w0)−w1` | **一致** ✓（是正後期待 `x0−w1`／`x0−w1−w2` の算術も正） |
| `t_r2_chain_defaultx_offsets_leftward_from_base` :175・`t_r4_free_position_feeds_scope_chain` :524・`t_r6_chain_uses_clamped_previous_position` :363（assert 不変・:377 コメントのみ追随） | **一致** ✓（t_r6 は是正後も `x0−w1` が左外→クランプで assert 不変が成立） |
| DD3 否定 assert（:156-160）が是正後も有効 | **成立** ✓（`x0−w1 ≠ wa.right−w1`、w0≠0 ゆえ） |
| DPIS = [96,120,144,192]（`resolver_test_support.rs:4`） | **一致** ✓（要件 2.4/3.5 の行列と同一） |
| `prepare_emo2_returns_two_scope_placements` :57・`s1.char_pos (1052,640)` :80・`s1.balloon_pos (1198,565)` :84・`s1.balloon_offset (146,−75)` :95・導出コメント :38-51 | **一致** ✓。是正後値の算術検証: `1486−336=1150` ✓・右置き基準 `1150+336=1486`＋wp `−190`＝`1296` ✓・offset `1296−1150=146` 不変 ✓ |
| `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120` :87 が char 絶対位置を assert しない | **一致** ✓（assert 対象は char_size・balloon_size・balloon_offset・相対恒等式のみ。R4.2 の無改変合格檻として妥当） |
| `persist.rs:397-409` の `merge_scope restore` ログ・`default_char_x` = resolver 出力（:402） | **一致** ✓（`default_char_x = placement.char_pos.x`。move 演出・保存位置復元のいずれの汚染も受けない判定チャネルの選定は正しい。ログは無条件発火＝プロファイル削除時も出る） |
| COMPAT §8 の体裁先例（kero-balloon R3.8 行・列構成 項目｜裁量｜根拠｜出典 spec） | **一致** ✓（§8 表実在・R3.8 行が「否定した先行 AC の名指し・アーカイブ非改変・オラクルと測定値」の同型を提供） |
| 上書き対象 R2.9 の実在（`completed/areka-P0-window-placement/requirements.md`） | **一致** ✓（「scope1（相方）を scope0 のサーフェス画像幅ぶん左へずらした位置へ置く（SSP de-facto…）」を実記載） |
| DPI 120 檻の寸法錨 543×859／420×500 | **算術一致** ✓（434×5/4=542.5→543・687×5/4=858.75→859・round half away from zero・336×5/4=420・400×5/4=500） |
| `tools/measure-ssp-rects.ps1` 実在・証跡ログ 8 本実在 | **一致** ✓（spec ディレクトリで確認） |
| scg⇄wpl 直列必達（scg 先）・W6.5 rebase | **一致** ✓（`.kiro/steering/roadmap.md:68/:70/:95`） |

## Critical Issues（最大 3）

🔴 **Critical Issue 1**: 従判定ツールの再利用は無改修では不成立（プロセス名ハードコード）
**Concern**: 設計 C5 は「`tools/measure-ssp-rects.ps1` を areka の窓へ向けて」従判定（外部矩形実測）を行うとするが、同ツールの `Get-SspWindows` は `Get-Process -Name ssp`（:42）をハードコードしており、areka プロセスの窓は列挙されない（無改修だと "(no visible ssp windows)" を返し証跡が空になる）。
**Impact**: 従判定は補強証跡に限定され合否は主判定（ログ grep）で確定するため GO を覆さないが、R6.4 の字義「是正後の二体の**窓矩形を実測**し」を満たす唯一のチャネルが黙って空振りする恐れがある。
**Suggestion**: 実装フェーズ（tasks）で `-ProcessName` パラメタ化（既定 `ssp`）の小改修を C5 の前提タスクに含める。読み取り専用の性質・既存証跡ログの再現性は変えない。
**Traceability**: R6.4（窓矩形の実測）
**Evidence**: design.md「C5: 実機受け入れ」従判定の節／`tools/measure-ssp-rects.ps1:42`

（他に GO を左右する問題は検出されなかった。上記 1 件のみ。）

## Design Strengths

1. **判定チャネルの汚染回避が構造的**: 実機主判定に `char_x`（復元後値）でなく `default_char_x`（`persist.rs:402`・resolver 出力そのもの）を採る決定は、ゴースト演出 `\![move]` と保存位置復元という 2 つの汚染源を同時に遮断し、追加実装ゼロで R6.2 の決定論判定を成立させる。是正前実機ログで同式が gap=123 を返す事実（before/after の対）まで確認済みで、判定式の妥当性が実データで裏付いている。
2. **退行防止が型と檻の二重**: `prev` の `Option<(i32,i32)>`→`Option<i32>` 縮小で欠陥式（前スコープ幅参照）の再導入をコンパイルエラー化し、さらに新設檻 `t_r2_unequal_widths_leave_no_gap` に欠陥式の否定 assert（不等幅入力でのみ判別可能）を置く。「名前が嘘をつくテスト」の根本原因（等幅入力では観測不能）への対処が要件 3.2 と正確に噛み合っている。

## Final Assessment

**Decision: GO**

**Rationale**: 是正の本体は SSP 実測（H1 確定・`ssp-oracle-notes.md` 正本）に基づく 1 分岐の式変更で、波及面（テスト 4 本＋フィクスチャ 1 本＋doc 引用）は設計時点の tree に対する file:line 照合で全数一致を確認した。要件 6 本の Traceability に欠落はなく、wpl との同一関数直列関係（P5 非接触・檻分離）も roadmap 台帳と整合する。Issue 1 は従判定（補強証跡）に限定された小改修であり、tasks で吸収可能なため GO を妨げない。

**Next Steps**:
1. `/kiro-spec-tasks areka-P0-scope-chain-gap` でタスク生成。その際 Issue 1（measure-ssp-rects.ps1 の `-ProcessName` パラメタ化）を C5 系タスクの前提に組み込む。
2. 実装は C1（式是正＋型縮小）→ C2/C3（檻）→ C4（§8）→ C5（実機）の依存順（design の Component 依存と一致）。

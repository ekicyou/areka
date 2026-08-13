# 技術設計: areka-P0-scope-chain-gap

## Overview

**Purpose**: 本設計は、areka の二体既定配置（scope0=本体・scope1=相方）でキャラ幅が異なると幅差ぶんの隙間（実機 DPI 120 で 123px）が生じる挙動バグを、SSP 実測で確定した規則 **H1（完全隣接・`scope_n.L = scope_{n-1}.L − scope_n 自身の幅`・gap 0・DPI 不変）** に従って是正する。実測オラクル（Requirement 1）は**要件フェーズで完了済み**であり、正本は `ssp-oracle-notes.md`（証跡ログ・計測ツール込み）。本設計はその確定規則を前提とし、再実測・再裁定は行わない。

**Users**: ゴースト利用者は二体が SSP と同じ規則（隣接）で並ぶ画面を得る。保守開発者は名前・メッセージ・内容が一致するテスト群と、先行要件上書きの正典記録（COMPAT §8）を得る。

**Impact**: 変更の本体は `crates/areka/src/placement/resolver.rs` の `resolve_placement` P2 分岐 1 箇所（前スコープ幅の減算 → **自スコープ幅**の減算）。波及は研究フェーズで全数特定済み——連鎖値を絶対値で固定するテスト 3 本＋emo2 実寸フィクスチャ 1 本＋doc/コメントの式引用のみ。構造変更・新規抽象・新規ファイルは導入しない。

### Goals

- P2 連鎖式を確定規則 H1 へ是正し、幅差由来の隙間（`w(n−1) − w(n)` に比例する項）を解消する（2.1, 2.2）。
- 「隣接」を名乗るテストが実際に gap 0 を主張する形へ是正し、不等幅×複数 DPI の行列で確定規則を決定論的に全網羅する（3.1–3.5）。
- emo2 実寸フィクスチャの位置期待値をオラクル由来値へ追随させ、バルーン offset の SSP 突合テストを不変量監視役として無改変で維持する（4.1–4.3）。
- 先行 spec `completed/areka-P0-window-placement` R2.9 の上書きを `doc/COMPAT_ARCHITECTURE.md` §8 へ先例（kero-balloon R3.8 行）と同じ体裁で記録する（5.1–5.5）。
- 本番ゴースト＋実 DPI≠96 の実機で、ログ突合による決定論的な受け入れ判定を行う（6.1–6.4）。

### Non-Goals

- `defaultx` の意味論変更（右端からの左方向オフセット・0＝基準密着は不変・2.8）。
- `windowposition.limit`／P5 バルーン基本位置（別 spec `windowposition-limit` の領分。同関数内だが本仕様は P5 のハンクに触れない）。
- 重なり回避等の複雑な配置ロジックの導入（2.9）。
- バルーン offset の基準（キャラ窓左上相対・kero-balloon R3.8 確定・不変）。
- 位置の追従・保存・復元の実装変更（`spawn.rs`・`follow.rs`・`persist.rs` は無改変。persist の restore ログは**観測にのみ**用いる）。
- SSP コールドブート定常値の模倣。SSP 自己不整合 2 件（起動時サーフェス寸レース・ゴースト演出 `\![move]` の物理 px 無スケール適用）は互換対象外として §8 に記録するのみ（要件討議 #2 裁定・2.3／6.4）。

## Boundary Commitments

### This Spec Owns

- `resolve_placement`（`crates/areka/src/placement/resolver.rs`）の **P2 連鎖式（X 基準）** と、連鎖状態 `prev` の内部表現。
- P2 連鎖規則を検定するテスト群（`resolver_resolve_tests.rs` の T-R2 系・T-R4 補・T-R6 補コメント）の**意味・名称・メッセージ**。
- emo2 実寸フィクスチャテスト（`placement_prepare_tests.rs`）の**位置期待値**。
- `doc/COMPAT_ARCHITECTURE.md` §8 の **window-placement R2.9 上書きエントリ**（新規 1 行）。
- 実機受け入れの判定手順と証跡ログ（spec ディレクトリ）。

### Out of Boundary

- P1（Y・bottom）・P3（free）・P4（クランプ）・P5（バルーン暫定 offset）の**規則そのもの**（P2 からの参照関係は現状維持）。
- `spawn.rs`・`follow.rs`・`persist.rs`（初期解決の下流。記号参照ゆえ自動追随）。
- `ScaleRatio` 丸め権威（round half away from zero・非ゼロ長最小 1px）の変更・例外追加（2.6）。
- アーカイブ済み spec（`completed/areka-P0-window-placement`）の文書（5.4）。
- SSP 再実測（R1 完了済み。再検証が必要になった場合の手順は `ssp-oracle-notes.md` 冒頭に記録済み）。

### Allowed Dependencies

- `ScaleRatio`（既存丸め権威）——フィクスチャ経路（measure/prepare）が使用。resolver 自体は物理 px 純関数のままスケーリングを持ち込まない。
- `persist.rs:397-409` の `merge_scope restore` ログ（target `areka::persist::restore`）——実機受け入れの**読み取り専用**観測点。フィールド追加・変更は行わない。
- `tools/measure-ssp-rects.ps1`（本仕様 R1 で作成済み・コミット済み）——実機受け入れの従判定（外部矩形実測）に再利用。
- `AREKA_APP_SMOKE_EXIT_MS`（既存の有界自動終了機構）。

### Revalidation Triggers

- P2 式・`prev` 型の変更 → **`windowposition-limit`（W6.5）は本仕様の着地後の檻へ rebase 必達**（resolver.rs 同一関数 30 行差・scg 先の直列関係。roadmap 台帳）。
- emo2 フィクスチャ位置期待値の変更 → 下流 `emo2-conformance-e2e`（二体間隔の目視項目）・`balloon-visibility`（char 位置従属）は本仕様の是正値を前提に再確認。
- §8 エントリ追加 → 互換対応表の参照者（以後の placement 系 spec）は R2.9 を正典として引用してはならない。
- バルーン offset の SSP 突合テストが不合格化した場合 → 新たな欠陥のシグナル（4.3）。期待値書き換えは禁止で、原因特定が先。

## Architecture

### Existing Architecture Analysis

`resolve_placement`（resolver.rs:124-212）は物理 px 一貫・wintf 非依存・panic しない純粋関数で、P1（Y bottom）→ P2（X 連鎖基準）→ P3（free）→ P4（クランプ）→ P5（バルーン暫定 offset）の 5 規則を 1 パスで適用する。P2 の現行実装（:155-158）:

```rust
let base_x = match prev {
    None => work_area.right.saturating_sub(w),
    Some((prev_x, prev_w)) => prev_x.saturating_sub(prev_w),   // ← 欠陥: 前スコープの幅を引く
};
```

`prev: Option<(i32, i32)>`（:131-132）は「(P4 クランプ後 char_x, char 幅)」を保持し :178 で更新される。**幅の第 2 要素は欠陥式のためだけに存在する**。

### 是正の形（設計決定）

- **最終形は最小是正（gap 分析 Option A）**。Option C（実測先行ハイブリッド）の Step 0 は要件フェーズで消化済みで、H1 確定により設計分岐（H2 定数項・H3 構造置換）は消滅した。規則注入用の enum 等の抽象（Option B）は、確定後は 1 規則しか使わないため導入しない（YAGNI・「単純な基準配置のみ」の維持）。
- **是正式**: `base_x(n≥1) = prev_x.saturating_sub(w)`（**自スコープの幅**）。これにより defaultx=0・非クランプ時に `scope_n の右端 = scope_{n−1} の左端`（gap 0）が恒等的に成立する。
- **`prev` の縮小**: 幅が不要になるため `Option<(i32, i32)>` → **`Option<i32>`**（クランプ後 char_x のみ）。欠陥式へ戻す退行を型レベルで書きにくくする副次効果を持つ。
- **不変の周辺規則**: defaultx 減算（`bottom_x = base_x − default_x.unwrap_or(0)`）・P4 クランプ・「クランプ後の実配置が連鎖基準」（2.7）・P1/P3/P5 はすべて無改変。resolver にスケーリング・丸めは引き続き存在しない（2.6 は構造的に充足）。
- **doc 引用の書き換え方針（research §6 判断 #6 の裁定）**: 旧要件番号「2.9」の引用は、是正式＋本仕様の要件番号（2.1/2.2）へ振り直し、**履歴参照を 1 箇所（モジュール doc）にのみ**「旧 window-placement R2.9 は本仕様で上書き（COMPAT §8 参照）」の形で残す。テストメッセージ内は簡潔に「隣接（scg 2.1/2.2）」系へ統一し、行ごとに履歴注記を繰り返さない。

### Technology Stack

新規依存なし。Rust 2024（既存 crate 内変更）＋ PowerShell 7（既存計測ツール `tools/measure-ssp-rects.ps1` の再利用）＋ Markdown（COMPAT §8）。

## File Structure Plan

新規ソースファイルなし。変更は既存ファイルへの局所改修のみ。

### Modified Files

- `crates/areka/src/placement/resolver.rs` — P2 分岐の式是正（:155-158）・`prev` 型縮小（:131-132・:178）・モジュール doc の式引用更新（:98-102）・インラインコメント更新（:151-152）。P5 ハンク（:180-188）は**非接触**（wpl の領分）。
- `crates/areka/src/placement/resolver_resolve_tests.rs` — 連鎖檻の真実性是正:
  - `t_r2_scope_chain_defaultx_zero_stays_adjacent`（:130）— 期待値を `x0 − w1`／`x0 − w1 − w2` へ是正し、**gap 0 の明示 assert** と**欠陥式（`x0 − w0`）の否定 assert** を追加。名前は是正後に真実となるため維持。
  - `t_r2_chain_defaultx_offsets_leftward_from_base`（:175）— 期待値 `x0 − w1 − dx1` へ追随。
  - `t_r4_free_position_feeds_scope_chain`（:524）— 期待値 `x0 − w1` へ追随。
  - `t_r6_chain_uses_clamped_previous_position`（:363）— assert 不変・コメント（:377）の式のみ追随。
  - doc コメント内の式引用（:127-128・:154・:164・:193・:545）を是正式へ更新。
  - **新設** `t_r2_unequal_widths_leave_no_gap` — 規則の檻の本丸（後述 C2）。
- `crates/areka/src/placement/placement_prepare_tests.rs` — `prepare_emo2_returns_two_scope_placements`（:57）の `s1.char_pos`（:80）`(1052,640)→(1150,640)`・`s1.balloon_pos`（:84）`(1198,565)→(1296,565)`・導出 doc コメント（:38-51）更新。`s1.balloon_offset (146,−75)`（:95）は**不変のまま assert 維持**。**新設** `prepare_emo2_at_dpi_120_places_scopes_adjacent`（後述 C3）。
- `doc/COMPAT_ARCHITECTURE.md` — §8 表へ 1 行追加（後述 C4）。
- `.kiro/specs/areka-P0-scope-chain-gap/tools/measure-ssp-rects.ps1` — `-ProcessName` パラメタ追加（既定 `ssp`・後方互換。C5 従判定で areka プロセスへ向けるための前提改修）。

### Unchanged (regression net / monitors)

- `crates/areka/src/placement/placement_windowposition_tests.rs` — `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`（:87）は char 絶対位置を参照しない（静的確認済み）。**無改変合格が要件 4.2 の檻**。
- `crates/areka/src/placement/{spawn_follow_pipeline_tests.rs, spawn_assembly_tests.rs}`・persist/follow 系 — resolver 出力を記号参照（`p.char_pos` 透過）するのみで自動追随。
- `crates/areka/src/placement/{mod.rs:368, windowposition.rs:63-64}` — P5 の式引用で P2 非依存（確認済み・追随不要）。
- `crates/areka/src/placement/persist.rs` — 無改変（restore ログは観測のみ）。

### Evidence (spec directory)

- `.kiro/specs/areka-P0-scope-chain-gap/ssp-oracle-notes.md`・`ssp-rects-*.log`・`tools/measure-ssp-rects.ps1` — R1 の正本（既存・非改変）。
- `.kiro/specs/areka-P0-scope-chain-gap/real-run-signoff-<date>.log` — 実機受け入れ（C5）の証跡として実装フェーズで追加。

## Requirements Traceability

| 要件 | 概要 | 実現要素 |
|---|---|---|
| 1.1, 1.2, 1.3, 1.4, 1.5, 1.6 | SSP 実測オラクルの採取と記録 | **完了済み（C0）**。正本 `ssp-oracle-notes.md`＋証跡ログ 8 本＋`tools/measure-ssp-rects.ps1`。H1 確定・R1.5 判定=不一致・move 前単離は move 除去プローブで達成（1.6）・仮説外規則への合わせ込みなし（1.4） |
| 2.1 | 確定規則に従う X 決定 | C1: P2 是正式 `base_x(n≥1) = prev_x − w(n)` |
| 2.2 | 幅差比例の隙間を生じさせない | C1＋C2 の gap 0 明示 assert（不等幅入力） |
| 2.3 | 実機同条件（DPI 120・543×859/420×500）で規則適用期待値と一致（許容差 1px） | C3 新設檻（決定論形・k=5/4 フィクスチャ）＋C5 実機判定（同条件実機形） |
| 2.4 | DPI 96/120/144/192 で規則不変 | C2: `DPIS` 行列で全檻を実行 |
| 2.5 | 等幅を特殊扱いしない | C2 新設檻に等幅ケースを併置（同一式で検定） |
| 2.6 | 既存丸め権威のみ・新丸め規約禁止 | C1: resolver は物理 px 純関数のまま（スケーリング非導入）。フィクスチャ経路は既存 `ScaleRatio` のみ |
| 2.7 | クランプ後実配置を連鎖基準とする現行原則の維持 | C1: P4→`prev` 更新順序を無改変。C2: `t_r6_chain_uses_clamped_previous_position` が assert 不変のまま監視 |
| 2.8 | defaultx 意味論不変 | C1: defaultx 減算項無改変。C2: DD3 否定 assert（右端に戻らない）維持 |
| 2.9 | 複雑な配置ロジック非導入 | C1: 1 分岐の式変更のみ・新規抽象なし（設計決定） |
| 3.1 | 「隣接」を名乗る檻が確定規則どおりの幾何を検定 | C2: gap 0 明示 assert への是正 |
| 3.2 | 不等幅入力の使用 | C2: 既存 400/320/200 系を維持＋新設檻も不等幅 |
| 3.3 | 名前・メッセージ・内容の三者一致 | C2: メッセージの「密着（2.9）」→「隣接（scg 2.1/2.2）」系へ統一 |
| 3.4 | 連鎖値依存テストの期待値追随 | C2: `t_r2_chain_defaultx_offsets_leftward_from_base`・`t_r4_free_position_feeds_scope_chain` |
| 3.5 | 不等幅×複数 DPI 行列の決定論全網羅 | C2: 全檻 `DPIS=[96,120,144,192]` ループ・純関数直接呼び出し |
| 4.1 | emo2 実寸フィクスチャの位置期待値更新 | C3: `s1.char_pos`/`s1.balloon_pos` のオラクル由来値への更新 |
| 4.2 | バルーン offset SSP 突合テストの無改変合格 | C3: `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120` 非接触＋全数実行で確認 |
| 4.3 | 突合テスト不合格時は欠陥シグナル扱い | C3: 期待値書き換え禁止・原因特定・記録の手順を明文化 |
| 5.1 | §8 へ本裁定エントリ追加・先行 AC 名指し | C4: window-placement R2.9 の名指し |
| 5.2 | オラクル・実測値・実測条件の記載 | C4: emo2-probe・DPI 96/192・gap 0 の記載 |
| 5.3 | 「SSP de-facto」札の無検証事実の明記 | C4: research.md:78→:122 の経緯を同一行に畳む |
| 5.4 | アーカイブ非改変 | C4: completed spec 非接触（File Structure Plan に不在） |
| 5.5 | 先例（R3.8 行）と同じ体裁 | C4: 同表・同列構成の 1 行 |
| 6.1 | 本番ゴースト＋実 DPI≠96 必達 | C5: emo2＋DPI 120 実機実行 |
| 6.2 | 有界自動終了＋ログ突合の決定論判定 | C5: `AREKA_APP_SMOKE_EXIT_MS`＋`merge_scope restore` grep |
| 6.3 | DPI 96 のみ緑は不合格 | C5: 合否条件に明記 |
| 6.4 | 窓矩形実測と規則期待値（gap 0・1px 以内）一致 | C5: 主判定（ログ）＋従判定（外部矩形実測） |
| 7.1 | 実表示寸が判明した時点で連鎖を再解決し初期配置を確定 | C6: resnap 直後の一度きりの再解決（P2 式を実窓へ適用） |
| 7.2 | 下端中央原点の保存規則は不変（接地点を動かさない） | C6: Y 非変更・scope0 非移動・`resize_window_to` 非接触 |
| 7.3 | 明示的に再配置されたスコープは引き戻さない | C6: 既定位置との一致判定（move／drag 側へフックを足さない） |
| 7.4 | 確定後のサーフェス切替では再解決しない（横滑り防止） | C6: 確定済みフラグで二度目以降を no-op |
| 7.5 | 実機受け入れは定常表示状態の実測でも隙間 0 | C5 拡張: 定常時の外部矩形実測を合否判定に加える |
| 7.6 | 演出の移動指令を判定から除外できる形で受け入れ判定 | C5 拡張: 移動指令を除いた複製ゴーストでの実行を手順に含める |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|---|---|---|---|---|---|
| C0 SSP オラクル記録 | spec 文書 | 確定規則の正本（完了済み・参照のみ） | 1.1–1.6 | なし | — |
| C1 P2 連鎖式是正 | placement/resolver | 隣接規則（自幅減算）の実装 | 1.5, 2.1, 2.2, 2.6, 2.7, 2.8, 2.9 | ScaleRatio 非経由（純物理 px） | Service |
| C2 連鎖檻の真実性是正 | placement tests | 名前＝内容＝メッセージの一致と規則の全網羅 | 2.2, 2.4, 2.5, 3.1–3.5 | C1（P0） | State（テスト固定） |
| C3 フィクスチャ追随＋不変量監視 | placement tests | 実寸期待値の追随と offset 不変量の監視 | 2.3, 4.1, 4.2, 4.3 | C1（P0）・ScaleRatio（P2） | State（テスト固定） |
| C4 COMPAT §8 記録 | doc | R2.9 上書きの正典記録 | 5.1–5.5 | C0（P0） | — |
| C5 実機受け入れ | 検証手順 | 実機での決定論的合否判定と証跡 | 2.3, 6.1–6.4 | C1（P0）・persist restore ログ（P0・読取専用）・measure-ssp-rects.ps1（P2） | Batch（手順） |

### placement/resolver

#### C1: P2 連鎖式是正

| Field | Detail |
|---|---|
| Intent | P2 の X 基準を「前スコープ左端 − 自スコープ幅」へ是正し、隣接（gap 0）を実装する |
| Requirements | 1.5, 2.1, 2.2, 2.6, 2.7, 2.8, 2.9 |

**Responsibilities & Constraints**
- `resolve_placement` の公開シグネチャ・出力型 `ScopePlacement` は**無変更**（下流の記号参照が自動追随する前提を壊さない）。
- panic しない契約・saturating 演算・入力順保存・出力長＝入力長の事後条件を維持。
- P5 ハンク（:180-188）に差分を作らない（wpl との直列関係を乱さない）。

**Contracts**: Service [x]

##### Service Interface（変更後の P2 仕様）

```rust
// シグネチャ無変更
pub fn resolve_placement(
    cfg: &PlacementConfig,
    work_area: RectPx,
    scopes: &[ScopeInput],
) -> Vec<ScopePlacement>;

// 内部状態（縮小）: P2 連鎖の前スコープ状態＝クランプ後 char_x のみ
let mut prev: Option<i32> = None;

// P2（是正後）: base_x(0) = work_area.right − w(0)
//              base_x(n≥1) = prev_x − w(n)      // ← 自スコープの幅（scg 2.1/2.2）
let base_x = match prev {
    None => work_area.right.saturating_sub(w),
    Some(prev_x) => prev_x.saturating_sub(w),
};
// 以降（defaultx 減算・P3・P4 クランプ・prev = Some(x)・P5）は無改変
```

- Preconditions: 現行と同一（任意の `scopes`・任意の work area で panic しない）。
- Postconditions: 既存事後条件（長さ・順序・`balloon_offset ≡ balloon_pos − char_pos`）に加え、**隣接不変量**——`defaultx=0` かつ両者非クランプのとき `out[n−1].char_pos.x == out[n].char_pos.x + w(n)`（gap 0）——が成立する。
- Invariants: 連鎖基準は P4 クランプ後の実配置（2.7）。resolver 内にスケーリング・丸めは存在しない（2.6）。

**Implementation Notes**
- Integration: doc（:98-102・:151-152）の式引用を是正式＋scg 2.1/2.2 へ更新し、モジュール doc に 1 箇所のみ「旧 window-placement R2.9 は本仕様で上書き（COMPAT §8）」の履歴注記を置く。
- Validation: C2/C3 の檻＋既存比較系檻（`t_r5_seam_output_identical_to_bottom`・`t_r4_free_both_unspecified_equals_bottom`・`postconditions_order_length_and_offset_identity`）が回帰網。
- Risks: なし（純関数 1 分岐・波及全数特定済み）。`prev` 型縮小により欠陥式の再導入はコンパイルエラーになる。

### placement tests

#### C2: 連鎖檻の真実性是正

| Field | Detail |
|---|---|
| Intent | 「隣接」の名を持つ檻が gap 0 を実際に主張し、不等幅×DPI 行列で確定規則を全網羅する |
| Requirements | 2.2, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5 |

**Responsibilities & Constraints**（research §6 判断 #4 の裁定）
- **既存 3 本の改修＋新設 1 本**の粒度とする。既存檻の名前（`…_stays_adjacent` 等）は是正後に真実となるため**改名せず**、assert 内容とメッセージを名前へ一致させる（3.1/3.3）。
- 改修内容:
  - `t_r2_scope_chain_defaultx_zero_stays_adjacent`: 期待値 `out[1].x = x0 − w1`・`out[2].x = x0 − w1 − w2` へ是正。**gap 明示 assert**（`out[0].x − (out[1].x + w1) == 0`・`out[1].x − (out[2].x + w2) == 0`）と**欠陥式の否定 assert**（`assert_ne!(out[1].x, x0 − w0)`＝不等幅入力でのみ判別可能・退行の再発防止）を追加。既存の DD3 否定 assert（右端に戻らない・2.8）は維持。
  - `t_r2_chain_defaultx_offsets_leftward_from_base`: `out[1].x = x0 − w1 − dx1` へ追随（3.4）。
  - `t_r4_free_position_feeds_scope_chain`: `out[1].x = x0 − w1` へ追随（3.4・free 実位置基準の原則は不変）。
  - `t_r6_chain_uses_clamped_previous_position`: assert 不変（`x0 − w1` も左外→クランプ）。コメントの式のみ追随。2.7 の監視役。
- **新設** `t_r2_unequal_widths_leave_no_gap`（規則の檻の本丸）:
  - 不等幅（例: 400/320/200）で全隣接ペアの gap 0 を明示 assert（2.2/3.1/3.2）。
  - 等幅ケース（例: 320/320）を同一テスト内に併置し、同一式で配置されることを assert（2.5・等幅の特殊扱い排除）。
  - `DPIS=[96,120,144,192]` 全水準ループ（2.4/3.5）。
  - wpl が rebase しやすいよう P2 の檻として独立させ、P5 系の檻と混ぜない。
- メッセージ規約: 「密着（2.9）」等の旧引用は「隣接・gap 0（scg 2.1/2.2）」系へ統一（3.3・履歴注記は resolver doc の 1 箇所に集約）。

**Contracts**: State [x]（決定論檻・純関数直接呼び出し・実行順非依存）

#### C3: フィクスチャ追随＋不変量監視

| Field | Detail |
|---|---|
| Intent | emo2 実寸系の位置期待値を是正後の値へ追随させ、バルーン offset の不変量で波及漏れを監視する |
| Requirements | 2.3, 4.1, 4.2, 4.3 |

**Responsibilities & Constraints**
- `prepare_emo2_returns_two_scope_placements`（k₀=1/1・WA 1920×1040 系）の期待値更新（4.1）:
  - `s1.char_pos`: `(1052, 640)` → **`(1150, 640)`**（`1486 − 336`＝scope1 自身の幅）。
  - `s1.balloon_pos`: `(1198, 565)` → **`(1296, 565)`**（右置き基準 `1150+336=1486` ＋ wp 調整 `−190`）。
  - `s1.balloon_offset (146, −75)`・`s0` 系・寸法系 assert は**不変のまま維持**（offset は char 相対＝位置変化に不変）。導出 doc コメント（:38-51）を是正式で書き直す。
- **新設** `prepare_emo2_at_dpi_120_places_scopes_adjacent`: `prepare_ghost_windows_with_work_area(…, Some(120))`（k=5/4）で `s0.char_size = 543×859`・`s1.char_size = 420×500`（実機同条件の前提錨）を確認したうえで、**`s1.char_pos.x + 420 == s0.char_pos.x`**（scope1 右端＝scope0 左端）を assert する。2.3 の「実機実測と同条件で規則を自らの実表示寸へ適用した期待値と一致」の**決定論形**（許容差は等式成立＝0px。丸めは `ScaleRatio` 経由の寸法算出にのみ現れ、位置式は整数演算ゆえ成分誤差が出ない）。
- `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120` は**非接触**（4.2）。全テスト実行で無改変合格を確認する。不合格化した場合は 4.3 に従い、期待値の書き換えで黙らせることを禁じ、原因（char 位置への未知の依存＝新欠陥）を特定して spec 記録へ残してから是正する。

**Contracts**: State [x]

### doc

#### C4: COMPAT §8 記録

| Field | Detail |
|---|---|
| Intent | window-placement R2.9 の上書きを沈黙ルール対応表へ正典記録する |
| Requirements | 5.1, 5.2, 5.3, 5.4, 5.5 |

**Responsibilities & Constraints**（research §6 判断 #7 の裁定）
- **1 行に全要素を畳む**（R3.8 先例と同型・5.5）。列構成は既存表（項目｜裁量｜根拠｜出典 spec）に従う。
- 記載要素（5.1–5.3）:
  - **項目**: 複数スコープの既定 X 連鎖規則（正典沈黙箇所・先行 spec が「SSP de-facto」として規定していた項目）。
  - **裁量**: `scope_n.L = scope_{n−1}.L − scope_n 自身の幅`（完全隣接・gap 0）。連鎖基準はクランプ後実配置・defaultx は基準からの左方向オフセットで不変。
  - **根拠**: 実機確定（2026-08-11）＝参照実装 SSP を受理オラクルとした。move 除去プローブ（emo2-probe）・プロファイル削除・初回起動・DPI 96 と 192 の両方で誤差 0（DPI 不変）——ただし DPI 192 の「誤差 0」は**配置時の代表面寸**（868／854 物理 px・境界 2012）に対してであり、生観測は SSP の起動時サーフェス寸レースで見かけ隙間 104px を示す（`ssp-oracle-notes.md:95-98` が正）。**本裁定が否定した先行 AC は `completed/areka-P0-window-placement` R2.9**（「scope1 を scope0 のサーフェス画像幅ぶん左へずらす（SSP de-facto）」）——同 spec の research.md は当該項目を Unknown と記載したまま要件討議で確定しており、**「SSP de-facto」の札は SSP 実挙動と突合されないまま貼られていた**。アーカイブ済み spec は非改変とし、上書きの事実を本表と現行 spec に記録する。付記: SSP のコールドブート定常値には SSP 自己不整合 2 件（起動時サーフェス寸レースによる見かけ隙間・ゴースト演出 `\![move]` の物理 px 無スケール適用）が混入するため**互換対象外**（受理オラクルは規則そのもの・要件討議 #2 裁定）。
  - **出典 spec**: areka-P0-scope-chain-gap（`resolver.rs` P2・証跡 `ssp-oracle-notes.md`／`ssp-rects-boot-probeA-dpi96.log`／`ssp-rects-boot-probeA-dpi192.log`）。
- `completed/areka-P0-window-placement` 配下には一切書き込まない（5.4）。

### 検証手順

#### C5: 実機受け入れ

| Field | Detail |
|---|---|
| Intent | 本番ゴースト＋実 DPI≠96 の実機で、是正後の二体間隔を決定論的に合否判定する |
| Requirements | 2.3, 6.1, 6.2, 6.3, 6.4 |

**Responsibilities & Constraints**（research §6 判断 #5 の裁定＝二本立て・主はログ）
- **前提条件（6.1/6.3）**: 本番ゴースト emo2＋実 DPI≠96（開発機 DPI 120・k=5/4 実績）で実行。プロファイル削除＝初回起動（先例手順）。DPI 96 のみで緑の場合は不合格。
- **主判定（6.2/6.4）＝`merge_scope restore` ログ grep（追加実装ゼロ）**:
  - 実行: `AREKA_APP_SMOKE_EXIT_MS` による有界自動終了＋`RUST_LOG` で target `areka::persist::restore` を捕捉。
  - 判定式: scope0 行と scope1 行の **`default_char_x`／`char_w`** を突合し、`gap = default_char_x(scope0) − (default_char_x(scope1) + char_w(scope1))`。**合格条件: |gap| ≤ 1**（既存丸め権威由来の許容差・2.3/6.4）。
  - **`char_x` ではなく `default_char_x` を用いる**（設計決定）: `default_char_x` は resolver 出力そのもの（persist.rs:402）であり、ゴースト演出 `\![move]`（boot 約 1 秒後に scope1 を移動）と保存位置の復元のいずれの汚染も受けない。是正前の実機ログでは同式が gap=123 を返しており（scope0 3297・scope1 2754+420）、同一 grep が是正後に gap=0 を返すことが判定になる。
  - 付随確認: 是正が Y へ波及していないこと（`default_char_y` が各 scope の bottom 密着値であること）を同一ログで確認。
- **従判定（6.4 補強・証跡用）＝外部矩形実測**: `tools/measure-ssp-rects.ps1` を areka の窓へ向けて起動時系列の窓矩形を採取する。**前提改修（バリデーション指摘）**: 同ツールは対象プロセス名 `ssp` を固定でハードコードしている（`Get-Process -Name ssp`）ため、実装フェーズで `-ProcessName` パラメタ（既定 `ssp`・後方互換）を追加してから areka のプロセスへ向ける。ゴースト演出 move 適用**後**の値が混ざり得るため（オラクル採取時と同じ罠）、**合否は主判定で確定**し、従判定は初期出現時点の矩形が主判定と整合することの補強証跡とする。
- **証跡**: 実行ログ・grep 結果・判定値を `real-run-signoff-<date>.log` として spec ディレクトリへ保存（先例体裁）。

**Contracts**: Batch [x]
- Trigger: 実装完了後の受け入れ検証（tasks 最終段）。
- Input / validation: 上記前提条件。
- Output / destination: 合否判定＋spec ディレクトリの証跡ログ。
- Idempotency & recovery: プロファイル削除から再実行可能（読み取り専用計測・破壊的操作なし）。

### placement（初期配置の確定）

#### C6: 実表示寸での連鎖再解決（Requirement 7）

| Field | Detail |
|---|---|
| Intent | 初期配置を「実表示サーフェス寸が判明した時点」で確定させ、定常表示でも隣接（隙間 0）を成立させる |
| Requirements | 7.1, 7.2, 7.3, 7.4, 7.5, 7.6 |

**問題の機序（実機実測 2026-08-13・拡大率 200%）**

配置は scope0 の起動サーフェス寸（868 物理 px）で解決され隙間 0 になる。直後にゴーストが実表示サーフェス（`surface_id=1000`・764 物理 px）を選び、`resnap_shell_targets`→`resize_window_to`（`follow/window_move.rs:244-258`）が**下端中央固定**で再アンカーするため scope0 の左端が `(868−764)÷2 = 52` 右へ寄る。連鎖は再計算されないため 52px の隙間が残る。

**設計判断**

- **一度きりの確定**（連続維持ではない）。R7.4 の要求どおり、確定後のサーフェス切替では位置を動かさない。連続維持は追従（follow）の領分であり、`windowposition-limit`・DPI 遷移系と同じ関数・同じフレーム経路で干渉する。
  - 実測の裏づけ: emo2 は 3 分の会話で表情が複数回変わってもシェル面の寸法は不変（scope0 `764×1094`／scope1 `672×800` で固定・`size_changed=true` は起動時の 1 回のみ）。ゆえに一度きりで実害が出ない。
- **再解決は resolver の P2 式をそのまま実窓へ当てる**（設定・work area・バルーン寸を要さない）: scope 昇順に `new_x(n) = x(n−1) − w(n)`（`x(n−1)` は再アンカー後の実位置・`w(n)` は実表示幅）。Y は動かさない（R7.2）。scope0 は動かさない（連鎖の起点＝各キャラの接地点は不変）。
- **「未接触」判定で明示的再配置を避ける**（R7.3）。spawn 時の既定位置を保持し、現在位置が既定位置と一致するスコープのみ再解決の対象とする。`\![move]` や利用者のドラッグで動いたスコープは一致しないため自動的に除外される——move／drag 側にフックを増やさずに済む。
- **下端中央の再アンカー規則そのものは非接触**（`resize_window_to` は完了済み `areka-P0-surface-resize-resnap` の領分）。本仕様は再アンカー**後**の実位置を入力として受け取り、スコープ間の連鎖のみを直す。

**Contracts**: Service（純関数の判定部＋薄い結線）

- 判定部は「(scope, 現在位置, 現在寸, 既定位置) の列 → 移動指示の列」の純関数とし、GPU も World も要さない決定論檻に入れる（D9 の振り分け基準に従う）。
- 結線は resnap の直後に置き、確定済みフラグで二度目以降を no-op にする。
- Postconditions: 確定後、未接触スコープについて `x(n−1) == x(n) + w(n)`（隙間 0）が実表示寸で成立する。

## Error Handling

- 本是正はエラー経路を追加しない。`resolve_placement` の panic しない契約・saturating 演算を維持し、異常入力（空 scopes・巨大寸法・逆転 work area）は既存 P4/クランプ系の檻がそのまま担保する。
- 検証系の失敗シグナル運用のみ規定する: (a) バルーン offset SSP 突合テストの不合格＝新欠陥のシグナル（4.3・期待値書き換え禁止）、(b) 実機主判定の |gap| > 1＝不合格（6.4・原因調査へ）。いずれも「テストを黙らせる」是正を禁じる。

## Testing Strategy

### Unit Tests（resolver 純関数・決定論・DPIS=[96,120,144,192] 全水準）

1. `t_r2_unequal_widths_leave_no_gap`（新設）: 不等幅で全隣接ペア gap 0＋等幅併置＋欠陥式否定（2.2/2.4/2.5/3.1/3.2/3.5）。
2. `t_r2_scope_chain_defaultx_zero_stays_adjacent`（是正）: 3 スコープ一般連鎖の是正式＋gap 明示＋DD3 否定維持（2.1/2.8/3.1/3.3）。
3. `t_r2_chain_defaultx_offsets_leftward_from_base`（追随）: defaultx 合成が是正基準から左方向（2.8/3.4）。
4. `t_r4_free_position_feeds_scope_chain`（追随）: free 実位置が連鎖基準（3.4）。
5. `t_r6_chain_uses_clamped_previous_position`（assert 不変）: クランプ後連鎖の維持（2.7）。
6. 既存比較系（`t_r5_seam_output_identical_to_bottom`・`t_r4_free_both_unspecified_equals_bottom`・`postconditions_order_length_and_offset_identity`）: 無改変で回帰網として全実行。

### Integration / Fixture Tests（emo2 実寸・COM 初期化下）

1. `prepare_emo2_returns_two_scope_placements`（期待値更新）: k₀=1/1 の全 5 規則厳密値（4.1）。
2. `prepare_emo2_at_dpi_120_places_scopes_adjacent`（新設）: k=5/4・543/420 実寸で scope1 右端＝scope0 左端（2.3 の決定論形）。
3. `prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`（無改変）: 合格継続の確認が 4.2 の檻。
4. spawn/follow/persist 系（無改変）: 記号参照の自動追随を全実行で確認。

### Real Machine（受け入れ・C5）

1. emo2＋実 DPI≠96（開発機の実績は 120。実施時の設定に従う）・プロファイル削除・有界終了で起動し、`merge_scope restore` grep により |gap| ≤ 1 を判定（6.1/6.2/6.4）。
2. DPI 96 でも同判定を対照実行（規則の DPI 不変の実機面・2.4）。ただし 96 のみ緑は不合格（6.3）。
3. 従判定: 外部矩形実測ログを補強証跡として保存（6.4）。

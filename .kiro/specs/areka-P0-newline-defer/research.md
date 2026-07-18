# ギャップ分析: areka-P0-newline-defer

> SSP 準拠の改行遅延（deferred newline）を `areka-emo-text` 純粋層（layout）へ導入する P0 バグ修正のギャップ分析。
> 目的は「最終決定」ではなく、既存資産の把握・欠落能力の特定・複数実装案の提示・設計フェーズへの申し送り。

生成日: 2026-07-18 / spec language: ja / phase: requirements-generated（gap 分析）

---

## 0. 前提の実測再突合せ（brief 陳腐化チェック）

並走ワークツリーのマージで brief の file:line 参照が腐っていないかを実測で確認した（`parallel-worktree-brief-staleness` の定石）。

- `git merge-base HEAD origin/main` = `0bf923a`（本ブランチ base）。
- `git log <base>..origin/main -- crates/areka-emo-text` = **0 コミット**（base 以降 origin/main で emo-text への変更なし）。
- brief 引用行を実ファイルで全数照合した結果、**全て現存・記述と一致**:

| brief 引用 | 実体 | 種別 | 照合 |
|---|---|---|---|
| `layout.rs:211-224` | `TextItem::LineBreak { ratio } =>` 即時行送り（`block_pos += block_dir * pitch * ratio`） | **本番** | 一致（唯一の本番行送り分岐） |
| `layout.rs:748` | `fn trailing_line_break_opens_empty_line`（末尾改行が空行を開く） | テスト檻 | 一致 |
| `layout.rs:1016` | `fn trailing_empty_line_participates_in_overflow`（末尾空行があふれ参加） | テスト檻 | 一致 |
| `draw.rs:1705` | `fn scroll_overflow_drops_oldest_line_via_full_redraw`（あふれ後の全域再描画） | テスト檻 | 一致 |
| `canvas.rs:465` | `fn empty_lines_are_preserved_as_empty_glyph_residents`（空行も住人・行 index 1:1） | テスト檻 | 一致 |
| `state.rs:224-229` | `CueCommand::Choice`（choice-render シーム・本 spec は不触） | 本番 | 一致 |

結論: **brief は非陳腐化。line 参照はそのまま設計に持ち込める。**

---

## 1. 現状調査（Current State）

### 1.1 対象 crate と層構成

`crates/areka-emo-text/`（完了済み `areka-P0-emo-text-layer` の成果物）。関連ファイル:

- **`state.rs`** — `TextLayerState` / `ActorTextState { items: Vec<TextItem>, reveal }`。cue を受けて actor 別の追記正本 `items`（`TextItem::Glyph` / `TextItem::LineBreak { ratio }`）を構築。`apply` で `CueCommand::NewLine` → `items.push(LineBreak)`（state.rs:200-203）、`Clear`/`ClearAll` → `ActorTextState::default()` で全消去（state.rs:204-222）。`visible_glyphs(actor, t)` は**グリフのみ**を数える reveal カーソル（state.rs:252-256・改行は数えない）。
- **`layout.rs`** — `LayoutEngine::layout(items, visible_count, region, mode, font_height, metrics) -> Vec<PositionedLine>`（純関数・失敗経路なし）。`items` を走査し `visible_count`（＝可視 prefix）で切って行列を生成。**`visible_window(lines, region, mode) -> VisibleWindow`**（あふれ判定・スクロール可視窓の純粋決定・R7.4 分離シームの上半分）。
- **`canvas.rs`** — `ContentCanvas::from_layout(lines, region, mode)`。**layout 出力の行を 1:1 で住人（`Resident`）へ写すだけ**（canvas.rs:217-249・独自の改行走査を持たない・空行も空住人として保持）。
- **`draw.rs`** — COM 層。`DWriteMetrics`（実 metrics）を注入し、`window.first_visible_line` 以降の行を validrect 先頭へ詰めて描画（draw.rs:686-694）。あふれ発火＝先頭行が供給面から消える全域再描画（実 metrics 檻）。

### 1.2 データフロー（cue → 画面）

```
cue → state.items(追記正本) ──┐
                              ├→ LayoutEngine::layout(items, visible_count …) → Vec<PositionedLine>
state.visible_glyphs(t) ──────┘         │
                                        ├→ ContentCanvas::from_layout(lines …)  → residents（行 1:1）
                                        └→ LayoutEngine::visible_window(lines …) → VisibleWindow
                                                                                    │
                                                             draw（first_visible_line 以降を描画・全域再描画）
```

### 1.3 現行の即時（immediate）改行意味論＝バグの所在

`layout.rs:211-224`（本番・唯一の行送り分岐）:

```rust
TextItem::LineBreak { ratio } => {
    opened = true;
    lines.push(finish_line(std::mem::take(&mut current), …)); // ← 即座に行を閉じる
    block_pos += block_dir * pitch * ratio;                    // ← 即座に行送り
    inline_pos = inline_start;
}
```

Glyph 側の可視 prefix 打切りは `if placed == visible_count { break; }`（layout.rs:184）で Glyph アームにのみ存在するため、**最後の可視グリフより後ろの trailing `LineBreak` も走査され、空の新行を開く**。この空行が `visible_window` のあふれ判定入力に参加し（layout.rs:1016 の檻が明文化）、バルーン満杯付近では trailing `\n[150]` だけであふれ→先頭行スクロールアウト＝「1 行改行したように見える」（brief の因果連鎖 2〜3）。

### 1.4 慣習・制約（steering / メモリ由来）

- **層分離**: 判断分岐は純粋層（layout）に閉じ GPU 非依存で全網羅（`test-only-decision-branches-not-proven-wiring` / `deterministic-test-coverage-mandate`）。
- **檻の扱い**: 変更で落ちる既存テストは「仕様判断の変更＝意味あり → 更新／陳腐化 → 除外」を自分で判断（`obsolete-vs-broken-test-policy`）。本件は R7.2 が明示的に「陳腐化でなく意味の変更に伴う更新」と規定。
- **bin でなく crate**: `areka-emo-text` は lib crate ゆえテストは `#[cfg(test)] mod tests`（各ファイル内）で足りる。実機檻のみ別（後述 R8）。
- **実機サインオフ**: 有界 auto-exit＋ログ grep（`AREKA_APP_SMOKE_EXIT_MS=180000`・`RUST_LOG=info,kanade=trace`）＋出力画像の AI vision 目視（`emo-text-byte-equiv-default-font-blindspot` の盲点対策）。
- **ログ無し失敗経路の禁止**（`areka-log-first-no-silent-failure`）——ただし本件の layout は純関数で失敗経路を持たない設計。

---

## 2. 要件 → 資産マップ（Requirement-to-Asset Map）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| **R1** 改行の pending 化・ratio 累算 | layout 走査中に「即 push」→「pending 加算」へ | `layout.rs:211-224`（即時） | **Missing**（本番ロジック改訂・小） |
| **R2** 次グリフ配置時の一括実体化 | Glyph アームで pending をフラッシュしてから配置 | `layout.rs:180-226`（Glyph アーム） | **Missing**（走査ロジック改訂） |
| **R2.4** 別テキスト状態では実体化しない | layout は actor 別 `items` で独立呼出 | 既に per-actor 呼出（state.actors） | **充足**（構造的に自然成立） |
| **R3** あふれ判定を実体化時のみ評価 | trailing pending が lines に出ない → visible_window 入力に不参加 | `visible_window`（lines を入力） | **充足（派生）**（layout が空行を出さなければ自動的に不参加・visible_window 自体は非改変） |
| **R3.3** あふれ→スクロール機構自体は非改変 | 評価タイミングのみ移動 | `visible_window` / draw 全域再描画 | **Constraint**（機構非改変・タイミングのみ） |
| **R4** reveal 整合 | pending フラッシュは可視 prefix 内グリフ配置時のみ | `visible_count` 打切り（layout.rs:184） | **Missing**（フラッシュ位置と break 順序の設計） |
| **R4.2** カーソルが改行通過も次可視グリフ無し → 保留維持 | break を pending フラッシュ前に置く | 現行 break は Glyph 先頭 | **Missing**（break/flush 順序） |
| **R5** 破棄（蒸発） | Clear/ClearAll で items リセット・talk 終了で trailing pending 未フラッシュ | state.rs:204-222 の全消去・pending は layout 走査ローカル | **充足**（pending が per-frame transient なら state 改変不要） |
| **R6** 縦書き同一規則 | 軸読み替え正準表（`block_dir`/`inline_start`） | layout.rs:167-171（既に 3 方向畳込み） | **充足**（pending 加算も `block_dir * pitch * Σratio` で同式に乗る） |
| **R7** 決定論・純関数・全網羅／既存檻更新 | 純粋 layout の判断分岐檻・既存檻の意味更新 | layout/canvas/draw の `#[cfg(test)]` 群 | **Missing**（新檻＋既存檻更新） |
| **R8** 実機で現象消失・段落区切り維持 | emo2 fixture・pasta SHIORI での有界 auto-exit＋grep＋vision | `tests/`（バイナリ起動型・別 crate 上位） | **Unknown/Constraint**（実機檻の所在・grep marker 設計） |

**要点**: 本番の実変更は `LayoutEngine::layout` の走査ループ 1 箇所（約 10 行）に集約される。`visible_window` / `canvas.rs` / `draw.rs` の**本番コードは非改変**（layout が trailing 空行を出さなくなることで下流は自動的に正しくなる）。残りは**テスト檻の更新＋新規追加**が主。

---

## 3. 実装アプローチ案（A / B / C）

### Option A: `layout()` 走査ループ内のローカル pending（brief 本命）

走査中のローカル変数 `pending_ratio: f32`（＋必要なら `pending_present: bool`）を導入し、

- `LineBreak { ratio }` アーム: `pending_ratio += ratio`（行を開かない・`opened` を立てない）。
- `Glyph` アーム: `if placed == visible_count { break; }`（**pending フラッシュより前**）→ pending があれば「current 行を finish・`block_pos += block_dir * pitch * pending_ratio`・`pending_ratio = 0`」→ 折返し判定 → glyph 配置。
- ループ終了後: 残った `pending_ratio` は**フラッシュしない**（＝蒸発・R5.2）。

**トレードオフ**
- ✅ 変更が本番 1 ファイル・1 関数に閉じる（最小・純粋・GPU 非依存）。
- ✅ R5 蒸発が「per-frame transient」で自動成立（state 改変不要）。
- ✅ R6 縦書きは既存の `block_dir`/`pitch` 式へ自然に乗る。
- ✅ `visible_window`/`canvas`/`draw` 本番は非改変（下流波及なし）。
- ❌ 「連続改行を単一累算」と「先頭改行の扱い」で正準形を確定する必要（§4 の設計論点）。
- ❌ 既存檻の複数更新（layout/canvas/draw）が必要。

### Option B: state 層に pending を持たせ items を実体化時に整形

`ActorTextState` に pending 改行を保持し、実体化時に `items` へ確定的な `LineBreak` を差し込む案。

**トレードオフ**
- ✅ 「実体化済み/未実体化」が状態として明示化される。
- ❌ `items` は「追記順の後出し優先正本」（emo-text-layer の契約）であり、実体化で書き換えると reveal/Clear/ClearAll のタイミング契約や決定論檻に波及。canvas 1:1 不変条件（canvas.rs:198）とも二重管理になる。
- ❌ pending が cue 到着時刻に依存し始めると純粋性・決定論が state 層へ漏れる。要件が「遅延するのは**行送り（可視化・あふれ評価）**であり、cue バッファへの追記自体は維持」（R4.3・Adjacent expectations）と明言している方向に反する。
- 総じて**非推奨**（要件の設計方針と逆行）。

### Option C: ハイブリッド（layout に pending＋`visible_window` 入力整形の明示化）

Option A を基本に、あふれ判定側でも「未実体化空行を最新行集合から除く」入力整形を**明示的な関数**として足す案（brief の Boundary Candidate「最新行の定義から未実体化空行を除く」）。

**トレードオフ**
- ✅ 「あふれ入力の意図」がコードで自己文書化される。
- ❌ Option A で layout が trailing 空行を出さなくなれば `visible_window` の入力からは**自動的に**消えるため、二重の整形は冗長になりやすい。R3.3「機構自体は非改変・タイミングのみ移動」に対して過剰。
- 位置づけ: Option A で不足が判明した場合の**予備**（例: reveal 途中の中間空行が意図せず残るケースが設計で見つかったとき）。

**推奨**: **Option A を第一候補**（要件方針・最小侵襲・決定論に最も整合）。§4 の設計論点を design フェーズで確定する前提。

---

## 4. 設計フェーズへ持ち越す設計判断項目（要件ディスカッションへの申し送り）

> 以下は「情報提供・選択肢」であり最終決定ではない。番号は要件ディスカッション/design での討議単位。

1. **連続改行の実体化形（最重要）**: R1.3/R2.2 は「連続改行を ratio 累算した**単一**保留」「累算 ratio 合計に基づく行送り」と規定する。これは `\n\n` を「**中間空行 1 行を挟む**」現行 immediate 挙動から「**中間空行を持たない単一の大きな行送り（block_pos ジャンプ）**」へ変える。
   - 帰結: `[a, \n, \n, b]` は現行 3 行（a / 空 / b）→ 遅延では 2 行（a / b・間隔 = pitch×2）になる。**行 index・canvas 住人数・スクロール行カウントが変わる**。
   - 論点: これが SSP 準拠の意図（開発者観測）か、それとも「中間空行は保持しつつ trailing のみ蒸発」を望むのか。**要件文言は単一累算を支持**するが、visible_window の「行単位スクロール」との相互作用（行数が減る）を design で確認する。SSP 実機での連続改行の見え方を任意追観測する候補（brief §Current State の「必要なら追観測」）。

2. **先頭改行（glyph 出力前の pending）の扱い**: `[\n, a]`（先頭に改行、その後グリフ）で、次グリフ配置時に pending をフラッシュすると「空の current を finish」＝**先頭に空行**が生じ得る。
   - 論点: 先頭 pending も「実体化＝空行を開く」でよいか、それとも「先頭は行送りのみで空行を作らない」か。連続改行（論点 1）の正準形と整合させる必要。

3. **pending フラッシュと `visible_count` break の順序（R4.2 の要）**: break（`placed == visible_count`）を pending フラッシュより**前**に置くことで「reveal カーソルが改行を通過したが次可視グリフがまだ無い → 保留維持・行送りしない」が成立する。この順序を正準として design/tasks に固定する（実装時に取り違えると R4.2 が壊れる）。

4. **`opened` フラグの再定義**: 現行 `opened` は Glyph/LineBreak 双方で立つ。遅延化後は「LineBreak 単独では `opened` を立てない（trailing 改行だけの状態で末尾 finish を誘発しない）」必要がある。`[a, \n]` が 1 行（a のみ）になることの担保。

5. **canvas 1:1 不変条件の維持確認**: canvas.residents は layout 行と 1:1（canvas.rs:198・`first_visible_line` 直用）。layout が空行を出さなくなるだけなので**canvas 本番は非改変で 1:1 は保たれる**見込み。要件 Adjacent expectations も「item 列は非改変を理想・実体化判定は layout の解釈で吸収」と一致。design で「canvas 非改変」を明文確定する（不要な改変を防ぐ）。

6. **更新すべき既存檻の棚卸し（R7.2）**: 少なくとも以下は「意味の変更に伴う更新」対象:
   - `layout.rs:722`（`line_break_opens_empty_line` 系・即時反映を前提）／`layout.rs:748`（`trailing_line_break_opens_empty_line`）／`layout.rs:1016`（`trailing_empty_line_participates_in_overflow`）
   - `canvas.rs:468`（`empty_lines_are_preserved_as_empty_glyph_residents`・items=[Glyph, trailing LineBreak] で 2 住人を期待 → 遅延では 1 住人）
   - `draw.rs:1626-1630 / 1708-1730`（trailing 改行であふれ発火する描画檻）
   - 判定基準: 「trailing 改行のみで空行/あふれ」を前提にした檻は**更新**、中間改行（後続グリフ有り）を検証する檻は**維持**。個々の要否は実変更後に落ちたテストを見て確定する。

7. **新規追加すべき決定論檻（R7.1/R7.3）**: metrics 非依存の構造テストで、
   - 「満杯付近＋trailing pending 改行だけではスクロール未発火」（R7.3 前段）／「次グリフで実体化後は従来どおりあふれ発火」（R7.3 後段）
   - 連続改行の ratio 累算（論点 1 の確定形）／reveal 途中の R4.2（カーソル改行通過・次可視無し → 保留）／`\c`・talk 終了での蒸発（R5）／縦書き 3 方向での同一規則（R6）
   - `FixedMetrics`（layout.rs のテスト用 metrics）で全網羅可能。GPU/DirectWrite 不要。

8. **R8 実機檻の所在と grep 設計（Unknown）**: 実機サインオフは `areka-emo-text` の unit ではなく上位バイナリ（`areka` bin の smoke／`emo2_real_run` 系）で行う想定。A→B 切替トークで「現象消失」を決定論判定するための**ログ marker とその grep 条件**、および出力画像の AI vision 目視手順を design/tasks で具体化する必要（`areka-real-machine-signoff-bounded-auto-exit` の定石に接続）。emo2 fixture が A→B→A の段落区切りを再現する台本を持つか、専用スクリプトを注入するかは要調査（**Research Needed**）。

---

## 5. 影響範囲・工数・リスク

### 5.1 変更対象ファイル（見込み）

| ファイル | 本番 | テスト | 備考 |
|---|---|---|---|
| `layout.rs` | **改訂**（`layout` 走査ループ・pending 導入） | 更新＋新規 | 唯一の本番行送り分岐 |
| `canvas.rs` | 非改変（見込み） | 更新（1:1 前提檻の値更新） | `from_layout` は行を写すのみ |
| `draw.rs` | 非改変（見込み） | 更新（trailing 改行あふれ檻） | `first_visible_line` 消費のみ |
| `state.rs` | 非改変（見込み） | 影響小 | pending は layout ローカル・Clear/ClearAll は既に全消去 |

### 5.2 工数・リスク

- **Effort: S（1–3 日）**。本番変更は 1 関数・約 10 行。大半はテスト檻の更新と新規決定論檻の追加。純関数で GPU 不要ゆえ回帰檻を素早く固められる。
- **Risk: Low〜Medium**。
  - Low 要因: 変更が純粋層 1 関数に閉じる・下流本番非改変・決定論全網羅可能・brief 非陳腐化。
  - Medium 要因: §4 論点 1（連続改行の実体化形）と論点 2/4（先頭改行・`opened`）の正準形確定が視覚結果を左右する。ここを取り違えると段落区切り or 連続改行の見えが SSP と乖離する。R8 実機 grep 設計は Unknown（設計で具体化要）。

---

## 6. Research Needed（design フェーズへ）

- **[R-1]** SSP 実機での**連続改行**（`\n\n` 等）の見え方（中間空行を作るか・単一ギャップか）。要件は単一累算を支持するが、実機観測で裏取りできると論点 1/2 が確定する（brief が任意追観測を許容）。
- **[R-2]** emo2 fixture / pasta SHIORI で A→B→A の段落区切りと A→B 単純切替を**決定論的に再現する台本**の有無、および R8 実機檻の grep marker 設計（`AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit＋`RUST_LOG` grep＋vision 目視の具体手順）。
- **[R-3]** 完了済み `areka-P0-emo-text-layer` の R2.2／R7 系檻の全リスト（更新対象の網羅確認・本 gap では layout/canvas/draw の該当檻を特定済みだが、design で漏れなく棚卸し）。

---

## 7. まとめ（設計フェーズ推奨）

- **推奨アプローチ**: Option A（`LayoutEngine::layout` 走査内のローカル pending・ratio 累算・次可視グリフ配置時フラッシュ・trailing 蒸発）。本番は 1 関数に閉じ、下流（visible_window/canvas/draw）は非改変で自動的に正しくなる。
- **確定すべき鍵**: §4 論点 1（連続改行の単一累算 vs 中間空行）・論点 3（break/flush 順序＝R4.2 の要）・論点 4（`opened` 再定義）。これらが視覚正準を決める。
- **検証戦略**: 純粋 layout の判断分岐を `FixedMetrics` で決定論全網羅（R7）。既存 immediate 前提檻は「意味の変更に伴う更新」（陳腐化除外ではない）。実機 R8 は有界 auto-exit＋grep＋vision の定石で A→B 現象消失・A→B→A 段落維持を確認。
- 次アクション: 要件ディスカッションで §4 の設計判断項目（特に論点 1）を詰め、`/kiro-design areka-P0-newline-defer` で設計へ。

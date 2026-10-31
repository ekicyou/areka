# ギャップ分析: choice-ranges-one-function

> 2026-10-31 作成。基準は main `da020c10` の上の作業枝（`d93f2426`）。要件と brief は確定済みのものを読んだ。結論ではなく、設計で決める材料と案を並べる。

## 1. 分析の要約

- **今の形**: 本番の並び（範囲の割り当て `annotate_lines` → 帯 `line_bands` → 強調の描画 `decorate_canvas` → 描画 `render_styled` → 描き替えがあったときだけ当たり `derive_hit_rows`）は `actor_present.rs` の `present_actor`（316・325・336・347・367 行目）にそのまま書かれている。3 つの関数は `choice.rs` の公開の純粋な関数で、検査の 2 か所（`actor_lookahead_shapes_tests.rs` 183〜185 行目・`choice_anchor_tests.rs` 131〜133 行目）が同じ 3 つを自分で並べ直している。brief の記述と食い違いは無い。
- **足りないもの**: 「行の列 → 範囲・帯・当たり」を 1 回で出す本番の関数が無い。もう 1 つ、`choice_anchor_tests.rs` は `choice.rs` の子の検査なので、`actor.rs` の中の非公開の子 `present`（＝`actor_present.rs`）の関数には今のままでは手が届かない。**`actor.rs` に 1〜2 行の再輸出（か `mod present` の公開範囲の変更）が要る**（要件 5.6 の「理由と範囲を明記」に当たる）。`actor_lookahead_shapes_tests.rs` は `actor` の子なので `actor.rs` を変えずに呼べる。
- **案**: まとめた関数を「毎フレーム 3 つとも出す 1 つの関数（当たりは描き替えたときだけ使う）」にする案 A と、「範囲と帯を出し、同じ行の列を抱えたまま当たりを後で出す 2 段の形」にする案 B が主な候補。済んだ選択肢とホバーの絞りは、関数の外（今の呼び手）に残す案と、済んだ選択肢の絞りだけ中へ入れる案がある。
- **調べが要る点**: 要件 4.2「帯を強調の側とクリックの側で別々に出す変異」は、強調の側の帯だけを崩した場合、決まった字幅の 2 か所の検査では見えない（2 か所とも強調の帯を判定していない）。どの変異を入れるか・赤になる検査をどこに頼るかを設計で決める必要がある。
- **規模と危うさ**: 規模 S（3〜5 タスク）。危うさは低い（既存の純粋な関数を束ねるだけ・新しい依存も新しいファイルも無い）。中くらいに上がり得るのは、上の変異の見え方と `actor.rs` の再輸出の 2 点だけ。

## 2. 今のコードの調べ

### 2.1 本番の並び（`crates/areka-emo-text/src/actor_present.rs`・555 行）

| 段 | 行 | 呼び出し | 入力 |
|---|---|---|---|
| 行の列 | 279 | `arrange_lines(...)` | 状態・配置の入力・`render.metrics`（DWrite の実測の字幅） |
| 読み返しの窓 | 301 | `window_for(&lines, ...)` | 並びの前で決まる（範囲には効かない） |
| 範囲 | 316 | `annotate_lines(&lines, spans)` | `spans = actor_state.choices()`（済んだ選択肢も含む全部） |
| 帯 | 325 | `line_bands(&lines, resolved.mode, &render.metrics)` | |
| ホバーの絞り | 328〜333 | 済んだ選択肢を指すホバーを `None` に | `runtime.choice_hover`・`is_settled_ordinal` |
| 強調の描画 | 336 | `decorate_canvas(canvas, &segments, hover, ..., &bands)` | |
| 描画 | 347 | `render_styled(...)` → `changed` | GPU |
| 当たり（描き替えたときだけ） | 367 | `derive_hit_rows(&lines, &segments, mode, &region, &bands)` | `region` は描画の前から決まっている |
| 済んだ選択肢の絞り＋窓の上の矩形 | 369〜393 | `spans` から通し番号で引き、済んだ選択肢を外し、`to_window_physical`（`committed` は描画の後に決まる）で `ChoiceHitRow` を作る | |
| 箱の字の矩形 | 397〜407 | `glyph_cells(&lines, &bands, ...)` | 箱だけ |

- `derive_hit_rows` の入力（`lines`・`segments`・`mode`・`region`・`bands`）は**すべて描画の前に揃っている**。描画の後でしか決まらないのは `committed`（面に映したスクロール）で、これは窓の上の矩形への写し（`to_window_physical`）と箱の字の矩形（`glyph_cells`）だけが使う。つまり当たりの行（窓の上の写しの前の形）は描画の前に出せる。
- `bands` は強調・当たり・箱の字の矩形の 3 か所へ配られている。

### 2.2 3 つの関数（`crates/areka-emo-text/src/choice.rs`・790 行・触らない）

- `annotate_lines(lines, spans) -> Vec<LineChoiceSegment>`
- `line_bands<M: GlyphMetrics + ?Sized>(lines, mode, metrics) -> Vec<LineBand>`
- `derive_hit_rows(lines, segments, mode, region, bands) -> Vec<CanvasHitRow>`
- どれも同じ入力なら同じ出力・失敗なし。束ねるのに `choice.rs` を変える必要は無い（公開の型と関数だけで組める）。

### 2.3 検査の 2 か所

| 場所 | 置かれ方 | 行の列の得方 | 判定の中身 |
|---|---|---|---|
| `actor_lookahead_shapes_tests.rs` 164〜227 行目 `choice_highlight_and_click_ranges_come_from_the_judged_rows` | `actor.rs` の子（`#[cfg(test)] mod lookahead_shapes_tests`） | `rig.lines()` → `arrange_for_test(place, &FixedMetrics, t)` | 強調の範囲が覆う字＝見えている選択肢の字・当たりの行内の区間＝強調の区間（**ブロック軸の帯は比べていない**） |
| `choice_anchor_tests.rs` 96〜139 行目 `hit` / `hit_in` | `choice.rs` の子（`#[cfg(test)] mod anchor_tests`・`use super::*`） | `LayoutEngine::layout(...)` を直接（実行時の仕組みも `arrange_for_test` も使わない） | 範囲の区間・当たりの矩形を**上下左右とも数で**比べる（帯の丈 12・寄せ 1、縦書きの寄せ ±2 など） |

### 2.4 見える範囲（公開の範囲）の向き

- `actor_present.rs` は `actor.rs` の `mod present;`（非公開）として繋がる。中の関数を `pub(super)` にすれば `crate::actor` とその子孫から見える。
  - `actor_lookahead_shapes_tests`（`actor` の子孫）→ **`actor.rs` を変えずに `super::present::まとめた関数` で呼べる**。
  - `choice_anchor_tests`（`choice` の子）→ `present` が非公開なので、関数の公開の範囲を上げても**届かない**。`actor.rs` に `#[cfg(test)] pub(crate) use present::まとめた関数;` のような再輸出を 1 行足すか、`mod present` を `pub(crate) mod present` にする必要がある。
- 層の向き: `choice_anchor_tests.rs` は純粋な層（`choice`）の子の検査で、そこから結線の層（`actor`）の関数を呼ぶことになる。本番の依存の向きは変わらない（検査のビルドだけ）。`lib.rs` の純粋な層の走査（`PURE_SOURCES`・母数 81）は `windows` の語を探すだけで、`actor_present.rs` も `choice_anchor_tests.rs` も既に走査面に載っている。`crate::actor::...` を書いても走査は赤にならない。ファイルを足さないので一覧の母数も変わらない。

### 2.5 ほかにも 3 つを並べ直している所（要件の範囲の外）

要件 3.3 が名指しするのは 2 か所だけだが、同じ並べ直しはほかにもある:

- `layout_cluster_tests.rs` 307〜315 行目（絵文字の字の塊の当たりの幅）
- `viewbox_draw_vertical_band_readback_tests.rs` 133〜196 行目・`viewbox_draw_vertical_band_scroll_tests.rs` 106〜108 行目（GPU の読み戻し）
- `tests/staysee_balloon_fixture/script.rs` 354〜392 行目（crate の外の結合検査＝公開の口しか見えない。`actor_present.rs` の関数は公開しない限り呼べない）
- `choice_tests.rs`・`choice_decorate_tests.rs`・`choice_glyph_cells_tests.rs`・`layout_styled_tests.rs` は個々の関数の単体の檻で、並べ直しが目的ではない。

これらは今回の範囲の外のまま残る。残したときは「本番の並びが変わっても気付けない」穴がこれらには残る（特に `layout_cluster_tests` と縦書きの帯の 2 本）。

### 2.6 決まりの番人

- 1 ファイル 1,000 行の番人は `crates/*/tests/file_length_guard_test.rs`（`structure.md` 412 行目）。今の行数: `actor_present.rs` 555・`actor.rs` 825・`actor_lookahead_shapes_tests.rs` 421・`choice_anchor_tests.rs` 340。まとめた関数（数十行）を足しても、検査の並べ直しを消すので、どれも 1,000 行に届かない。
- `arrange_for_test`（`actor.rs` 608〜624 行目）はそのまま使える（要件 5.5）。`choice_anchor_tests.rs` はそもそも `arrange_for_test` を使っていない。

### 2.7 今ある本番経路の GPU の檻

`actor_choice_contract_tests.rs` に `present_frame` を本当に回す檻がある（`present_populates_choice_hit_rows_and_nochange_preserves_snapshot`・`a_decorated_choice_gets_a_taller_hit_band_through_the_production_frame`・ホバーの帯の読み戻し数本）。描き替えの無いフレームで当たりの写しが変わらないこと（要件 1.5）は前者が既に見ている。強調の帯の位置は読み戻しの檻が見ている。

## 3. 要件と今のコードの対応

| 要件 | 今あるもの | 差 |
|---|---|---|
| 1.1 まとめた関数 | 3 つの関数が別々 | **足りない**（新しい関数 1 つ） |
| 1.2 字幅を受け GPU 無しで呼べる | 3 つとも字幅を引数で受ける純粋な関数 | 束ねるだけで満たせる |
| 1.3 強調は関数の結果から | 316・325 行目で直接呼ぶ | 呼び替え |
| 1.4 当たりは同じ行の列・範囲・帯から関数を通して | 367 行目で直接呼ぶ | 呼び替え（形は案による） |
| 1.5 描き替えが無いフレームは当たりを作り直さない | `if changed` の中だけで写しを更新 | 案 A でも案 B でも保てる（写しを入れ替えるのは `changed` のときだけ） |
| 1.6 別の手順の範囲・帯を使わない | 箱の字の矩形 `glyph_cells` も同じ `bands` を使う | `glyph_cells` へ渡す帯もまとめた関数の結果から取る必要がある（**見落としやすい**） |
| 2.1〜2.5 見た目と当たりを変えない | 既存の檻 | 束ねるだけなら値は変わらない |
| 2.2 済んだ選択肢 | 369〜377 行目の絞り（呼び手） | 置き場所は**設計で決める** |
| 2.3 ホバー | 328〜333 行目の絞り（呼び手） | 置き場所は**設計で決める** |
| 3.1〜3.4 検査が関数を呼ぶ | 2 か所が並べ直し | 呼び替え（判定の中身はそのまま） |
| 3.3 `choice_anchor_tests.rs` | `choice` の子 | **制約**: `actor.rs` の再輸出が要る |
| 3.5 検査用の読み口は検査のビルドだけ | `arrange_for_test` が `#[cfg(test)]` の型 | 再輸出を `#[cfg(test)]` にすれば合う |
| 4.1〜4.3 変異で赤 | なし | 確かめの作業（下の 5 節） |
| 4.4 記録を残す | — | 置き場所（タスクの実装記録か本ファイル）を決める |
| 5.1〜5.4 | — | 守れる |
| 5.6 `actor.rs` を変えるなら理由と範囲 | — | **当たる**（2.4 節） |

## 4. 実装の案

### 案 A: 毎フレーム 3 つとも出す関数 1 つ（`actor_present.rs` に置く）

- 形の例: `pub(super) fn choice_ranges(lines, spans, mode, region, metrics) -> ChoiceRanges { segments, bands, hit_rows }`（`ChoiceRanges` は `actor_present.rs` の中の小さな構造体か、3 つ組）。
- `present_actor` は描画の前に 1 回呼び、`segments`・`bands` を強調の描画と箱の字の矩形へ、`hit_rows` を `if changed` の中で窓の上の写しへ使う。
- ✅ 一番短い。関数の中に並びが 1 か所だけ書かれ、当たりを別の行の列で作る道が構造の上で無くなる。
- ✅ 2 か所の検査は 1 行の呼び出しになる。
- ❌ 描き替えの無いフレームでも当たりの行を計算する（選択肢の数×行の数の小さな計算。選択肢が無ければ空の列で確保も無い）。要件 1.5 の「作り直さず」を「写しを入れ替えない」と読めば満たす。「計算もしない」と読むなら合わない——**読み方を決める必要がある**。

### 案 B: 2 段の形（範囲と帯を出し、同じ行の列を抱えて当たりを後で出す）

- 形の例: `fn choice_ranges<'a>(lines: &'a [PositionedLine], spans, mode, metrics) -> ChoiceRanges<'a>`（行の列・範囲・帯を持つ）と、その上の `fn hit_rows(&self, region) -> Vec<CanvasHitRow>`。
- ✅ 描き替えの無いフレームは当たりを計算しない（要件 1.5 を字面どおり満たす）。
- ✅ 当たりは構造体が抱える同じ行の列・範囲・帯からしか作れない。
- ❌ 関数とその上の手続きの 2 つになる（要件 1.1「関数 1 つ」をどう読むか）。寿命の引数が付き、`present_actor` の中の借用（`render` を可変で借りたまま `lines` を抱える）に気を配る必要がある。
- ❌ 案 A より数行長い。

### 案 C: 案 A に済んだ選択肢の絞りを入れる

- 関数が `spans` の代わりに場所の状態（`ActorTextState`）を受け、当たりの行から済んだ選択肢を外して返す。ホバーの絞りは強調の描画の入力なので呼び手に残す。
- ✅ 済んだ選択肢を当たりから外す規則も検査が本番の関数を通して見られる。
- ❌ 関数の受け取る物が増え、`choice_anchor_tests.rs` の `hit_in` は状態を渡す形に変わる（今は `place.choices()` を渡している。状態は手元にあるので渡せる）。
- ❌ `actor.rs` の `drop_settled_choices` も同じ絞りを持っているので、絞りが 2 か所に分かれたまま（今と同じ）。
- ❌ 当たりの写しの組み立て（通し番号で範囲を引いて ID・ラベル・参照を添える）は描画の後の `committed` が要るので、関数の外に残る。絞りだけ中へ入れると、外と中で同じ範囲の列を 2 回引くことになる。

### 案 D（参考・採らない理由の記録）: `choice.rs` に置く

- brief は `choice.rs` に置く案も挙げていたが、並走の約束（要件 5.1・5.2）で除外。`choice.rs` に置けば `choice_anchor_tests.rs` から `actor.rs` を変えずに呼べ、層の向きもきれいになる、という利点があったことだけ残す。

## 5. 変異で赤になるかの見立て（要件 4）

決まった字幅の 2 か所の検査に対する見立て（実際に入れて確かめるのは実装の段）:

| 変異 | 見立て | 赤になる検査 |
|---|---|---|
| 範囲へ空の行の列（か先頭を落とした列）を渡す | 赤 | 検査 8 の選択肢（覆う字が違う・最後の「ええと、」が無い）、アンカーの檻 |
| 帯へ空の行の列を渡す | 赤（帯が引けず当たりの行が出ない） | 検査 8 の選択肢（当たりと強調の区間が違う）、アンカーの檻 |
| 当たりへ空の行の列を渡す | 赤 | 同上 |
| 範囲を出す手順を抜く（空の列を返す） | 赤 | 同上 |
| 当たりを出す手順を抜く | 赤 | 同上 |
| 当たりの側だけ別の帯（寄せ 0 など）を使う | 赤 | アンカーの檻（上下の数を比べている）。検査 8 の選択肢は行内の区間しか比べないので**赤にならない** |
| **強調の側（呼び手へ返す帯）だけ別の帯にする** | **2 か所とも赤にならない見込み** | 2 か所とも強調の帯を判定していない。GPU の読み戻しの檻（`actor_choice_contract_tests.rs` のホバーの帯）なら見える見込みだが、決まった字幅の検査ではない |
| 同じ入力で帯を 2 回計算して別々に配る | 値が同じなのでどの検査も赤にならない | 純粋な関数なので観測できない。変異として意味を持たない |

要件 4.2 を満たすには、どちらの側を崩す変異にするか（当たりの側なら今の檻で赤）、強調の側を崩すなら GPU の檻に頼るか、決まった字幅の判定を足すか（要件 3.4・2.5 は「判定の中身を書き換えない」なので、足すなら新しい検査として足すことになる）を決める必要がある。

## 6. 規模と危うさ

- **規模: S**（3〜5 タスク）。まとめた関数・`present_actor` の呼び替え・検査 2 か所の呼び替え・`actor.rs` の再輸出・変異の確かめと記録。
- **危うさ: 低**。既存の純粋な関数を束ねるだけで、値の決め方は変えない。新しい依存・新しいファイル無し。気を付ける所は、箱の字の矩形へ渡す帯もまとめた関数の結果から取ること（要件 1.6）と、強調の側の変異の見え方（5 節）。

## 7. 設計で決めること（要件討議へ渡す）

1. **当たりの省き方**: 案 A（毎フレーム 3 つとも出し、写しの入れ替えだけ描き替えたときに限る）か、案 B（2 段の形で、描き替えの無いフレームは計算もしない）か。要件 1.5 の「作り直さず」をどちらの意味で読むか。
2. **`actor.rs` の変更の形**（要件 5.6）: `choice_anchor_tests.rs` から呼ぶために、`#[cfg(test)] pub(crate) use present::まとめた関数;` を 1 行足すか、`mod present` の公開範囲を上げるか。前者なら本番のビルドに何も増えない（要件 3.5 と合う）。
3. **済んだ選択肢の絞りの置き場**: 呼び手に残す（今のまま）か、まとめた関数へ入れる（案 C）か。ホバーの絞りは強調の描画の入力なので、呼び手に残すのが自然という見立て。
4. **要件 4.2 の変異の中身**: 当たりの側の帯だけを崩す変異にする（今のアンカーの檻で赤）か、強調の側も崩して確かめる（決まった字幅の 2 か所では赤にならない見込み＝GPU の檻に頼るか、新しい検査を足すか）か。
5. **箱の字の矩形へ渡す帯**: `glyph_cells` の帯もまとめた関数の結果から取る（要件 1.6 の読み）ことの確認。
6. **ほかの並べ直し**（2.5 節）を範囲の外のまま残すか。残すなら「残した穴」として記録だけするか、後の spec へ回すか。結合検査（`tests/staysee_balloon_fixture`）は公開の口しか見えないので、呼ぶならまとめた関数を公開する必要がある。
7. **変異の確かめの記録の置き場**（要件 4.4）: タスクの実装記録（`tasks.md` の実装メモ）か、本ファイルの追記か。
8. **まとめた関数が返す形**: 3 つ組か、`actor_present.rs` の中の小さな構造体か（名前も含む）。

### 7.1 要件討議での整理（2026-10-31）

- 1 は brief の「`changed` のときだけクリックの範囲を作る今の省き方は保つ」で読みが決まる（要件 1.5 を「計算し直さず」に直した）＝案 A は外れ、2 段の形か遅延かを設計で選ぶ。
- 5 は要件 1.6 に箱の字の矩形を足して決着（設計の議題から外す）。
- 4 と 6 は要件討議の議題に回し、推奨案で確定した: 4 は決まった字幅の新しい検査を 1 本足し、強調へ渡す帯と当たりの矩形のブロック軸が同じ帯から出ていることを判定する（要件 3.6・4.2）。6 は範囲の外のまま残し起票しない（要件の境界の節）。
- 2・3・7・8 は設計で決める。

## 8. 調べが要る点（設計の段へ持ち越す）

- 強調の側の帯を崩したとき、`actor_choice_contract_tests.rs` の GPU の読み戻しの檻が本当に赤になるか（実 GPU が要る。決まった字幅だけで確かめるという brief の前提と合うか）。
- 案 B を採るとき、`present_actor` の中で `render`（`runtime.surfaces` の可変の借用）・`actor_state`（`runtime.state` の借用）・行の列を抱えた構造体が同時に生きる借用が素直に通るか（フィールドが別なので通る見込み）。

## 9. 設計の段の決め（2026-10-31・`/kiro-spec-design -y`）

- **調べの種類**: 軽い調べ（既存の仕組みの延長）。外部の依存・新しい技術が無いので、外の調べはしていない。使った手引きは `kiro-spec-design` の規則（設計の原則・軽い調べ・まとめの 3 つの見方・設計の関門）だけ。
- **調べで確かめたこと**:
  - `present_actor` の `resolved` は `runtime.layout_input` から `cloned()` した手元の値、`lines` も手元の値。`render.metrics` は呼び出しの間だけ借りればよい。行の列と描画範囲を借りる構造体を描画の後まで持っても、`render`（可変の借用）・`runtime.choice_snapshot` の書き込みと借用は重ならない（8 節の 2 点目は解けた）。
  - 再輸出は元の公開の範囲より広くできないので、関数は `pub(crate)` にする。`present` が非公開の子なので、本番のビルドで届く範囲は今と同じ `actor` の中だけ。

### 9.1 決め

| 議題（7 節） | 決め | 理由 |
|---|---|---|
| 1 当たりの省き方 | 2 段の形（`choice_ranges` が範囲と帯を出して `ChoiceRanges` に持ち、`hit_rows(&self)` が描き替えたときだけ当たりを出す） | 遅延（`OnceCell` など）は内側の書き換えを足すだけで同じ効き目。2 段の形の方が短く、`if changed` の中で計算することが読んで分かる。`hit_rows` が引数を取らないので、別の行の列や帯で当たりを作る道が書けない |
| 2 `actor.rs` の変更の形 | `#[cfg(test)] pub(crate) use present::choice_ranges;` の 2 行だけ | `choice_anchor_tests.rs` は `choice` の子で、非公開の `present` に届かない。`choice.rs` は触れない。`#[cfg(test)]` なら本番のビルドに何も増えない（要件 3.5）。型 `ChoiceRanges` は名前を書かずに使えるので再輸出しない |
| 3 済んだ選択肢の絞りの置き場 | 今のまま呼び手（`present_actor`）に残す。ホバーの絞りも同じ | 写しの組み立てに描画の後の `committed` が要り、絞りだけ中へ入れると範囲の列を 2 回引くことになる。絞りは範囲・帯・当たりの矩形を作らない（選ぶだけ）ので要件 1.6 に当たらない |
| 7 変異の記録の置き場 | `tasks.md` の変異を確かめるタスクの実装の記録 | 本ファイルは設計の前の調べの記録。実装で確かめた事実は実装の記録に置く |
| 8 返す形と名前 | 関数 `choice_ranges`・型 `ChoiceRanges<'a>`（非公開の欄 `lines`・`region`・`mode`、`pub(crate)` の欄 `segments`・`bands`）・手続き `hit_rows` | 3 つ組だと当たりを出すときに呼び手が行の列と帯をもう一度渡すことになり、並びが呼び手へ漏れる |

### 9.2 まとめの 3 つの見方

- **一般化**: 後の spec が足す範囲の種類（`range-choice-tag` など）は、`choice_ranges` の中に足せばよい形になる。今は種類を足さない。
- **作るか使うか**: 既存の 3 つの関数をそのまま束ねる。新しい計算は無い。
- **削ったもの**: 毎フレーム 3 つとも出す形（要件 1.5 で除外）・遅延の形・型の再輸出・`mod present` の公開の範囲を上げる案・済んだ選択肢の絞りを中へ入れる案（7 節の案 C）。

### 9.3 要件 4.2 の変異と新しい検査

- 強調の側だけを崩す変異（返す `bands` の寄せをずらし、`hit_rows` は崩す前の帯を使う）は、既存の 2 か所では赤にならない（5 節）。新しい検査 `highlight_bands_and_hit_rows_share_one_band_in_every_writing_mode`（`choice_anchor_tests.rs`・3 つの書字方向）が、`ranges.bands` と `hit_rows()` の矩形のブロック軸を比べて決まった字幅のまま赤にする。「強調へ渡す帯＝`ranges.bands`」は `present_actor` がそれをそのまま `decorate_canvas` へ渡す字面で保つ。
- 変異の一覧（V1〜V7）と赤になる見込みは `design.md` の「変異の確かめ」。本当に赤になった検査の名前は実装の記録に残す。

### 9.4 残る危うさ

- 新しい検査の判定は `derive_hit_rows` の帯の当て方（近い辺＋寄せ−原点）を言い直している。`choice.rs` の当て方が後で変わればこの検査も直す要がある（そのときは帯の共有の意味が変わるので、直すのが正しい）。

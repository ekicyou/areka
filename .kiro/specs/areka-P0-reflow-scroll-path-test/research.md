# ギャップ分析: areka-P0-reflow-scroll-path-test

> 2026-10-16・`/kiro-validate-gap`。対象は確定済みの requirements.md（要件 1〜5）。読んだのはコードだけ（テストは回していない）。

## 1. まとめ

- **足りないのは検査 1 本と、その検査のための小さな支え 1 つだけ**。本番の手順（区切り → 配置＝`arrange_lines`、行の列 → 見える行の範囲＝`LayoutEngine::visible_window`）は、どちらも今のまま検査から呼べる。本番のコードを変える必要は無い（要件 5.1 と両立する）。
- 今ある支え（`actor_lookahead_tests.rs` の `Rig`・`Talk`・`walk`・`walk_with`・`reveal_violations`・`budoux_model`）は**行の列だけ**を段階ごとに残す。見える行の範囲（`VisibleWindow`）を段階ごとに残す口は無い。ここが唯一の欠け。
- 欄の高さ（2 行だけ入る）・3 行以上になる台本・「届いた字だけで区切ると表示済みの字が動く」形は、どれも今ある検査に無い。台本の区切りの結果は BudouX の模型の出力に依るので、**台本は回して確かめるまで決まらない**（下の「調べが要るもの」）。
- 規模は S（2〜3 タスク）、危うさは低い。触るファイルは `crates/areka-emo-text/src/actor_lookahead_tests.rs`（今 630 行）の 1 本だけで済む見込み。

## 2. 今あるもの

### 2.1 本番の経路（変えない）

| 手順 | 場所 | 中身 |
|---|---|---|
| 提示の入口 | `crates/areka-emo-text/src/actor_present.rs` の `present_actor`（109 行目〜） | `runtime.layout_input.get(place).cloned()`（119 行目）で配置の入力（`ResolvedBalloonText`）を引く |
| 区切り → 配置 | 同 `arrange_lines`（369〜411 行目） | 見える数を出し、`WrapMode::BudouxWordWrap` なら `lookahead.basis(place, arrived)` に聞いて、`Basis::Full` なら全文の字の列と区切りで、`Basis::Arrived` なら届いた字で `segment_plan` して `LayoutEngine::layout_styled` を呼ぶ |
| 行の列 → 見える行の範囲 | 同 259 行目 `LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode)` | 本体は `crates/areka-emo-text/src/layout.rs` 403〜457 行目。行の列・描画範囲・書字方向だけを入力にとる純関数。答えは `VisibleWindow { first_visible_line, block_offset }`（`Clone, Copy, Debug, PartialEq`） |
| 全文の枝の分かれ目（変異の的） | `crates/areka-emo-text/src/lookahead.rs` の `TalkLookahead::basis`（283〜323 行目） | 区間の全文があり、届いた内容がその先頭と一致すれば `Basis::Full`、無い・食い違うなら warn を 1 度出して `Basis::Arrived` |

- 本番の提示は `arrange_lines` と `visible_window` を**別々の 2 回の呼び出し**で続けて呼んでいる。両方をまとめて返す本番の関数は無い。
- `visible_window` のあふれの判定は「最後の行の下端 > 描画範囲の下端」（横書き）。行の矩形の丈は `font_height`（`layout.rs` の「行矩形の規約」）、行送りは `font_height + 2`（`FixedMetrics::line_pitch` → `TextLayerConfig::line_pitch`）。
- `validrect` を書かないバルーンの描画範囲は画像の全域（`region.rs` 260 行目〜の `TextRegion::resolve`。下端＝画像の高さ）。

### 2.2 検査の支え（`actor_lookahead_tests.rs`・そのまま使う）

| 支え | 使える形 |
|---|---|
| `budoux_model()` | `budoux_newline,1`・原点 (0,0)・`validrect`／`wordwrappoint` なし・字の丈 20 |
| `Rig::new` / `attach` / `play` | 本物の `TextLayerRuntime` と `spawn_emo_text` の受け口。`play` は parse → compile → 刻印 → `from_sheet` → `register_sink`（`Delivery::Staged` なら先渡しが届く・`WithoutPreview` なら先渡しを落とす） |
| `Rig::lines` / `rows` | `arrange_for_test`（`actor.rs` 516 行目・`#[cfg(test)]`・中身は `present::arrange_lines` を呼ぶだけ）に `FixedMetrics` を渡す＝本番と同じ手順の行の列 |
| `Rig.runtime` | `pub(super)`。検査のモジュールは `actor` の子なので、`TextLayerRuntime` の非公開の欄 `layout_input` も読める |
| `Talk::tick` / `Talk.times` | 合図の届く時刻と出終わる時刻の列（`pub(super)`） |
| `walk` / `walk_with` / `walk_injecting` | 段階ごとの `Stage { time, rows }` を返す。**Rig は関数の中で捨てるので、行の矩形も見える行の範囲も外へ出ない** |
| `reveal_violations` | (a) 前の段階で見えていた字が同じ行の同じ番目にある (b) 出終わったときのその字までと同じ、を判定し、違反の文を返す |
| `NINE_WIDE`・`EMO2_BOOT_TALK` | 幅 190（全角 9 字）・高さ 200（あふれない）。台本は 2 行で出終わる |

- 行の番号は配置の結果（送る前）の番号のまま `rows` に入る。見える行の範囲で切り取る前の列なので、**要件 2.2（行がまとめて上へ送られるのを「変わった」と数えない）は `rows` をそのまま比べれば自然に満たされる**。
- `Stage` は `actor_lookahead_shapes_tests.rs` の 29・257 行目で構造体の式（`Stage { time, rows }`）として作られている。`Stage` に欄を足すと、そのファイルも直すことになる。

### 2.3 今あるあふれの検査（範囲外・比べのため）

- `actor_scroll_retain_tests.rs` は `apply_cue` で合図を直に積む＝先渡しが無い＝`Basis::Arrived` の道筋だけを通る。実フォント・GPU・画素で判定する。全文の枝は踏まない。
- `layout_visible_window_tests.rs`・`layout_cursor_overflow_tests.rs` などは `visible_window` を純粋な行の列で検査する。経路（先渡し・合図の届き方）は通らない。

### 2.4 決まりごと（steering・roadmap）

- テストは兄弟のファイル・1 ファイル 1,000 行以下（`structure.md` 150・180 行目）。630 行に 100〜150 行足しても上限の内。
- 棚卸㉕のウェーブ C7 の約束: 「今ある `actor_lookahead_tests.rs` へ足すだけ・emo-text に新しいファイルを足さない・`actor.rs` と `lib.rs` に触らない」（`roadmap.md` 147 行目）。同じウェーブの `balloon-text-bugs` は `actor_present.rs`・`region.rs`（件 1 の形によっては `actor.rs`）を触る。
- 変異の一時の物はワークツリーの `target\` の下だけ。git の stash は他のセッションと共有なので使わない。

## 3. 要件と今あるものの対応

| 要件 | 今あるもの | 欠け・制約 |
|---|---|---|
| 1.1 先渡しありで本番の経路を流し、段階ごとに行の割り当てと見える行の範囲を取る | `Rig`・`Talk`・`Delivery::Staged` | **Missing**: 段階ごとの見える行の範囲を残す口が無い（`walk*` は行の列だけを返し、Rig を捨てる） |
| 1.2 3 行以上の台本・2 行だけ入る欄 | 欄の大きさは `attach` の `image` で選べる | **Missing**: 欄の大きさの定数と台本。高さは「行の下端 42 は入り、64 は入らない」＝42 以上 64 未満（字の丈 20・行送り 22 の場合）。**Research Needed**: 台本の出終わりの区切り |
| 1.3 届いた字だけで区切ると表示済みの字が動く形 | 検査 2 が `EMO2_BOOT_TALK` で同じ形を確かめた先例 | **Research Needed**: 3 行以上の台本でこの形になる字の並び。BudouX の模型の出力に依る |
| 1.4 本番の手順を呼ぶ（写しを作らない） | `arrange_for_test`（＝`arrange_lines`）・`LayoutEngine::visible_window`（`pub`） | **Constraint**: 2 つを続けて呼ぶ並びは本番の `present_actor` の中にしか無い。検査は同じ 2 つの関数を同じ順で呼ぶ（手順の中身を組み直すわけではない）。描画範囲と書字方向の出どころを `layout_input` にするか `ResolvedBalloonText::resolve` にするかは選べる |
| 1.5 一度もあふれなければ前提が崩れたとして失敗 | — | **Missing**: 段階のどこかで `first_visible_line > 0` を確かめる 1 行 |
| 1.6 決まった字幅・GPU なし・決定論 | `FixedMetrics`・`Rig` は GPU を使わない | 欠けなし |
| 2.1 段階ごとに (a)(b) を判定し失敗させる | `reveal_violations` | 欠けなし（そのまま呼んで `assert!`） |
| 2.2 送る前の行の番号で比べる | `rows` は送る前の列 | 欠けなし（見える範囲で切り取らないこと、を守るだけ） |
| 2.3 あふれの前後の段階の両方を判定に含める | `walk` は全段階を返す | **Missing**: 前後の段階の両方があることの確かめ（`first_visible_line == 0` の段階と `> 0` の段階が両方ある） |
| 3.1 出終わりの見える行の範囲が先渡しなしと同じ | `Delivery::WithoutPreview` | **Missing**: 先渡しなしの流れでも見える行の範囲を取る口（1.1 と同じ欠け） |
| 3.2 出終わりの行の割り当てが先渡しなしと同じ | 検査 3 が同じ比べ方の先例 | 欠けなし |
| 4.1 全文の枝を外す変異で赤 | 的は `TalkLookahead::basis`（`lookahead.rs` 283 行目） | **Constraint**: 変異は手で入れて戻す確かめで、コードには残らない。赤になるのは 1.3 の形が台本にあるとき（2.1 の (a)(b) が落ちる）。3.1・3.2 は変異でも変わらない（出終わりは全文＝届いた字なので） |
| 4.2 戻して時刻を新しくしてから緑 | 前例: 完了 `budoux-reveal-reflow` の tasks.md「変異を戻した後は touch して回し直す」 | **Constraint**: 戻し方と時刻の更新のしかた（下の判断 6） |
| 4.3 変異を残さない | — | 確かめの終わりに `git diff --stat` で本番のファイルに差が無いこと |
| 5.1 本番のコードを変えない | 2.1 の通り、今のまま呼べる | 欠けなし（`actor.rs` に検査用の読み口を足す案は約束にも反する） |
| 5.2 今ある検査の中身を変えない | — | **Constraint**: `Stage` に欄を足す・`walk_injecting` の戻り値を変える案は、`actor_lookahead_shapes_tests.rs` か今ある支えを書き換えることになる |

## 4. 作り方の案

### 案 A: 新しい小さな歩き方を足す（今ある支えは触らない）

- `actor_lookahead_tests.rs` の末尾に、`Rig`・`Talk` を直に使って段階ごとに「行の列」と「見える行の範囲」を残す関数（20〜30 行）と、検査 1 本（＋要るなら定数と台本）を足す。
- 見える行の範囲は、各段階で `rig.lines(&place, t)` の行の列を `LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode)` へ渡して得る。`resolved` は `rig.runtime.borrow().layout_input[&place]`（本番の提示が引くのと同じ値）か、`ResolvedBalloonText::resolve(model, image)`（`attach` が入れるのと同じ解決）から取る。
- 行の割り当ての判定は、残した行の列を `Stage { time, rows }` に直して `reveal_violations` に渡す。
- ✅ 今ある支え・`Stage`・兄弟のファイルに触れない（要件 5.2 と C7 の約束にいちばん素直）。
- ✅ 触るファイルは 1 本・足す行は 100〜150 行の見込み。
- ❌ `walk_injecting` の段階の回し方（`tick` → 行の列を取る）と同じ形の数行が 2 か所になる。

### 案 B: 今ある支えを広げる

- `walk_injecting` が段階ごとの見える行の範囲も返すようにする（`Stage` に `window` の欄を足す、または別の列を並べて返す）。
- ✅ 段階の回し方が 1 か所のまま。
- ❌ `Stage` に欄を足すと `actor_lookahead_shapes_tests.rs` の構造体の式 2 か所も直すことになり、「今ある検査の中身を書き換えない」（要件 5.2）と「今あるファイルへ足すだけ」に当たる。戻り値の形を変える案も、今の呼び手 7 本以上に手が入る。

### 案 C: `Rig` に読み口を 1 つ足し、検査は案 A の形

- `Rig` に `window(&place, t) -> VisibleWindow`（行の列を取って `visible_window` に渡す）を足し、新しい検査は `Rig`・`Talk` を直に回してこれを呼ぶ。`walk*` と `Stage` は変えない。
- ✅ 見える行の範囲の出し方が `Rig` の 1 か所にまとまる（後続の `balloon-scroll-fade` などが同じ支えを使える）。
- ❌ `Rig` は今ある支えなので「足すだけ」だが、触る場所が 1 つ増える。後続の使い手は今は無い（要るまで作らない、の読みもできる）。

### 比べ

| | 触るファイル | 今ある支えの変更 | 要件 5.2 |
|---|---|---|---|
| A | 1 本 | なし | 素直に満たす |
| B | 2 本 | あり | 当たる |
| C | 1 本 | `Rig` に足すだけ | 満たす（中身は変えない） |

## 5. 規模と危うさ

- **規模: S**（2〜3 タスク）。今ある支えと本番の 2 つの関数を呼ぶだけで、新しい仕組みは無い。
- **危うさ: 低**。触るのは検査のファイル 1 本。気を付ける点は次の 2 つ。
  - 台本の区切りが BudouX の模型に依る（下の調べ 1）。台本が「届いた字だけで区切ると動く」形になっていないと、変異で赤にならない（要件 4.1 が満たせない）。
  - 同じウェーブの `balloon-text-bugs` が `region.rs`・`actor_present.rs` を直す。描画範囲の決め方や提示の手順が変われば、この検査の欄の大きさの前提がずれうる。どちらが先に着地しても、後の側で検査を回し直せば分かる。

## 6. 調べが要るもの（Research Needed）

1. **台本と出終わりの区切り**: 幅 190（全角 9 字）の `budoux_model()` で、消去を挟まない 1 つの区間が 3 行以上になり、かつ合図が分かれて届くと届いた字だけの区切りでは表示済みの字が動く字の並び。`EMO2_BOOT_TALK` の「イイジャン！」＋「‥」「‥」「ええと、」と同じ仕掛け（短い合図が前の行に入ってしまい、後の字が届くと次の行へ移る）を 2 行目と 3 行目の境に置くのが候補。BudouX の模型の出力は回してみないと分からないので、設計か実装の最初に GPU なしの検査（`cargo test -p areka-emo-text --lib actor::lookahead_tests -j 2` の程度）で出終わりの行の列を確かめ、検査 1 の `EMO2_BOOT_FINAL` と同じく出終わりの区切りを定数で固定する。
2. **欄の高さの境**: 字の丈 20・行送り 22・行の矩形の丈 20 なら、2 行目の下端 42・3 行目の下端 64。高さ 42 以上 64 未満で「2 行だけ入る」。字の丈を変える・行間の既定（2）が変わると境も動くので、定数の注釈に式で書くか、検査の中で「2 行目までは送らない・3 行目で送る」を確かめる。
3. **変異で赤になる場所**: 変異（`basis` が常に `Basis::Arrived`）のとき、赤になるのが「あふれの前の段階だけ」か「あふれをまたぐ段階」か。1.3 の形を 2 行目と 3 行目の境（＝あふれが起きる所）に置けば、変異のときは見える行の範囲が送られる段階もずれる。どこで赤になれば「組み合わせに効いている」と言えるかは判断 3。

## 7. 設計で決めること（要件の討議へ渡す）

1. **見える行の範囲を段階ごとに残す支えの形**: 案 A（新しい小さな歩き方を足す）／案 B（今ある `walk_injecting`・`Stage` を広げる）／案 C（`Rig` に読み口を足す）。要件 5.2 と C7 の約束からは A か C。
2. **描画範囲と書字方向の出どころ**: 実行時の `layout_input`（本番の提示が引くのと同じ値・非公開の欄を検査の子モジュールから読む）か、`ResolvedBalloonText::resolve(model, image)`（`attach` が入れるのと同じ解決をもう 1 度呼ぶ）か。前者のほうが「本番と同じものを使う」（要件 1.4）に近い。
3. **変異の赤の出どころ**: 要件 4.1 は「赤になる」だけを求める。今ある検査 1 も同じ変異で赤になるので、新しい検査が赤になるだけでは「あふれとの組み合わせに効いている」証にならない恐れがある。字が動く形をあふれの境（2 行目と 3 行目の間）に置き、変異のときに「あふれの後の段階」でも違反が出ることまで確かめるか。
4. **1.3 の形の常設の確かめ**: 検査 2 と同じく、同じ台本を `Delivery::WithoutPreview`（先渡しを落とす＝変異と同じく `Basis::Arrived` を通る）で流して `reveal_violations` が違反を返すことを、新しい検査の中で毎回確かめるか。確かめれば、台本が後で変わっても「変異で赤になる形」が崩れたことに気付ける（変異の手作業を毎回せずに済む）。
5. **前提の確かめの範囲（要件 1.5・2.3）**: 「どこかの段階で `first_visible_line > 0`」だけにするか、「`== 0` の段階と `> 0` の段階が両方ある」「出終わりの行の数が 3 以上」「出終わりの区切りが定数と同じ」まで足すか。
6. **変異の入れ方と戻し方（要件 4.2・4.3）**: `lookahead.rs` の `basis` の頭に `return Basis::Arrived;` を入れる形か、`arrange_lines` の `match` の腕を差し替える形か（どちらも「常に届いた字で区切る」。前者は warn も出なくなる）。戻しは `git checkout -- <file>`（時刻も新しくなる）か、`target\` の下に取った写しから戻して時刻を更新するか。確かめの記録（赤の出力・戻した後の緑・`git diff --stat` が空）をどこに残すか。
7. **先渡しなしの側の warn**: 先渡しなしで流すと `basis` が warn「先渡しが無い」を 1 件出す。検査 3 のように捕まえずに流すか、検査 2 のように `capture` で包んで件数も確かめるか（要件にはない。出力が静かになるだけの違い）。

> 2026-10-16 要件の討議で確定: 判断 3・4 はどちらも取る（要件 1.7＝先渡しなしの対照を毎回・要件 4.4＝変異の赤にスクロールの後の段階の違反を含める）。判断 5 の「前後の段階が両方ある」は要件 2.4 に入れた。

## 8. 次の段

- 要件の討議（`/kiro-requirements-discussion`）で 7 の判断を決め、`/kiro-design areka-P0-reflow-scroll-path-test` へ進む。
- 設計の最初に 6-1 の台本を GPU なしで回して、出終わりの区切りを確かめる。

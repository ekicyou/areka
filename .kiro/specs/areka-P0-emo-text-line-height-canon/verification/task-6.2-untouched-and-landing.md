# タスク 6.2 の証跡——触っていないことの確認と着地条件（要件 9.1／9.2／9.3／9.5／9.6）

実施日: 2026-09-06 / ブランチ `claude/areka-p0-emo-text-line-height-b69d6c`（HEAD `49ece2fc`）
比較の基準: `main` との分岐点 `36d1c323`（`git merge-base main HEAD` の実測値。以下 `main...HEAD` はこの点からの差分）

この文書はタスク 6.2 の 3 つの確認——⑴ 触らないと約束した資産の差分が 0 件であること、
⑵ 意味論（あふれ判定・`\_l`・`\c` の全消去・比率つき改行・reveal のペース）に手が入っていないこと、
⑶ 対象 6 ファイルの着地行数とコミット列の順序——の実測を 1 か所にまとめたものです。
このタスクではソースを 1 行も変えていません（作業ツリーは `git status --porcelain` が空・本文書の新設のみ）。

---

## 1. 触らないと約束した資産の差分（要件 9.3）

`git diff --stat main...HEAD -- <パス>` を経路ごとに実行し、**すべて出力なし・終了コード 0** でした。

| # | 対象 | 実行したパス | 結果 |
|---|---|---|---|
| 1 | バルーン fixture（要件 9.3 が名指し） | `crates/pilot/examples/shiori-host-32/fixtures/` | 出力なし・0 |
| 2 | kanade | `crates/areka-kanade` | 出力なし・0 |
| 3 | pasta（`vendors/pasta` サブモジュール＝`pasta_core` の実体） | `vendors/` | 出力なし・0 |
| 4 | pasta 辞書ほか全経路（横断検索） | `'*pasta*'` | 出力なし・0 |
| 5 | sakura（parser／compile） | `crates/areka-sakura` | 出力なし・0 |
| 6 | parsers | `crates/areka-parsers` | 出力なし・0 |
| 7 | 本体クレート | `crates/areka` | 出力なし・0 |
| 8 | pilot 全体 | `crates/pilot` | 出力なし・0 |
| 9 | ゴースト層 | `crates/areka-ghost` | 出力なし・0 |
| 10 | 依存の宣言 | `Cargo.toml`（追跡 1 件）。`Cargo.lock` は `.gitignore` で無視され追跡 0 件＝この半分は反証不能（レビュー所見・2026-09-06） | 出力なし・0 |

### 1.1 正直に分類しておく 2 件（差分が 0 であることの意味）

- **`crates/areka-parsers`**: タスク 4.1 は「`BalloonModel` にバルーン名の取得口が無い」ことを実走で見つけましたが、
  parser を改変して口を足すことは**しませんでした**（台帳 §7 #10）。欄は定数
  `BALLOON_NAME_PLACEHOLDER = "(名前なし)"` で埋め、引受先を `areka-P0-ukadoc-survey-assets` の brief へ登記しています。
  差分 0 件はその判断どおりの結果です。
- **`crates/areka-sakura`**: タスク 3.4 は観測用 example が `text_playback_duration` を呼ぶように直しましたが、
  変えたのは**呼ぶ側**（`crates/areka-emo-text/examples/emo-text-layer.rs:118` の `use` と
  `examples/emo-text-layer/scenario.rs:151` の呼び出し）だけで、
  定義側（`crates/areka-sakura/src/duration.rs` の `CHAR_NOMINAL_MS` と `text_playback_duration`）は無改変です。

### 1.2 本ブランチが触った経路の全数（横断確認）

`git diff --name-status main...HEAD` は **66 件**を返し、その置き場は次の 3 経路だけでした。
上の 10 経路が 1 件も現れないことを、この全数からも二重に確かめています。

| 経路 | 件数 | 内訳 |
|---|---|---|
| `crates/areka-emo-text/` | 53 | `src/` の非テスト 10（本仕様の対象 6 ファイル＋`canvas.rs`・`lib.rs`・`viewbox.rs`・`viewbox_draw.rs`）・`src/` の兄弟テスト 30・`tests/` 9・example 4 |
| `.kiro/specs/` | 12 | 本 spec の文書 9（新設）・`ukadoc-survey-assets` brief 1・完了 spec `emo-text-layer` の注記 2 |
| `doc/COMPAT_ARCHITECTURE.md` | 1 | §8 の裁量記録 |

新設は全体で 13 件（`crates/` 4 件＝タスク 6.1 の証跡 §2.1 が挙げる Rust ソース 4 ファイル・`.kiro/specs/` 9 件）です。

---

## 2. 意味論に手が入っていないこと（要件 9.1／9.2）

### 2.1 あふれ判定 `visible_window` の本体が 1 文字も変わっていないこと（9.1）

両版から関数本体だけを取り出して比べました。

```
git show main:crates/areka-emo-text/src/layout.rs | awk '/pub fn visible_window/,/^    }$/'
git show HEAD:crates/areka-emo-text/src/layout.rs | awk '/pub fn visible_window/,/^    }$/'
diff <両者>
```

→ **両版とも 47 行・`diff` は出力なし・終了コード 0**。
「最新行の遠端 > 境界」・最小スキップの探索・全行超過時の飽和という判定の分岐は、増減も並べ替えもありません。

`layout_visible_window_tests.rs` は再導出の対象（境界 36 → 34 など）ですが、**判定の分岐を増減していない**ことを
構造の 3 つの数で確かめました。

| 量 | main | HEAD |
|---|---|---|
| `#[test]` の本数 | 10 | 10 |
| `assert` を含む行の本数 | 17 | 17 |
| テスト関数名の並び | — | `diff` 出力なし・終了コード 0（1 つも改名・増減なし） |

差分は期待値と前提コメントだけで、`#[ignore]`・許容幅の拡大・分岐の削除はありません。

### 2.2 `\_l` の語彙・原点・書字方向ごとの解決規則（9.2）

- **解決層 `cursor_tag.rs`**: `git diff --stat main...HEAD -- crates/areka-emo-text/src/cursor_tag.rs` → **出力なし・終了コード 0**。
- **語彙層 `parse_cursor_coord`（`state.rs`）**: `state.rs` の差分行（37 挿入・8 削除）を
  `parse_cursor_coord|Clear|NewLine|RevealSchedule|interval` で絞ると **1 行も一致しません**（終了コード 1）。
  検索が空振りでないことは、HEAD 側の `state.rs` に同じ語がそれぞれ 7／12／7／6／22 か所あることで確かめています。
- **`state.rs` の差分の中身**（全 45 行を目視で分類）: 調整値の持ち物を係数
  `line_pitch_factor: f32` から行間 `line_gap: f32`（既定 2.0）へ替え、`line_pitch(font_height)` と
  `normalized()` を足し、doc をそれに追随させただけです。`CueCommand` の消費側・改行の扱い・reveal の日程には触れていません。
- **`\_l` へ渡す値の経路**: `layout.rs` の差分に `CursorBasis` を含む行は 1 行もありません。
  変わったのは `FixedMetrics::line_pitch` が返す値（自前の乗算＋切り上げ →
  `TextLayerConfig::line_pitch` への委譲）だけで、`CursorBasis.line_pitch` へ**渡し方**は不変です。
- **兄弟テストの構造**: `\_l` 系 6 ファイルの `#[test]` 本数は main と HEAD で完全に一致します
  （`cursor_tag_tests` 18／`cursor_tag_resolve_tests` 12／`layout_cursor_tests` 12／
  `layout_cursor_overflow_tests` 5／`layout_cursor_vertical_canon_tests` 11／`cursor_tag_test_support` 0）。
  差分は注入定数 `LINE_PITCH` 13 → 12 とそれに従う期待値だけです。

#### 2.2.1 唯一の、記録された優先順位の変更（台帳 §7 #12）

`\_l` が描画範囲の遠辺の近くへ跳んだ直後のグリフは、従来は描画範囲の外へ置かれていたものが、
本仕様の後は折り返されます。これは**語彙・原点・解決規則の変更ではありません**——
`\_l` の解決結果（行内位置の跳躍先）は同じで、その後に配置層のゲート③'（描画範囲の遠辺による無条件折返し・要件 6.2）が
必ず通るようになったための、**配置層の hard 判定が優先する**という順位の裁定です
（タスク 4.2 のレビューで裁定・台帳 §7 #12 が正本）。既存テストへの影響は 0 本
（`layout_test_support` の領域はすべて soft == hard に解決するため発火しない）。

### 2.3 `\c` の全消去（9.2）

全消去の経路は `viewbox.rs` の `clear_requested` / `FramePlan::FullClear` / `commit(FullClear)` と、
それを立てる `actor.rs` の `request_clear()` です。

- `git diff main...HEAD -- crates/areka-emo-text/src/viewbox.rs` の差分行を
  `request_clear|FullClear|clear_requested` で絞ると **1 行も一致しません**（終了コード 1）。
  HEAD 側の `viewbox.rs` に同じ語が 1／10／9 か所あるので、空振りではありません。
- 製品コード 8 ファイル（`draw` `layout` `state` `region` `actor` `choice` `viewbox` `viewbox_draw`）の差分をまとめて
  同じ語で絞っても一致 0 行です（次項 2.4 と同じ検索）。

### 2.4 比率つき改行と reveal のペース（9.2）

- **比率つき改行**: 送り量を決める `apply_pending_newline`（`*block_pos += block_dir * pitch * sum`）を
  両版から取り出して比べました → **両版とも 13 行（レビューの実数え・当初 12 と記載）・`diff` は出力なし・終了コード 0**。
  比率 `sum` の掛かり方も、行頭への復帰も、pitch を外から受け取る形も不変です
  （変わったのは呼び手が渡す `pitch` の値だけ）。
- **reveal のペース**: 製品コード 8 ファイルの差分行を
  `parse_cursor_coord|RevealSchedule|request_clear|FullClear|clear_requested|CursorMove|reveal|playback_duration|CHAR_NOMINAL`
  で絞ると、一致するのは **doc コメント 2 行だけ**でした。

  | 版 | 行 |
  |---|---|
  | main | `/// 調整値（line_pitch 係数）。reveal ペースは配送 duration 由来ゆえ char_wait は持たない` |
  | HEAD | `/// 調整値（行送りの行間 line_gap）。reveal ペースは配送 duration 由来ゆえ char_wait は…` |

  主張（「配送された再生時間から導出する・自前の文字待ちを持たない」）はそのままで、
  括弧の中の持ち物の呼び名だけが追随しています。単一真実源
  `crates/areka-sakura/src/duration.rs`（`CHAR_NOMINAL_MS = 50` と `text_playback_duration`）は差分 0 件（§1 の #5）。

---

## 3. 対象 6 ファイルの着地行数（要件 9.5）

タスク 6.1 の証跡 §2.2 が同じ表を出しているので、その値を**再測して一致することを確かめた**うえで引き写します
（`wc -l` の実数え・`main` 側は `git show main:<path> | wc -l`）。

| ファイル | 着手時（台帳 §1.2） | main 側の実数え | 着地（HEAD） | 増分 | 1,000 行以下 | 上限まで |
|---|---|---|---|---|---|---|
| `crates/areka-emo-text/src/draw.rs` | 980 | 980 | **988** | +8 | ○ | 12 行 |
| `crates/areka-emo-text/src/layout.rs` | 890 | 890 | **933** | +43 | ○ | 67 行 |
| `crates/areka-emo-text/src/state.rs` | 499 | 499 | **528** | +29 | ○ | 472 行 |
| `crates/areka-emo-text/src/region.rs` | 863 | 863 | **932** | +69 | ○ | 68 行 |
| `crates/areka-emo-text/src/actor.rs` | 879 | 879 | **883** | +4 | ○ | 117 行 |
| `crates/areka-emo-text/src/choice.rs` | 550 | 550 | **554** | +4 | ○ | 446 行 |

6 ファイルとも 1,000 行以下で着地しています。最大は `draw.rs` の 988 行。
タスク 6.1 の実測（同日・commit `49ece2fc` 時点）と 6 ファイルとも同値でした。

参考——境界の**例外 2 件**として設計が明示的に改変を認めたファイル（要件 9.5 の 6 ファイルには含まれないが、
同じ上限の下にある）:

| ファイル | main | 着地 | 増分 | 1,000 行以下 | 設計上の位置づけ |
|---|---|---|---|---|---|
| `crates/areka-emo-text/src/viewbox.rs` | 762 | **846** | +84 | ○ | Out of Boundary 例外 1（R-2 の決着・タスク 3.4）＋例外 2（決定 3・タスク 7.1） |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 806 | **831** | +25 | ○ | Out of Boundary 例外 2（決定 3・タスク 7.1） |

行数の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`・上限 1000）は、
例外表を 1 行も変えないまま 6 本すべて緑です（タスク 6.1 の証跡 §2.3・同日実測）。

### 3.1 `draw.rs` を触る唯一の進行中 spec であること（9.5）

```
git log --oneline main..HEAD -- crates/areka-emo-text/src/draw.rs
```

→ **1 件**（`94ba66f3` タスク 2.1）。本ブランチ以外の進行中の作業が `draw.rs` に載っていないこと（着手時の確認）は
台帳 §1.1 が記録しており、そのときの出力は 0 件でした。

進行中の spec が `draw.rs` を自分の仕事として挙げていないことも見ました。
`grep -ln "draw.rs" .kiro/specs/*/tasks.md` の一致は **2 件**で、内訳は次のとおりです。

| spec | 一致行の性格 | 判断 |
|---|---|---|
| `areka-P0-emo-text-line-height-canon`（本仕様） | 自分のタスク | — |
| `areka-P0-emo2-conformance-e2e` | 走行 A 中断の**根因の記述**（`draw.rs:766-774` が先頭行を skip する経路を指す）で、同じ行が「別 spec を切って先に直す」＝本仕様を引受先に指名している | 引受先の指名であって `draw.rs` への作業宣言ではない。改変も 0 件（§1 の全数 66 件に e2e 側のコードは含まれない） |

`areka-P0-text-decoration-canon`（`draw.rs` の分割を持つ後続）は、ディレクトリに `brief.md` **1 ファイルのみ**で
`tasks.md` が存在しません。着手前であり、`draw.rs` を触っていません。

---

## 4. コミット列が設計の順序どおりであること（要件 9.6）

`git log --reverse --format="%h %s" main..HEAD` は **31 件**を返します。
内訳は仕様フェーズ 12 件（要件・設計・タスクの生成と裁定の反映）と実装フェーズ 19 件です。
実装フェーズの 19 件を design.md §12「コミット順」の段へ対応づけます。

| # | commit | タスク | 触った `crates/` の件数 | §12 の段 |
|---|---|---|---|---|
| 1 | `e1ffc671` | 1.1 着手時の前提確認と再導出台帳の確定 | 0 | 段 1 の前提（9.5 の着手時確認） |
| 2 | `d7286a0a` | 1.1 の実装ノート（文書のみ） | 0 | 同上 |
| 3 | `9bd274ec` | 1.2 **正典表・裁量記録・アーカイブ注記** | 0 | **段 1**（意味論の確定＝文書だけ） |
| 4 | `e02c784e` | 1.3 根拠画像の受領待ちと読み取り値の対応づけ | 0 | 段 1 |
| 5 | `94ba66f3` | 2.1 **行送りの式を差し替え** | 7 | **段 2**（実装の追随） |
| 6 | `601f94ad` | 帯からのインク 1 画素はみ出しの裁定を記録（文書のみ） | 0 | 段 2〜3 の間（R-1 の裁定） |
| 7 | `95b7761f` | 2.2 帯の防御式を保ったまま値と記述を追随 | 6 | 段 2 |
| 8 | `7af4c0b8` | 3.1 配置・折返し・カーソルタグの期待値を再導出 | 13 | 段 3（台帳 A） |
| 9 | `59cf3240` | 3.2 表示・結線・状態側の期待値を再導出 | 5 | 段 3（台帳 A／B） |
| 10 | `b48494d2` | 3.3 COM 層・実フォント側の期待値を再導出 | 8 | 段 3（台帳 B） |
| 11 | `7fe1d3d9` | 3.4 容量前提と参照描画比較の導き直し＋R-2 の修正 | 10 | 段 3（台帳 C）＋境界の例外 1 |
| 12 | `d3469c17` | 3.5 退役テストを同数の代替へ差し替え | 1 | 段 3（台帳 D） |
| 13 | `3d6a6d88` | 4.1 `TextRegion.inline_limit` の保持と粗いバルーンの警告 | 3 | 段 4 |
| 14 | `573b5b2d` | 4.2 配置層に描画範囲の遠辺による無条件折返し | 3 | 段 4 |
| 15 | `5e49aa10` | 5.1 実物バルーン × メニュー 3 台本の容量テスト | 1 | 段 5 |
| 16 | `2a500034` | 5.2 裁定値の実フォント読み戻しテスト（帯の第 2 回裁定） | 3 | 段 5 |
| 17 | `28184883` | 7.1 ダーティ矩形ごとに交差する行だけを描く | 6 | **段 3′**（裁定の追補・決定 3） |
| 18 | `dec1461a` | 7.2 観測用 example の前提を導き直す | 4 | 段 3′（決定 4） |
| 19 | `49ece2fc` | 6.1 旧式の残存検査と全体テストの証跡 | 1（テストの doc 1 語） | 段 6 |

順序は **1 → 2 → 3 → 4 → 5 → 3′ → 6** で、design.md §12 の指定と一致します。
§12 は 3′ について「実装順では 5 の後・6 の前に置いてよい」と書いており、実際に 5.2 の後・6.1 の前に入っています。

### 4.1 正典表と実装がずれた中間状態を残していないこと（9.6）

**確かめ方**: 各コミットが `crates/` の何ファイルを触ったかを数えました（上表の「触った `crates/` の件数」列）。

- 実装フェーズの最初の 4 件（`e1ffc671`〜`e02c784e`）は `crates/` を **0 件**しか触っていません。
  意味論の確定（正典表・`doc/COMPAT_ARCHITECTURE.md` §8 の裁量記録・完了 spec の注記）だけが先に載り、
  コードは 1 行も動いていない状態です。
- 製品コードが最初に動くのは `94ba66f3`（2.1）で、これは正典表が載った `9bd274ec` の **2 コミット後**です。
  したがって「実装が正典表を追い越した中間コミット」は 1 つもありません。

**`94ba66f3` の時点で既存テストが赤だったこと**は、隠さずここに書きます。同コミットのメッセージが
「既存テスト 59 本は想定どおり赤。期待値の再導出は 3.1／3.2／3.3 が持つ」と述べ、
`tasks.md` の申し送り（2.1 の欄）が内訳（lib 53・pipeline 3・scale_invariance 2・choice_fixture 1）を、
2.2 の欄が「2.2 で 1 本増えて計 60 本」を記録しています。
これは design.md §12 段 2 が明示的に許した状態です——
「この時点で既存テストは赤でよいが、次のコミットまでに緑にする」。
赤を緑へ戻したのは段 3 の 5 コミット（`7af4c0b8`〜`d3469c17`）で、
弱体化・`#[ignore]`・許容幅の拡大は行っていません（タスク 2.1 の禁止条項・各タスクの完了状態）。
着地時点（HEAD `49ece2fc`）で crate 571 本・ワークスペース 7,120 本がいずれも終了コード 0 であることは
タスク 6.1 の証跡 §3 が実測しています。

なお `94ba66f3` の変更 8 ファイルのうち `crates/` は 7 件で、その内訳は製品コード 4
（`state.rs` `draw.rs` `layout.rs` `actor.rs`）・兄弟テスト 2・example 1 です。
正典表（design.md §4.1・COMPAT §8）はこの時点で既に確定済みなので、
コミット単体を取り出しても「式は新・記述は旧」という食い違いは起きません。

### 4.2 作業ツリーの状態

```
git status --porcelain
```

→ 本文書を書く前の時点で **出力なし・終了コード 0**（未コミットの変更なし）。
本タスクが増やすのはこの証跡 1 ファイルだけです。

---

## 5. 結論

| 要件 | 判定 | 根拠 |
|---|---|---|
| 9.1 あふれ判定の式・分岐が不変 | **成立** | `visible_window` 本体の抽出比較が両版 47 行で `diff` 出力なし（§2.1）。`layout_visible_window_tests.rs` は `#[test]` 10 本・`assert` 17 行・関数名の並びまで一致し、差分は期待値と前提コメントのみ |
| 9.2 `\_l`・`\c`・比率つき改行・reveal が不変 | **成立** | `cursor_tag.rs` の差分 0 件（§2.2）。`state.rs`／`viewbox.rs` の差分行に `parse_cursor_coord`／`Clear`／`NewLine`／`RevealSchedule`／`interval`／`request_clear`／`FullClear`／`clear_requested` が 1 行も現れない（検索は空振りでないことを確認済み）。`apply_pending_newline` は両版 12 行で `diff` 出力なし（§2.4）。reveal 関連の差分は doc の呼び名 2 行のみで、単一真実源 `areka-sakura/src/duration.rs` は差分 0 件。**唯一の例外は台帳 §7 #12 に記録済みの優先順位**——`\_l` の語彙・原点・解決規則は不変で、配置層の hard 判定（要件 6.2）が優先する（§2.2.1） |
| 9.3 fixture・kanade・pasta・sakura を改変しない | **成立** | 10 経路すべてで `git diff --stat main...HEAD` が出力なし・終了コード 0（§1）。全数 66 件の置き場も `crates/areka-emo-text/`・`.kiro/specs/`・`doc/COMPAT_ARCHITECTURE.md` の 3 経路だけ（§1.2） |
| 9.5 `draw.rs` 唯一の進行中 spec・6 ファイルが 1,000 行以下で着地 | **成立** | 6 ファイルの着地は 988／933／528／932／883／554 でいずれも 1,000 行以下（§3）。`draw.rs` を触った本ブランチのコミットは `94ba66f3` の 1 件、他の進行中 spec の `tasks.md` に現れる `draw.rs` は e2e の根因記述 1 件だけで、そこは本仕様を引受先に指名している。`text-decoration-canon` は `brief.md` のみで `tasks.md` が無い（§3.1） |
| 9.6 連続したコミット列・正典表と実装のずれた中間状態なし | **成立** | 実装フェーズ 19 件が §12 の段 1 → 2 → 3 → 4 → 5 → 3′ → 6 に順序どおり対応（§4）。正典表の確定（`9bd274ec`・`crates/` 0 件）が製品コードの最初の変更（`94ba66f3`）に先行し、逆転は 1 件もない。`94ba66f3` 時点の既存テスト 59 本の赤は §12 段 2 が明示的に許した状態で、段 3 の 5 コミットで緑（§4.1） |

**未決は 0 件です。** 触らないと約束した経路の差分はすべて 0 件で、意味論の 4 項目は不変、
6 ファイルの行数とコミット順は設計の指定どおりに着地しています。

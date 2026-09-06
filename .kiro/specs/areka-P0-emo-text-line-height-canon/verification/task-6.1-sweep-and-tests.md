# タスク 6.1 の証跡——旧式の残存検査と全体テスト（要件 2.4／7.6／8.6／10.4）

実施日: 2026-09-06 / ブランチ `claude/areka-p0-emo-text-line-height-b69d6c`（HEAD `dec1461a`・未コミットの作業ツリー）

**版**: 第 2 版。初回走行が残した未決 1 件（機械検査が文言どおりには 0 件にならない）を、コントローラの裁定「案 A」（2026-09-06）で解消し、走り直した結果を反映しています（§1.0・§1.4・§3.5）。

この文書はタスク 6.1 の 3 つの検査——⑴ 旧係数 1.25 の残存検査、⑵ 新設ファイルの置き場と行数、
⑶ 対象クレートとワークスペース全体のテスト——の実測を 1 か所にまとめたものです。
製品コード（`crates/areka-emo-text/src/` の非テストファイル）には 1 行も触れていません。

## 0. 道具の置き換え（重要）

design.md「機械検査（2.4）」は `rg`（ripgrep）で検索を書いていますが、**この機械に `rg` は入っていません**
（`command -v rg` が終了コード 1・`PATH` 上の各所と `~/.cargo/bin`・Chocolatey・VS Code 同梱のいずれにも無し）。
そこで同じ意味の `grep` へ置き換えて実行しました。対応は次のとおりです。

| design の記述 | 実行したもの | 差異 |
|---|---|---|
| `rg -n <pat> <paths>` | `grep -rnI -E <pat> <paths>` | `-r` で再帰・`-I` でバイナリ除外。`rg` は `.gitignore` を読むが対象は追跡下のソース木のみなので結果は同じ |
| `\| rg <pat>` | `\| grep -E <pat>` | 同一 |
| `\| rg -v <pat>` | `\| grep -vE <pat>` | 同一 |

出力形式（`パス:行番号:本文`）も同じです。終了コードは `${PIPESTATUS[n]}` で明示的に取り出しました。

---

## 1. 旧係数の残存検査（要件 2.4）

### 1.0 経緯——初回走行の未決と 2026-09-06 の裁定（案 A）

初回走行（同日・本文書の第 1 版）では第 3 段が **5 件**残り、機械検査を文言どおりに読むと通りませんでした。
5 行はいずれも履歴・注記つき引用・改訂前後の対照で、`1.25` を**現行の**行送り係数として述べる行は 0 行でしたが、
第 3 段の除外語（`旧式`／`本仕様で改訂`／`履歴`）がその 5 行に 1 つも含まれていなかったためです。

コントローラが 5 行を実際に開いて確かめたうえで、**案 A を裁定しました**（2026-09-06）——
検査の定義（3 段の式・除外語）は 1 文字も変えず、5 行それぞれに `旧式` の 1 語を添える。
本文書はその是正後に走り直した結果です。是正した 5 行と、これが検査のごまかしに当たらない理由は 1.4 に書きます。

### 1.1 実行した検索（3 段）

```
grep -rnI -E "1\.25" \
  crates/areka-emo-text/src crates/areka-emo-text/tests crates/areka-emo-text/examples \
  doc/COMPAT_ARCHITECTURE.md .kiro/specs/areka-P0-emo-text-line-height-canon/design.md \
  | grep -E "line_pitch|行送り|係数" \
  | grep -vE "旧式|本仕様で改訂|履歴"
```

| 段 | 意味 | ヒット件数 | 終了コード | 初回走行 |
|---|---|---|---|---|
| 1 | 対象 5 経路に現れる `1.25` の全行 | **82 件** | 0 | 82 件・0 |
| 2 | うち同じ行に `line_pitch`／`行送り`／`係数` を含む行 | **10 件** | 0 | 10 件・0 |
| 3 | うち `旧式`／`本仕様で改訂`／`履歴` を含まない行 | **0 件** | **1** | 5 件・0 |

**第 3 段は出力なし・終了コード 1** です。`grep` は 1 件も一致しないときにだけ終了コード 1 を返すので、
終了コード 1 そのものが 0 件の証拠になります（`wc -l` が 0 を返すことに頼っていません）。
design.md「機械検査（2.4）」が定めた「期待 0 件」を、定義を変えないまま満たしました。

第 1 段・第 2 段の件数は初回走行と同じ 82／10 です。是正は既存の 5 行へ語を 1 つ足しただけで、
行を消しても足してもいないため、母数が動かないことが期待どおりであることの裏づけになります。

### 1.2 第 2 段の 10 行（全文一覧・design が証跡に添えよと求めるもの）

長い行は先頭 120 字ほどで切っています。**10 行すべてが第 3 段の除外語（`旧式`）を持ちます**——
うち 5 行はもともと持っていた行、残る 5 行が 1.4 で語を添えた行です（★印）。

```
  crates/areka-emo-text/src/layout_wrap_tests.rs:24:/// `fixed_metrics_line_pitch_ceils_fractional_values`（旧式 `ceil(font_height × 1.25)` の…
★ crates/areka-emo-text/tests/kero_menu_capacity_test.rs:9://! 行送りが旧式の `ceil(font.height × 1.25)` ＝ 35 で 3 行目の下端（138）が…
  crates/areka-emo-text/tests/kero_menu_capacity_test.rs:235:/// 行送りだけを**旧式**（`ceil(font_height × 1.25)`）へ戻すテスト専用の metrics（要件 8.7 の対照）。
  crates/areka-emo-text/tests/kero_menu_capacity_test.rs:725:/// 判定が生きていることの対照。行送りだけを旧式（`ceil(28 × 1.25)` ＝ 35）へ戻すと、
  crates/areka-emo-text/tests/kero_menu_capacity_test.rs:742:        "旧式の行送りは ceil(28 × 1.25) = 35"
★ doc/COMPAT_ARCHITECTURE.md:214:| **【上書き】…**（…）| …「行送りピッチ」の行（旧式 `line_pitch = ceil(font.height × 1.25)`＝28 なら 35）→ **本行の式（30）**…
★ .kiro/specs/…/design.md:11:**Impact**: 行送りの源が 2 系統（旧式＝係数 1.25 の `ceil`・実フォント比の行ボックス丈…）に散っている現状を…
  .kiro/specs/…/design.md:73:| 行送りピッチ | `ceil(font.height × 1.25)`（旧式） | `state.rs:48-74`・`draw.rs:476-479`…
★ .kiro/specs/…/design.md:403:- Validation: …既存 `:590-596` の「既定値 1.25」（旧式）は「既定 `line_gap` 2.0」へ。
★ .kiro/specs/…/design.md:556:| **A 純粋層・`FixedMetrics`**… | …行末「doc の「旧式 `ceil(×1.25)`」を「`font_height + 行間 2`」へ」 |
```

### 1.3 10 行の 1 行ずつの分類（全行を開いて読んだ結果）

「種別」は初回走行で全行を開いて読んだときの判断で、是正では変えていません。
「3 段目で落ちるか」の列が、是正の前後で変わったところです。

| # | 位置 | 種別 | 3 段目で落ちるか（是正後） | 初回 | 読んだ内容と判断の根拠 |
|---|---|---|---|---|---|
| 1 | `layout_wrap_tests.rs` の退役テストを記録する doc コメント | 履歴 | 落ちる（`旧式`・元から） | 落ちる | 要件 7.2 の「退役の個別記録」。退役した `fixed_metrics_line_pitch_ceils_fractional_values` の検証対象が旧式であったことを述べる |
| 2 | `kero_menu_capacity_test.rs` の冒頭 doc「この決定論テストが塞ぐ穴」 | 履歴（症状の原因） | 落ちる（★`旧式の`を追加） | 落ちない | 実機症状の原因を旧式で説明した文。直後の 2 行が「行送りを正典（`font.height + 行間` ＝ 30）へ直した後、その症状が二度と起きないことを固定する」と述べており、現行式は 30 であることが同じ段落で明示されている |
| 3 | 同ファイル `LegacyPitchMetrics` の型 doc | 注記つき引用（要件 8.7 の対照） | 落ちる（`旧式`・元から） | 落ちる | 旧式へ戻すテスト専用の `GlyphMetrics` 実装であることの宣言 |
| 4 | 同ファイル 8.7 の対照テストの doc | 注記つき引用 | 落ちる（`旧式`・元から） | 落ちる | 同上 |
| 5 | 同ファイル 8.7 の対照テストの assert メッセージ | 注記つき引用 | 落ちる（`旧式`・元から） | 落ちる | 同上 |
| 6 | `doc/COMPAT_ARCHITECTURE.md` §8 の【上書き】行 | 注記つき引用 | 落ちる（★`旧式 `を追加） | 落ちない | `1.25` の出現は 2 か所。⑴ 上書きされた完了 spec の補足正準の**逐語引用**「`line_pitch = ceil(font.height × 1.25)`＝28 なら 35」に矢印「→ **本行の式（30）**」を付した対照——ここへ `旧式` を添えた。⑵ `research.md` のリスク登記の**見出し名**「行送りピッチ 1.25 係数」の引用（消化注記を添えた対象を指すため）。同じ行の本文は新式（`font.height + 行間`・行間の既定 2・28 なら 30）を正典として述べている |
| 7 | `design.md` の Impact 段落 | 履歴 | 落ちる（★`旧式＝`を追加） | 落ちない | 「行送りの源が 2 系統（…）に散っている**現状を** … **へ畳む**」＝本仕様の着手前の状態を述べ、直後で新式へ畳むと宣言している |
| 8 | `design.md` §4.1 正典表の「行送りピッチ」行 | 注記つき引用（新旧対照） | 落ちる（`旧式`・元から） | 落ちる | 「改訂前」列に旧式、「改訂後」列に新式を並べた表の 1 行 |
| 9 | `design.md` の `state_cue_apply_tests` の Validation 行 | 注記つき引用（改訂指示） | 落ちる（★`（旧式）`を追加） | 落ちない | 「既存 `:590-596` の『既定値 1.25』は『既定 `line_gap` 2.0』へ」＝**書き換える対象**として旧値を引用したもの。同じ行の前半は新しい期待値（28 → 30・12 → 14・10 → 12）を述べている |
| 10 | `design.md` の再導出台帳 A 群の表 1 行 | 誤検知（対象外の `1.25`）＋改訂指示 | 落ちる（★`旧式 `を追加） | 落ちない | 巨大な表 1 行に `1.25` が 4 回現れる。⑴ `state_reveal_tests`（**再生時刻**の 1.25・作業なし）⑵ `viewbox_axis_tests`（**DPI 拡大率 k**・作業なし）⑶ `actor_tests`／`actor_scale_refresh_tests`（同 k・作業なし）——⑴〜⑶ は要件 2.4 が明記する対象外で、旧式ではないので語を添えていない。⑷ 行末の「doc の『`ceil(×1.25)`』を『`font_height + 行間 2`』へ」＝改訂指示で、これだけが旧式を指すため**そこへ**語を添えた |

**まとめ**: 10 行すべてが第 3 段で落ち、残りは 0 行です。10 行の内訳は
履歴 3（#1・#2・#7）・注記つき引用 6（#3〜#6・#8・#9）・誤検知を含む表 1 行（#10）で、
**現行の行送り係数を 1.25 と述べる行は 0 行**という初回走行の読みは是正の前後で変わっていません。

### 1.4 決着——2026-09-06 の裁定「案 A」と、是正した 5 行

初回走行は決着案を 2 つ挙げていました。案 A（残る 5 行に除外語を 1 語添える）と
案 B（機械検査の定義を「第 2 段までの一覧＋目視」へ改める）です。
コントローラが 5 行を実際に開いて内容を確かめたうえで、**案 A を採る**と裁定しました（2026-09-06）。

添えた語は `旧式` の 1 語だけで、次の 5 か所です（文の書き換えはしていません）。

| ★ | ファイル | 目印になる語句 | 添えた形 |
|---|---|---|---|
| 2 | `crates/areka-emo-text/tests/kero_menu_capacity_test.rs`（冒頭 `//!`） | 「行送りが `ceil(font.height × 1.25)` ＝ 35 で 3 行目の下端（138）…」 | 「行送りが**旧式の** `ceil(...)` ＝ 35 で…」 |
| 6 | `doc/COMPAT_ARCHITECTURE.md`（§8 の【上書き】行） | 「「行送りピッチ」の行（`line_pitch = ceil(font.height × 1.25)`＝28 なら 35）」 | 「…の行（**旧式** `line_pitch = ceil(...)`＝28 なら 35）」 |
| 7 | `design.md`（Overview の **Impact**） | 「行送りの源が 2 系統（係数 1.25 の `ceil`・…」 | 「…2 系統（**旧式＝**係数 1.25 の `ceil`・…」 |
| 9 | `design.md`（`TextLayerConfig` の Validation 行） | 「既存 `:590-596` の「既定値 1.25」は…」 | 「…の「既定値 1.25」**（旧式）**は…」 |
| 10 | `design.md`（Testing Strategy 再導出台帳の **A 純粋層・`FixedMetrics`** の行） | 行末の「doc の「`ceil(×1.25)`」を「`font_height + 行間 2`」へ」 | 「doc の「**旧式** `ceil(×1.25)`」を…」 |

表の 2 行（#6・#10）はどちらも 1 行が非常に長い表の行なので、**行を分けずに語だけを差し込みました**。
5 ファイルとも改行の作法（`doc/COMPAT_ARCHITECTURE.md` は CRLF、他は LF）と総行数は変わっていません。

**これが検査のごまかしに当たらない理由**を書いておきます。第 3 段の除外語は
「その行が旧式について述べていることを、行の中で名乗っていること」を判定するためのものです。
是正した 5 行は初回走行で全文を読んだとおり、いずれも
**履歴（着手前の状態・実機症状の原因）**か**注記つき引用（上書きされた正典の逐語引用・書き換える対象の引用）**か
**改訂指示**であって、`1.25` を現行の行送りとして述べている行は 1 つもありません。
つまり 5 行はもともと「旧式について述べる行」であり、添えた語はその性質を読み手にも機械にも
明示しただけです。検査の式・除外語・対象経路は 1 文字も変えていませんし、
`1.25` を現行の値として述べる行を隠すために語を足した箇所はありません
（#10 の `1.25` のうち DPI 拡大率 k と再生時刻の 3 か所には、旧式ではないので語を添えていません）。

案 B（検査の定義を緩める）を採らなかったので、**この検査は今後も他人が同じコマンドで再現できます**。
語が足りずに取りこぼす向きの弱さ（現行式を述べる行に偶然 `旧式` が含まれる）は残りますが、
それは 1.2 の第 2 段一覧（10 行の全文）を証跡に添えることで押さえています。

### 1.5 より広い検索——旧式そのものが製品コードに残っていないか

design の 3 段検索は語の同居を条件にするため、`1.25` を数式の形でだけ書いている箇所を落とします。
そこで旧式の式の形と旧識別子を直接探しました。

```
grep -rnI -E "ceil\(.*1\.25|× ?1\.25|\* ?1\.25|line_pitch_factor" \
  crates/areka-emo-text/src crates/areka-emo-text/tests crates/areka-emo-text/examples crates/areka/src
```

→ **28 件**（終了コード 0）。全件の分類:

| 種別 | 件数 | 内訳 |
|---|---|---|
| DPI 拡大率 k としての `1.25`（要件 2.4 が明記する対象外） | **20** | `actor_scale_refresh_tests.rs` 1・`viewbox_axis_tests.rs` 2・`viewbox_dirty_tests.rs` 3・`viewbox_draw_live_diff_tests.rs` 1・`viewbox_plan_commit_tests.rs` 2・`tests/scale_invariance_test.rs` 7・`crates/areka/src/placement/` 4 |
| 要件 8.7 の対照（テスト専用の旧式実装とその doc） | **6** | `tests/kero_menu_capacity_test.rs` の 6 か所。うち唯一の**実行される式**が `LegacyPitchMetrics` の `(font_height * 1.25).ceil()` で、これは `impl GlyphMetrics for LegacyPitchMetrics` の中＝製品コードではない |
| 履歴（退役テストの記録） | **1** | `layout_wrap_tests.rs` の退役 doc（1.3 の #1 と同一行） |
| 履歴（他クレートの症状説明） | **1** | `crates/areka/src/emo2_boot/spine_conformance_script.rs`——1.6 参照 |

**製品コードの 0 件を明示します**（沈黙で済ませない）:

- `crates/areka-emo-text/src/` の非テストファイルのうち行送りの定義点にあたる 4 ファイル
  （`state.rs`・`layout.rs`・`draw.rs`・`choice.rs`）に `1.25` は **0 件**
  （`grep -nI "1\.25"` が終了コード 1）。
- 旧識別子 `line_pitch_factor` は **リポジトリ全域の `.rs` に 0 件**
  （`grep -rnI "line_pitch_factor" --include="*.rs" --include="*.md" .` のヒットはすべて `.md`＝
  本仕様の brief／design／research／tasks／台帳／設計バリデーションと、完了 spec `areka-P0-cue-playback-duration`
  のアーカイブ。いずれも撤去の計画または履歴を述べる文書）。

### 1.6 1 件の申し送り（本仕様の担当範囲の外）

`crates/areka/src/emo2_boot/spine_conformance_script.rs` の走行 A の症状を説明するコメント
（「走行 A（2026-09-05）の症状 #1 はこの層では観測できない」の節）が、

> 実機の症状「相方側バルーンでメニューの先頭の選択肢が描かれない」は、行送り
> `ceil(font.height × 1.25)`＝35px と相方側 validrect の高さ 93px の関係で 3 行が収まらない
> という**字の配置**の欠陥である

と**現在形**で述べています。同じ節は続けて「その決定論テストは引受先の別仕様（emo-text の行送り正典化）が持つ」
と書いており、その引受先が本仕様です。本仕様の着地で 35px は 30px になり、この記述は事実の説明としては
旧式の履歴になりました。

**要件 2.4 の検査対象ではありません**（対象は `crates/areka-emo-text/src/` の非テストファイル・
テストと `examples/` の doc コメント・正典表と裁量記録。`crates/areka` は含まれず、
design の機械検査の経路にも入っていない）。また `areka-P0-emo2-conformance-e2e` は
走行 A〜D を採り直す前提で本仕様の引き渡し（要件 10.1／10.2）を受け取るため、
同 spec の再走時にこの節も見直されます。**本タスクでは触らず、記録だけ残します。**

---

## 2. 新設ファイルの置き場と行数（要件 8.6）

### 2.1 本ブランチで新設したファイル（実数え）

```
git diff --name-status main...HEAD --diff-filter=A -- crates/
```

→ **4 件**（終了コード 0）。design.md「File Structure Plan」が挙げる 4 ファイルと**一致**します
（design の「4 ファイル」は主張として検証し、実測と食い違いませんでした）。

| ファイル | 置き場の規約 | 行数 | 1,000 行未満 | include 元 |
|---|---|---|---|---|
| `crates/areka-emo-text/src/layout_hard_limit_tests.rs` | 兄弟ファイル `<stem>_<theme>_tests.rs`（stem＝`layout`・theme＝`hard_limit`） | 415 | ○ | `layout.rs` の `#[cfg(test)] #[path = "layout_hard_limit_tests.rs"] mod hard_limit_tests;` 宣言（`layout_cursor_wiring_tests` と `layout_segmented_tests` の宣言の間） |
| `crates/areka-emo-text/src/region_inline_limit_tests.rs` | 兄弟ファイル（stem＝`region`・theme＝`inline_limit`） | 314 | ○ | `region.rs` の同形の `#[cfg(test)] #[path] mod inline_limit_tests;` 宣言（`region_vertical_canon_tests` の宣言の直前） |
| `crates/areka-emo-text/tests/kero_menu_capacity_test.rs` | 統合テストの置き場 `tests/` | 820 | ○ | cargo が `tests/` を自動で拾う（宣言なし） |
| `crates/areka-emo-text/tests/line_pitch_readback_test.rs` | 統合テストの置き場 `tests/` | 686 | ○ | 同上 |

4 件とも「兄弟ファイルまたは `tests/`」の規約に従い、最大は `kero_menu_capacity_test.rs` の 820 行です。

なお本ブランチが新設したファイルは crates 外を含めて 12 件で、残る 8 件は spec 文書と根拠画像
（`verification/` 配下と `evidence/`）です。Rust ソースの新設は上の 4 件だけです。

### 2.2 既存 8 ファイルの行数（着手時 → 現在）

着手時の値は 6 ファイルが `verification/derivation-ledger.md` §1.2 の実測、
`viewbox.rs`・`viewbox_draw.rs` の 2 ファイルは design.md「Modified Files」の括弧内の値です。
念のため 8 ファイルとも `git show main:<path> | wc -l` で main 側を数え直し、両者は一致しました。

| ファイル | 着手時（台帳 §1.2／design） | main 側の実数え | 現在 | 増分 | 1,000 行未満 | design の見込み |
|---|---|---|---|---|---|---|
| `crates/areka-emo-text/src/state.rs` | 499 | 499 | **528** | +29 | ○ | ≈ 530 |
| `crates/areka-emo-text/src/layout.rs` | 890 | 890 | **933** | +43 | ○ | ≈ 910 |
| `crates/areka-emo-text/src/region.rs` | 863 | 863 | **932** | +69 | ○ | ≈ 890 |
| `crates/areka-emo-text/src/draw.rs` | 980 | 980 | **988** | +8 | ○ | ≈ 985 |
| `crates/areka-emo-text/src/actor.rs` | 879 | 879 | **883** | +4 | ○ | 不変 |
| `crates/areka-emo-text/src/choice.rs` | 550 | 550 | **554** | +4 | ○ | 不変 |
| `crates/areka-emo-text/src/viewbox.rs` | 762 | 762 | **846** | +84 | ○ | ≈ 840 |
| `crates/areka-emo-text/src/viewbox_draw.rs` | 806 | 806 | **831** | +25 | ○ | ≈ 830 |

最大は `draw.rs` の **988 行**（上限 1,000 まで残り 12 行）。次が `layout.rs` 933・`region.rs` 932 です。
`region.rs` と `viewbox.rs` は design の見込みを 42／6 行上回りましたが、いずれも上限の内側です。

### 2.3 行数の見張りが例外表無改変のまま緑であること

見張りは `crates/log-capture-kit/tests/file_length_guard_test.rs`（上限 `LINE_LIMIT` ＝ 1000）。

**例外表が無改変であること**（2 通りで確認）:

- `git diff main...HEAD --stat -- crates/log-capture-kit/tests/file_length_guard_test.rs` → **出力なし**（終了コード 0）。
  main からの差分が 1 行も無い。
- `git status --porcelain -- <同ファイル>` → **出力なし**（終了コード 0）。作業ツリーにも未コミットの変更なし。
- 例外表 `OVER_LIMIT_ALLOWED` は 11 件（`OVER_LIMIT_ALLOWED_COUNT = 11` と逐語で二重に持つ形）で、
  `crates/areka-emo-text` を指す項目は **0 件**（同ファイルを `areka-emo-text` で検索して終了コード 1）。
  本仕様は例外表に触れていないし、触る必要も生じていない。

**見張りのテストが緑であること**:

```
cargo test -p log-capture-kit --test file_length_guard_test 2>&1 | grep -E "^test |^test result"; echo exit=${PIPESTATUS[0]}
```

→ **終了コード 0**・6 本すべて緑（3.84 秒）。

```
test the_over_limit_allow_table_has_no_duplicate_entries ... ok
test every_over_limit_exception_is_still_over_the_limit ... ok
test the_over_limit_allow_table_declares_its_own_size_and_reasons ... ok
test no_source_file_exceeds_the_line_limit_outside_the_allow_table ... ok
test the_measurement_is_not_vacuous_and_matches_the_allow_table ... ok
test dropping_a_known_exception_turns_the_guard_red ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.84s
```

このうち `no_source_file_exceeds_the_line_limit_outside_the_allow_table` が
「例外表の外に上限超過は無い」を、`every_over_limit_exception_is_still_over_the_limit` が
「表に載っているのに今はもう超過していない項目が無い」を、
`dropping_a_known_exception_turns_the_guard_red` が「見張り自身が赤を出せる」ことを押さえています。

---

## 3. テストの実行（要件 7.6・10.4）

### 3.1 対象クレート

```
cargo test -p areka-emo-text --no-fail-fast 2>&1 | grep -E "^test result|^     Running|^error\[|^error:"; echo exit=${PIPESTATUS[0]}
```

→ **終了コード 0**（17:19:54 → 17:20:16・約 22 秒。直前の見張りテストでビルド済み）。
`error` 行 0 件。

| 実行された対象 | 結果 |
|---|---|
| `unittests src\lib.rs`（兄弟ファイル群を含む） | 512 passed / 0 failed / 2 ignored |
| `tests\attach_wiring_test.rs` | 6 passed |
| `tests\choice_fixture_test.rs` | 3 passed |
| `tests\draw_readback_test.rs` | 2 passed |
| `tests\emo2_fixture_e2e_test.rs` | 3 passed |
| **`tests\kero_menu_capacity_test.rs`（新設）** | **6 passed** |
| **`tests\line_pitch_readback_test.rs`（新設）** | **3 passed** |
| `tests\physical_extent_arbitration_test.rs` | 4 passed |
| `tests\pipeline_test.rs` | 4 passed |
| `tests\scale_invariance_test.rs` | 8 passed |
| `tests\shipped_fixture_region_test.rs` | 6 passed |
| `tests\vertical_fixture_test.rs` | 11 passed |
| `tests\viewbox_blit_spike.rs` | 2 passed |
| `tests\viewbox_scroll_test.rs` | 1 passed |
| Doc-tests | 0 passed |

合計 **571 passed / 0 failed / 2 ignored**。

**2 件の `ignored` について**（要件 7.2 の「`#[ignore]` にしない」に照らして明示）:
`viewbox_draw::png_dump_tests::diag_dump_budoux_wordwrap_pngs` と
`viewbox_draw::png_dump_tests::diag_dump_horizontal_pngs` の 2 本で、
どちらも `#[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]` が付いた目視診断用です。
**本仕様が新たに付けたものではありません**——`git grep -c "#\[ignore" main -- crates/areka-emo-text/` が
`viewbox_draw_png_dump_tests.rs:3`（節見出しのコメント 1 行＋属性 2 か所）を返し、
現在のブランチの実測と同じです。

### 3.2 ワークスペース全体

```
cargo test --workspace --no-fail-fast 2>&1 | grep -E "^test result: FAILED|^test result|^error\[|^error:|^     Running unittests|^   Doc-tests" > <一時ファイル>; echo exit=${PIPESTATUS[0]}
```

→ **終了コード 0**（17:20:38 → 17:23:44・**約 3 分 6 秒**）。

| 指標 | 値 |
|---|---|
| `test result` 行の数（＝テストの実行単位） | **103** |
| 合計 passed | **7,120** |
| 合計 failed | **0** |
| 合計 ignored | 40 |
| `test result: FAILED` の行 | **0 件** |
| `^error` で始まる行 | **0 件** |

うち最大は `crates/areka` の `unittests src\main.rs` の 1,564 passed / 0 failed / 2 ignored。

memory の申し送り「workspace test は i686 host-32 成果物が要る」に該当する失敗は**起きませんでした**
（i686 の成果物がすでに置かれている状態で走ったため）。仮に不足していれば該当クレートで赤になりますが、
今回は 103 単位すべてが `ok` です。

### 3.3 Yu Gothic UI の在ること

新設の `tests/line_pitch_readback_test.rs` は実フォント（Yu Gothic UI・`font.height,28`）を読み戻す
テストで、フォント不在なら文字送りが縮退値になって赤で止まります。

```
cargo test -p areka-emo-text --test line_pitch_readback_test 2>&1 | grep -E "^test result"; echo exit=${PIPESTATUS[0]}
```

→ 3.1 の一覧のとおり **3 passed / 0 failed・終了コード 0**。
したがってこの機械には Yu Gothic UI が在り、要件 10.4 が DoD に含めると定めた
「実フォント読み戻し」は実際に実行されて緑です。
新設の `tests/kero_menu_capacity_test.rs`（6 passed）も先頭で「あ」の送りが縮退値 28 未満であることを
確かめてから進む形なので、こちらも実フォントで走ったことの裏づけになります。

（要件 10.4 が DoD の**外**と定める SSP の画素実測と実機一周は、本タスクでは行っていません。）

### 3.4 書式と静的解析（門ではない・数だけ記録）

| 検査 | コマンド | 結果 |
|---|---|---|
| 書式 | `cargo fmt --all -- --check; echo exit=$?` | **終了コード 0**（差分なし） |
| 静的解析 | `cargo clippy -p areka-emo-text --all-targets 2>&1 \| grep -cE "^warning\|^error"; echo exit=${PIPESTATUS[0]}` | **164 行**・終了コード 0。うち `^error` は **0 件**（同じ出力を `grep -E "^error"` で数えて終了コード 1） |

clippy の 164 行の内訳（多い順・上位）は `collapsible_if` 系 60・`type_complexity` 28・
`chunks` の定数長 16・引数が多い 8・不要な参照 7 で、いずれも lint であり `error` はありません。
本仕様が増やしたぶんかどうかは main 側と比較していません（比較には `git checkout` が要り、
本タスクでは禁じられているため）。**門にはしていません。**

### 3.5 裁定後の走り直し（2026-09-06・§1.4 の是正のあと）

§1.4 の是正はコメントと文書の語を 1 つ足しただけですが、うち 1 か所は Rust の
テストファイル（`tests/kero_menu_capacity_test.rs` の `//!` doc）なので、
**それでも壊れていないことを実測で示すため**、対象クレートとワークスペース全体を走り直しました。

| 走り直した検査 | コマンド | 結果 |
|---|---|---|
| 是正したファイルのテスト | `cargo test -p areka-emo-text --test kero_menu_capacity_test 2>&1 \| grep "^test result"; echo exit=${PIPESTATUS[0]}` | **終了コード 0**・`ok. 6 passed; 0 failed; 0 ignored` |
| 対象クレート全体 | `cargo test -p areka-emo-text --no-fail-fast 2>&1 \| grep -E "^test result\|^     Running\|^error\[\|^error:"; echo exit=${PIPESTATUS[0]}` | **終了コード 0**・15 単位・合計 **571 passed / 0 failed / 2 ignored**・`error` 行 0 件（17:32:06 → 17:32:13） |
| ワークスペース全体 | `cargo test --workspace --no-fail-fast 2>&1 \| grep -E "^test result: FAILED\|^test result\|^error\[\|^error:\|^     Running unittests\|^   Doc-tests" > <一時ファイル>; echo exit=${PIPESTATUS[0]}` | **終了コード 0**・**103 単位**・合計 **7,120 passed / 0 failed / 40 ignored**・`test result: FAILED` 0 件・`^error` 0 件（17:32:22 → 17:33:18・約 56 秒。ビルド済みのため初回より短い） |
| 書式 | `cargo fmt -p areka-emo-text -- --check; echo exit=$?` | **終了コード 0**（差分なし） |

103 単位・7,120 passed・40 ignored という数は §3.1／§3.2 の初回走行と**完全に一致**します。
是正が動きを 1 つも変えていないことの裏づけです。

---

## 4. 結論

タスク 6.1 の完了状態は 4 つ。実測との対応は次のとおりです。**4 つとも成立**しています。

| # | 完了状態 | 判定 | 根拠 |
|---|---|---|---|
| 1 | 検索が 0 件 | **成立** | §1.1。3 段の検索が出力なし・**終了コード 1**（＝0 件）。design.md「機械検査（2.4）」が定めた式・除外語・対象経路は 1 文字も変えていない。第 2 段の 10 行の全文一覧は §1.2、1 行ずつの分類は §1.3。初回走行で残った 5 行は §1.4 の裁定（案 A・2026-09-06）に従って `旧式` の 1 語を添えて解消した（5 行はいずれも履歴・注記つき引用・改訂指示であり、`1.25` を現行の行送り係数として述べる行は元から 0 行）。より広い形の検索（旧式の式の形と旧識別子）でも製品コードの 0 件を §1.5 で明示している |
| 2 | 見張りが例外表無改変のまま緑 | **成立** | §2.3。`git diff main...HEAD` と `git status` の双方で当該ファイルの差分ゼロ、例外表 11 件に `areka-emo-text` は 0 件、テスト 6 本緑・終了コード 0 |
| 3 | 両テストが終了コード 0 | **成立** | §3.1・§3.2（初回走行）と §3.5（裁定後の走り直し）。クレートは 571 passed / 0 failed・終了コード 0、ワークスペースは 103 単位・7,120 passed / 0 failed・終了コード 0。いずれも出力を絞る処理の終了コードではなく `${PIPESTATUS[0]}` で `cargo` 自身の終了コードを取っている |
| 4 | 新設 4 ファイルが規約に従い 1,000 行以下で例外表に触れない | **成立** | §2.1（4 件＝design の主張と一致・兄弟 2／`tests/` 2・最大 820 行）・§2.2（既存 8 ファイルの最大は `draw.rs` 988 行）・§2.3（例外表無改変） |

**未決はありません。** 初回走行で唯一残っていた「機械検査が文言どおりには 0 件にならない」は、
§1.4 の裁定（案 A）と 5 行への語の追加で解消し、走り直して 0 件・終了コード 1 を確認しました。

**申し送りとして残す 2 件**（どちらも欠陥ではなく、本タスクの担当範囲の外）:

- §1.6 の `crates/areka/src/emo2_boot/spine_conformance_script.rs` のコメント——旧式 35px を現在形で述べているが、
  要件 2.4 の検査対象（`crates/areka-emo-text` と正典表）に含まれず、design の機械検査の経路にも入っていない。
  引受先の `areka-P0-emo2-conformance-e2e` が走行を採り直すときに見直される。
- §2.2 の `crates/areka-emo-text/src/draw.rs` の **988 行**——上限 1,000 まで残り 12 行。
  見張りは緑だが余裕が小さいので、次にこのファイルへ足すときは分割を先に考える必要がある。

**本タスクで変更したファイル**: この証跡 1 ファイルと、§1.4 の是正で `旧式` の 1 語を添えた 3 ファイル
（`crates/areka-emo-text/tests/kero_menu_capacity_test.rs` の doc コメント 1 行・
`doc/COMPAT_ARCHITECTURE.md` の 1 行・`design.md` の 3 行）の計 4 ファイルです。
**製品コード（`crates/areka-emo-text/src/` の非テストファイル）とテストの実行部分・`tasks.md` には触れていません。**
是正はすべてコメントと文書の語であり、動きを変える変更は 1 つもありません（§3.5 の走り直しが同じ数を返しています）。

---

## 5. 作業ツリーの状態（本文書を書いた時点）

```
git status --porcelain
```

```
 M .kiro/specs/areka-P0-emo-text-line-height-canon/design.md
 M crates/areka-emo-text/tests/kero_menu_capacity_test.rs
 M doc/COMPAT_ARCHITECTURE.md
?? .kiro/specs/areka-P0-emo-text-line-height-canon/verification/task-6.1-sweep-and-tests.md
```

4 件だけで、§4 の「本タスクで変更したファイル」と一致します（`M` 3 件が §1.4 の語の追加、`??` 1 件が本文書）。
HEAD は `dec1461a` のまま——本タスクではコミットしていません。
`crates/areka-emo-text/src/` の非テストファイル・`tasks.md`・行数の見張りの例外表
（`crates/log-capture-kit/tests/file_length_guard_test.rs`）はいずれも一覧に現れません。

`git diff --stat` は 3 ファイルで **5 insertions / 5 deletions**（＝語を添えた 5 行のみ・行の増減なし）です。

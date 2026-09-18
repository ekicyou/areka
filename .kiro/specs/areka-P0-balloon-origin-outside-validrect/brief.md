# Brief: areka-P0-balloon-origin-outside-validrect

> 2026-09-18 `/kiro-discovery` 再入（同日 5 度目）で起票。開発者の指示「ukadoc を確認し、バグが認められるなら修正 spec を立ち上げるように」。
> 本文の file:line は**起票時の実測値**（2026-09-18・main `ca3c4fdc`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: バルーン定義に `origin.x,0`／`origin.y,0` を書いてある第三者のバルーンを areka で使う人。**各行の行頭が 1 文字ぶん欠けて表示される。** 同じバルーンは SSP では欠けずに読める。

実例（2026-09-18・開発者の実機目視）: ゴースト emo2 の開発版に同梱のバルーン `emo2-kakukaku` は、共通の `descript.txt` に `origin.x,0`／`origin.y,0` を書き、面別の `balloons0s.txt`／`balloonk0s.txt` が `validrect.left` を 36／24 と定める。areka で起動すると「ちがうよう。」が「がうよう。」になり、「つまり、、」の「つ」が欠けた。欠け幅は 36÷28≒1.3 文字・24÷28≒0.9 文字で、2 つのバルーンとも実測と合う。2 行を消すと直る（`未指定の origin 成分を書字開始角へ寄せる key="origin.x" corner=36.0` の記録で確認）。

areka 同梱の検体が無傷なのは、開始点の寄せ戻しを撤去した PR #124（`b9ede5ad`・2026-08-29）が**同じ PR の中で**検体 5 本から `origin` の宣言を消したからにすぎない。宣言が残っている世の中のバルーンは今も欠ける。α は第三者が自分のバルーンを持ち込む前提なので、α の範囲で直す。

## バグと認めた根拠（ukadoc）

- `origin.x`（https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#origin.x_2c_5ea7_6a19_20_2a1:1）: 「テキスト開始位置のX座標。……**通常は指定せずvalidrectの定義に任せる場合が多い。** 横書き=validrect.left 縦書き=validrect.right」。`origin.y` も同文で既定は `validrect.top`。
- `validrect`（同ページ `vertical,0/1` の項）: 「**テキストを描画してよい範囲。**」
- 範囲の外を宣言したときにどうなるかは、ukadoc のどこにも書かれていない（沈黙）。

判定:

1. **先の裁定の前提が崩れた。** 撤去の根拠は `doc/COMPAT_ARCHITECTURE.md:177` の「validrect 外の宣言は **SSP でも壊れた定義**であり、字義どおりに配置することが互換の最善手」だった。これは SSP を見ずに置いた仮定で、2026-09-18 の目視（同じ定義が SSP で欠けずに出る）が反証した。目視証跡は根拠に使える（開発者方針）。
2. **areka は範囲外の開始点を「字義どおりに表示」できていない。** areka の文字の面は validrect と同じ範囲しか持たないので、その外から書き始めた文字は表示されずに切り落とされるだけである。「宣言どおりの位置に出す」でも「描画してよい範囲に収める」でもない第三の結果で、どの作者も望まない。
3. **ukadoc の 2 文と両立する解は「範囲の内へ戻す」しかない。** 「描画してよい範囲」の外には描かない。かつ文字を欠かさない。この 2 つを同時に満たすには、開始点を validrect の内へ戻すほかない。

よって正典違反ではなく「正典の沈黙に対して置いた仮定が誤っていた」ことによる互換欠陥として、バグと認める。

## Current State

- 開始点の解決の唯一の定義点＝`crates/areka-emo-text/src/region.rs:432` の `resolve_origin_component`。宣言された成分は validrect の内外を問わず宣言どおりに返し、範囲外なら `debug!` を 1 件残すだけ（`:443-450`）。未宣言の成分だけが書字開始角へ落ちる（`:453-456`）。呼び出しは `:290`（x）と `:297`（y）。
- 撤去を固定しているテスト＝`region.rs:677` の `origin_components_resolve_literally_and_independently`（「y は 46 へ寄らない（寄っていたら旧クランプが残っている）」と明記）。モジュール冒頭の doc（`:24-37`）も撤去を正典として説明している。
- 撤去前の形（`git show b9ede5ad -- crates/areka-emo-text/src/region.rs`）＝`clamp_origin_component`。範囲内ならその値、**範囲外なら書字開始角**（最寄りの辺ではない）。成分ごとに独立。テスト名 `fixture_origin_clamps_to_start_corner`／`out_of_range_component_clamps_independently`／`negative_origin_resolves_from_opposite_edge_before_clamp`。
- 文書の登記＝`doc/COMPAT_ARCHITECTURE.md:177`（撤去の行）・`:183`（`\_l` の行。「origin クランプ撤去により……常に一致」と撤去を前提に書いてある）・`:209`（「未宣言成分だけが書字開始角へ落ちる」）。
- `\_l` の数値座標の原点は**解決後の** `origin`（`TextRegion::start()`）なので、解決の規則を変えれば `\_l` は自動で追随する。宣言された `origin` を原点に使うテスト（`layout_cursor_center_origin_tests.rs:559`・`:623`、`layout_cursor_wiring_tests.rs:192`、`cursor_tag_tests.rs:48` ほか）が**範囲内の値を使っているかどうかは未確認**（要件段階で全数を見る）。
- 範囲外の `origin` を今も宣言している検体は 1 本だけ＝`crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi/descript.txt`（`origin.x,0`／`origin.y,0`・面別層の `validrect.left,36`）。使っているのは `crates/areka/src/placement/transition_judge_offset_signoff_tests.rs` だけ（窓の配置の検査で、文字の開始点は見ていない）。
- ⚠ `region.rs` は **951 行**。テストを同じファイルへ足すと 1,000 行の番人（`log-capture-kit` の `file_length_guard_test.rs`）が赤くなる。

## Desired Outcome

- `origin` を validrect の外に宣言したバルーンでも、**行頭が欠けずに**表示される（横書き・縦書き右送り・縦書き左送りの 3 方向とも）。
- validrect の内に宣言した `origin` は、今までどおり宣言した位置から書き始める（1 ピクセルも動かない）。
- 未宣言のときの挙動も不変。areka 同梱の検体の表示は 1 ピクセルも動かない。
- 範囲外の宣言を無視したことが、バルーンの作者に分かる形で 1 度だけ記録される。
- `doc/COMPAT_ARCHITECTURE.md` §8 の撤去の行が「撤去を取り下げた」行へ改まり、取り下げの理由（前提の反証）と日付が残る。

## Approach

**範囲外に宣言された成分は「宣言が無いもの」として扱い、書字開始角へ落とす**（撤去前の形の復元。成分ごとに独立）。

- ukadoc が「通常は指定せず validrect の定義に任せる」と述べる、その通常の形へ戻すだけなので、新しい意味論を発明しない。
- 範囲の判定は今 `debug!` の判定に使っている式（両端を含む）をそのまま使える。判定の結果を返値に使うよう 1 関数を直すだけで、呼び出し側・`\_l`・折返し・描画は無改変。
- 撤去を固定しているテスト 1 本と冒頭 doc を書き直し、3 方向 × 2 成分の範囲外／範囲内／端ちょうどの表を**兄弟ファイル**へ新設する（`region.rs` へは足さない）。

採らなかった案:

| 案 | 採らない理由 |
|---|---|
| 最寄りの辺へ寄せる | 右の外に宣言すると行頭が右端に張り付き、1 文字ごとに折り返す。欠けは消えるが読めない。撤去前もこの形は採っていなかった |
| 宣言どおりの位置のまま、文字の面を validrect の外まで広げて描く | ukadoc の「描画してよい範囲」に反する。面の寸法・当たり判定・スクロール・DPI 追従へ波及して規模が L になる |
| `0` だけを未宣言として扱う | 観測した 1 件にしか効かない。`origin.x,10` と書かれれば同じ欠けが再発する |
| 現状維持（既知の非互換のまま） | α で第三者のバルーンを受け入れると同じ苦情が再来する。ゴースト側で 2 行を消せるのは作者本人だけ |

## Scope

- **In**:
  - `resolve_origin_component` の範囲外の腕を書字開始角へ落とす形に直す（成分ごとに独立・3 書字方向）
  - 範囲外の宣言を無視した旨の記録（レベルと回数は要件で決める。先例＝折返し基準が描画範囲の外に解決されたときの `warn!` 1 回・`actor.rs`）
  - 撤去を固定しているテストの書き直しと、範囲外／範囲内／端ちょうどの表の新設（兄弟ファイル）
  - 検体 `emo2-kakukaku-offsetdpi` を「範囲外を宣言した実物の検体」として 1 本の検査に使う（検体は書き換えない）
  - `\_l` の原点に宣言された `origin` を使う既存テストが範囲内の値であることの全数確認（範囲外が混じっていたら期待値を見直す）
  - `region.rs` 冒頭 doc・`doc/COMPAT_ARCHITECTURE.md:177`／`:183`／`:209` の追随
- **Out**:
  - validrect の内に宣言された `origin` の扱い（不変）
  - 折返し基準（`wordwrappoint`）が描画範囲の外に解決される件（`balloon-canon-residue` 項目 14 の所有）
  - プロパティ `currentghost.balloon.scope(ID).basepos.x`／`.y` の公開（`currentghost-property-tree`・α 後。値は解決後の開始点から導けば足りる、とだけ申し送る）
  - SSP が範囲外の `origin` を内部でどう処理しているかの実測（SSP 実測主義は取らない。根拠は ukadoc の 2 文と目視 1 件で足りる）
  - 上流 `ghost_dev` の emo2（2026-09-18 に 2 行を削除済み）

## Boundary Candidates

- **開始点の解決**（`region.rs::resolve_origin_component` の 1 関数）
- **記録**（範囲外の宣言を無視したことの 1 回きりの告知）
- **文書の追随**（冒頭 doc・COMPAT §8 の 3 行）

## Out of Boundary

- 文字の面の寸法・描画・折返し・`\_l` の解決（いずれも解決後の開始点を受け取るだけで、無改変のまま追随する）
- バルーン定義の読み取り（`areka-parsers`。`origin` は今までどおり宣言の有無を潰さずに運ぶ）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-balloon-vertical-canon`（撤去を行った側。要件 3.10／3.11 を本仕様が上書きする）・完了 `areka-P0-cursor-tag-canon`（`\_l` の原点＝解決後の `origin`）
- **Downstream**: `areka-P0-default-balloon-bundle`（既定バルーンの `origin` がどう書かれていても欠けない）・`areka-P0-alpha-release-signoff`（第三者のバルーンの持ち込み）・`areka-P0-currentghost-property-tree`（`basepos`）

## Existing Spec Touchpoints

- **Extends**: なし（新規）。ただし完了 `areka-P0-balloon-vertical-canon` の要件 3.10（宣言どおりに用いる）を上書きする。アーカイブ本体は改変せず、上書きの事実を COMPAT §8 と本仕様に記録する（先例＝同表 `:147`・`:153`・`:209`）
- **Adjacent**: `areka-P0-emo-text-canon-residue`（α 後・項目 14 が `region.rs` の定数 `BALLOON_NAME_PLACEHOLDER` を差し替える＝同じファイル。本仕様が先に着地する）・`areka-P0-balloon-canon-residue`（項目 14＝折返し基準の範囲外。別の欠陥）・α の A0 の 3 本（`nar-install` が書き換える 38 ファイルに `region.rs` は含まれない＝検体パスの参照 0 を実測。`doc/COMPAT_ARCHITECTURE.md` は行の追記どうしなので後着が取り込む）

## Constraints

- 規模は S。編集するソースは `crates/areka-emo-text/src/region.rs` の 1 関数と冒頭 doc、テストは兄弟ファイルの新設（`region.rs` は 951 行で、足すと 1,000 行の番人が赤くなる）。
- 同梱の検体の表示を 1 ピクセルも動かさないことを、修正の前後をまたいで緑のままの検査で示す（PR #124 が使った手＝開始点 sakura (36,46)／kero (24,40) の固定）。
- 「範囲外の宣言が書字開始角へ落ちる」検査は、範囲外の腕を潰して赤くなることを確かめてから採る（到達する経路を踏ませる）。
- 要件段階で決めること: ⑴ 記録のレベル（`warn!` か `debug!` か）と回数 ⑵ 端ちょうどを範囲内とみなすこと（現行の記録の判定と同じ＝両端を含む）の明文化 ⑶ COMPAT §8 `:183` の「二択が発生しない」の言い直し（解決後の開始点は今後も 1 つなので結論は変わらないが、理由の文が撤去を前提にしている）。

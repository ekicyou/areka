# Brief: areka-P0-surface-element-nesting

> 2026-10-01 `/kiro-discovery`（動く画像）の議論で起票（開発者の問い「階層化エレメントが扱えるかどうか、階層化エレメントを実際に surfaces.txt で定義できるようにしたいがどのように実現すればよいのか」）。roadmap「動く画像」節。**流れ＝手書きで定義できるようになる（本 spec）→ 画像を自動で定義へ分解する（`animated-image-playback`）**（開発者）。
> 本文の file:line は起票時（main `5e37745e`）の実測＝**着手時に引き直すこと**。

## Problem

- **シェルの作者**: 目や口などの部品を 1 つのサーフェスにまとめ、いくつもの表情のサーフェスから使い回したい。今は部品の画像を表情ごとに element定義で並べ直すしかなく、部品にまばたきなどのアニメーションを持たせると、表情ごとに同じアニメーションを書き写すことになる。
- areka の設計文書（`doc/COMPAT_ARCHITECTURE.md` の element の節・`doc/discussion-260702-m1-roadmap.md`「element に他サーフェス参照可（入れ子）→ surface 合成は再帰的」・完了 `areka-P0-emo-compose` の要件 7）は element からサーフェスを指す入れ子を前提にしているが、**型に実装されていない**。

## Current State（起票時の実測）

- **在るもの（再帰の骨組み）**:
  - `flatten_surface`（`crates/areka-emo-compose/src/plan.rs`）は pattern が指すサーフェスへ再帰する。位置のずれを足し合わせ、先祖の積み上げで循環を止める。
  - アトラスの `resolve_indirect`（`crates/areka-emo-atlas/src/manifest.rs`）は pattern の参照を推移的にたどって画像を集める。
- **無いもの**:
  - element は画像専用。`Element{layer, path: ElementPath}`（`crates/areka-parsers/src/shell/model.rs`）には描画メソッドの欄も無く、`decode.rs` は `overlay` 以外の行を読み捨てる。
  - 入れ子の内側は `PatternState` を見ない（`plan.rs` の `is_top_level=false`）＝内側のアニメーションは止まった絵になる。
  - 面の切り替えで、その窓のループ再生が全部消える（`crates/areka-seriko/src/actor.rs` のシェル面切替の分岐 → `looper.rs` の `on_surface_changed`）。
- 正典: pattern定義は描画メソッドを残し、ファイル名の欄でサーフェスの番号を指す（`animation*.pattern*,overlay,100,…`）。ukadoc の element定義の項「element定義されたサーフェスをアニメーションパーツとして用いた場合も、一枚の画像のように振る舞う(SSP 2.3.53～)」。**element からサーフェスを指す書き方は正典に無い＝areka 独自**。

## Desired Outcome

```
surface1
{
  element0,overlay,body.png,0,0
  element1,overlay,100,40,60
}

surface100
{
  element0,overlay,eye_open.png,0,0
  animation0.interval,sometimes
  animation0.pattern0,overlay,101,50,0,0
  animation0.pattern1,overlay,-1,50,0,0
}
```

- element定義のファイル名の欄に数字だけを書くと、そのサーフェスを部品として置ける。入れ子は何段でもよい。
- 子のサーフェスのアニメーションが親の中で動く。
- **子の時計は親の切り替えで巻き戻らない**。`surface1` と `surface2` がどちらも `100` を置いているとき、`\s[1]` から `\s[2]` へ替えても `100` のまばたきは途切れずに続く。

## Approach（2026-10-01 discovery の開発者確定事項）

1. **書き方**: 描画メソッドはそのまま、**ファイル名の欄が数字だけならサーフェスの番号**、それ以外は画像のファイル名。
   - 前例は正典の pattern定義。`shell-balloon` の brief の見込み（「将来のサーフェスの参照は、描画メソッドはそのまま、ファイル名の欄でサーフェスを指す」）とも合う。
   - 読み分けは 3 通り＝描画メソッドが `balloon` なら名前／欄が数字だけならサーフェス／それ以外は画像。
   - SSP では数字の名前の画像が見つからず、その部品が描かれないだけ。
   - 採らなかった書き方: `surface100`（拡張子の無い画像と区別が付かない）・新しい描画メソッド `surface`（子を overlay 以外で重ねる道を塞ぐ）。
2. **当たり判定は親へ持ち込む**（開発者確定）。置いた位置だけずらし、重なりは親の element の順（後が手前＝画家のアルゴリズム）に従う。
3. **内側のシェル内バルーン（`balloon` の element定義）は、警告をログに残して無視する**（開発者確定・最初の版）。同じ子を 2 か所に置くと同名の箱が 2 つでき、「スコープ × 名前」の行き先が決まらないため。
4. **子の時計は独立して動き、親の切り替えで巻き戻らない**（開発者確定）。持ち方は discovery の推し＝要件で確かめる:
   - 時計は「スコープ × 子サーフェス」ごとに 1 つ。同じ子を 1 つの親に 2 か所置けば、同じ時計で揃って動く。
   - 初めて表示されたときに動き出し、シェルが替わるかゴーストが降りるまで止めない。見えなくなっても捨てないので、`1 → 3 → 1` と戻っても巻き戻らない。
   - 一番上のサーフェス自身のアニメーションは、今までどおり面の切り替えで最初から（正典どおり）。
5. **循環**は今の `flatten_surface` の先祖の積み上げで止め、警告をログに残す。`element0` がサーフェスを指すときも、画像のときと同じく `surface*.png` を破棄する（正典の element0 の規則）。

## Scope

- **In**:
  - パーサ: element の中身の型（画像／サーフェスの番号）。`shell-balloon` の `balloon` と合わせて 1 つの列挙にする。
  - 合成器: element からの再帰（`flatten_surface` の流用）・位置のずれ・循環。
  - アトラス: element の参照もたどって画像を集める。
  - seriko: 子のサーフェスのアニメーションを表に載せる。子の時計を独立させ、面の切り替えで消さない。
  - 合成のキャッシュの鍵: 子の時計の状態を含める。
  - 当たり判定の持ち込み。内側の `balloon` の警告。
  - 決定論テストと試験用シェル（リポジトリ内の検体）。
  - `doc/COMPAT_ARCHITECTURE.md` §8 への areka 独自の語の登記。
- **Out**:
  - 動く画像の分解（`animated-image-playback`）・読み込み（`animated-image-decode`）。
  - pattern が指すサーフェスの内側のアニメーションを動かすか（正典は「一枚の画像のように振る舞う」＝要件で確かめる。今は止まった絵）。
  - 内側のシェル内バルーンを許すこと（最初の版は無視）。
  - 着せ替え（bind）の子への持ち込み（要件で確かめる）。

## Boundary Candidates

- パーサの element の型（`areka-parsers` の shell）＝転記だけ。
- 合成と当たり判定（`areka-emo-compose`）。
- 子の時計（`areka-seriko` の `table.rs`・`looper.rs`・`actor.rs`）。

## Out of Boundary

- 台本の再生（sakura）・`\s[...]` の意味は変えない。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。
- **Downstream**: `animated-image-playback`（動く絵を子サーフェスへ分解して置く＝本 spec の入口と独立した時計に乗る）。`balloon-element-order`（Out に挙げた「サーフェスを element定義で置く」が本 spec）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**:
  - `shell-balloon`: 同じ element の型に描画メソッド `balloon` を足す。後から着地した方が 3 通りの読み分けを揃え、内側の `balloon` の警告を入れる。
  - `animated-image-decode`: 並走する。**見張る継ぎ目＝`areka-emo-atlas/src/manifest.rs`**（こちらは element の参照のたどり、向こうはコマの鍵）。
  - `currentghost-property-tree`: `seriko.*` のプロパティ＝読むだけ。

## Constraints

- アニメのエンジンは 2 つ（sakura・seriko）のまま。1 ファイル 1,000 行。決定論テスト網羅は必達。ログ無しの失敗の経路を作らない（循環・無い番号・内側の `balloon` は警告）。入れ子の無いシェル（emo2 を含む）の見た目・合成の回数・1 コマの時間を変えない。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M〜L（**16〜22 タスク＝上限 20 の境目**。一度も切り出していないが、少しまたぐだけなら削らずに進める。24 を超えたら継ぎ目は「静的な入れ子（parser・atlas・合成・当たり判定）」と「子の時計（seriko・`PatternState`・`ComposeKey`）」）。
- brief の記述は実物と一致。**足りなかった事実**:
  - `crates/areka-emo-atlas/src/manifest.rs` の `derive` は集合の全サーフェスの element をもともと集める。入れ子で要る変更は「数字だけの element を画像の鍵から外す」こと＝`manifest.rs` には必ず触る。
  - `Element` の形を enum へ変えると、`ElementPath::new` を書く約 20 ファイル（parser・atlas・compose・present・emo-text のテスト・`emo2_boot/balloon_background_tests.rs`）へ波及する。**`ElementPath` のまま下流で読む**（例 `ElementPath::surface_ref()`）形なら波及しない＝「parser は転記・解決は下流」の原則とも合う。
  - `PatternState`（鍵は animation id）を子の時計向けに鍵ごと変えると約 40 ファイルへ波及する（`emo2_boot/adapter.rs` を含む）。
  - 外形の計算（`compute_extent`・`flatten_extent`）は合成の再帰と別の関数で、合わせて変える。
- **触るファイル**: `crates/areka-parsers/src/shell/{model.rs, decode.rs}`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-emo-compose/src/{plan.rs 730, atlas_bind.rs, hit.rs 714, fold.rs, pattern.rs}`・`crates/areka-seriko/src/{table.rs, looper.rs, actor.rs 645, state.rs}`・`crates/areka-emo-present/src/cache.rs`・検体・`doc/COMPAT_ARCHITECTURE.md` §8。テストは新しい兄弟ファイルへ（`plan_ops_tests.rs` 1374・`fold_tests.rs` 968・seriko の `bind.rs` 1043 は伸ばさない）。
- **議題**: element の型を parser の enum にするか `ElementPath` のまま下流で読むか（`shell-balloon` との並びと接触面が決まる）／子の時計の持ち方／pattern が指すサーフェスの内側を動かすか／着せ替え（bind）を子へ持ち込むか。
- **並べ方**: `shell-balloon` の後（同じ element の読み方とシェルのパーサを触る）。wintf の「兄弟の重なり順が描画と当たり判定で逆」（roadmap の覚え書き）は、子を初めて複数作る本 spec か `shell-balloon` の要件で裁定する。

## 2026-10-03 ウェーブ C3-③（予定・10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」。C3 は C2 の着地で brief が動くので、着手の前に同じウェーブの他の spec と触るファイルを照合し直す（`roadmap.md`「ウェーブ編成」の C3 の行）。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（16〜21 タスク）。上限 20 の境目は変わらず。一度も切り出していないので削らずに進める（24 を超えたときの継ぎ目は棚卸⑳のとおり「静的な入れ子（parser・atlas・合成・当たり判定）」と「子の時計（seriko・`PatternState`・`ComposeKey`）」）。下の「揃えない」を採れば 1〜2 タスク減る。
- 前提の状態: `shell-balloon` は着地済み（PR#227）＝満たす。
- 崩れた前提／古くなった位置:
  - **`shell-balloon` は `Element` の型に触らなかった**（完了 design の論点 B「B-1 の変形」）。箱は同じ文面を読む 2 つ目の転記 `areka_parsers::shell::parse_boxes` → `ShellBoxes`（新規 `crates/areka-parsers/src/shell/boxes.rs`）で拾い、畳み込みは `crates/areka-emo-compose/src/boxes.rs` の `fold_boxes`（`BoxLayout`・`BoxReport`）、読み込みは `crates/areka-emo-present/src/shell_target.rs` の `load_shell_target`。`model.rs` は無改変、`decode.rs` は `parse_targets` を `pub(super)` にしただけ、`fold.rs` は `expand_targets` を `pub(crate)` にしただけ、`areka-emo-atlas` と `plan.rs`・`hit.rs` は無改変。＝本 spec が乗る画像の側の土台（`Element{layer, path}`・`decode_elements` は `overlay` だけ・`manifest.rs` の `collect_elements` は全 element の道を鍵へ入れる・`plan.rs` の `is_top_level`）は棚卸⑳の記述のまま。
  - そのため brief の「`shell-balloon` の `balloon` と合わせて 1 つの列挙にする」「後から着地した方が 3 通りの読み分けを揃える」は前提が変わった。`decode_elements` は `overlay` の行しか読まず、`balloon` の行は `parse_boxes` の側にしか来ないので、ファイル名の欄が数字だけの `overlay` 行をサーフェスとして読むだけで 3 通りは衝突しない（名前が数字の箱 `elementN,balloon,100,…` は今後も箱）。列挙へ揃えると `ElementPath::new` を書く 29 ファイルへ波及する（棚卸⑳の 20 から増えた）。
  - 内側の `balloon` の警告の置き場所が具体になった: 箱の表は `fold_boxes` がサーフェス番号ごとに作るので、子として置かれたサーフェスの箱は黙って使われないだけになる。警告は `boxes.rs`（compose）か `shell_target.rs` の報告の 1 か所で出す。
  - wintf の兄弟の重なり順（描画は `visual_sync.rs` の `visual_hierarchy_sync_system` が先頭の子を最前面、当たり判定は `tree_iter.rs` の `DepthFirstReversePostOrder` が最後の子を最前面）は、**`shell-balloon` も裁定していない**。`shell-balloon` は窓の子を `[差し込み口, 箱…, 絵]` の順に挿し（`actor_box.rs` の `box_child_index`）、箱は絵より手前に描かれるが、当たり判定は絵が先に引かれる（`surface_hit_cells_tests.rs` の「絵が不透明なら今までどおり絵が受ける」）。窓のハンドラの前段が `shown_boxes` の四角で箱を先に見るので実害は出ていない。本 spec の子サーフェスは 1 枚の絵へ合成される（`flatten_surface`）ので窓の子は増えず、この裁定を本 spec に乗せる必要はない＝重なり順を実際に分ける `balloon-element-order` の要件へ送るのが筋。
  - `PatternState` の欄 `frames` は私有のまま（`pattern.rs`）。子の時計を別の欄で足せば、型を名指しする 82 ファイルへの波及は避けられる。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/shell/model.rs`（数字だけの判定を `ElementPath` に足すなら。下流で読むなら触らない）
  - `crates/areka-emo-atlas/src/manifest.rs`（`collect_elements`・`resolve_indirect` で数字だけの element を画像の鍵から外し、指す先へたどる）
  - `crates/areka-emo-compose/src/{plan.rs 730, atlas_bind.rs, hit.rs 714, fold.rs, pattern.rs, boxes.rs}`
  - `crates/areka-seriko/src/{table.rs, looper.rs, actor.rs 647, state.rs}`
  - `crates/areka-emo-present/src/cache.rs`（`ComposeKey`）・必要なら `shell_target.rs`（内側の箱の警告）
  - 新規: 検体（入れ子のシェル）・兄弟のテストファイル
  - `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）:
  1. 「3 通りを 1 つの列挙に揃える」をやめてよいか（推し: やめる。`Element` は画像とサーフェスの 2 通りを `ElementPath` のまま下流で読み、箱は `ShellBoxes` のまま。揃えると 29 ファイルへ波及し、`shell-balloon` が構造で守った「箱の無いシェルは前と同じ」の保証も崩れる）。
  2. 子の時計の持ち方・pattern が指すサーフェスの内側を動かすか・着せ替え（bind）を子へ持ち込むか（棚卸⑳から据え置き）。
- 見つけた穴:
  - 今の main でも、数字だけの element（`element1,overlay,100,…`）を書いたシェルは、`manifest.rs` の `collect_elements` が `100` を画像の鍵に入れ、`bake`（`lib.rs`）が `100` というファイルを読みに行って `NotFound` を記録する。黙ってはいないが、本 spec の着地までは「部品が描かれない＋読み込み失敗の記録」になる（SSP と同じ見た目）。
  - 同じ C3 の `mcp-expression-table` が表情の説明を読むために `crates/areka-parsers/src/shell/model.rs`（descript の見出しを捨てている注記の所）か `decode.rs` を触る見込み＝本 spec と同時に走らせるなら、本 spec は `model.rs`・`decode.rs` に触らない形（数字だけの判定を compose 側に置く）にする。

## 2026-10-04 ウェーブ C3-⑤（棚卸㉑）

- 段は「優先」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka-parsers/src/shell/mod.rs`（C3-⑧ が 1 行足す）・`AtlasKey`・`AtlasTable::new` の形・emo-text・`input_events/` に触らない。数字だけの判定は compose の側に置く。

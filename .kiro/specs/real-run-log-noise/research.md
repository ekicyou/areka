# 調べた結果（ギャップ分析）: real-run-log-noise

> 2026-10-29・main `932d2c30` のワークツリーで、コードを読んで確かめた（areka は起こしていない）。
> 対象は確定した requirements.md。ここに書くのは事実と案で、決めるのは要件の討議である。

## 1. 要点

- **毎フレームの 3 種類（件 1・件 2・件 3）は、出どころが 1 か所である。** 再追従の口 `refresh_actor_binding`（`crates/areka-emo-text/src/actor_attach.rs` 238 行目〜）が、作り直すかを決める材料を作るために、毎フレーム・スコープごとに `ResolvedBalloonText::resolve_with_background`（`actor_decoration.rs` 146 行目）を呼んでいる。その中で
  - `TextRegion::resolve` → `region.rs` 479 行目の `debug!`（件 1）と、432・471 行目の `debug!`、
  - `ResolvedChoiceStyle::resolve`（`actor_decoration.rs` 155 行目）→ `choice.rs` 664・674 行目の `warn!`（件 3）、
  - `ResolvedFont::resolve_with_background`（`draw.rs` 243 行目）→ その中の 283 行目で `ResolvedChoiceStyle::resolve` をもう 1 回 → 件 3 の `warn!` がもう 1 件、

  が出る。何も変わらなければ 274 行目の `debug!`（件 2）で終わる。**`draw.rs` 283 行目は描画の繰り返しではなく、この解き直しの一部である。** ゆえに件 3 は「解き直しのたびに 2 件」で、`choice.rs` の側だけを直せば `draw.rs` に触らずに止まる。
- **「変わり目に 1 回」の先例は 4 つある。** `warn_coarse_wrap_threshold`（`actor_attach.rs` 52 行目・前の領域と比べる）・`warn_ignored_origin`（`actor_decoration.rs` 103 行目・同じ形）・`note_region_collapse`（`actor_attach.rs` 83 行目・場所の集合で変わり目を覚える）・`text_scale_warned`（`crates/areka/src/emo2_boot/frame/scale_text.rs`・スコープの集合）。箱の側は `box_definition_warned`（名前ごとに 1 度）。どれも登録口（`register_actor`・`register_box`）か結線の側で数え、純粋な解決の関数は記録を数えない。
- **`purple/a/null.png` は「何も着けない」選択肢で確定した。** 同梱の emo2 の `surfaces.txt` で、この絵を持つのは `surface1414` だけで、それを使うのは `animation1403.pattern2,overlay,1414`（`animation1403.interval,bind`）だけ。`descript.txt` は `sakura.bindgroup1403.name,まばたき,----`。また、この PNG は**パレットの PNG（色の種類 3）で `tRNS` を持ち、全画素が「透明」の番号**（番号 1 の α＝0）である＝絵そのものが「透明な絵」として作られている。areka は WIC の変換前の形式（8 ビットの番号つき）を「α 無し」と読み、抜き色の道へ回す。結果はどちらも全透明で同じ。
- **起動 1 回で 2 件出る理由も分かった。** シェルの読み込み `load_shell_target` が 1 回の起動で 2 回呼ばれる——位置の採寸（`crates/areka/src/placement/measure.rs` 343 行目）と、起動の資産の組み立て（`crates/areka/src/emo2_boot/assets.rs` 334 行目）。それぞれが焼き込みを 1 回する。
- **実機の数え直しの道具は揃っている。** `real-run-kit`（`Launch::Sample { name, balloon }`・`rust_log`・`safety_exit`＝環境変数 `AREKA_APP_SMOKE_EXIT_MS` の有界の自動終了・標準出力の置き場）と、どっとさくらの確かめの起動の命令（`completed/areka-P0-dot-sakura-specimen/verification.md` 4 節）。行数を数えて上限と比べる道具は `tools/` に無い。

## 2. 今のコードと先例

### 2.1 毎フレームの呼ばれ方（件 1・件 2・件 3）

| 段 | 場所 | 呼ばれる頻度 |
|---|---|---|
| 結線の毎フレームの走査 | `crates/areka/src/emo2_boot/frame/scale_text.rs` の `run_text_scale_phase`（135 行目で `refresh_actor_scale`） | 毎フレーム・装着済みのスコープごと |
| 再追従の判定 | `actor_attach.rs` `refresh_actor_binding`（265 行目で解き直し・270 行目で比べる） | 同上 |
| 解き直し | `actor_decoration.rs` `ResolvedBalloonText::resolve_with_background` | 同上（＋装着・箱の登録） |
| 書き始めの位置 | `region.rs` `TextRegion::resolve` → `resolve_origin_component`（479 行目）・`resolve_or`（432 行目）・範囲外（471 行目） | 解き直しのたび |
| 選択肢の飾り | `choice.rs` `ResolvedChoiceStyle::resolve`（664・674 行目の `warn!`） | 解き直しのたびに 2 回（`actor_decoration.rs` 155 行目と `draw.rs` 283 行目） |

- 再追従の判定は「binding が同じ」かつ「解き直した `ResolvedBalloonText` が前と同じ」で、`TextRegion` も `ResolvedChoiceStyle` も `PartialEq` の値である。解き直しは同じ入力から同じ値を返す純粋な関数（モデル・画像の大きさ・背景色が入力）。
- 結線は、装着のときに覚えたスコープごとの同じ `BalloonModel` を毎フレーム渡す（`scale_text.rs` の doc「再パースしない」）。`BalloonModel` は `Clone`・`PartialEq`・`Eq`。
- `TextLayerRuntime`（`actor.rs` 768 行）は、場所ごとの前の解決を `layout_input` に持つ。登録口 `register_actor` は `previous = layout_input.get(&place)` で前の値を取り、前と違うときだけ警告を出す（先例の 2 つ）。

### 2.2 同じ解き直しの中に在る、ほかの「毎フレーム出うる」記録（要件の外）

同じ `resolve_with_background` の鎖には、作者の誤りがあるバルーンで**毎フレーム**出るはずの記録がほかにも在る。2026-10-10 と 10-23 の走行では数に出ていない（そのバルーンが誤りを持っていなかった）。

- `writing.rs`: `vertical` の不正な値の `warn!`（289 行目）・`writing_mode` の未知の値の `warn!`（309 行目）・2 つのキーの矛盾の `debug!`（217 行目）
- `wrap.rs`: 折返しの未知の値の `warn!`（55 行目）
- `balloon_overrides.rs`: 飛ばした値の `warn!`（68・110 行目）
- `draw.rs`: `font.height` が 0 の `warn!`（266 行目付近）

これらは件 3 と同じ形（説明は「1 件」・実際は解き直しのたび）である。本 spec の要件には入っていない（討議の項目 6）。

### 2.3 記録の文言・件数を固定しているテスト

| テスト | 固定していること | 本 spec の変更で動くか |
|---|---|---|
| `crates/areka-emo-text/tests/attach_wiring_test.rs` 749 行目 | 作り直した再追従の 1 回の捕捉で「文字層の再追従」を含む行がちょうど 1 件・`info` | 捕捉するのは作り直した呼び出しだけ。件 2 の行は「何も変わらない」呼び出しで出るので、どの案でもこの 1 件は変わらない。ただし件 2 の新しい行を「作り直した呼び出しの中」で出す形にすると 2 件になる |
| `crates/areka-emo-text/src/region_vertical_canon_tests.rs`（860 行）の 6 本（434・456・522・587・747 行目付近） | `TextRegion::resolve` 1 回の呼び出しで、範囲外・未宣言の成分ごとに `debug` ちょうど 1 件・範囲内は 0 件 | 件 1 を `trace!` へ下げる案、または解決の関数から記録を外す案では、期待値を直す。解き直しを減らす案（3 節の案 C）では動かない |
| `region_vertical_canon_tests.rs` の `resolve_counting` の説明 | 未宣言の `wordwrappoint` は `resolve_or`（432 行目）が `debug!` を出すので宣言しておく | 432 行目の扱いを変えると説明が古くなる |
| `crates/areka-emo-text/src/choice_style_resolve_tests.rs`（218 行） | 縮退の値（警告の件数は数えていない・説明に「warn は解決時 1 回」） | 説明の語だけ直す |
| `crates/areka-emo-text/src/actor_region_warn_tests.rs`（894 行・上限 1,000 に近い） | 先例の 2 つの警告の「装着 1 件・3 回の同値の再追従で 0 件・作り直すが領域同値で 0 件・値が変わると 1 件」 | 動かない。**要件 5.1 の型そのもの**だが、足す余白が無い |
| `crates/areka-emo-atlas/src/emo2_e2e.rs`・`emo2_golden.rs`・`testdata/emo2_shell_golden.txt`・`crates/areka-emo-compose/src/golden_tests_blink_static_tests.rs` | `null.png` が全透明の空の行（`EMPTY orig=382x547`）として索引表に載る | 記録だけを変える案なら動かない。検体を直す案（件 4 の ⑴）では全部動く |
| `crates/areka-emo-atlas/src/lib.rs` のテスト（405〜678 行目） | 0 寸の元画像・全透明の絵で `warn` が出る | 要件 3.2・3.3 のとおり残す |

道具（`tools/`）と `doc/` は、件 1〜4 の文言を使っていない（grep で 0 件・`doc/roadmap-overview-2026-10-16.html` の 1 件は roadmap の写しの文）。

`log-capture-kit` の `count_levels` は `trace` も数える（`LevelCounts::trace`）。「`debug` では 0 件・`trace` では出る」をテストで固定できる。

### 2.4 焼き込みの入口に届いている材料（件 4）

- 1 枚ごとに手元にあるもの: セット（`key.set`）・相対パス・元の寸法・正規化の扱い `rule`（`rule.key` が `Some`＝抜き色の道＝「α 無し」と読んだ絵）・動く絵のコマ（`frames`）・トリムの結果。
- **呼び出しの引数 `sets: &[SurfaceSet]` には、面の定義そのもの（`surfaces`・その `animations` の `patterns`）が入っている。** マニフェストの導出（`manifest.rs` の `ManifestDeriver`）は既にアニメーションの参照先を辿っているが、結果には「どの絵を・どう参照したか」を残さず、鍵（セット・相対パス）の一覧だけを返す。ゆえに「参照のされ方は届いていない」は 1 枚ごとの材料としては正しいが、同じ呼び出しの中に元の材料は在る。
- `null.png` の画素: 382×547・パレット 2 色（どちらも黒）・`tRNS` は `ff 00`（番号 0 は不透明・番号 1 は透明）・全画素が番号 1。
- 判定の「α 有り」は `decode/wic_arm.rs` の `pixel_format_has_alpha`（変換前の形式が 32 ビット以上の α つきのときだけ真）。番号つきの形式は `tRNS` があっても偽になる。

### 2.5 実機の数え直しに使えるもの（要件 4）

- `crates/real-run-kit`: `RealRun::new(exe).launch(Launch::Sample { name: "emo2", balloon: Some("emo2-kakukaku") }).rust_log(...).safety_exit(...)`・`stdout_path()`。`safety_exit` の既定は 45 秒で、`AREKA_APP_SMOKE_EXIT_MS` として子へ渡る（有界の自動終了）。32 ビットの補助が要る検体には `ensure_i686`（`tools/sync-i686-helper.ps1` と同じ揃え方）。
- 使っている先例: `crates/areka/tests/emo2_real_run.rs`（環境変数 `AREKA_EMO2_REAL_RUN` が無ければ飛ばす・印を待って閉じる）。
- どっとさくら: 検体の登記 `crates/sample-ghost-kit/src/lib.rs` の `dot_sakura`（同梱バルーン `bottle`）。SHIORI は華和梨の 32 ビット。10-23 の走行は `verification.md` 4 節の PowerShell の命令（`target\` の下に記録を書き出す）。
- 自動終了の終わり際の `WARN`（`kanade: 強制終了指示` など）は範囲外なので、数えるのは件 1〜4 の文言だけに絞る。

## 3. 要件ごとの対応表

| 要件 | 今あるもの | 足りないもの | 印 |
|---|---|---|---|
| 1.1 件 1 を毎フレーム出さない | `TextRegion::resolve` の `debug!` | 毎フレームの解き直しで出ないようにする手 | Missing |
| 1.2 件 2 を毎フレーム出さない | `refresh_actor_binding` 271 行目の `debug!` | 水準を下げるか、場所ごとに「もう知らせた」を覚える | Missing |
| 1.3 作り直しの `info` 1 件 | 289・300 行目の `info!` | 無し（変えない） | — |
| 1.4 変わり目の読める記録 | 解決の関数の `debug!` | 変わり目だけで出す仕掛け（または `trace`） | Missing（討議 1） |
| 1.5 432・471 行目 | 同じ関数の `debug!` | 討議 3 の結果しだい | Unknown |
| 1.6 結果・判定を変えない | 純粋な解決・同値の比べ | 無し | Constraint |
| 2.1〜2.4 飾りの警告を場所ごとに 1 件 | 先例 `warn_coarse_wrap_threshold`（前の値と比べる） | `ResolvedChoiceStyle` は警告の元の文字列（`mergepennot` など）を持たない＝登録口で値を出す材料が要る | Missing |
| 2.5 描き方を変えない | `resolve` の縮退 | 無し | Constraint |
| 2.6 説明を直す | `choice.rs` 640〜645 行目 | 書き直し | Missing |
| 2.7 `draw.rs` に触らない | `draw.rs` 283 行目は `resolve` を呼ぶだけ | 無し（`choice.rs` から警告を外せば足りる） | Constraint |
| 3.1 `null.png` の `warn` 0 件 | — | 見分けの条件（討議 2） | Unknown |
| 3.2・3.3・3.5・3.6 | `lib.rs` 166〜186 行目の条件 | 無し | Constraint |
| 3.4 見分けた絵の低い水準の記録 | `rule.key` の `debug!`（135 行目）が先例の形 | 1 行 | Missing |
| 4.1〜4.5 実機の数え直し | `real-run-kit`・自動終了・先例の命令 | 文言ごとに数えて上限と比べる手順（使い捨ての命令か、既存の実機テストへの追記か） | Missing（討議 7） |
| 5.1 決定論のテスト | `log-capture-kit` の `capture`・`count_levels`・先例の檻 | 新しい檻。`actor_region_warn_tests.rs` は余白が無いので別の兄弟へ | Missing |
| 5.4 今のテストの追随 | 2.3 の表 | 案しだい | Constraint |
| 5.5 新しいファイルを足さない | 兄弟のテストのファイル（`actor_scale_refresh_tests.rs` 331・`actor_decoration_tests.rs` 230・`choice_style_resolve_tests.rs` 218・`region_tests.rs` 520） | 無し | Constraint |

## 4. 作り方の案

### 案 A: 1 行ずつ水準を下げる（一番小さい）

- 件 1（479 行目）・件 2（271 行目）と、入れるなら 432・471 行目を `trace!` へ。
- 件 3 は別に直す（下の「件 3 の直し方」）。`trace!` へ下げるのは要件 2（`warn` を 1 件）に合わないため。
- 良い点: 直しは数行。`logging.md` の水準の表（「フレームごとの処理」は `trace!`）と合う。
- 悪い点: `trace` まで開けた確かめでは同じ量が出る。要件 1.4 の「変わり目の記録」は `trace` を開けたときだけ読める。`region_vertical_canon_tests.rs` の 6 本の期待値を `debug` から `trace` へ直す。2.2 の隠れた毎フレームの警告は残る。

### 案 B: 変わり目だけで出す（先例の形をそのまま使う）

- 件 1: `TextRegion::resolve` から `debug!` を外し、`TextRegion` に「未宣言で角へ寄せた成分」を運ぶ値を足す（`ignored_origin` と同じ形）。登録口で前の領域と違うときだけ 1 件出す。432・471 行目も同じ形にできる（471 行目は既に `ignored_origin` が値を運んでいる）。
- 件 2: `TextLayerRuntime` に場所の集合（「何も変わらなかったことをもう知らせた」）を 1 つ足し、登録口で消す（`note_region_collapse` の形）。
- 良い点: `debug` で開けた記録に変わり目の行が残る。
- 悪い点: 純粋な解決の記録を登録口へ移すので、`region_vertical_canon_tests.rs` の 6 本は「解決は記録 0 件・登録口で 1 件」へ書き直しになる。足す値・欄が案 A より多い。

### 案 C: 何も変わっていないフレームでは解き直さない（根の 1 か所を直す）

- `refresh_actor_binding` の先頭で、binding が同じで、かつ前に解いたときのモデルと背景色が同じなら、解き直さずに `false` を返す。違うときは今までどおり解き直して比べる。前に解いたモデルと背景色を場所ごとに覚える map を 1 つ足す（`actor.rs` は 768 行で余白がある）。
- 解き直しは純粋な関数なので、入力が同じなら結果も同じ＝作り直すかの答えは今と同じ（要件 1.6）。
- 件 1・432 行目・471 行目の `debug!` と、2.2 の隠れた毎フレームの記録は、装着と本当に変わったときだけ出るようになる（要件 1.4 の「変わり目ごとに 1 件」が今の文言のまま満たせる）。`region_vertical_canon_tests.rs` は動かない。
- 件 2 の行は残るので、案 A か案 B の件 2 の手を合わせる。件 3 も別に直す（解き直し 1 回で 2 件出るため・拡大率の変わり目でも出直すため）。
- 悪い点: 境界の節の「再追従の判定は変えない・変えるのは記録だけ」を字のとおりに読むと、判定の**書き方**に触れる（答えは変えない）。開発者に確かめる。モデルの比べ（構造体の比べ）が毎フレーム 1 回増える代わりに、解き直し（フォント・選択肢・折返し・書字方向・領域の解決）が毎フレーム無くなる。

### 件 3 の直し方（どの案でも要る・`draw.rs` には触らない）

- `ResolvedChoiceStyle::resolve`（`choice.rs`）から 2 つの `warn!` を外し、縮退した元の値（`cursor.blendmethod` の値・`cursor.style` の下線の値）を値として返す形にする。`ResolvedChoiceStyle` は `Copy` の列挙なので、文字列は `ResolvedBalloonText` の側に別の欄で運ぶのが素直（`TextRegion::ignored_origin` と同じ考え方）。
- 警告は登録口（`register_actor`）で、前の `layout_input` の値と比べて違うときだけ出す（`warn_coarse_wrap_threshold` と同じ形）。新しい状態の欄は要らない。箱は `register_box` の `box_definition_warned` の下に並べる。
- `draw.rs` 283 行目は `resolve` を呼び続けるが、警告が無くなるので何も出ない＝`emo-text-dwrite-fidelity` と重ならない。
- 欄に `balloon`（`balloon_label_of`）を足せば、ほかの 2 つの作者向けの警告と揃う（討議 4）。

### 件 4 の案

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ⑴ 検体を直す | `surface1414` から絵を外す（要素の無い面にする）か、パターンを外す | 警告の条件を変えない | emo2 は配布物に入る（`tools/package.ps1`）。`.nar` の作り直しと、2.3 の表の金の値（golden）4 か所が動く。emo2 は areka の外の作り（`vendors/sample_ghost/README.md`） |
| ⑵-a 抜き色の道で全透明になった絵は下げる | `rule.key.is_some()` を条件に足す（1 行＋低い水準の 1 行） | 一番小さい | `null.png` がこの道に来るのは、areka がパレットの `tRNS` を「α 無し」と読むからである。この読みを直す（正しい直し）と `null.png` は α の道へ移り、警告が戻る＝2 つの直しが知らないうちに絡む。抜き色の絵の全透明（作者が全面を抜き色で塗った絵）を一律に許すことにもなる |
| ⑵-b 参照のされ方で見分ける | 焼き込みの入口に届いている `sets[].surfaces` から、「その絵だけを持つ面が、`bind` のアニメーションのパターンからしか参照されていない」（着せ替えの選択肢）を求め、その全透明は下げる | 作者の意図（着せ替えの「無し」）に結び付く。抜き色か α かに左右されない | 面とアニメーションを辿る数十行。`manifest.rs`（353 行）か `lib.rs`（683 行）に置く。索引表の結果は変えない（要件 3.6） |
| ⑶ 今のまま | 理由を書く | 直しは 0 | 起動のたびに 2 件のまま |

- 2 件の重なりは、採寸と表示で `load_shell_target` を別々に呼ぶため。まとめるなら areka の側（`placement/measure.rs` と `emo2_boot/assets.rs`）で 1 つを使い回す直しになり、本 spec の範囲の外（⑶ を選ぶときの付け足しの論点）。

## 5. 規模と危うさ

- **規模: S（4〜6 タスク）**。件 3 の移し替え・件 1／件 2 の手・件 4 の条件・テストの追随・実機の数え直し。新しいファイルは要らない。案 C か ⑵-b を選ぶと各 1 タスク増える。
- **危うさ: 低〜中**。どれも先例の形の繰り返しで、外の依存も無い。中になるのは案 C（判定の書き方に触れる。答えが変わらないことを今の再追従のテスト `actor_scale_refresh_tests.rs`・`attach_wiring_test.rs` と、モデルだけが変わる場合の新しい檻で押さえる）。

## 6. 設計へ持ち越す調べもの

- 案 C を採るとき: 結線がバルーンを付け替えるとき（`register_actor_view`）に、覚えたモデルを入れ替える点。背景色の受け取り（`set_balloon_background`）の後の再追従で、比べる材料に背景色が入っていること。
- 件 3: 箱（`balloon.*`ブレス）の飾りの警告を、名前ごとに 1 度にするか、場所ごとにするか。
- 件 4 ⑵-b: `bind` 以外（`interval,always` など）のパターンから参照される全透明の絵をどう扱うか（作者の誤りとして残すのが要件 3.3 の読み）。負の番号（-1・-2）は既に画像を持たない扱い（`manifest.rs`）。
- 要件の外の発見（起票の候補・`/kiro-discovery`）: **パレットの PNG の `tRNS`（番号ごとの透明度）を areka は使わず、α 無しとして抜き色で扱う**（`wic_arm.rs` の `pixel_format_has_alpha`）。番号の一部だけが透明な絵では、作者の意図と違う結果になる見込み。`null.png` は全部が透明なので結果が同じだった。

## 7. 討議に出す項目（推奨つき）

1. **件 1・件 2 の出し方** — 推奨: **案 C（何も変わらないフレームでは解き直さない）＋件 2 の行は `trace!`**。根の 1 か所で件 1・432／471 行目・2.2 の隠れた毎フレームの記録がまとめて止まり、要件 1.4 は今の `debug` のまま「変わり目ごとに 1 件」で満たせ、`region_vertical_canon_tests.rs` も動かない。件 2 の「何も変わらなかった」は判定の行ではなく毎フレームの処理そのものなので `trace!` が水準の表どおり。境界の「判定は変えない」に触れる（答えは変えない）ので、だめなら案 A（1 行ずつ `trace!`）。
2. **`null.png` の `WARN`** — 推奨: **⑵-b（参照のされ方で見分ける）**。この絵は `bind` のアニメーション 1403（まばたき「----」）からしか使われない着せ替えの「無し」で、絵そのものも `tRNS` で透明に作ってある＝作者の意図ははっきりしている。⑵-a は 1 行で済むが、パレットの `tRNS` の読みを直した日に警告が戻る。あわせて要件の「α を持たない PNG」は「パレットの `tRNS` で全部透明に作られた PNG（areka は α 無しと読む）」へ直すのがよい。
3. **`region.rs` 432・471 行目** — 推奨: **件 1 と同じ扱いに入れる**。案 C なら手間 0。471 行目には装着 1 回の作者向けの `warn` が別に在るので、毎フレームの `debug` は読む人に何も足さない。
4. **飾りの警告の「1 回」の単位** — 推奨: **バルーンの置き場ごと・値が初めて決まったときと変わったとき 1 件（今の要件のまま）＋`balloon` の欄を足す**。前の `layout_input` と比べるだけで新しい状態は要らず、ほかの 2 つの作者向けの警告と同じ形・同じ欄になる。プロセスで 1 回は要件 2.4（付け替えで値が変わったら出し直す）に合わない。
5. **折返し基準の `WARN`** — 推奨: **今のまま残し、本 spec へは入れない**。装着 1 回に 1 件（`register_actor` で前の領域と比べる）で正しく働いており、同梱の `emo2-kakukaku` の粗さ（254＞240）を知らせる正しい警告である。要件 4 の数え直しでは数える文言から外す。持ち主の片付け（`coverage-roadmap-refresh` の範囲 6）で消えないよう、その brief に 1 行の申し送りを置くかだけ開発者に聞く。
6. **2.2 の隠れた毎フレームの記録（書字方向・折返し・上書き・`font.height`）** — 推奨: **案 C を採るなら本 spec で一緒に止まる（追加の手間 0）ので、要件に 1 項足して檻を 1 本置く**。案 A を採るなら本 spec では触らず、`/kiro-discovery` で起票する。
7. **実機の数え直しの形** — 推奨: **使い捨ての命令（どっとさくらの確かめと同じ形・`target\` の下）で 2 回回し、文言ごとの行数と上限を確かめの記録に表で残す**。記録の量が戻るのを見張るのは要件 5.1 の決定論の檻が担う。繰り返し回せる形を残したいなら、既存の `crates/areka/tests/emo2_real_run.rs` へ環境変数で開ける数え直しの 1 本を足す案もある（どっとさくらは 32 ビットの補助が要る）。上限は「スコープ数 × 成分数 × 変わり目の回数」で決め、装着とバルーンの切り替えの回数を数え直しの前提として書く。

---

## 8. 設計の段（2026-10-29・`/kiro-spec-design -y`）

> 討議の 8 つの決め（requirements.md の末尾）を前提に、コードを読み直して設計を書いた（areka は起こしていない）。

### 8.1 調べ方と使った手引き

- **調べ方**: 軽い調べ（既存の仕組みの延長）。外の依存は足さないので外部の調べはしていない。コードの読み直しは本文の主の文脈で行い、調べ係は出していない。
- **使った手引き**: `kiro-spec-design` の `design-principles.md`（境界を先に・依存の向き）・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、ステアリングの `logging.md`（水準の表・テストの捕捉は `log-capture-kit`）・`structure.md`（1 ファイル 1,000 行・テストは兄弟のファイル）・`roadmap.md` の C12 の表の ④ の行（`draw.rs` に触らない・新しいソースファイルを足さない）。

### 8.2 読み直して分かったこと

- **普通のバルーンの `layout_input` を書く口は `register_actor` だけ**（`actor_attach.rs` 150 行目）。箱は `actor_box.rs` の `register_box`・`unregister_box`・`set_box_layout` が書くが、箱は毎フレーム解き直さない（表示が変わったときの登録でだけ解く）。普通のバルーンの場所の登録を外す口は無い。→ 「解き直しの入力」の記憶は普通のバルーンの場所だけに置き、`register_actor` で消して、入力を知る 2 つの道（装着・再追従）で入れ直せば、「記憶が在れば `layout_input` はその入力から解いた値」の不変条件が保てる。
- **解き直しの入力は 3 つだけ**: `WritingMode::resolve(model)`・`ResolvedFont::resolve_with_background(model, background)`（`draw.rs` 243 行目〜。中の `ResolvedChoiceStyle::resolve`・`balloon_overrides::overrides`・`LookLayers::from_balloon` も model と背景色だけを読む）・`ResolvedChoiceStyle::resolve(model.cursor(), …)`・`TextRegion::resolve(model, image_size, mode)`・`WrapMode::resolve(model)`。外の状態（時刻・フォントの台帳・環境変数）は読まない。
- **バルーンの付け替え**（研究 6 節の持ち越し 1）: `switch.rs` の `finish_balloon` は名前・背景色・`balloon_models` を差し替えるだけで、文字の層の結び直しは同じフレームの再追従（`refresh_actor_scale` に新しい定義が渡る）が行う。記憶の入力に定義と背景色が入っているので、付け替えのフレームは必ず解き直す。結果が前と同じ（文字の設定が同じバルーン）なら今と同じく `false` で、記憶だけを新しい入力へ替える（次のフレームから省ける）。
- **`draw.rs` 283 行目の `ResolvedChoiceStyle::resolve`** は解き直しの中（`ResolvedFont::resolve_with_background`）と、箱の束の受け取り（`actor_box.rs` 96 行目の `set_box_layout`）でも呼ばれる。`resolve` から `warn!` を外せば両方とも無音になる。
- **箱の飾りの警告の単位**（持ち越し 2）: 箱の定義のほかの 2 つの警告（折返し基準・無視した `origin`）は `box_definition_warned`（箱の名前ごとに 1 度・箱の束を差し替えると数え直す）。箱の登録には前の配置の入力が毎回無い（隠す・出すで登録を外して付け直す）ので、置き場ごとの比べはできない。→ 箱は名前ごとに 1 度にそろえる。
- **着せ替えの判定の語**: `Interval::Bind` と `Interval::BindRandom`（`bind+random`）が着せ替えの仲間（emo-compose の `plan.rs` の `is_bind_interval` と同じ）。同梱の emo2 の `animation1402` は `bind+random`、`animation1403`（`null.png` を持つ `surface1414` を参照）は `bind`。`emo2.nar` の `shell/master/surfaces.txt` を読み直して確かめた（89 行目 `animation1403.pattern2,overlay,1414,0,0,0`・232 行目 `element0,overlay,purple/a/null.png,0,0` は `surface1414` の中だけ）。
- **`bind` でないパターンから参照される全透明の絵**（持ち越し 3）: 作者の誤りとして `warn` を残す（要件 3.3 の読み）。持ち主の面が 1 つでも条件を外れたら見分けない（迷ったら警告に倒す）。
- **今のテストへの影響**: `attach_wiring_test.rs` 749 行目（作り直しの呼び出しの `info` をちょうど 1 件）・`region_vertical_canon_tests.rs`（解決 1 回の `debug` の件数）・`emo2_golden.rs` と金の値・`animated_tests.rs` の全透明の檻は、どれも動かない。前提が変わるのは `choice_style_resolve_tests.rs` の説明の語（「warn は解決時 1 回」）だけ。

### 8.3 まとめ方（synthesis）

- **一般化**: 件 1・件 2・`region.rs` の 2 つの `debug!`・要件 1.7 の警告は、どれも「入力が同じなのに毎フレーム解き直す」1 つの問題の現れ。根の 1 か所（再追従の判定）を直し、解き直しの中の記録には手を入れない。
- **作るか使うか**: 新しい仕組みは作らず、先例の形を使う。前の値と比べて変わったときだけ出す（`warn_coarse_wrap_threshold`・`warn_ignored_origin`）・名前ごとに 1 度（`box_definition_warned`）・`TextRegion::ignored_origin` と同じく「純粋な層は値を運び、記録は登録の側」。焼き込みの見分けはマニフェストの導出と同じ面の索引の作り方。
- **削ったもの**: ⑴ 件 1 を変わり目だけで出すための新しい値や場所の集合（案 B）は、根の直しで要らなくなった ⑵ 飾りの縮退の値を `ResolvedBalloonText` に入れる形は採らない——入れると再追従の比べの材料が増え、飾りの値だけが違うバルーンへの付け替えで作り直しが起き、判定の答えが変わる（要件 1.6）。前の入力の記憶に定義が入っているので、そこから比べれば足りる ⑶ 数え直しの道具は `tools/` に足さない（使い捨て・討議 7）。

### 8.4 設計の決め

| 決め | 中身 | 理由 |
|---|---|---|
| 記憶の中身 | 場所ごとの（バルーンの定義・画像の原寸・背景色） | `resolve_with_background` の引数と同じ 3 つ。結線の同値は別に今どおり比べる |
| 省く条件 | 結線が同じ かつ 記憶の入力が同じ | 同じ入力から同じ値が出るので、答えは解き直して比べた場合と同じ |
| 結果が同じで入力だけ違う | `false`・記憶を新しい入力へ替える・飾りの警告を照らす | 付け替えの次のフレームから省ける。答えは今と同じ |
| 件 2 の行 | 上の段と中の段で `trace!`（文言は今のまま） | 毎フレームの処理そのもの（`logging.md` の表） |
| 飾りの警告（普通のバルーン） | 記憶を入れ直すときに前の入力の値と比べ、新しい値が在って前と違えば 1 件・`balloon` の欄 | 討議 4。前の入力の記憶をそのまま使い、新しい状態を持たない |
| 飾りの警告（箱） | `register_box` の `box_definition_warned` の分岐で 1 件 | 持ち越し 2。箱の定義のほかの警告と同じ単位 |
| 焼き込みの見分け | 持ち主の面がどれも 1 回以上参照され、参照がどれも `bind`／`bind+random` のアニメーションから | 討議 2・持ち越し 3。迷ったら警告 |
| 見分けた絵の記録 | `debug!`・欄は今の `warn!` と同じ・文言は別（「でトリム後 0 寸」を含まない） | 要件 3.4。数え直しで混ざらない |
| 数え直しの上限 | 記録から数える解き直しの回数 `R` の倍数・`R` ≤ 50 | 要件 4.3。フレーム数に比例しない。毎フレームへの戻りは `R` 自身で捕まえる |
| 触るファイルの追加 | `actor.rs`（欄 1 つ）・`actor_box.rs`（呼び出し 1 つ）・emo-atlas の `manifest.rs`・`emo2_e2e.rs` | C12 の ④ の行に無いが、同じウェーブで触る spec は無い（③ は `draw*.rs` だけ） |

### 8.5 危うさと手当て

- **解き直しが新しい入力を読むようになる**（例: `balloon-font-file` がフォントファイルの場所を読む）と、記憶に足さない限り答えが変わる。→ 設計の Revalidation Triggers の 1 つめに書き、`ResolveInputs` の説明に「`resolve_with_background` の引数と同じ 3 つ」と書く。
- **直の道 `register_actor`**（テストと見本だけ）の後の最初の再追従は、記憶が無いので今どおり解き直し、飾りの警告が「前は無い」としてもう 1 件出る。本番の結線は直の道を使わない。
- **実機の上限 `R` ≤ 50** は、台詞ごとのバルーンの面の切り替えが多い走行で超える見込みは低い（50 秒の走行）。超えたら不合格とし（要件 4.4）、行の時刻を見て出来事ごとか毎フレームかを確かめて原因を直す。

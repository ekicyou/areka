# Requirements Document

## Project Description (Input)

- **困っている人**: 利用者とゴーストの作者。当たり判定を `collisionex`（矩形・楕円・円・多角形・`region`）で書いたシェルでは、areka のマウスのイベントの Reference4（当たり判定の名前）がいつも空になる。クローディアの当たり判定は全部 `collisionex` なので、撫で（`OnMouseMove`）・ダブルクリック・ドラッグのどれでも「どこを触ったか」がゴーストに届かない（`mouse-drag-events` の実機で、R1・R2 の 4 回のドラッグの Reference4 が全部空だった）。
- **今の状態**: 読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_collisions` は、番号が数字だけの `collisionN`（矩形）だけを値にし、`collisionex` の行は記録なしに読み飛ばす。当たり判定の解決 `crates/areka-emo-compose/src/hit.rs` は矩形と名前だけを見る。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `collisionex` の行は `absent`・担当なし。
- **変えたいこと**: `collisionex*,名前,形,座標…` を読み、当たり判定の解決が形に応じて内外を判定する。`collision` と `collisionex` の重なりは今の画家の順（後に書いたものが手前）にそろえる。読めない行は記録を 1 件残す。`region` の行は読んで型へ運ぶが、働き（画像を読んで当たり判定にする）は `hit-id-pass` が持つので、この仕様では使わずに記録だけを残す。決定論のテストで形ごとの内外と重なりを固定し、実機でクローディアの撫でとドラッグの Reference4 に名前が載ることを確かめる。

（出どころ: `.kiro/specs/collisionex-regions/brief.md`。最新の測りは同 brief の「2026-11-17 棚卸㊴の再測定」の節。同じウェーブの約束の正本は `.kiro/steering/roadmap.md`「ウェーブ編成」の C22 の行。）

## Introduction

本仕様は、シェルの `surfaces.txt` の不定形当たり判定 `collisionex*` を読み、当たり判定の名前を形ごとに解決できるようにする。ukadoc（[descript_shell_surfaces の collisionex*](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#collisionex-star)）は書式を「`collisionex*,ID,タイプ,座標1,座標2...`」と書き、`*` を「同じsurface内で重複しない通し番号」、形を次の 5 つと定める。

- `rect`: 「長方形。座標は始点XYと終点XYの4つ。」
- `ellipse`: 「楕円。座標は楕円を囲む長方形、始点XYと終点XYの4つ。」
- `circle`: 「真円。中心点のXYの値、半径の順で記述する。」
- `polygon`: 「多角形。座標は各頂点、交差可。」
- `region`: 「[SSP 2.5.19～]指定した画像ファイルの指定色の領域を当たり判定として扱う。」指定の形は「ファイル名,R,G,B,領域反転(falseかtrue)」で、領域反転は「省略可能」。

名前（ukadoc の「ID」）は `collision*`（名前が最後の欄）と違い、2 番目の欄にある。

受益者は利用者とゴーストの作者であり、この仕様での「目に見える成果」は次のとおり。

- `collisionex` で書いたシェルを触ると、触った所の当たり判定の名前がマウスのイベントの Reference4 に載る（`collision*` で書いたシェルと同じ届き方）。
- 書き損じた `collisionex` の行は、黙って捨てられず、どの行がなぜ使われなかったかが記録に残る。
- `region` の行は捨てられず型に残り、後の `hit-id-pass` が読み手に触れずに働きを足せる。

本文の「読み手」は `surfaces.txt` を読んでシェルの値にする部分、「当たり判定の解決」はサーフェスの座標の点から当たり判定の名前を求める部分を指す。

## Boundary Context

- **In scope**:
  - `collisionex*` の 5 つの形（`rect`・`ellipse`・`circle`・`polygon`・`region`）の読み。`surface*`ブレスと `surface.append*`ブレスの両方。
  - 4 つの形（`rect`・`ellipse`・`circle`・`polygon`）の内外の判定と、`collision*` と `collisionex*` を合わせた重なりの順。
  - 子のサーフェス（element定義の入れ子）から持ち込む当たり判定を、形を保ったまま element定義の位置だけずらすこと。
  - 読めない行と、読めたが働かない `region` の行の記録。
  - `region` の行の欄（ファイル名・R・G・B・領域反転）を型へ運ぶこと（働きは持たない）。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 2 行（`collision*` と `collisionex*`）・`doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）への追記・決定論のテスト・実機の確かめ。
- **Out of scope**:
  - `region` の働き（画像を読んで指定色の領域を当たり判定にすること）＝`hit-id-pass`。
  - `animation*.collision*`・`animation*.collisionex*`（アニメーション中だけの当たり判定）。
  - `collision-sort`（`ascend`／`descend`）の忠実な解決。今と同じく値は読むが、並びは変えない。
  - カーソルの形（`The Hand`）。
  - マウスのイベントの送り方そのもの（どのイベントを・いつ・どの Reference で送るか）。名前が載るだけで、送り方は変えない。
  - areka 本体（`crates/areka/`）のファイル。`crates/areka/src/input_events/mod.rs` の説明の 1 行（「The Hand／collisionex は実装しない（7.4）」）は `audit-areka-input-menu` へ申し送り済みで、この仕様では直さない。
- **Adjacent expectations**:
  - 上流: 完了 `shell-parse`（読み手）・`emo-compose`（当たり判定の解決）・`surface-element-nesting`（子の当たり判定の持ち込み）・`collision-geometry`（境界を含む規則・画家の順）・`collision-dpi-hittest`（表示の拡大率で点を作者の座標へ戻す）。これらの決めはそのまま引き継ぐ。
  - 下流: `hit-id-pass`（`region` の働き）は、この仕様が型へ運んだ `region` の欄だけを使い、読み手に触れない。撫で・クリック系のイベントと、当たり判定の名前ごとのカーソルの形の行は、名前を受け取るだけで、この仕様の後も自分の働きを変えない。
  - 同じウェーブ C22 の約束（roadmap の C22 の行）: ⑴ この仕様は areka 本体のファイルに触らない ⑵ `crates/log-capture-kit/tests/file_length_guard_test.rs` の上限越えの表と数の行は `audit-test-kits` だけが触る ⑸ 依存を足さない（`Cargo.lock` の席は空き）・emo-text に新しいソースファイルを足さない。並べ方そのものは roadmap が持つ約束で、この仕様の要件ではない。
  - 同時に走らせない相手（brief の測り）: 読み手の 2 ファイルを分け合う `animated-image-import`・`element-clipping-option`・`draw-methods-canon`・`seriko-script-triggers`・`config-parse-diagnostics`・`shell-tooltip`・`surfaces-basepos`、`hit.rs` を分け合う `hit-id-pass`。

## Requirements

### Requirement 1: `collisionex` の行の読み

**Objective:** ゴーストの作者として、`collisionex` で書いた当たり判定が読まれてほしい。それにより、ukadoc どおりに書いたシェルで当たり判定が生きるため。

#### Acceptance Criteria

1. When `surface*`ブレスまたは `surface.append*`ブレスの中に `collisionex` に数字だけの番号が続く行があるとき, the 読み手 shall その行を当たり判定 1 件として、番号・名前（2 番目の欄）・形・座標を値にする。
2. When 形が `rect` または `ellipse` のとき, the 読み手 shall 3 番目から 6 番目の欄を始点 X・始点 Y・終点 X・終点 Y として読む。
3. When 形が `circle` のとき, the 読み手 shall 3 番目から 5 番目の欄を中心 X・中心 Y・半径として読む。
4. When 形が `polygon` のとき, the 読み手 shall 3 番目以降の欄を X・Y の組の並びとして書いた順に頂点として読む。
5. When 形が `region` のとき, the 読み手 shall 3 番目以降の欄をファイル名・R・G・B・領域反転として読み、領域反転の欄が無いときは `false` とする。
6. The 読み手 shall 名前を書いたまま（大小文字も空の名前もそのまま）運び、意味を解釈しない。
7. The 読み手 shall 形の名前を ASCII の大小文字を区別せずに見分ける（決め A5・SSP の測りに合わせる）。
8. The 読み手 shall 同じブレスの中の `collision*` と `collisionex*` を、ファイルに書いた順のまま 1 つの当たり判定の並びに置く。
9. The 読み手 shall 既存の `collision*`（矩形・名前が最後の欄）の読み方を変えない。

### Requirement 2: 形ごとの内外の判定

**Objective:** 利用者として、立ち絵の楕円や多角形の部位を触ったとき、その部位の名前が届いてほしい。それにより、触れ合いの台詞が触った所に応じて返るため。

#### Acceptance Criteria

1. When 点が `rect` の始点と終点で囲む範囲の中または辺の上にあるとき, the 当たり判定の解決 shall その当たり判定に当たったとする（`collision*` の矩形と同じ境界を含む規則）。
2. When 点が `ellipse` の囲みの長方形の中（辺の上を含む）にあり、かつその長方形に内接する楕円の中または周の上にあるとき, the 当たり判定の解決 shall その当たり判定に当たったとする。
3. When 点と `circle` の中心との距離が半径以下のとき, the 当たり判定の解決 shall その当たり判定に当たったとする。
4. When 点が `polygon` の頂点を書いた順に結び最後の頂点から最初の頂点へ閉じた多角形の中または辺の上にあるとき, the 当たり判定の解決 shall その当たり判定に当たったとする。
5. When `polygon` の辺が交差しているとき, the 当たり判定の解決 shall 点が中かどうかを巻き数の規則（辺を書いた順にたどったとき、点のまわりを回る数が 0 でなければ中）で決める（決め A3・SSP の測りに合わせる）。
6. If `rect` または `ellipse` の始点が終点より右または下にある（反転している）, the 当たり判定の解決 shall その当たり判定にはどの点も当たらないとする（`collision*` の矩形と同じく並べ直さない・仮定 A8）。
7. If `circle` の半径が負である, the 当たり判定の解決 shall その当たり判定にはどの点も当たらないとする（仮定 A8）。
8. While 当たり判定が `region` である, the 当たり判定の解決 shall その当たり判定にはどの点も当たらないとし、その下に重なる当たり判定の判定を続ける。
9. The 当たり判定の解決 shall 整数の座標の点で判定し、同じ当たり判定の並びと同じ点に対して常に同じ名前（または「当たり無し」）を返す。
10. If 座標や半径が極端に大きい・小さい値である, the 当たり判定の解決 shall 異常終了せず、決まった結果を返す。
11. While 表示の拡大率が 1 でない, the 当たり判定の解決 shall 窓の点を作者の座標へ戻してから 1〜8 と同じ判定をする（今の矩形と同じ戻し方）。

### Requirement 3: 重なりの順

**Objective:** ゴーストの作者として、`collision` と `collisionex` を混ぜて書いても、重なった所でどの名前が返るかが決まっていてほしい。それにより、重ねて書いた部位の当たり方を読み手の順から予測できるため。

#### Acceptance Criteria

1. When 点が複数の当たり判定に当たるとき, the 当たり判定の解決 shall `collision*` と `collisionex*` の区別なく、1 つの並びの中で後に書いたものの名前を返す（今の画家の順・仮定 A1）。
2. When `surface.append*`ブレスが当たり判定を足すとき, the 当たり判定の解決 shall 足した当たり判定を元のサーフェスの当たり判定の後ろ（手前）に置く（今の `collision*` と同じ）。
3. When 後の `surface*`ブレスが前の行と同じ番号の当たり判定を書くとき, the 読み手と当たり判定の解決 shall `collision*` と `collisionex*` を同じ番号の並び（ukadoc の collisionID）とみなし、前の行を今の `collision*` と同じ規則で置き換える（決め A2・番号の並びが 1 つであることは SSP の測りに合わせる。置き換えは areka の既存の決め）。同じブレスの中で同じ番号が 2 度書かれたときは、今の `collision*` と同じく両方を残し、後に書いた方を手前とする。
4. When サーフェスが element定義で子のサーフェスを入れ子にしているとき, the 当たり判定の解決 shall 子の当たり判定を形を保ったまま element定義の X・Y だけずらして持ち込む（矩形と楕円は始点と終点、円は中心、多角形は全部の頂点、`region` は置く位置）。
5. The 当たり判定の解決 shall 子から持ち込む当たり判定の順と、親に同じ名前があるときに持ち込まない規則を、今の `collision*` と同じにする。親の同じ名前が `region` の当たり判定であっても同じ規則を当て、`region` のための特別扱いを作らない（決め 6）。
6. The 当たり判定の解決 shall `collision-sort` の値で並びを変えない（今と同じ）。

### Requirement 4: 読めない行と働かない行の記録

**Objective:** ゴーストの作者として、書き損じた `collisionex` の行がどれで、なぜ使われなかったかを知りたい。それにより、書いた当たり判定が生きていないことに気づいて直せるため。

#### Acceptance Criteria

1. If `collisionex` の行の形の名前が 5 つの形のどれでもない, the 読み手 shall その行を当たり判定にせず、記録を 1 件残す。
2. If 形が求める数の座標が足りない（`rect`・`ellipse` は 4 つ、`circle` は 3 つ、`polygon` は 3 頂点＝6 つ）, the 読み手 shall その行を当たり判定にせず、記録を 1 件残す。
3. If 座標・半径・頂点の欄が整数として読めない, the 読み手 shall その行を当たり判定にせず、記録を 1 件残す（仮定 A4）。
4. If `polygon` の座標の数が奇数である, the 読み手 shall その行を当たり判定にせず、記録を 1 件残す（仮定 A4）。
5. If `rect`・`ellipse`・`circle` の行に形が求める数より多い座標がある、または `region` の行に領域反転より後の欄がある, the 読み手 shall 前から求める数だけを使って当たり判定にし、記録を 1 件残す（仮定 A4）。
6. If `region` の行のファイル名が空、R・G・B が 0〜255 の整数でない、または領域反転が `true`・`false` のどちらでもない, the 読み手 shall その行を当たり判定にせず、記録を 1 件残す（仮定 A4）。
7. When シェルの読み込みで `region` の行を 1 つ以上読めたとき, the 読み手 shall それらを型に運んだうえで、`region` がまだ当たり判定として働かないことを、読み込みごとに 1 件の記録にまとめ、読めた `region` の行の数と最初の行を添える（決め A6）。
8. If `collisionex` に続く番号が数字として読めない（番号が無い・桁があふれる）, the 読み手 shall 今の `collision*` と同じく、その行を飛ばして番号が読めない記録を 1 件残す。
9. The 読み手 shall 記録にブレスの見出しと行の欄を `,` で繋いだもの（字句の段が欄の前後の空白を除いた後の綴り）を添え、どの理由で使わなかったかを名指す（今のシェルの記録と同じ書き方・重さは警告）。
10. The 読み手 shall 読めない行があっても残りの行の読みを止めない。

### Requirement 5: `region` の欄を型へ運ぶ

**Objective:** 後の仕様 `hit-id-pass` の作り手として、`region` の行が読み手の型に残っていてほしい。それにより、読み手に触れずに `region` の働きを足せるため。

#### Acceptance Criteria

1. When `region` の行を読めたとき, the 読み手 shall 番号・名前・ファイル名（書いたまま）・R・G・B・領域反転を、ほかの形と同じ当たり判定の並びの中の書いた位置に置く。
2. The 読み手 shall `region` のファイル名を開かず、画像を読まない。
3. The 当たり判定の解決 shall `region` の当たり判定を並びから除かず、並びの位置（重なりの順）を保つ。

### Requirement 6: 下流への名前の届き方

**Objective:** 利用者として、`collisionex` で書いたシェルでも、`collision` で書いたシェルと同じイベントに名前が載ってほしい。それにより、撫で・クリック・ドラッグに部位ごとの反応が返るため。

#### Acceptance Criteria

1. When 当たり判定の解決が `collisionex` の当たり判定の名前を返すとき, the areka shall その名前を、`collision*` の名前と同じく、当たり判定の名前を使うマウスのイベントの Reference4 に載せる。
2. The areka shall マウスのイベントの種類・送る時・ほかの Reference の中身を、この仕様の前と変えない。
3. The areka shall 当たり判定の名前ごとのカーソルの形の行を、`collisionex` の名前に対しても `collision*` の名前と同じに引く（カーソルの働きそのものは変えない）。
4. The areka shall `collision*` だけで当たり判定を書いたシェルの当たり方と記録を、この仕様の前と変えない。

### Requirement 7: 文書と網羅台帳

**Objective:** areka の開発者として、正典が沈黙する所の決めと網羅の状態が文書に残っていてほしい。それにより、後の仕様と SSP との見比べで根拠を引けるため。

#### Acceptance Criteria

1. The 開発作業 shall 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `collisionex*` の行の状態・担当・説明を、着地した働きに合わせて書き直す。状態は `degraded`・担当は `collisionex-regions`・説明は「4 つの形は働く・`region` は型へ運び記録だけ」とする（決め A7）。
2. The 開発作業 shall 同じ台帳の `collision*` の行の状態を `degraded` のままとし、説明から「`collisionex` を読み飛ばす」旨を除いて、`degraded` の理由を「重なりの順が正典（先に書いた方が上位）と逆」に書き換える（決め A7）。
3. The 開発作業 shall `doc/COMPAT_ARCHITECTURE.md` §8 の表に、正典が沈黙する所の決め（楕円・円・多角形の境界・交差する多角形の中の決め方・形の名前の大小文字・`collision*` と `collisionex*` の番号の並び・読めない行の線引き・`region` が働かないこと）を、正典の URL と逐語の引用を添えて足す。
4. The 開発作業 shall areka 本体のファイル（`crates/areka/src/input_events/mod.rs` の説明の 1 行を含む）を書き換えない。

### Requirement 8: 確かめ

**Objective:** areka の開発者として、形ごとの内外と重なりが決定論のテストで固定され、実機でクローディアの名前が届くことを確かめたい。それにより、後の変更で当たり方が黙って変わらないため。

#### Acceptance Criteria

1. The 開発作業 shall 決定論のテストで、4 つの形のそれぞれについて中・外・境界の上の点、反転や負の半径の形、交差する多角形の点を固定する。
2. The 開発作業 shall 決定論のテストで、`collision*` と `collisionex*` を混ぜて重ねた並び・`surface.append*` で足した並び・同じ番号の置き換え・子のサーフェスから持ち込んでずらした形の当たり方を固定する。
3. The 開発作業 shall 決定論のテストで、Requirement 4 の読めない行と `region` の行のそれぞれについて、当たり判定になるか・記録が 1 件残るかを固定する。
4. The 開発作業 shall 決定論のテストで、`region` の行の欄が型に運ばれることと、`region` に点が当たらず下の当たり判定が返ることを固定する。
5. When 実機でクローディア（`sample-ghost-kit` の `claudia`）を動かすとき, the 開発作業 shall 撫で（`OnMouseMove`）とドラッグ（`OnMouseDragStart`／`OnMouseDragEnd`）の Reference4 に当たり判定の名前（`Head`・`Bust` など）が載ることを、areka の記録で確かめる。
6. When 実機で確かめるとき, the 開発作業 shall 矩形・楕円・多角形の当たり判定のそれぞれに少なくとも 1 回ずつ当て、その名前が Reference4 に載ることを確かめる（クローディアは `rect` 78・`ellipse` 32・`polygon` 28 行を持ち、`circle`・`region` は持たない＝この 2 つは 1〜4 の決定論のテストで確かめる）。
7. While 実機で画面を使って確かめる, the 開発作業 shall 排他テストの机で順を取ってから画面を操作し、終わったらすぐに机を返す。
8. The 開発作業 shall `collisionex` の行を「値にしない」と固定した既存の決定論のテスト（名前を最後の欄に書いた誤った書式のもの）を、正しい書式（名前が 2 番目の欄）で新しい振る舞いを固定するテストへ書き直す。

## 決め（2026-11-17 要件の討議・開発者が全議題を推奨案で承認）

本文の「仮定 Ax」はこの節の決め Ax を指す。

- **A1（重なりの順）**: 今の画家の順（後に書いたものが手前）を保つ。`collision*` と `collisionex*` を区別しない。正典は [caption_z-axis](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#caption_z-axis) で「あるサーフェスの当たり判定は、基本的にsurfaces.txtにおいて先に登場したcollision定義が上位となる。」と書き、画家の順はその逆（完了 `collision-geometry` の要件の討議の議題 1 の開発者の裁定を引き継ぐ）。クローディアの `surface0` では当たる 49,222 点のうち 243 点（`Face`／`Bust` と `Hair` の重なり）で SSP と違う名前になる見込み。逸脱は `doc/COMPAT_ARCHITECTURE.md` §8 に書く。
- **A2・A3・A5（SSP で測った・2026-11-17）**: 設計の前に SSP 2.9 で測った（手順と結果は `research.md` 9 節・試しのシェルは `evidence/ssp-probe-surfaces.txt`）。
  - A2（番号の並び）: **1 つ**。前のブレスの `collision4` と後のブレスの `collisionex4` を、SSP は「Surface=0/Collision=4 二重定義されています。」と警告した＝同じ番号として数える。置き換えは areka の既存の決め（`doc/COMPAT_ARCHITECTURE.md` §8「同じ面の番号が 2 つ以上の `surface*`ブレスに出たとき」＝前の同じ collision番号の行を除く）に従う。なお SSP は警告のうえで**両方を残す**（`collision4`・`collisionex4` も、`collision5` どうしも、どちらの領域も当たった）。このずれは `collisionex` に固有でなく既存の `collision*` の決めなので、この仕様では変えない。
  - A3（交差する多角形）: **巻き数の規則**（0 でなければ中）。五芒星（頂点を 1 つ飛ばしに結んだ多角形）の中心の五角形＝辺が 2 回回る所が当たった。偶奇の規則なら当たらない所なので、仮定（偶奇）を改めた。凸包の中で星の外の 2 点は当たらなかった＝凸包ではなく辺どおりの多角形。
  - A5（形の名前の大小文字）: **区別しない**。`Ellipse`・`RECT` と書いた行がどちらも当たった。
- **A4（読めない行の線引き）**: 次の行は丸ごと使わずに記録を 1 件残す。数でない座標の行、多角形の座標の数が奇数の行、`region` の欄が不正な行。多すぎる座標は前から求める数だけ使い、記録を残す。今の `collision*` の振る舞い（欠けた座標を黙って 0 にする）は変えない。0 で埋めた形が別の所に当たる害を避けるため。
- **A6（`region` の記録の粒度）**: シェルの読み込みごとに 1 件へまとめ、読めた `region` の行の数と最初の行を添える。ukadoc は `region` を「大量に(100～)使用すると」と書いており、行ごとに出すと記録があふれるため。
- **A7（台帳の状態）**: `collisionex*` と `collision*` はどちらも `degraded`。`collisionex*` は `region` が働かないため。`collision*` は重なりの順が正典と逆のため。どちらも台帳の語の定義（動くが正典どおりではない）に合わせた。`implemented` にしないので、正典 URL の証拠のコメントは要らない。
- **A8（反転・大きさ 0 の形）**: 測らずに areka の裁量として §8 に書く。反転した囲みの楕円と負の半径の円は何にも当たらない（今の矩形と同じく並べ直さない）。幅か高さが 0 の楕円と半径 0 の円は、境界を含む規則をそのまま当てた結果（線分・1 点）に当たる。
- **決め 6（親の `region` と子の同じ名前）**: 親に同じ名前の `region` があれば、子の同じ名前の当たり判定を持ち込まない。今の規則をそのまま当て、`hit-id-pass` が着地するまでの間のための特別扱いは作らない。

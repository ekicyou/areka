# Requirements Document

## Project Description (Input)

- **困っている人**: ゴーストの作者と利用者。`overlay`・`base` 以外の描画メソッドで書いた element定義・pattern定義が描かれない。`overlay-fast`（旧い綴り `overlayfast`）・`interpolate`・`replace`・`asis`・`reduce`・pattern定義の `base` を使うシェルでは、その部品や着せ替えやアニメーションのコマが消えるか、土台に別の絵が残る。「．さくら」のシェル `master` の面 12・22（`animation0.pattern0,base,…`・全部が透明な `surface12.png`・`surface22.png`）は、出すと何も描かれない見込みである。
- **今の状態**: 読み手（`crates/areka-parsers/src/shell/`）は `overlay` と `base` の行だけを画像の element定義にし、ほかの語の行は描けない行として 1 行 1 件の `warn!` を出す。element定義の値 `Element` には描画メソッドの欄が無い。合成の段（`crates/areka-emo-compose`）は全部の語を型として持つが、描けるのは `overlay`（premultiplied の SourceOver を CPU の整数で転写）だけで、「描ける語か」の門 9 か所がどれも「`overlay` だけ真」の 1 関数を通る。表示の段（`crates/areka-emo-present`）は、合成の答えの 1 枚の絵を描画面に載せる。当たりの α マスク・画像の書き出し・窓を出さない書き出しは、この CPU の合成の画素を読む。
- **変えたいこと**: ukadoc の `descript_shell_surfaces` が element定義・pattern定義・着せ替えで使えると書く重ね方の描画メソッド（`overlay`・`overlay-fast`／`overlayfast`・`interpolate`・`replace`・`asis`・`reduce`）と pattern定義の `base` を、areka が正典どおりに描く。合成は Direct2D の道で行い（2026-10-15 開発者の決め）、element ごとに重ね方を指した描画の木の節へ移す。読み手は描画メソッドの語を `blend-*` も含めて落とさずに運び、「描けるか」は合成の 1 か所が答える。合成の答えを読む働き（クリックの透過・当たり判定の名前・画像の書き出し）は、画面に出る絵と同じ答えを読む。決定論のテストで語ごとの画素を固定し、実機で「．さくら」の面 12・22 を確かめる。

（出どころ: `.kiro/specs/draw-methods-canon/brief.md`。最新の測りは同 brief の「2026-11-27 棚卸㊹の再測定」の節。）

## Introduction

本仕様は、シェルの `surfaces.txt` の描画メソッドのうち、重ね方の語（`overlay-fast`・`overlayfast`・`interpolate`・`replace`・`asis`・`reduce`）と pattern定義の `base` を描けるようにする。`overlay` はすでに描け、この仕様の後も同じ見え方を保つ。`blend-*`（新書式・`-fast` 付き・旧い別名 `overlaymultiply`・`overlayscreen`）は後の `draw-methods-blend` が描く。この仕様は、その語を読み手が落とさずに運び、描けない語として記録する形までを持つ。

正典は ukadoc の [描画メソッド](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#introduction_drawmethod) とその語ごとの項である。要点（要約）:

- **ベースレイヤと新規レイヤ**: どの合成系の語も「元の画像（ベースレイヤ）」と「新たな画像（新規レイヤ）」を合成する。element定義と着せ替えは、直前までの合成の結果をベースレイヤにして次を重ね、1 枚の静止画を得る。着せ替えでない通常のアニメーションでは、pattern定義の各コマがベースサーフェスと新規レイヤを合成した独立したコマで、前のコマの結果は次のコマに影響しない。
- **新規レイヤの範囲の外**は、全透明とは別の「空の領域」で、何も合成されずベースレイヤがそのまま残る（`reduce` だけは例外で、範囲の外を透明相当として扱う）。
- [`overlay`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#overlay): ベースレイヤに新規レイヤを単に重ねる。着せ替えの `add`・`bind` も同じ効果。
- [`overlay-fast`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#overlay-fast)（SSP 2.8.36・[`overlayfast`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#overlayfast) は旧称・廃止予定なし）: ベースレイヤの不透明度に応じて重ねる。不透明な所は完全に合成し、全透明な所は一切合成しない。
- [`interpolate`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#interpolate): ベースレイヤの透明度に応じて重ねる（`overlay-fast` の対）。全透明な所は完全に合成し、不透明な所は一切合成しない。
- [`replace`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#replace): 新規レイヤの範囲の中を、透過色・半透明の所も含めてベースレイヤを残さず上書きする。範囲の外はベースレイヤがそのまま残る。
- [`asis`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#asis): `overlay` に近いが、新規レイヤの透過（透過色、および `.pna` か `seriko.use_self_alpha` があるときの画像自体の α）を無視して重ねる。element定義で合成したサーフェスを他のサーフェスのアニメーションの部品として `asis` で合成したときの表示は未定義（「Windows 上では普通、透過領域は黒で表示されるだろう」）。
- [`reduce`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#reduce): ベースレイヤと新規レイヤの不透明度を掛け合わせる（有効な透過色は完全に透明として扱う）。色（RGB）は無視する。新規レイヤの範囲の外は透明相当として扱う。
- [`base`](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#base): ベースサーフェスを新規レイヤで完全に置き換え、collision もコマのサーフェスに定義されたものへ替わる。pattern定義では XY 座標を無視する。着せ替えと element定義では最初（element0・pattern0）にしか使えず、それ以外は `overlay` に読み替える。element定義では、画像の合成でない語（`base`・`move`・`insert`・`start`・`stop`・`alternativestart`・`alternativestop`）は無効で `overlay` として扱う。
- 最初の element（ベースが空のとき）では、ほとんどの語が `base` と同義になる。ただし `asis` で透過色がそのまま表示される点は有効である。

受益者はゴーストの作者と利用者であり、この仕様での「目に見える成果」は次のとおり。

- 6 つの重ね方の語と pattern定義の `base` で書いた部品・コマ・着せ替えが、正典どおりに描かれる。
- 「．さくら」の面 12・22 が `base` の土台で描かれ、読み込みのときの「描けない描画メソッド」の記録が出なくなる。
- 窓が透けて見える所はクリックが下の窓へ通り、触った所の当たり判定の名前も同じ見え方に従う（`reduce` で抜いた所・`replace` で透明に上書きした所を含む）。
- `overlay` だけで書いたシェルの見え方は変わらない。

本文の語: 「ベースレイヤ」「新規レイヤ」は上の正典の語。「合成の答え」はサーフェスを組んだ結果の絵（画面に出る絵）。「重ね方の語」は `overlay`・`overlay-fast`・`overlayfast`・`interpolate`・`replace`・`asis`・`reduce` の 7 つの綴り（意味は 6 つ）。「描けない語」は、この仕様の後も areka が描かない描画メソッドの語（`blend-*` の全部・`auto`・未知の語）。「仮定 n」は末尾の「仮定と討議の決め」の節の番号。

## Boundary Context

- **In scope**:
  - 読み手が element定義の描画メソッドの語を、`blend-*` を含めて落とさずに `Element` へ運ぶこと。画像の element定義にする語の 1 関数と、描けない行の転記を連動させること（3 つの転記が element定義の行を漏れなく重なりなく分ける）。
  - 重ね方の 6 つの意味を、element定義・pattern定義（通常のアニメーション）・着せ替えで描くこと。
  - pattern定義の `base`（通常のアニメーションと着せ替えの pattern0）。コマのサーフェスの当たり判定への切り替え（仮定 3）。
  - 「描ける語か」の門を、語ごとの 1 か所の答えに替えること。描けない語の記録の出し方。
  - 合成を Direct2D の道へ移し、合成の答えを読む働き（クリックの透過・当たり判定の名前・画像の書き出し・窓を出さない書き出し）を画面と同じ答えへ付け替えること（Requirement 7）。バルーンの背景色はバルーンのアトラスの頁を読み、シェルの合成の答えを読まないので付け替えない（`crates/areka-runtime/src/balloon_background.rs`）。
  - wintf（`crates/wintf/`）の足りない口: クリックの透過を合成の入れ物の 1 画素で決める当たりの形、描画の木を画面の外の絵へ描く口、WARP の描き手でその口を回す足場の公開（Requirement 7.7・7.8）。
  - wintf の「まとめの節」（透明から始まる 1 枚に描き、重ね方つきで親へ重ねる節）。入れ子の面を 1 枚として重ね（Requirement 2.7・正典「element合成の結果が一枚の画像のように扱われる」）、emo の描画面を原寸で合成してから拡大する（Requirement 8.1・2026-11-27 設計の討議）。
  - 画像の読み手（`crates/areka-emo-atlas`）の `asis` 用の読み（乗算せず・透過色を抜かず・縁を切らない）と、`seriko.use_self_alpha,0` の読みの疑いの確かめと直し（Requirement 2.5・2.9）。
  - 決定論のテスト（語ごとの画素）・網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の該当の行・`doc/COMPAT_ARCHITECTURE.md` §8・実機の確かめ。
- **Out of scope**:
  - `blend-*` の画素の式と、網羅台帳の `blend-*` の 55 行・旧い別名 2 行＝`draw-methods-blend`。この仕様は語を運び、描けない語として記録するところまで。
  - `move` と、アニメーションを呼ぶ語（`insert`・`start`・`stop`・`alternativestart`・`alternativestop` ほか）＝`seriko-pattern-calls`。
  - `auto`＝`element-clipping-option` の後の別件。element定義のオプション（クリップなど）の働き＝`element-clipping-option`（並びを運ぶ欄は本仕様で足す・Requirement 1.6）。
  - 外形の求め方（`extent-element-offset`）・動く画像の再生（`animated-image-playback`）・α の宣言（`self-alpha-declaration`）。
  - 描画メソッドの語の綴りの正規化（`canonical_method_name`）と登録簿（`known_method`）の作り直し。広げる語の分だけ触る。
  - wintf の上に挙げた口のほかの変更。中身の側が「画素どおり」を申告して範囲だけを描き直す仕組みは、この仕様では足さない（更新の所要の詰めは element ごとの行列の変換が入った後に測り直して決める）。
  - 表示の段（窓・スワップチェーン）の作り。合成の精度（f16）＝`compose-f16-precision`。
- **Adjacent expectations**:
  - 上流（どれも完了・決めはそのまま引き継ぐ）: `element-base-method`（語の集合の 1 関数・描けない行の警告・element定義の `base`）・`emo-compose`・`seriko-runtime`・`surface-element-nesting`・`wintf-d2d-composite-rules`（節ごとの重ね方・描画面ごとの入れ物・1 画素と全体の読み戻し・WARP のテスト）・`wintf-visual-tree`・`wintf-visual-damage-redraw`（範囲だけの描き直し）・`hit-id-pass`（当たりの ID の描き先・α 0.5 未満は当たり無し）。
  - `hit-id-pass` の申し送り: α マスクの出どころを替えるときは、クリックの透過と当たり判定の名前の解決を同じ新しい出どころへ一緒に付け替える。
  - 下流: `draw-methods-blend` は、この仕様の読み手と「描けるか」の 1 か所に乗り、読み手に触らずに `blend-*` を描く。`compose-f16-precision`・`element-clipping-option`・`ghost-alias-file` はこの仕様の後。
  - 同じウェーブ（C28）の約束: 合成の crate の `Cargo.toml` に依存を足さない（`Cargo.lock` の席は `sylphya-sqlite-kvs`）。画像の書き出しの付け替えは emo-present の `presenter/snapshot.rs`（書き出しの口）と `presenter/read.rs` の中で閉じ、返す値の形（`ComposedSurface` のバイトと寸法）を保って、本体の `crates/areka/src/mcp/dump_surface.rs` に触らない。wintf は `talk-thread-count-growth` の持つ `ecs/world/thread_registry.rs` に触らない（ほかの wintf のファイルは C28 で重ならない）。emo-atlas・compose の `world.rs`・emo-present の `hit_picture.rs`・`snapshot.rs` を触る表に足す（C28 のほかの 5 本は触らない）。seriko は `table.rs` のテストの期待 1 か所のほかに触らない。`plan_extent.rs` の本体に触らない。台本の側（`crates/areka-parsers/src/sakura/`・`crates/areka-sakura/`）に触らない。§8 と網羅台帳は別々の行だけ。

## Requirements

### Requirement 1: 描画メソッドの語を運ぶ

**Objective:** ゴーストの作者として、element定義に書いた描画メソッドの語がそのまま合成に届いてほしい。それにより、書いた語どおりの重ね方で部品が描かれるため。

#### Acceptance Criteria

1. When element定義の行が、画像を指す重ね方の語（`overlay`・`overlay-fast`・`overlayfast`・`interpolate`・`replace`・`asis`・`reduce`）・`base`・`blend-*` の語（新書式・`-fast` 付き・旧い別名）のどれかで書かれているとき, the areka shall その行を画像の element定義として読み、書かれた描画メソッドの語を落とさずに合成へ運ぶ。
2. The areka shall element定義の語を、完了 `element-base-method` の決めどおり書かれた綴りの完全一致で比べ（大文字の綴りは画像の element定義にしない）、`overlay-fast` と旧い綴り `overlayfast` を同じ意味の語として扱う（仮定 12）。
3. The areka shall element定義の各行を、今の 3 つの転記（画像の element定義・箱の転記（`balloon`）・描けない行の転記）のちょうど 1 つに分け、どれにも入らない行もどれか 2 つに入る行も作らない。
4. When 読み手が画像の element定義にする語を広げたとき, the areka shall 描けない行の転記と記録の対象から、その語の行を同じ答えで外す（2 つの判定を別々に持たない）。
5. If element定義の語が未知の語、大文字の綴り、または `add`・`bind` である, the areka shall その行を今と同じく描けない行として記録し、読み込みを失敗にしない（完了 `element-base-method` の決め）。
6. The areka shall element定義の行の座標の後に書かれたオプションの並びを、原文のまま `Element` へ運ぶ（転記だけ・働きは `element-clipping-option`）。

### Requirement 2: 重ね方の語の見え方

**Objective:** ゴーストの作者として、`overlay-fast`・`interpolate`・`replace`・`asis`・`reduce` で書いた部品・コマ・着せ替えが ukadoc の説明どおりに見えてほしい。それにより、SSP 向けに作ったシェルが areka でも同じに見えるため。

#### Acceptance Criteria

1. When 重ね方の語が `overlay` であるとき, the areka shall 新規レイヤをベースレイヤの上に、新規レイヤの不透明度に応じて単に重ねる。
2. When 重ね方の語が `overlay-fast` または `overlayfast` であるとき, the areka shall ベースレイヤが不透明な所では新規レイヤを完全に合成し、全透明な所では一切合成せず、半透明な所ではベースレイヤの不透明度が高いほど濃く合成し、合成の後の不透明度はベースレイヤの不透明度のままにする。
3. When 重ね方の語が `interpolate` であるとき, the areka shall ベースレイヤが全透明な所では新規レイヤを完全に合成し、不透明な所では一切合成せず、半透明な所ではベースレイヤの透明度が高いほど濃く合成する（新規レイヤをベースレイヤの下へ差し込んだ見え方）。
4. When 重ね方の語が `replace` であるとき, the areka shall 新規レイヤの範囲の中を、透過色の所と半透明の所も含めて新規レイヤの色と不透明度でそのまま置き換え、範囲の外のベースレイヤは変えない。
5. When 重ね方の語が `asis` であるとき, the areka shall 新規レイヤの透過（透過色と画像自体の α）を無視して、新規レイヤの範囲の中を不透明な新規レイヤの色で覆う。透過色の画素と α＝0 の画素も、その画像に書かれた本来の色で表示する（透明な縁を切る前の原寸で描く）。
6. When 重ね方の語が `reduce` であるとき, the areka shall 合成の後の不透明度を「ベースレイヤの不透明度 × 新規レイヤの不透明度」にし、色はベースレイヤのものを保ち、新規レイヤの有効な透過色の所と新規レイヤの範囲の外を不透明度 0 として扱う（ベースレイヤ 40%・新規レイヤ 60% の所は 24%）。
7. When element定義で合成したサーフェスを、他のサーフェスのアニメーションの部品として `asis` で合成するとき, the areka shall そのサーフェスの透明な所を黒（#000000）の不透明として表示する（正典が未定義とする所の areka の決め。正典の「Windows 上では普通、透過領域は黒」に合わせる）。
8. The areka shall 重ね方の語ごとの見え方を、element定義・通常のアニメーションの pattern定義・着せ替えの pattern定義のどれでも同じ規則で決める（違うのはベースレイヤが何かだけ・Requirement 3）。
9. When `seriko.use_self_alpha,0` のシェルが α を持つ絵を使うとき, the areka shall 画像に書かれた本来の色を不透明で表示し、半透明だった画素を暗くしない（今の読みが乗算済みの色に α＝255 を当てている疑い `crates/areka-emo-atlas/src/normalize.rs:76` を設計の段で確かめ、本当なら `asis` の読みと同じ所で直す）。

### Requirement 3: ベースレイヤの決まり

**Objective:** ゴーストの作者として、どの絵の上に重ねられるかが正典どおりであってほしい。それにより、`reduce` や `replace` のようにベースレイヤで答えが変わる語も期待どおりに見えるため。

#### Acceptance Criteria

1. While element定義でサーフェスを組んでいるあいだ, the areka shall 各 element を、番号の順に直前までの合成の答えをベースレイヤにして合成し、最後の答えを 1 枚の静止画としてそのサーフェスの絵にする。
2. While 着せ替えの pattern定義を合成しているあいだ, the areka shall 最初の pattern をベースサーフェスに合成し、以後の pattern は直前までの合成の答えをベースレイヤにして合成する。
3. While 通常のアニメーションのコマを合成しているあいだ, the areka shall 各コマを、そのコマより下に描くもの（ベースサーフェスと、今と同じ描く順で先に描くもの）をベースレイヤにして合成し、前のコマの答えを次のコマに持ち越さない。同時に動く複数のアニメーションは ID の順に重ね、後のアニメーションのコマのベースレイヤには先に描いたアニメーションのコマを含める（SSP の測り・`research.md` §10.3・仮定 6）。
4. When 最初の element の時点でベースレイヤが空であるとき, the areka shall `asis` 以外の重ね方の語を `base` と同じに扱い、新規レイヤを色と不透明度のままベースレイヤへ写す（`overlay-fast`・`reduce` も描く。正典の「ほとんどのものが `base` と同義」と SSP の測り・`research.md` §10.1）。`asis` は Requirement 2.5 のとおり透過色を本来の色で表示する（正典の「`asis` で透過色がそのまま表示される点は有効」に従い、透過色を抜く SSP の振る舞いは写さない・仮定 8）。
5. The areka shall ベースレイヤの外にはみ出す新規レイヤの扱いと、サーフェスの外形の求め方を今のまま変えない（外形は `extent-element-offset` の持ち分）。

### Requirement 4: pattern定義の `base`

**Objective:** ゴーストの作者として、pattern定義の `base` でサーフェス全体を別の絵に描き替えたい。それにより、パラパラ漫画のアニメーションや、土台を差し替える着せ替えが動くため。

#### Acceptance Criteria

1. When 通常のアニメーションのコマが `base` であるとき, the areka shall そのコマのあいだ、ベースサーフェスをコマが指すサーフェスの合成の答えで丸ごと置き換え、そのコマに書いた X・Y 座標を無視する。
2. When `base` のコマが続くアニメーションを再生するとき, the areka shall コマごとにサーフェス全面をそのコマの絵で描き直す（全部が透明な土台の絵の上でも、指したサーフェスの絵が見える）。
3. When 着せ替えの最初の pattern（pattern0）が `base` で、その着せ替えが ON であるとき, the areka shall ベースサーフェスを pattern0 が指すサーフェスの合成の答えで置き換え、以後の pattern をその上に合成する。
4. When 着せ替えの pattern0 以外が `base` であるとき, the areka shall その pattern を `overlay` として描く。
5. When `base` のコマまたは着せ替えでベースサーフェスが置き換わっているあいだ, the areka shall 当たり判定（領域と、当たりの ID の描き先で引く名前）を、置き換えた先のサーフェスに定義された collision に替える。「今の土台の面」は合成の 1 か所が答え、当たりの領域と ID の描き先がその答えで引く。
6. When `base` のコマが終わり、または着せ替えが OFF になったとき, the areka shall ベースサーフェスの絵と当たり判定を元のサーフェスのものへ戻す。
7. If `base` のコマが指すサーフェスが無い、または `-1` である, the areka shall 今の pattern定義の決まり（無いサーフェスの扱い）と同じに扱い、panic にも読み込みの失敗にもしない。

### Requirement 5: element定義の `base` と合成でない語

**Objective:** ゴーストの作者として、element定義に書いた `base` や合成でない語が正典どおりに読み替えられてほしい。それにより、今描けているシェルの見え方が保たれるため。

#### Acceptance Criteria

1. When element定義の語が `base` であるとき, the areka shall 今と同じく、その行を書いた X・Y の位置に `overlay` として描く（完了 `element-base-method` の決め）。
2. When 着せ替えの pattern定義の語が `add` または `bind` であるとき, the areka shall 今と同じくその pattern を `overlay` として描く（element定義の `add`・`bind` は Requirement 1.5 のとおり描けない行のまま）。
3. The areka shall element定義・着せ替えで `move` と、アニメーションを呼ぶ語を、今の読み方のまま扱う（働きは `seriko-pattern-calls` の持ち分）。

### Requirement 6: 描けない語の記録

**Objective:** ゴーストの作者として、areka が描かない描画メソッドを書いたとき、それが一度だけ分かってほしい。それにより、記録を読んで直せ、記録の洪水で他の記録が埋もれないため。

#### Acceptance Criteria

1. The areka shall 「描けるか」を描画メソッドの語ごとに 1 か所で答え、読み込みの記録・合成・当たり判定・外形の求め方のどれも同じ答えを使う。
2. When シェルの読み込みで描けない語（`blend-*` の全部・`auto`・未知の語）の element定義の行または pattern定義があるとき, the areka shall 読み込み 1 回につき 1 行 1 件、どのサーフェス・どの行・どの語かが分かる `warn!` を出し、その部品またはコマを描かない。
3. While 同じシェルを表示しているあいだ, the areka shall 合成のたびや `always` のアニメーションの周ごとに、描けない語の `warn!` を出し直さない（合成のたびの記録は `debug!` 以下）。
4. When この仕様で描けるようになった語（重ね方の語・pattern定義の `base`）の行を読み込むとき, the areka shall その行について「描けない描画メソッド」の記録を出さない。

### Requirement 7: 画面と同じ絵を読む働き

**Objective:** 利用者として、見えている絵とクリックの通り方・触った所の名前・書き出した画像が食い違わないでほしい。それにより、透けて見える所で下の窓が押せ、MCP や書き出しで見える絵が画面と同じになるため。

#### Acceptance Criteria

1. When マウスがシェルの窓の上の点にあるとき, the areka shall その点で画面に出ている合成の答えの不透明度が 0.5 未満ならクリックを下の窓へ通し、0.5 以上なら窓で受ける（重ね方の語で透明になった所を含む）。
2. When 当たり判定の名前を求めるとき, the areka shall クリックの透過と同じ合成の答えで「不透明度 0.5 未満の点は当たり無し」を確かめ、2 つの判定が同じ点で食い違わないようにする。
3. When MCP の `dump_surface` でサーフェスの画像を求められたとき, the areka shall 画面に出ている合成の答えと同じ画素を返し、今の応答の形を変えない。
4. When 窓を出さない書き出し（`--offline-dump`）でサーフェスを書き出すとき, the areka shall 画面に出す合成と同じ描き方で描いた画素を書き出す（`offline_dump/` は `wintf` を直に使わず、emo-present の口を通す）。
5. The areka shall バルーンの背景色（`\f[disable]` の文字色に使う色）を、今と同じくバルーンのアトラスの頁の画素から求め、シェルの合成を移しても答えを変えない。
6. If 合成の答えの画素が読めない, the areka shall `error!` を記録し、クリックの透過と当たり判定の名前を完了 `hit-id-pass` の「読めないとき」の扱いと同じ答えにし、panic にしない。
7. When クリックの透過を決めるとき, the areka shall その点の不透明度を、合成の入れ物から 1 画素をその場で読み戻して決め、前のフレームの値も CPU の α の表も使わない（2026-10-15 の決め ⑶）。足りない当たりの形は wintf に足す。
8. The areka shall 画面・`dump_surface`（原寸）・`--offline-dump`・合成の画素のテスト（WARP）を、wintf の同じ描画の木の描き方で描き、描き方を 2 つ持たない。足りない「画面の外の絵へ描く口」と WARP の足場は wintf で公開する。

### Requirement 8: 今のシェルの見え方と描き直しの範囲

**Objective:** 利用者として、合成のやり方が変わっても、今まで描けていたシェルの見え方と軽さが変わらないでほしい。それにより、更新の後も同じゴーストを同じ負荷で使えるため。

#### Acceptance Criteria

1. The areka shall `overlay` だけで書いたシェルの合成の答えを、今の合成の答えと色の各成分の差 1 以内に保つ（仮定 2）。
2. When まばたきなど、サーフェスの一部だけが変わったとき, the areka shall 1 回の更新の所要を移す前と移した後で同じ検体で測って記録し、移す前より重い行はその理由と一緒に記録し、範囲だけを描き直す回の答えをサーフェス全体を描き直した答えとバイト単位で一致させる。所要の詰めは element ごとの行列の変換が入った後に測り直して決める。
3. The areka shall 合成を移しても、フレームを 1 つ遅らせる形や時刻を丸める形を取らず、合成の答えを今と同じフレームで画面に出す。

### Requirement 9: 決定論のテスト

**Objective:** 開発者として、語ごとの画素がテストで固定されていてほしい。それにより、後の `draw-methods-blend` や合成の変更で見え方が黙って変わらないため。

#### Acceptance Criteria

1. The areka shall 重ね方の 6 つの意味と pattern定義の `base` のそれぞれについて、ベースレイヤが不透明・半透明・全透明の所と、新規レイヤの範囲の外の所の画素を、決定論のテストで固定する。
2. The areka shall 合成の答えの画素のテストを、CPU の描き手（WARP）で、色の各成分の許容差 1 で固定する（2026-10-15 開発者の決め・`tech.md` の Testing）。
3. The areka shall 範囲だけの描き直しの答えと全体の描き直しの答えのバイト単位の一致を、テストで固定し、Requirement 8.2 の所要の測りを記録に残す。
4. The areka shall クリックの透過と当たり判定の名前が同じ合成の答えを読むことを、重ね方の語で透明になった点を含めてテストで固定する。
5. The areka shall Requirement 1.3 の「漏れなく重なりなく分ける」をテストで固定し、Requirement 6 の記録の文面と件数（読み込み 1 回につき 1 行 1 件・描けるようになった語の行は 0 件）をテストで固定する。
6. When 今の CPU の合成の見本のテスト（`golden_tests*.rs`）が、合成を移したことで成り立たなくなったとき, the areka shall 画素の値を固める見本を WARP の見本へ移し、決定論・外形・着せ替えの有無・切り詰めの同値を固める見本を合成の計画の値のテストに書き直し、CPU の転写だけを固めるテスト（`blit_rect_tests.rs`・`presenter_partial_recompose_tests.rs`・`blit_transparent_alpha_tests.rs` ほか）を陳腐化として取り下げ、ファイルごとに理由を記す。

### Requirement 10: 台帳・文書・実機の確かめ

**Objective:** 開発者として、どの語が描けるかが台帳と文書に正しく載り、実機で確かめられていてほしい。それにより、網羅の報告と実際の働きが食い違わないため。

#### Acceptance Criteria

1. The areka shall 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の element定義・`base`・`overlay-fast`・`overlayfast`・`interpolate`・`replace`・`asis`・`reduce` の行を、この仕様の後の働きに合わせて直す（`blend-*` の行と旧い別名 2 行には触らない）。
2. The areka shall `doc/COMPAT_ARCHITECTURE.md` §8 に、正典が沈黙または未定義とする所の areka の決め（Requirement 2.7・3.3・3.4 と、仮定から決まったもの）を行として足し、pattern定義の `base` を「未対応」とする今の行を直す。
3. When 「．さくら」のシェル `master` で面 12・22 を出したとき（`\s[12]`・`\s[22]`）, the areka shall `base` の土台の絵を描き、読み込みのときの「描けない描画メソッド」の `warn!` を 0 件にする（`move` の働きは `seriko-pattern-calls` と並べて見る）。
4. When クローディアのシェルを読み込むとき, the areka shall surface28・animation100 の `base` の pattern についての「描けない描画メソッド」の記録 3 行を出さず、読み込みの記録の檻（`claudia_base_surfaces_are_full_size_and_records_are_fixed`）の期待値からその 3 行を外す（この檻の期待する描けない記録と警告以上の行は 0 行）。
5. The areka shall seriko のテストの期待（pattern定義の `base` のコマを「描けない」と確かめている 1 か所）を、この仕様の答えに合わせて直す。

## 仮定と討議の決め（2026-11-27 の要件の討議・全議題を推奨案で承認）

番号は下書きの仮定の番号。「決め」は討議で決まったこと、「残り」は設計の前に確かめること。

1. **規模と分割**: 決め＝分けない。合成の答えを読む働き（Requirement 7）を後へ出すと、画面が GPU になった後も読み手が CPU の合成を読み続け、CPU の合成が残る。規模を理由にさらに分けない。
2. **今の見本と見え方の差**: 決め＝`overlay` だけのシェルは色の各成分の差 1 以内（Requirement 8.1）。見本の扱いは Requirement 9.6。CPU の整数の SourceOver と D2D の丸めの差の最大は設計の段で WARP で測る。
3. **pattern定義の `base` の当たり判定**: 決め＝正典どおり替える（Requirement 4.5）。compose の `world.rs` と emo-present の `hit_picture.rs` まで及ぶ。
4. **`asis` の透過色の色**: 決め＝正典どおり本来の色で表示する（Requirement 2.5）。emo-atlas に `asis` で使う絵だけの読み（乗算せず・透過色を抜かず・縁を切らない）を足す。
5. **element定義で合成したサーフェスを `asis` で部品にしたとき**: 決め＝透明な所を黒の不透明にする（Requirement 2.7）。§8 に記す。
6. **通常のアニメーションのベースレイヤ**: 決め＝SSP の測り（`research.md` §10.3）どおり、ID の順に重ね、先に描いたアニメーションのコマを後のコマのベースレイヤに含める（Requirement 3.3）。
7. **element定義のオプションの並び**: 決め＝転記の欄を本仕様で足す（Requirement 1.6）。
8. **最初の element で語の答え**: 決め＝`asis` 以外は `base` と同じ（SSP の測り `research.md` §10.1 と正典が一致）。`asis` は正典の文に従い透過色を表示する。SSP 2.x は最初の `asis` で透過色を抜いたが、正典が明記する所なので写さず、§8 に「SSP と違う所」として記す（Requirement 3.4）。`interpolate` の半透明の所で SSP の赤が半分になる癖（§10.2）も写さない。
9. **窓を出さない書き出し**: 決め＝emo-present の口を通し、画面と同じ wintf の描き方で描く（Requirement 7.4・7.8）。
10. **バルーンの背景色**: 決め＝付け替えない（Requirement 7.5）。
11. **範囲だけの描き直し**: 決め＝所要を移す前と移した後で測って記録し（移す前より重い行は理由と一緒に）、答えの一致を固める（Requirement 8.2）。今も GPU の側は毎回全面を描いており、範囲だけなのは CPU の重ね直しだけ。「画素どおり」の申告はこの仕様では足さない。所要の詰めは element ごとの行列の変換が入った後に測り直して決める。
12. **語の比べ方**: 決め＝今のまま（element定義は完全一致・pattern定義は正規化）。揃えると大文字の `Overlay` の element定義の行の見え方が変わるため（Requirement 1.2）。
13. **WARP のテストの置き場**: 決め＝wintf の「画面の外の絵へ描く口」と WARP の足場を本仕様で公開し、emo-present のテストがそれを通る（Requirement 7.8・9.2）。WARP の装置は公開済みの `wintf::com::d3d11::d3d11_create_device` で作れ、`Cargo.toml` に依存を足さない。
14. **wintf の足りない口の置き場**（ギャップ分析で見つけた）: 決め＝本仕様の範囲を広げ、wintf の当たりの形（Requirement 7.7）・画面の外へ描く口・WARP の足場を本仕様で作る。C28 で wintf を触るのは `talk-thread-count-growth` の `ecs/world/thread_registry.rs` だけで、重ならない。

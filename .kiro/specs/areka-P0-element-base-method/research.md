# ギャップ分析: areka-P0-element-base-method

- 作成: 2026-10-05（`/kiro-validate-gap`）
- 対象: `requirements.md`（確定済み・本分析では変更しない）と、今のコード
- 進め方: 要件ごとに今のコードの該当箇所を読み、足りないものと選べる道を並べた。決めるのは設計の段で、ここでは材料だけを出す。
- 行番号は 2026-10-05 時点のワークツリー（`7a76cc5d`）で実際に読んだもの。

---

## 1. まとめ

- **読み手が `overlay` 以外の element定義を捨てているのが根**。`decode_elements` は第 2 欄が `overlay` と完全に一致する行だけを値にする（`crates/areka-parsers/src/shell/decode.rs:198-199`）。`Element` 型に描画メソッドの欄は無い（`crates/areka-parsers/src/shell/model.rs:78-87`）。
- **`base` を描く部分は、合成器に新しい描き方を足さなくても成り立つ**。要件 1.1・1.4 は「`overlay` と書いた同じ行と同じ絵」なので、畳み込み（`fold.rs` の `normalize_element`）へ `overlay` として届けば、位置・外形・入れ子・画像の読み込み失敗の記録まで今ある経路がそのまま働く。
- **気を付ける点は 3 つ**（要件の作者が挙げたもの）。どれも実在を確かめた（3 章）。
  - (a) `ComposeMethod::Base` の「XY 無視」の注記は pattern定義向けで、element定義の X,Y は `overlay` と同じ扱いにする必要がある。
  - (b) 合成器の `from_name` は `add`・`bind` を `Overlay` に写すので、読み手から語をそのまま通すと要件 2.1 に反して描かれる。
  - (c) 描けない描画メソッドの `element0` を面の表へ入れると、`surface*.png` を土台に敷く規則が止まり、見た目が変わる（要件 2.5 に反する）。
- **記録（要件 2）は「読み込み 1 回につき 1 件」が要点**。畳み込みは読み込み 1 回の中で何度も走るので、畳み込みの中で `warn!` を出すと繰り返す。今ある型（事実を値で返し、`load_shell_target` が 1 度だけ出す）に乗せるのが合う。
- 規模は S〜M、リスクは低〜中（理由は 6 章）。

---

## 2. 今のコードの調べ

### 2.1 読み手（`areka-parsers` の `shell`）

| 事実 | 場所 |
|---|---|
| `decode_elements` は `fields.get(1) == Some("overlay")` の行だけを `Element` にする。ほかの行は何もせず次へ進む（記録なし） | `decode.rs:191-218`（判定は `:198-199`） |
| `surface*`ブレスと `surface.append*`ブレスは同じ `decode_elements` を通る | `decode.rs:173`・`decode.rs:422` |
| element番号は `rest.parse::<u32>().unwrap_or(0)`。数字でない番号（`elementX`）は 0 に倒れる | `decode.rs:202` |
| `Element` の欄は `layer`・`path`・`x`・`y` の 4 つ | `model.rs:78-87` |
| pattern定義の側は描画メソッドを原文のまま運ぶ型 `DrawMethod` を既に持つ（`Pattern.method`） | `model.rs:145`・`model.rs:167-180` |
| 描画メソッド `balloon` の行は別の転記 `parse_boxes` が同じ文面から読む（第 2 欄が `balloon` の行だけ）。転記だけで記録はしない | `boxes.rs:1-13`・`boxes.rs:118-137`（判定は `:122`） |
| クレートは `tracing` に依存している（記録を出す道具はある） | `crates/areka-parsers/Cargo.toml:14` |
| 「`element0,base,...` は値にならない」ことを固定しているテストがある（本 spec で付け替えが要る） | `decode_tests_lenient_input_tests.rs:22-45`（同ファイルの 278 行目からのテストも `base` の吸収を前提にしている） |

### 2.2 畳み込みと面の表（`areka-emo-compose`）

| 事実 | 場所 |
|---|---|
| `normalize_element` は `Element` を `NormalizedElement` に写し、描画メソッドは常に `ComposeMethod::Overlay`、位置は `Transform::translate(element.x, element.y)`、置くものは `element_kind`（数字だけの欄はサーフェスの番号） | `fold.rs:283-291` |
| `surface*`ブレスも `surface.append*`ブレスもこの 1 関数を通る | `fold.rs:101-102`・`fold.rs:266-267` |
| 土台の絵の決定は「層 0 の element が在るか」だけを見る。在れば `surface*.png` を使わない（`shadowed`）、無ければ層 0 に敷く（`used`）。描画メソッドは見ない | `base_image.rs:98-106` |
| 外形は、画像の element を描画メソッドに関係なく数える | `plan.rs:685-713` |
| 転写命令は element の `method` をそのまま載せ、実行器が `is_implemented()` でない命令を `warn!` して飛ばす（合成のたびに出る） | `plan.rs:207-211`・`blit.rs:89-97` |
| 数字だけの欄の element は子として再帰し、当たり判定の持ち込み・入れ子の表・循環の報告の辺にもなる。どれも描画メソッドを見ない | `plan.rs:145-169`・`hit_import.rs:60-83`・`nesting.rs:75-82`・`nesting.rs:251-254` |
| 箱（`balloon`）の「画像より下の番号」の報告は `SurfaceMaster.elements` の最大の番号を見る | `crates/areka-emo-compose/src/boxes.rs:319-333` |
| 画像の束縛は `ElementKind::Image` の element を全部アトラスから引き、無ければ `warn!` | `atlas_bind.rs:30-54` |

### 2.3 描画メソッドの登録簿（`method.rs`）

| 事実 | 場所 |
|---|---|
| `is_implemented()` が真なのは `Overlay` だけ | `method.rs:139-141` |
| `known_method` は `"overlay" \| "add" \| "bind"` を `Overlay` に写す。`"base"` は `Base` | `method.rs:167`・`method.rs:172` |
| `from_name` は未知の語で `warn!` を出す（呼ぶたびに出る）。記録を出さない照会 `is_implemented_name` が別にある | `method.rs:151-158`・`method.rs:181-183` |
| 語の比べ方は「前後の空白を落とす・小文字にする・`-` と `_` を除く」 | `method.rs:130-132` |
| `Base` の注記は「ベース完全置換・XY 無視（先頭層以外は overlay 読替）」 | `method.rs:44-45` |
| `is_implemented()`／`is_implemented_name` は pattern定義の門でも使われている（コマ・pattern0・入れ子の表） | `plan.rs:426`・`plan.rs:490-491`・`nesting.rs:96`・`nesting.rs:186`・`crates/areka-seriko/src/table.rs:199` |
| 「`Overlay` だけが実装済み」を固定するテスト | `method.rs:325-339` |

### 2.4 読み込みの入口（`areka-emo-present` の `shell_target.rs`）

| 事実 | 場所 |
|---|---|
| `surfaces.txt` の解析は読み込み 1 回につき 1 度（`parse` と `parse_boxes`） | `shell_target.rs:287-288` |
| 面の表は「聞くためだけ」に 1 つ組み（`probe`）、その後スコープの数だけ `build_world` が組み直す。畳み込みと束縛の `warn!` は「組むたびに出る」と明記されている | `shell_target.rs:382`・`shell_target.rs:217-227` |
| 読み込み 1 回につき 1 度だけ出す記録は、事実を値（`BaseImageReport`・`NestReport`・`BoxReport`・`dangling`）で `ShellTarget` に載せ、`load_shell_target` が出す | `shell_target.rs:301-341`・`shell_target.rs:431-454` |
| 焼く絵の一覧は `shell.surfaces` の複製から、数字だけの欄の element を外したもの。マニフェストは渡された `Surface.elements` の全部の名前を集める | `shell_target.rs:394-399`・`crates/areka-emo-atlas/src/manifest.rs:82-90` |
| `surface.append*`ブレスの element は焼く一覧に入っていない（`shell.surfaces` だけを複製している） | `shell_target.rs:394` |

### 2.5 クローディアの実物（`target/nar-samples/manual/claudia/ghost/claudia/shell/master/`）

| サーフェス | 書き方 | 自分の番号の画像 | 今の見え方の理由 |
|---|---|---|---|
| surface6 | `element0,base,surface0.png,0,0`＋`element1,overlay,surface1000.png,112,100`（81〜82 行） | 無い | 1 行目が捨てられ、`element1` だけになる |
| surface26 | `element0,base,surface0.png,0,0`＋`element1,overlay,surface1001.png,114,100`（123〜124 行） | 無い | 同上 |
| surface11 | `element0,base,surface10.png,0,0`＋`element1,overlay,anthony_eyes11.png,130,270`（154〜155 行） | 無い | 同上 |
| surface19 | `element0,base,surface19.png,0,0`（167 行） | 在る | 1 行目が捨てられて層 0 が空き、`surface19.png` が土台に敷かれる |
| surface29 | `element0,base,surface29.png,0,0`（139 行） | 在る | 同上 |

- `overlay`・`base` 以外の描画メソッドの element定義はクローディアには無い（`base` 5 行・`overlay` 3 行だけ）。要件 2.6（警告 0 件）の実機での確かめに使える。
- **brief の説明との小さなずれ**: brief は「`element0` があると `apply_base_images` は手を出さないので surface19・29 は偶然正しく見える」と書くが、実際は逆で、`base` の行が読み手で捨てられて**層 0 が空くから** `surface19.png` が敷かれている（`base_image.rs:98-106`）。結論（偶然正しく見える）は同じだが、仕組みはこちらである。本 spec の後は `element0` が値になるので、surface19・29 は「`element0` が指す絵」で描かれ、`surface*.png` は「使わなかった」側に数えられる（見た目は同じ・記録の数は変わる＝5 章の項目 6）。

### 2.6 文書

| 事実 | 場所 |
|---|---|
| 台帳の `base` の行は `vocabulary-only`・担当なし。注記に「element の行の側は … 何も記録せずに落とす」 | `doc/ukadoc-coverage/ledger/assets.toml:6256-6265` |
| 台帳の element定義の行は `degraded`・担当 `areka-P0-shell-parse`。注記に「`overlay` 以外の `element0` を持つ面で画像が土台に使われる（正典では破棄 … 既知のずれ）」 | `doc/ukadoc-coverage/ledger/assets.toml:7011-7022` |
| `doc/COMPAT_ARCHITECTURE.md` §8 は 122 行目から。element定義に関わる既存の節は 345 行目から（動く絵） | `doc/COMPAT_ARCHITECTURE.md:122`・`:345` |
| roadmap は本 spec を `extent-element-offset`・`element-clipping-option` と同時に走らせないと書き、`collisionex-regions` が本 spec に依存する | `.kiro/steering/roadmap.md:140`・`:511-512` |

---

## 3. 要件の作者が挙げた 3 点

### (a) element定義の `base` の X,Y

- 畳み込みは element の X,Y を必ず位置にする（`fold.rs:287`）。`base` の行が `overlay` と同じ道で `NormalizedElement` になれば、X,Y は `overlay` と同じに扱われる（要件 1.1・4.4 のとおり）。
- 危ないのは、`ComposeMethod::Base` を命令（`BlitOp.method`）まで運んで実行器で `Base` 専用の枝を作る道。`method.rs:44` の注記「XY 無視」を見て X,Y を捨てる実装にすると要件 1.1 に反する。注記は pattern定義の話として書き直すか、element定義では `Base` の値を面の表へ持ち込まない形にするのが安全。
- 外形は今、画像の element の X,Y を数えていない（`plan.rs:711-712` は面の累積の位置だけを足す）。これは `extent-element-offset` の持ち物で、`base` も `overlay` と同じ穴に乗る（要件 3.5・境界のとおり）。

### (b) `add`・`bind` が黙って描かれる

- 読み手が第 2 欄をそのまま運び、畳み込みが `ComposeMethod::from_name`（または `known_method`）で解くと、`element1,add,x.png` は `Overlay` になって描かれる（`method.rs:167`）。要件 2.1 は「描けるのは `overlay`・`base`・`balloon` だけ、ほかは描かずに警告」なので反する。
- さらに `from_name` は未知の語で `warn!` を出し（`method.rs:155`）、畳み込みは読み込み 1 回の中で何度も走る（`shell_target.rs:217-227`）ので、要件 2.3（1 行につき 1 件）にも反する。
- つまり element定義の「描けるか」の判定は、合成器の登録簿（pattern定義と共用）とは別に、**element定義だけの 3 語の表**として持つ必要がある。

### (c) 描けない描画メソッドの `element0` が今の見た目を保つこと

- 今は行が読み手で消えるので層 0 が空き、`surface*.png` が土台になる（`base_image.rs:101-105`）。台帳もこの状態を「既知のずれ」として書いている（`assets.toml:7020`）。要件 2.5 はこれを**保つ**と決めている。
- 描けない行を `SurfaceMaster.elements` に入れると、次が一度に変わる。
  - 土台: 層 0 が在ると判定され `surface*.png` が使われなくなる（`base_image.rs:98-100`）。
  - 外形: 描かないのに大きさへ数えられる（`plan.rs:705-712`）。
  - 記録: 実行器の `warn!` が合成のたびに出る（`blit.rs:89-97`）。
  - 入れ子: 欄が数字だけなら子の再帰・当たり判定の持ち込み・循環の報告に入る（`plan.rs:154-168`・`hit_import.rs:61`・`nesting.rs:251-254`）。
  - 箱: 「画像より下の番号」の報告の基準が動く（`crates/areka-emo-compose/src/boxes.rs:319-321`）。
  - 焼く一覧: その名前の絵を読みに行く（`manifest.rs:87-89`）。
- したがって、描けない行は**面の表（`SurfaceMaster.elements`）と焼く一覧のどちらにも入れない**のが、要件 2.4・2.5 を 1 か所で満たす形になる。

---

## 4. 要件と今あるものの対応

凡例: **ある**＝今の経路がそのまま使える／**足りない**＝新しく要る／**制約**＝今の作りが縛る／**未確認**＝設計で確かめる

| 要件 | 要るもの | 今あるもの | 差 |
|---|---|---|---|
| 1.1・1.2 | `element0,base` を `overlay` と同じ絵・大きさで置く | `overlay` の element の全経路（`fold.rs:283-291`→`plan.rs`→`blit.rs`） | **足りない**: 読み手が行を捨てる（`decode.rs:198-199`）。届けば残りは**ある** |
| 1.3 | `element0` が `base` なら `surface*.png` を使わない | 層 0 が在れば使わない規則（`base_image.rs:98-100`） | 届けば**ある** |
| 1.4 | `element1` 以降の `base` も `overlay` と同じ | 同上の経路（番号を問わない） | 届けば**ある** |
| 1.5 | `surface.append*`ブレスでも同じ | 読み手も畳み込みも共用（`decode.rs:422`・`fold.rs:101-102`） | 届けば**ある**。**制約**: 追記の element の絵は焼く一覧に入らない（`shell_target.rs:394`）ので、ほかで名指しされない絵は今の `overlay` でも束縛されない。`overlay` と同じ振る舞いなので要件 1.5 とは合うが、テストの絵の置き方に注意 |
| 1.6 | 数字だけの欄は番号として読む | `element_kind`（`nesting.rs:33-43`）・焼く一覧からの除外（`shell_target.rs:394-399`） | 届けば**ある** |
| 1.7 | 画像を読めないときの記録が `overlay` と同じ | 焼く段の脱落の `warn!`（`shell_target.rs:315-317`）・束縛の `warn!`（`atlas_bind.rs:43-49`） | 届けば**ある** |
| 2.1・2.2 | 描けない行ごとに、サーフェスの番号・element番号・語を持つ警告 | 事実を値で返して入口が出す型（`NestIssue`＝`nesting.rs:214-227`・`shell_target.rs:431-454`） | **足りない**: 行を拾う所も、報告の値も、記録も無い |
| 2.3 | 読み込み 1 回・1 行につき 1 件 | `load_shell_target` が 1 度だけ出す場所（`shell_target.rs:301-341`） | **制約**: 畳み込み・束縛・実行器で出すと繰り返す（`shell_target.rs:217-227`・`blit.rs:89-97`） |
| 2.4・2.5 | その行だけ描かず、見え方は前と同じ | 今は読み手で消えるので結果としてそうなっている | **制約**: 3 章 (c)。面の表・焼く一覧に入れない |
| 2.6 | 描ける語だけのシェルで 0 件 | — | **足りない**（判定の表）。`balloon` の行（`boxes.rs:122`）を描ける側に数えること |
| 3.1 | `overlay` だけのサーフェスは不変 | 既存のテスト群（`golden_tests*.rs`・`plan_*_tests.rs`） | **ある**（檻）。`Element` に欄を足す場合は構造体リテラルの書き足しが要る（6 章） |
| 3.2 | `element0,base,surfaceN.png` は前と同じ絵 | 2.5 節 | 絵・大きさは同じになる見込み。**未確認**: 記録の数（`used`／`shadowed`）が変わる（5 章の項目 6） |
| 3.3 | pattern定義の `base` は今のまま（描かず警告） | `is_implemented()` の門（`plan.rs:426`・`:490-491`・`nesting.rs:96`・`:186`） | **制約**: `is_implemented()` に `Base` を足すと pattern定義の `base` が描かれ始める |
| 3.4 | `element0` の無いサーフェスの土台は不変 | `base_image.rs:101-105` | **ある** |
| 3.5 | 外形の規則は変えない | `plan.rs:662-758` | **ある**（触らない） |
| 4.1 | DLL なしで大きさと画素を判定 | メモリ上の復号器 `MemoryDecoder` を使う既存のテスト（`composer_tests.rs:99-140`・`shell_target_base_image_tests.rs:106-120`） | **足りない**（テストの追加）。型は**ある** |
| 4.2 | 警告が 1 件・0 件であることを判定 | 記録を捕まえる道具 `capture_logs`（`log_capture.rs:34`）と、報告の値を直接比べるテスト（`nesting_report_tests.rs`） | **足りない**（テストの追加）。型は**ある** |
| 4.3 | 無改変のクローディアで実機確認 | 検体は `target/nar-samples/manual/claudia/` に在る | 実機の手順は tasks の段 |
| 4.4 | `COMPAT_ARCHITECTURE.md` §8 に 1 行 | §8 の表（`:122` から） | **足りない**（追記） |
| 4.5 | 台帳の 2 行を更新 | `assets.toml:6256-6265`・`:7011-7022` | **足りない**（書き換え）。状態の語と担当は設計で決める（5 章の項目 7） |

---

## 5. 設計で決めること（要件の討議に回す項目）

1. **`Element` に描画メソッドの欄を足すか**。足すと「読み手は転記だけ」の決めごと（`model.rs:143-145` の pattern定義と同じ型）に揃うが、`areka_parsers::shell::Element` の構造体リテラルが 6 クレートに 40 か所あまり在り（`Element {` の直後に `layer:` が続く所を数えて 46——`areka-parsers` 21・`areka-emo-compose` 10・`areka-emo-atlas` 7・`areka-emo-present` 4・`areka-emo-text` 3・`areka` 1。ほとんどはテストで、製品コードで確かめたのは `decode.rs:200` と `shell_target.rs:624`）、全部に 1 行足すことになる。足さない道（6 章の案 A）もある。ただし brief の Scope（In）は「読み手の `Element` へ描画メソッドの欄を足すこと」と書いているので、案 A を選ぶなら brief からの外れとして設計に理由を書く（要件の討議で追記）。
2. **`base` をどこで `overlay` と同じにするか**。読み手／畳み込み（`fold.rs:283-291`）／合成器（`is_implemented` と実行器）の 3 か所が候補。合成器で `Base` を「実装済み」にすると pattern定義の門（`plan.rs:426`・`:490-491`・`nesting.rs:96`・`:186`）が連動して要件 3.3 に反するので、その道を選ぶなら門を分ける必要がある。
3. **描けない行をどこで落とし、報告をどの値で運び、どこで記録するか**。3 章 (c) から、面の表と焼く一覧の両方に入れない必要がある。記録は `load_shell_target`（`shell_target.rs:301-341`）で 1 度だけ出すのが今の型。報告の値を読み手で作るか（行の単位）、畳み込みで作るか（展開した番号の単位）で次の項目が変わる。
4. **複数番号の見出しと追記での「1 行につき 1 件」の数え方**。`surface0,1 { element0,replace,… }` は 1 行が 2 つのサーフェスに効く。要件 2.1 は「サーフェスの番号を含む」、2.3 は「1 行につき 1 件」。1 件に番号を並べるのか、番号ごとに 1 件か、見出しの代表の番号だけか（要件の討議で「1 行は 1 件・見出しの番号をすべて載せる」と要件 2.1・2.3 に明記した。残るのは値の持ち方だけ）。相手の無い `surface.append*`ブレスの中の行（面の表に届かない）で出すかどうかも同じ問い。
5. **語の比べ方**。読み手の今の判定は完全一致（`decode.rs:198`）で、合成器は空白・大小文字・`-`・`_` を無視する（`method.rs:130-132`）。`Base`・` base ` を `base` と読むか。今 `Overlay`（大文字）は捨てられているので、比べ方を緩めると `overlay` の側の振る舞いも変わる（要件 3.1 との兼ね合い）。`add`・`bind` は描かない側（3 章 (b)）。
6. **`element0,base,surfaceN.png` で記録の数が変わることを認めるか**。surface19・29 は絵は同じだが、`surface*.png` が「使った」から「`element0` が在るので使わなかった」へ移り、`debug!` が 2 行増え、`info!` の `used`／`shadowed` の数が変わる（`shell_target.rs:301-306`・`:335-341`）。要件 3.2 は絵と大きさだけを縛っているので反しないが、明示しておくのがよい。
7. **台帳の状態の語と担当**。element定義の行は `degraded` のまま注記と担当を変えるのか、状態も変えるのか。`base` の行は「element定義では描ける・pattern定義では未対応」をどの状態の語で表すか。`ComposeMethod::Base` の注記（`method.rs:44-45`）と `fold.rs:279-282` の「M1 固定 Overlay」の注記の直しも同じ時に。
8. **今の縮退を固定しているテストの付け替え**。`decode_tests_lenient_input_tests.rs:22-45`（と 278 行目からのテスト）は「`base` の element は値にならない」を固定している。`boxes_tests.rs:180` の「画像の element は `overlay` だけ」も文面の確認が要る。消すのでなく、新しい約束（`base` は値になる・ほかは落ちて報告に載る）へ書き替える。
9. **element番号が数字でない行**。`elementX,base,…` は番号 0 に倒れる（`decode.rs:202`）ので、`base` を読むと土台を置き換える側に入る。`overlay` でも今そうなので「`overlay` と同じ」には合うが、警告に載せる element番号を原文で持つか数で持つかに関わる（箱の側は原文の文字列で持つ＝`crates/areka-parsers/src/shell/boxes.rs:58-60`）。

---

## 6. 実装の道の候補

### 案 A: 読み手だけを広げる（`Element` は変えない）

- **内容**: `decode_elements` の判定を「`overlay` または `base`」にする（`decode.rs:198`）。`Element` は今の 4 欄のまま。描けない行は、`parse_boxes` と同じ形の「2 つ目の転記」（同じ文面から、描けない element定義の行だけを見出しの記述子つきで並べる純粋な関数）を足し、`load_shell_target` が `parse` の隣で呼んで（`shell_target.rs:287-288`）記録する。
- **触る所**: `decode.rs`・読み手の新しい小さな転記（または `boxes.rs` の隣）・`shell_target.rs`（記録）・テスト・文書。
- **良い点**: 変更が最小。構造体リテラル 40 か所あまりに触らない。面の表・合成器・焼く一覧は 1 行も変わらないので、3 章 (a)(b)(c) の罠に入る道が無い。`is_implemented()` も不変で要件 3.3 が自動で保たれる。
- **悪い点**: `Element` に描画メソッドが残らないので、「`base` と書かれていた」ことは下流から見えない（brief の「読み手に描画メソッドの欄を足す」とは違う形になる。要件は欄の有無を縛っていない）。同じ文面を 3 度なめる。後続の `element-clipping-option` がオプションの転記で `Element` を広げるなら、そのとき欄を足す作業が改めて要る。
- **規模・リスク**: S（1〜3 日）・低。

### 案 B: `Element` に欄を足し、畳み込みでふるい分ける

- **内容**: `Element` に `method: DrawMethod` を足し、読み手は element定義の全部の行を転記する（`balloon` の行をどうするかは要決定——画像の `Element` に入れると `boxes_tests.rs:180` の約束が変わる）。畳み込みの `normalize_element`（`fold.rs:283-291`）の手前で、element定義だけの表（`overlay`・`base` → `ComposeMethod::Overlay`、`balloon` → 黙って除く、ほか → 除いて報告へ）でふるい分け、描けない行は `SurfaceMaster.elements` に入れない。報告は `NestReport` と同じく面の表から値で取り出し（`world.rs:171-173` の型）、`load_shell_target` が 1 度だけ出す。焼く一覧の除外（`shell_target.rs:394-399`）にも同じ表を当てる。
- **触る所**: `model.rs`・`decode.rs`・`fold.rs`・報告の新しい値（`nesting.rs` の隣か新しい小さなモジュール）・`world.rs`・`shell_target.rs`・構造体リテラルの全部・テスト・文書。
- **良い点**: 読み手が「転記だけ」に揃う（pattern定義と同じ）。描けるかの判定が下流の 1 か所に集まる。`element-clipping-option` など後続が同じ欄・同じふるいに乗れる。
- **悪い点**: ふるいを通らずに生の `Shell.surfaces[].elements` を読む所が残ると罠 (c) に落ちる。今の読み手は `fold.rs`・`shell_target.rs:394-399`・`manifest.rs:82-90`（`areka-emo-atlas` のテスト用の入口 `emo2_e2e.rs:100`・`:214`・`emo2_golden.rs:81` も生の `Shell` を渡す）。複数番号の見出しでは畳み込みが番号ごとに走るので、5 章の項目 4 の数え方を決めないと 1 行が複数件になる。リテラルの書き足しが広い。
- **規模・リスク**: M（3〜7 日）・中（ふるいの漏れが見た目の変化として出る。檻で押さえられる）。

### 案 C: 案 B ＋ 合成器で `Base` を実装済みにする

- **内容**: 案 B に加えて `ComposeMethod::Base` を `NormalizedElement.method`・`BlitOp.method` まで運び、`is_implemented()` と実行器（`blit.rs:89`）に `Base` の枝を足す。
- **良い点**: 「合成器が `base` を知っている」形になり、将来 pattern定義の `base` を実装するときの足場になる。
- **悪い点**: `is_implemented()` は pattern定義の門と共用（2.3 節）なので、そのままでは pattern定義の `base` が描かれ始めて要件 3.3 に反する。門を element 用と pattern 用に分ける改造が要る。`method.rs:44` の「XY 無視」と element定義の X,Y の扱い（3 章 (a)）を実行器で分ける必要も出る。要件 1.1・1.4 は「`overlay` と同じ絵」なので、`Base` の枝は `Overlay` と同じ画素の式になり、得るものが少ない。`method.rs:325-339`・`blit.rs:704-712` のテストも付け替えになる。
- **規模・リスク**: M・中〜高（pattern定義の側への波及）。

### 見立て（決定ではない）

- 要件を満たすだけなら案 A が最短で、罠に入らない。brief の「読み手に描画メソッドの欄を足す」と後続（`element-clipping-option`）の足場を取るなら案 B。案 C は本 spec の範囲（pattern定義の `base` は対象外）に対して重い。
- どの案でも共通: 記録は `load_shell_target` の 1 か所・描けない行は面の表と焼く一覧に入れない・`is_implemented()` は変えない。

---

## 7. 設計の段へ持ち越す調べもの

- **字句解析が欄の前後の空白を落とすか**（`lexer.rs` は本分析で読んでいない）。5 章の項目 5 の比べ方に関わる。
- **ukadoc の `add`・`bind` の項が element定義について何と書いているか**。要件 2.1 は描ける語を 3 つに限っているので結論は変わらないが、警告の文面と台帳の注記で正しく書くために引き直す（`ukadoc:descript_shell_surfaces` の描画メソッドの節）。
- **`areka-emo-present/src/balloon.rs:614` と `areka-seriko/src/resolve.rs:226` の `parse` の呼び出し**が `Surface.elements` をどう使うか（案 B でふるいの漏れになるかどうか）。本分析では呼び出しの在ることだけを確かめた。
- **surface0.png・surface10.png の実寸**（要件 4.3 の 333×500 の出どころの確認。実機の段で `dump_surface` を使えば取れる）。
- **実機の確かめ方**: クローディアは `collisionex` だけで書かれている（`surfaces.txt:84-92` ほか）ので、当たり判定は本 spec の確認対象に入れない（`collisionex-regions` の持ち物）。

---

## 8. 次の段

- 要件の討議（`/kiro-requirements-discussion areka-P0-element-base-method`）で 5 章の 9 項目を扱う。
- その後 `/kiro-design areka-P0-element-base-method`。

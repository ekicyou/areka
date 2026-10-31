# 設計の検証: script-small-tags

- 検証の日: 2026-10-31
- 基準: ブランチ `claude/kiro-start-script-small-tags-97dc1c` の `5cdbe73f`（design.md を作ったコミット）
- 読んだ物: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C14 と「1 本だけの席」の行）・C14 の兄弟の brief 4 本（`choice-ranges-one-function`・`placement-measure-bake-once`・`emo-atlas-palette-alpha`・`popup-menu-residue`）・設計が名前を挙げたソース
- 進め方: 対話なしで最後まで行った（議題は設計の討議へ渡す）

## 1. まとめ

設計は、今ある 3 つの流れ（台本の命令・`\!` の汎用の合図と受け手の表・面ごとのバルーンの絵）に小さく足す形で、要件 1〜8 の全部の条件に担う物を割り当てている。設計が名前を挙げた関数・型・行の位置・数（受け手の表の 40 件・純粋の一覧の 81 本）は、どれもコードの実物と合った。直すべき所は 3 つで、どれも形を変えずに設計の文を少し直せば済む。判定は **GO**（3 つを設計の討議で片づけてからタスクへ進む）。

## 2. 確かめたこと（コードの実物との照合）

### 2.1 ファイルの行数（上限 1,000 行）

設計が直すファイルは全部 1,000 行未満で、足す量を見込んでも越えない。

| ファイル | 今の行数 | 備考 |
|---|---|---|
| `crates/areka-seriko/src/actor.rs` | 908 | 一番近い。設計は「950 行を超えるなら `actor_bind.rs` へ出す」と逃げ道を書いている |
| `crates/areka-emo-text/src/draw.rs` | 841 | 欄 1 つだけ |
| `crates/areka-emo-text/src/actor.rs` | 825 | 欄 1 つだけ |
| `crates/areka-parsers/src/balloon/model.rs` | 790 | 読み口 1 つ |
| `crates/areka-emo-text/src/state.rs` | 755 | 腕の頭に数行 |
| `crates/areka-emo-text/src/viewbox_draw_render.rs` | 661 | 塗る段を足す。いちばん増える見込みだが余裕あり |
| そのほか（`decode.rs` 510・`compile.rs` 625・`bind.rs` 583・`consumer_ledger.rs` 529 など） | 500 台以下 | 問題なし |

### 2.2 設計が「ある」と言った名前

次は全部コードに在った: `fold_choice_marker`・`is_choice_marker`・`ReadNote::MarkerIgnored`・`DetectionPoint::ReadMarkerIgnored`・`apply_named_bind`・`parse_bind_directive`・`BindDirective::{Apply, Toggle, CategoryWide}`・`category_parts_in_order`・`current_binds`・`NamedBindResult`・`prefix_chain`・`SeriesFamily`・`MarkerChain`・`MarkerImage`・`ElementDecoder`・`AlphaParams`・`set_balloon_markers`・`select_balloon_face`・`glyph_style_advance`（送り幅を決める唯一の点）・`highlight_rect`・`bitmap_of`・`scan_sysvar`・`substitute_system_vars`・`try_register`・`RAISE_TAG_CARRIER`・`register_actor_binding`・`refresh_actor_binding`。`seriko` の `actor.rs` の 633 行目（壊れた運び物）・656 行目（担当外）・670〜677 行目（トグル形とカテゴリ単位形の読み飛ばし）も設計のとおり。

### 2.3 受け手の表の数と見本

- `consumer_ledger_tests.rs` の 298 行目の手書きの数は `40`、330 行目と 342 行目に「40 件共存下でも」の文がある。40 → 42 の書き直しの位置は設計のとおり。
- `consumer_ledger_agreement_tests.rs` の `samples()` は `("bind", None, vec!["腕", "伸び", "1"])` の形で、設計の足す 2 行と同じ形。見本の檻が調べるのは emo2 の受け口だけで、`Seriko` と `TextLayer` は外（`emo2_sink_of` が `None`）＝足した 2 行は「どの emo2 の受け口にも届かない」で緑になる。

### 2.4 C14 のウェーブの約束

- `crates/areka/src/emo2_boot/assets.rs` は触らない: 守れる。`ScopeBalloonDefinitions` の欄は非公開で、`assets.rs` が使うのは `ScopeBalloonDefinitions::single` と `load_scope_balloon_definitions` だけ。設計は両方の形を変えない。`placement-measure-bake-once` の brief も「`script-small-tags` が `assets.rs` に触れるときだけ重なる」と書いており、重なりは 0。
- `crates/areka-emo-text/src/actor_present.rs` は触らない: 守れる。印の見た目は `ResolvedFont` の欄に入れて描き手まで届けるので、`actor_present.rs` の並びは変わらない。
- `choice-ranges-one-function` は `actor_present.rs`・`choice.rs`・2 つの検査のファイルだけで、この設計の触るファイルと重ならない。emo-text の「1 本だけの席」は `choice-ranges-one-function` が使わない約束なので、この spec が使ってよい（`roadmap.md` 204 行目の直しも設計に入っている）。

### 2.5 埋め込みを隣の本文とまとめても、範囲と診断はずれないか

ずれない、と確かめた。

- `check_script` の診断は、読みの列を前から順に見て、`CharCursor` で台本の位置を文字の数に直す。まとめた `Text` は台本の中で続いた 1 つの範囲で、順序も崩れない。`Text` の読みには印（`ReadNote`）が付かないので、まとめで印が混ざることも無い。
- 捨てた番号（`CharCodeDropped`）はまとめないので、自分の範囲で「読めない引数」が出る（要件 6.2）。
- アンカーの対応の崩れは読みの添字で数えるが、`check_script` と再生の組み立て（`compile`・`compile_strict`）は同じ「まとめた後の列」を見るので添字がそろう。
- strict の合図の位置（`cue_spans`）は合図 1 つに命令 1 つの範囲で、まとめた `Text` の合図は 2 つを覆う範囲になる。誤りの位置を指す所ではないので困らない。
- 環境変数の先回しの展開（`sysvar.rs` の `expand_system_vars`）は、比べる前に `join_texts` で隣の `Text` をつなぐので、読み手がまとめても比べの答えは変わらない。

### 2.6 目印の字 U+FFFC が外へ漏れないか

| 字を使う所 | 漏れるか |
|---|---|
| MCP の `dump_balloon` | 漏れない（絵を返すだけ。印は同じ面に塗るので絵に写る） |
| 行の描画（`viewbox_draw_render.rs` の 3 か所） | 設計が全角の空白へ読み替える |
| 行の指紋（`viewbox_diff.rs`）・分かち書き（`segment.rs` の BudouX） | 中だけで使う。外へは出ない（分かち書きの切れ目が少し変わり得るだけ） |
| 照合の写し（`draw.rs` の `DrawExecutor`・テスト専用） | テストの中だけ |
| **アンカーの範囲の文字（`state_anchor.rs` の `extend_open_anchor`）** | **漏れる。下の「重い問題 1」** |

## 3. 重い問題（3 つ）

### 重い問題 1: アンカーの中の印が、目印の字のまま SHIORI へ送られる

- **何が起きるか**: 設計は「目印の字の積み方は `Text` の腕と同じ（開いているアンカーを伸ばす）」と書く。今の `Text` の腕は `state.extend_open_anchor(&glyph_units)` を呼び、これはアンカーの範囲を伸ばすだけでなく、範囲の文字（`span.label`）にも字をつなぐ。その文字は `choice_drain.rs` を通って `OnAnchorSelectEx` の Reference0 になる。そのまま作ると、`\_a[ID]\![*]本文\_a` を押したとき、ゴーストに「U+FFFC＋本文」が届く。研究の記録（research.md 9.2）は「字の列を外へ出す所は今は無い見込み」としていたが、この 1 か所を見落としている。
- **なぜ重いか**: 台本の外（ゴーストの辞書）へ、areka の中だけの印の字が出る。ゴーストの作者には見えない字で、Reference0 で分けている辞書の分岐が外れる。
- **直し方の案**: 目印の字を積むときは、アンカーの範囲（`glyph_range`）だけを 1 つ伸ばし、範囲の文字（`label`）には何もつながない（`state_anchor.rs` に小さな口を 1 つ足すか、`extend_open_anchor` に「文字を足さない」を渡す）。テストを 1 本足す: アンカーの中に印を書いても、範囲の文字が印の無い台本と同じであること。範囲の数（当たり）には印も入るので、押せる所は印を含む。SSP が Reference0 に何を入れるかは測っていないので、§8 に「印は範囲の文字に入れない」と書いておく。
- **触る要件**: 4.1・4.2・5.1（印を渡す）と、アンカーの既にある約束（範囲の文字＝表示した文字）。

### 重い問題 2: 着せ替えの新しい関数の答えに、パーツ名と着脱が入っていない

- **何が起きるか**: 設計の `apply_bind_directive` は `Vec<NamedBindResult>` を返し、呼び手（`actor.rs`）は「返った列を今の記録の決まりで 1 つずつ記録する」とある。ところが `NamedBindResult` は `Applied { id, outcome }`・`NoNamespace`・`NotFound`・`MustSelectKept { id }` で、パーツ名も着脱（`on`）も持たない。今の記録の行（`seriko: bind 適用`・`mustselect` の警告・名前の解決の失敗）はどれも `category`・`part`・`on` の欄を出している。トグル形では `on` を関数の中で決め、カテゴリ単位形ではパーツの名前の列も関数の中で引くので、呼び手は欄を埋められない。
- **なぜ重いか**: 要件 3.3「同じ水準の記録」と 3.5（名前を記録に残す）、実機の確かめ（8.3）が grep する `seriko: bind 適用` の行が、トグル形とカテゴリ単位形でだけ欄の欠けた行になる。契約のまま作ると実装の途中で形を変えることになる。
- **直し方の案**: 返す列の 1 つずつに、当てたパーツ名と着脱を添える（例: `Vec<BindStep { part: String, on: bool, result: NamedBindResult }>`）。`NoNamespace` と「名前の無いカテゴリ」の 1 件は、パーツ名を空にするか、書かれたパーツ名をそのまま入れると決めて書く。表示を 1 回だけ出すときは、`refresh` が何も返さなかったら最後の `Changed` の指令を出す（今の `unwrap_or(command)` と同じ）ことも書き足す。
- **触る要件**: 3.3・3.5・3.8〜3.10・8.3。

### 重い問題 3: 面を替えて印の絵だけが変わると、もう描いた行の印が古い絵のまま残る

- **何が起きるか**: 設計の `set_inline_marker` は「値が前と違うとき、そのアクターの今の場所の値（`layout_input`）にも押し直す」。面の切り替え（`select_balloon_face`）では、この呼び出しの後で新しい面のモデルが入り、再追従（`refresh_actor_binding`）が「解いた値」と `layout_input` を比べる。押し直しが先に済んでいると、字の書式が同じ面どうしでは 2 つが等しくなり、再追従は描画の資源を作り直さない。描き直しは行の指紋（字・装飾の番号・寸）が変わった行だけで、印の絵は指紋に入らないので、もう描いた行は古い絵のまま、新しい行だけ新しい絵になる。先例の背景色（`set_balloon_background`）は値を覚えるだけで押し直さず、変化は再追従の比べが見つけて全部を描き直す。
- **なぜ重いか**: 面ごとに `marker.filename` を変えたバルーンで、1 つのバルーンの中に 2 種類の印が混ざる（要件 5.1・5.2 の「面ごとに読む」の決めと食い違う）。起こるのは印の絵だけが違う面どうしに限られるので狭いが、決定論のテストでは見つけにくい種類のずれ。
- **直し方の案**: 背景色の先例にそろえ、`set_inline_marker` は値を覚えるだけにして、押すのは装着・再追従・箱の 3 か所（解いた直後）に限る。そうすれば面の切り替えのときは比べが違いを見つけて作り直す。押し直しを残すなら、値が変わったときに描画の資源も捨てる（再追従の作り直しと同じ扱い）と書く。テストを 1 本: 印の絵だけが違う 2 つの面を切り替えたら、再追従が作り直しを返すこと。
- **触る要件**: 5.1・5.2・5.6。

## 4. 良い所

1. **実物に足を着けた設計**: 名前・行の位置・数・ファイルの大きさ・兄弟の spec との重なりを、コードを読んで確かめた上で書いている。照合した限り食い違いは無かった。`ScopeBalloonDefinitions` の欄が非公開であることを使って `assets.rs` を触らずに済ませるなど、ウェーブの約束を守る道を具体的に示している。
2. **今ある仕組みに乗せる形**: 印を「1 字分の目印の字」として積むので、現れる時刻・選択肢の範囲・消去・押し出し・スクロール・空回し・縦書きが新しい腕なしに付いてくる。運び方は `\f` と同じ汎用の合図と受け手の表、着せ替えの新しい形は今の `apply_named_bind` を続けて当てるだけで、決まりを増やさない。正典が黙っている所（印の丸の大きさと色・カテゴリ単位の形）は SSP を測った答えに合わせ、§8 に根拠ごと書く段取りになっている。

## 5. 判定

**GO**

- 理由: 3 つの重い問題は、どれも設計の形（読み手・組み立て・受け手・文字の層の分け方）を変えずに、関数 1 つの答えの形と、口 1 つの扱いを直せば片づく。要件の全部の条件に担う物と確かめのテストが割り当てられており、コードとの照合でも食い違いは無かった。
- 次の段: 設計の討議（`kiro-design-discussion`）で重い問題 1〜3 を議題にして design.md に反映し、その後 `/kiro-spec-tasks script-small-tags` へ進む。
- 討議のついでに見てよい小さな点（議題にしなくてよい）:
  - 目印の字は分かち書き（BudouX）にもそのまま入る。印と後ろの選択肢の字のあいだで切れるかどうかは実機の確かめ（8.3）で見る（research.md 9.5 の「印が行の末尾に残る」と同じ件）。
  - `marker_surface.rs` の `bitmap_of` は別の構造体（矢印の印の面）の口なので、印の絵を D2D の絵にする所は写しを作らず、自由関数にして 2 か所から呼ぶ形がよい。

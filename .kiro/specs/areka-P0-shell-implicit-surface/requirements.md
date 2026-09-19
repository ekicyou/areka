# Requirements Document

> 本文のコードの引用は「何の定義か（関数名・型名＋ファイル）」で指す。行数・件数・寸法は**本ブランチでの実測値（2026-09-19）**であり、設計・実装の着手時に引き直すこと。検体の場所は `cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体名>` の `folder=` 行が教える（以下 `<folder=>` と書く）。
>
> **brief.md を読む人へ**: brief の正典の読みは 2 か所で正典の逐語と食い違っており、範囲も 2 度広がった（開発者裁定 2026-09-19＝抜き色・「テンプレートゴーストを動かすのに要る実装は本仕様が巻き込む」）。正しい読みと裁定は本書の「正典の引き直し」と要件 10 にある。設計は brief ではなく本書に従うこと。

## Introduction

### 誰が困っているか

第三者の利用者と、そのゴーストのシェル作者。里々の標準テンプレート『Ｒポストと狛犬』（検体名 `R_POST_and_KOMAINU`）と YAYA の標準テンプレート「紺野ややめ」（検体名 `konnoyayame`）は、第三者が最初に手にする 1 体の代表である。どちらも areka では**絵が 1 枚も出ない**。

### いま何が起きているか（本ブランチで実測・2026-09-19）

絵が出ない原因は 2 つ重なっている。1 つ目を直しても、2 つ目が残る限り絵は出ない。

**原因 1: ファイル名だけで置かれた絵を、面として読む経路が無い。**

- 両検体とも SHIORI は応答し、起動挨拶の台本も返り、終了も正常である。失敗するのは絵の合成だけで、記録は 2 体とも同じ並びになる: `定義層が皆無で外形 0×0 の退化データ: EmptyComposition surface_id=0` → `measure: scope0（surface id 0）の採寸合成に失敗 reason=surface 0 has no layers at all (extent 0x0)` → `窓配置の準備に失敗しました——検証用ダミー窓へフォールバックします`。キャラクター窓が建たないので、バルーンの文字も右クリックメニューも出ない。
- 両検体の `shell/master/surfaces.txt` に `element` 行は **0 件**。絵は**ファイル名の慣習**（`surface0000.png` が面 0 の絵）だけに任されている。areka は面を `elementN,overlay,…` 行**だけ**から組む（`crates/areka-parsers/src/shell/decode.rs` の `decode_elements`）。ファイル名から面の番号を取るコードは `crates/` のシェル資産の経路に **0 件**である。
- 検体の実測（`<folder=>/shell/master/` 直下・`surface` で始まる `.png` の全数）:

  | 検体 | 画像の番号（先頭の 0 を除いた値） | 波括弧の宣言 | 画像あり・宣言なし | 宣言あり・画像なし | `element` 行 | `animation` 行 | `.pna` |
  |---|---|---|---|---|---|---|---|
  | `R_POST_and_KOMAINU` | 0〜6・10・100・200（10 枚・すべて 4 桁表記） | 0〜6・100・200（9 個） | 1 件（10） | 0 件 | 0 | 0 | 0 |
  | `konnoyayame` | 0〜7・10・11・1030〜1033・1040〜1043（18 枚・すべて 4 桁表記） | 0〜7・10・11（10 個） | 8 件（1030〜1033・1040〜1043） | 0 件 | 0 | 7（面 0 の `animation0` のみ） | 0 |
  | `emo2` | 0・10（2 枚・`surface0.png`／`surface10.png`） | 多数 | 0 件 | 多数 | 多数の面が持つ（面 0・面 10 はどちらも `element0` を持つ） | あり | 0 |

  実寸（IHDR）: `R_POST_and_KOMAINU` の面 0＝236×462・面 10＝140×160・面 100＝143×373。`konnoyayame` の面 0＝260×390・面 10＝200×200・面 1031＝72×30。`emo2` の `surface0.png`＝434×687・`surface10.png`＝427×463。
- `konnoyayame` の面 0 のまばたき（`animation0.interval,sometimes`・`pattern0`〜`5`・描画メソッド `overlay`・位置 93,103）は、コマの相手に面 1031・1032・1033（と終端の `-1`）を引く。この 3 つは**画像だけが在って宣言が無い番号**である。里々の検体はこの形を 1 度も踏まない。
- 宣言も画像も無い番号がコマの相手に引かれたとき、いまの実装は**解析から表示までのどの段でも黙って飛ばし、記録は 0 件**である（`crates/areka-emo-atlas/src/manifest.rs` の `ManifestDeriver::resolve_indirect`＝`continue`／`crates/areka-emo-compose/src/plan.rs` の `push_static_element_ops`＝記録なしで `return`／`crates/areka-seriko/src/table.rs` の `AnimationTable::from_world` は相手の面の有無を見ない）。コマを持つ側の面も起動も失敗しない。
- `\s[N]` の N が宣言の無い番号のときは、`crates/areka-emo-present/src/presenter/show.rs` の `apply_show` が `error!`（「合成失敗 → 表示は適用前のまま」）を出し、前の絵が残る。
- `defaultsurface` の宣言は 3 検体ともシェルにもゴーストにも **0 件**。areka の側にも読む経路は **0 本**（`grep defaultsurface crates/` → 0 件）。起動時は本体側＝面 0・相方側＝面 10 を固定で出す（`crates/areka/src/placement/measure.rs` の `measure_native_scope_sizes`）。

**原因 2: α チャンネルを持たない絵の透過（左上の色を抜き色にする）が実装されていない。**

- 両テンプレートの絵は **α チャンネルを持たない**。`R_POST_and_KOMAINU` の `surface*.png` は全 10 枚が PNG の色の型 2（24bit RGB）、`konnoyayame` の `surface*.png` は全 18 枚が色の型 3（パレット）で、透明を表す `tRNS` チャンクは 2 体とも **0 件**。どちらのシェルの `descript.txt` にも `seriko.use_self_alpha` の宣言は **0 件**。`emo2` のシェルは PNG 58 枚のうち 57 枚が色の型 6（RGBA）で、`seriko.use_self_alpha,1` を宣言している。
- areka は透過の扱いを宣言から読まず、常に「α を採る」（`UseSelfAlpha::On`）を固定で渡す（`crates/areka/src/emo2_boot/assets.rs` の `build_boot_assets`・`crates/areka/src/placement/measure.rs` の `build_shell_assets`。「areka は常に `use_self_alpha,1`」は開発者確認済みの方針＝steering `roadmap.md` 仮裁定 10）。
- その下で α の無い絵に当たると、`crates/areka-emo-atlas/src/normalize.rs` の `Normalizer::select_source` は正典の表どおり抜き色（`AlphaSource::KeyColor`）を選ぶ。しかし `Normalizer::normalize` が実装しているのは「`UseSelfAlpha::On` かつ α チャンネルあり」の 1 通りだけで、他は `NormalizeError::Unsupported` を返す。`crates/areka-emo-atlas/src/lib.rs` の `bake` はそれを `BakeError::Normalize` として**その画像を索引表から落として続行**する。落ちた画像は描かれず、外形にも数えられない。
- したがって、原因 1 だけを直して面 0 が `surface0000.png` に解決されても、その画像は焼く段で落ち、面 0 は今日と同じ外形 0×0 の `EmptyComposition` になる。
- `emo2` でこの経路を踏む画像は**ちょうど 1 枚**＝`purple/a/null.png`（382×547・色の型 3・`tRNS` あり・全 208,954 画素が左上の画素と同じ色で、復号すると全画素が透明）。今日は焼く段で落ちている（`crates/areka-emo-atlas/src/emo2_e2e.rs` の定数 `SHELL_NORMALIZE_SEAM_KEY` の説明と、テスト `emo2_shell_all_elements_baked` がこの 1 件の脱落を件数で留めている）。この画像を引くのは面 1414 の `element0,overlay,purple/a/null.png,0,0` の 1 行だけ、面 1414 を引くのは面 1000 の `animation1403.pattern2,overlay,1414,0,0,0` の 1 行だけである。

**原因 3: まばたきの間隔の語 `sometimes` を areka は再生しない。**

- `konnoyayame` の面 0 のまばたきは `animation0.interval,sometimes` である。areka が再生を駆動する間隔の語は `random,数値` と `bind+random,数値` の **2 つだけ**で、`sometimes` は値として読むが再生しない（`crates/areka-parsers/src/shell/model.rs` の `Interval::Other`・`crates/areka-seriko/src/table.rs` の `AnimationTable::from_world` が `debug!` を残して採らない）。網羅台帳の `ukadoc:descript_shell_surfaces:sometimes:1` は「語彙のみ」・担当は**空欄**である。
- したがって、原因 1・2 を直して面 1031〜1033 が解決でき、絵が抜かれても、実機ではまばたきしない。
- 両検体の `animation*.interval` の語の全数: `konnoyayame` は `sometimes` が 1 件・他の語は **0 件**。`R_POST_and_KOMAINU` は `animation` 行そのものが **0 件**。

**ほかに妨げが無いことの事前確認（2026-09-19・両検体を `RUST_LOG=debug` で 25 秒ずつ実走）**:

- 起動挨拶の台本は 2 体とも正しく復号されている（`konnoyayame`＝`Text("初めまして。")` ほか・`R_POST_and_KOMAINU`＝`Text("はじめまして。")` ほか。文字化け **0 件**）。SHIORI 通信の初期の文字コードが `Shift_JIS`（`source="default"`）でも、`charset,UTF-8` の `konnoyayame` の応答は正しく読めている。
- 起動挨拶が使う台本の命令は 2 体とも、消去・面の切り替え（`konnoyayame` は面 10・5・6・11・4・1、`R_POST_and_KOMAINU` は面 0・10）・文字・待ち・改行だけで、未対応の命令は **0 件**。
- `ERROR` は 2 体とも 3 行で、すべて面 0 の合成失敗（原因 1）から来ている。`WARN` のうち窓が無いことに由来しないものは、バルーンの相方側の面 2・3 が本体側の系列へ縮退した 2 行だけで、これは同梱バルーン `StayseeBalloon` の側の既知の縮退であって検体の動作を妨げない。
- 絵が出た後でなければ見えないもの（まばたき・クリック透過・メニュー・バルーンの左右）は、要件 8 の実機確認で確かめる。

### 何を変えるか

1. シェルのフォルダ直下にある `surface<数字>.png` を、正典どおり**その番号の面の絵**として扱う。先頭の 0 は無視する（`surface0.png`＝`surface0000.png`）。この解決は、起動時の既定の面・`\s[N]`・アニメーションや着せ替えのコマの相手・窓の採寸・`surface.append` の「既にある面」の判定の**すべてに同じく効く**。`element0` を持つ面では正典どおり画像を使わない。
2. α チャンネルを持たない絵は、正典どおり**左上の 1 画素と同じ色を透明にして**描く（開発者裁定 2026-09-19・要件 10.1）。
3. まばたきの間隔の語 `sometimes`（と、同じ仕組みの `rarely`）を、正典どおりの頻度で再生する（開発者裁定 2026-09-19・要件 10.7・要件 11）。
4. `emo2` は、上の `null.png` 1 枚の扱い（落ちる → 全画素が透明な絵として焼かれる）を除いて何も変わらず、画面に出る画素は 1 つも変わらない。

## 正典の引き直し（brief との差分を含む）

brief が引いた正典は `dev_shell` の 2 文だけだった。ukadoc の現行ページ（2026-09-19 取得）で引き直した結果を表に置く。要件の根拠はすべてこの表の逐語である。

| # | 出どころ | 逐語 |
|---|---|---|
| C1 | [dev_shell](https://ssp.shillest.net/ukadoc/manual/dev_shell.html) | 「サーフェスは、「surface○○.png」（○○部分には0以上の数字が入る）という名前の画像を用意するか、後述のsurfaces.txtで複数の画像を合成する事で用意します。」 |
| C2 | 同上 | 「サーフェスはsurface0000.png、surface0010.png等の様に記述してもsurface0.png、surface10.pngと同様に認識されます。」 |
| C3 | 同上（element 合成編） | 「なお、surface\*.pngのような名前のpng画像は、element0より下のパーツとみなされる点に注意してください。」 |
| C4 | [descript_shell_surfaces「ベースサーフェスの実体」](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#caption_basesurface) | 「あるsurfaceのベースサーフェスは、IDに対応するsurface\*.pngと、IDに対応するsurfaceブレスでのelement指定で定義される。」「surface\*.pngがあるサーフェスでは、その画像がsurface\*のベースサーフェスとなる。ただし同時にelement0が定義されている場合、surface\*.pngの内容は破棄されelement0の定義内容で置き換わる（これは互換性確保のための仕様である）。」「element1以降の定義がある場合は、それらの上に積み重なる形で順次合成され、最終的な合成結果がベースサーフェスとして扱われる。」 |
| C5 | [descript_shell_surfaces `element*`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#element_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30d5_30a1_30a4_30eb_540d_2cX_5ea7_6a19_2cY_5ea7_6a19_28_2c_30aa_30d7_30b7:1) | 「サーフェスに対応するsurface\*.pngという画像がある場合、element0が定義されていると元のsurface\*.pngの内容が破棄されて、element0で置き換えられる」 |
| C6 | [descript_shell_surfaces「surface\*ブレスとsurface.append\*ブレス」](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#caption_surface.append) | 「surface.appendによる定義は、「既に（surfaceブレス、またはsuface\*.pngの存在によって）ある定義」に対してのみ付け加えられる。」（`suface` は原文の綴り） |
| C7 | [dev_shell](https://ssp.shillest.net/ukadoc/manual/dev_shell.html)（アニメーション編） | 「100という部分は、このコマで描画するのはsurface100だという意味です。」「基本的にはsurface100.pngのことだと考えてよいです。」 |
| C8 | [manual_shell](https://ssp.shillest.net/ukadoc/manual/manual_shell.html) | 「SSPでは暗号化PNG(DAP/DDP/DFP/DGP)・GIF（非アニメ）・JPEG・BMPも使用可能だが、非推奨。」 |
| C9 | 同上（`surface*.png`） | 「画像左上の1ドット(座標0,0)と同色の領域は透過色（抜き色）とされ、表示上透過される。」 |
| C10 | [dev_shell](https://ssp.shillest.net/ukadoc/manual/dev_shell.html) | 「サーフェス画像では左上端の1ドットが「透過色」としてゴーストとしての表示上透過されます。」「透過色と同じ色がキャラクター内にもあった場合、その部分も透過されてしまいますので、透過色にはキャラクターに使っていない系統の色を用いるなどしてください。」 |
| C11 | [descript_shell `seriko.use_self_alpha`](https://ssp.shillest.net/ukadoc/manual/descript_shell.html#seriko.use_self_alpha_2c_5024:1) | 「1またはtrueを指定した場合は、透明度のある画像（アルファチャンネル付きPNG）および.pnaのある画像はアルファチャンネルを参照して透明度として使用する。アルファチャンネルも.pnaも存在しない場合は画像左上の色をキー色とする従来挙動になる。fullを指定した場合は上記に加え、アルファチャンネルのない画像も全て不透明として扱う（左上のキー色透過を行わない）。」 |
| C12 | [descript_balloon `use_self_alpha`](https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#use_self_alpha_2c_5024:1) | 「アルファチャンネルも.pnaも存在しない場合は画像左上の色をキー色とする従来挙動になる。」 |
| C13 | [manual_shell](https://ssp.shillest.net/ukadoc/manual/manual_shell.html)（注記） | 「JPEGについては圧縮によって色がずれやすいため透過色と相性が悪いという弊害もある。」 |
| C14 | [dev_shell](https://ssp.shillest.net/ukadoc/manual/dev_shell.html) | 「ゴーストとしてキャラクターが一人だけで相方が要らない場合でも、単色で塗り潰した画像(＝全て透明表示。後述)などをsurface10.pngとして用意してください。」 |
| C15 | [descript_shell_surfaces `sometimes`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#sometimes:1) | 「そのサーフェスである間毎秒2分の1の確率で再生。」 |
| C16 | [descript_shell_surfaces `rarely`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#rarely:1) | 「そのサーフェスである間毎秒4分の1の確率で再生。」 |
| C17 | [descript_shell_surfaces `random,数値`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#random_2c_6570_5024:1) | 「そのサーフェスである間毎秒数値分の1の確率で再生。」 |

**brief と違う点（2 件・どちらも正典の逐語で決まる）**:

1. **`element0` を持つ面では、画像は下に敷かれず、使われない。** brief は C3 だけを読んで「`element` を持つ面にも 1 層増える」と書き、`emo2` の面 10 の外形が 336×400 → 427×463 へ広がると見立てた。C4・C5 は「`element0` が定義されていれば画像の内容は破棄される」と明言している。`emo2` の面 0・面 10 はどちらも `element0` を持つので、**増える層は 0**・外形も合成結果も変わらない。brief の Desired Outcome 2（`emo2` が適用前と同一）は、この読みでのみ満たせる。C3 の「element0 より下」が効くのは、画像が在って `element0` が無く `element1` 以降だけが在る面である（C4 の 3 文目）。brief が挙げた「面 0 が同じ画像を自分の上に二重に重ねる」事態も、「`element0` より下を表す層の番号が足りない」という設計上の難所も、この読みでは生じない。
2. **画像だけで存在する面にも `surface.append` は効く。** brief は「作る位置は要件で明示する」と判断を残したが、C6 が「画像の存在によってある定義」を追記の対象に数えている。

## Boundary Context

- **In scope**（利用者・シェル作者から見える範囲）:
  - シェルのフォルダ直下の `surface<数字>.png` から面の番号を得ること（先頭の 0 を無視）。
  - 面の土台の絵（ベースサーフェス）が、画像と `element` 行の有無でどう決まるか。
  - その結果が、起動時の既定の面・`\s[N]`・アニメーション／着せ替えのコマの相手・窓の採寸・`surface.append` の対象判定の全部で同じになること。
  - α チャンネルを持たない絵を、左上の 1 画素と同じ色を透明にして描くこと（抜き色）。
  - アニメーションの間隔の語 `sometimes`・`rarely` を正典どおりの頻度で再生すること（要件 11）。
  - 実機確認で見つかった、テンプレートゴースト 2 体の動作を妨げる欠陥を直すこと（要件 8.4・要件 10.7）。
  - `emo2` の画面に出る画素と寸法が変わらないこと（合否判定）。
  - 記録・決定論テスト・実機確認 3 体（`R_POST_and_KOMAINU`・`konnoyayame`・`emo2`）。
- **Out of scope**:
  - `.pna`（α マスク）。3 検体とも **0 件**で、areka は `.pna` 非対応が開発者確認済みの方針（steering `roadmap.md` 仮裁定 10）。読まない。今の「未実装として落とす」扱いを変えない（変更 0）。
  - `seriko.use_self_alpha,full`（α の無い絵を全面不透明にする）の腕。今の「未実装として落とす」扱いを変えない（変更 0）。
  - `seriko.use_self_alpha`／`use_self_alpha` の宣言を読むこと。読む経路は **0 本**のまま、常に `1` 相当で扱う。
  - PNG 以外の形式（C8 が非推奨と明記）。読まない。アニメ GIF／APNG／WebP の自動アニメーションも持たない。
  - `defaultsurface` の宣言の導出。読む経路は **0 本**のまま、本体側＝面 0・相方側＝面 10 の固定を前提に置く。
  - `alias.txt`・`surfaces*.txt`（追加の定義ファイル）・`surfacetable.txt`。読む道は今日も無く、本仕様も足さない。
  - `surfaces.txt` が無いシェル、波括弧が 0 個で画像だけが在るシェル。C1 は成り立つ形として書いているが、今は起動の失敗（`ShellRead`／`ShellEmpty`）になる。3 検体に **0 件**で、テンプレートゴーストを動かすのに要らない（要件 10.7 の物差し）。今の失敗の扱いを変えない（変更 0・引受先の登記は要件 9.5）。
  - 間隔の語のうち `sometimes`・`rarely` 以外（`always`・`runonce`・`never`・`yen-e`・`talk,数値`・`periodic,数値` など）。再生の仕組みが別に要り、テンプレートゴースト 2 体に **0 件**。今の「値として読むが再生しない・`debug!` を残す」扱いを変えない（変更 0）。
  - `element` 行の `overlay` 以外の描画メソッド。`decode_elements` は `overlay` の行だけを値にし、他は記録なしで読み飛ばす（網羅台帳 `element*` の項＝縮退・担当 `areka-P0-shell-parse`）。本仕様はこの縮退を直さない。`overlay` 以外で書かれた `element0` は読み飛ばされるので、要件 2.1 の表では「`element0` なし」に数えられる（要件 2.7・3 検体に **0 件**）。
  - 全画素が透明になる面（C14 の「単色で塗り潰した surface10」）を表示の対象にしたときの扱い。3 検体に **0 件**で確かめられない。本仕様は全透明の面の既存の扱いを変えない（要件 4.7・引受先の登記は要件 9.5）。
  - 着せ替え・当たり判定（`collision*`）の意味論そのもの、`point.basepos`（`areka-P0-surfaces-basepos`）。
  - 網羅台帳でページ `dev_shell`・`manual_shell` を項目へ割ること（`areka-P0-ukadoc-coverage-roadmap` の仕事・要件 9）。
- **Adjacent expectations**:
  - **前提（完了済み）**: `areka-P0-nar-install`（検体は共有ヘルパ `sample_ghost_kit::SampleRoot::acquire` 経由で受ける。`vendors/sample_ghost/<検体名>/` の直パスはもう無い）・`areka-P0-charset-canon`（シェル本文の復号）・`areka-P0-emo-atlas`（透過の正規化の器と、未実装の腕を型で示す継ぎ目）・`areka-P0-emo2-conformance-e2e`（`emo2` の寸法と合成結果を留めている既存テスト）・`areka-P0-popup-menu-minimal`（右クリックメニュー。実機確認 1 件を本仕様へ引き渡し済み）。
  - **`areka-P0-seriko-runtime`（完了済み）**: 間隔の語の再生の器（`AnimationTable`・乱数の駆動）を持つ。本仕様は `sometimes`・`rarely` を既存の乱数の駆動へ読み替えるだけで、再生の仕組みそのものは変えない。
  - **直列（同時に走らせない）**: `areka-P0-surfaces-basepos`（同じ `crates/areka-parsers/src/shell/` に触りうる・後着が rebase）。
  - **下流**: 里々／YAYA の実ゴースト適合一般・`areka-P0-alpha-release-signoff`（α の検体 3 体＋既定バルーン）。
  - **バルーン**: バルーンの絵も同じ透過の正規化を通る（`crates/areka-emo-present/src/balloon.rs` の `build_balloon_target_from_faces`）ので、抜き色はバルーンの面にも同じく効く（C12）。リポジトリ内のバルーンで面として焼かれる絵に α の無いものは **0 枚**（`StayseeBalloon` の PNG 24 枚は全て α あり・`emo2-kakukaku` の `balloons0.png`／`balloonk0.png` は色の型 6）なので、既存のバルーンの見た目は変わらない。

## Requirements

### Requirement 1: ファイル名の慣習から面の番号を得る

**Objective:** シェル作者として、`surface0000.png` のように桁をそろえて置いた絵が、`surface0.png` と同じ面 0 として読まれてほしい。そうすれば、ファイル一覧が番号順に並ぶ 4 桁表記のまま、`surfaces.txt` に 1 行も足さずに絵が出る。

#### Acceptance Criteria

1. The areka shall シェルのフォルダ（`shell/<シェル名>/`）の**直下**にあるファイルのうち、名前が「`surface`＋ 1 文字以上の数字だけ ＋`.png`」の形のものを、その数字を 10 進数として読んだ番号の面の画像と認める（C1）。
2. The areka shall 番号を読むとき先頭の 0 を無視し、`surface0.png`・`surface00.png`・`surface000.png`・`surface0000.png` をいずれも面 0、`surface0010.png` を面 10 と認める（C2）。
3. The areka shall 次の名前を面の画像と**認めない**（それぞれ 0 件として扱う）: `surface` で始まらないもの（`menu_background.png`・`thumbnail.png` など）／数字以外を含むもの（`surfaces.txt`・`surfacetable.txt`・`surface+0.png`・`surface-1.png`・`surface.png`）／拡張子が `.png` でないもの（`surface0.pna`・`surface0.jpg` など）／サブフォルダの中のもの（`emo2` の `CityPop\surface0010.png` は `element` 行が名指しする部品であって、面 10 の画像ではない）／フォルダ。
4. The areka shall 接頭辞 `surface` と拡張子 `.png` の大文字・小文字を区別しない（正典は沈黙。バルーン画像の系列解決＝`crates/areka-emo-present/src/balloon.rs` の `face_id_of` と同じ扱いにそろえる areka の裁量）。
5. If 同じ番号に解決される画像が 2 つ以上ある（`surface0.png` と `surface0000.png` の同居など）, then the areka shall ファイル名の辞書順で最小の 1 つを採り、採った名前と捨てた名前を `warn!` で 1 回記録する（正典は沈黙。フォルダの走査順に結果が左右されないこと。バルーンの `select_faces` と同じ規則）。
6. If 数字の部分が面の番号として表せないほど大きい, then the areka shall そのファイルを面の画像と認めず、`debug!` で記録する。
7. If シェルのフォルダの一覧そのものが取れなかった, then the areka shall 失敗を `error!`（フォルダのパスと OS のエラー付き）で記録し、起動の失敗として既存の失敗経路へ渡す。「画像 0 件」として黙って先へ進まない。
8. The areka shall 同じフォルダの内容に対して、何度読んでも同じ「番号 → ファイル名」の対応を得る。

### Requirement 2: 面の土台の絵の決まり方

**Objective:** シェル作者として、絵をファイル名だけで置いた面も、`element` で組んだ面も、両方を混ぜた面も、正典どおりの 1 枚になってほしい。そうすれば、SSP 向けに作ったシェルが areka でも同じ絵になる。

#### Acceptance Criteria

1. The areka shall 面 N の土台の絵を、面 N の画像（要件 1）と面 N の `element` 行の有無から、次の表のとおりに決める（C3・C4・C5）。「`element0` あり」は、描画メソッド `overlay` で書かれた `element0,…` の行が在ることを指す（`overlay` 以外の扱いは要件 2.7）。

   | # | 面 N の画像 | `element0` | `element1` 以降 | 土台の絵 |
   |---|---|---|---|---|
   | ア | あり | なし | なし | 画像 1 枚。外形は画像の実寸 |
   | イ | あり | なし | あり | 画像を最も奥に置き、その上に `element` を番号の小さい順に重ねたもの |
   | ウ | あり | あり | 問わない | `element` 行だけで組んだもの。**画像は使わない**（画像由来の層は 0） |
   | エ | なし | 問わない | 問わない | 現状のまま（`element` 行だけで組む） |

2. When 面 N の画像が在り、`surfaces.txt` に面 N の波括弧が無い, the areka shall 面 N を「存在する面」として扱い、表のアで絵を決める（`R_POST_and_KOMAINU` の面 10・`konnoyayame` の面 1030〜1033・1040〜1043 がこの形）。
3. When 面 N の波括弧が在って `element` 行が 0 件で、面 N の画像が在る, the areka shall 波括弧の中の他の定義（当たり判定・アニメーション・`point.*`・`*.balloon.offset*`）をそのまま生かし、絵だけを表のアで決める（両テンプレートの面 0 がこの形）。
4. While 面 N の画像も `element` 行も無い, the areka shall 現状どおり、その面を表示の対象にしたときは `EmptyComposition`（定義層が皆無で外形 0×0）として失敗させ、既存の記録をそのまま出す。宣言も画像も無い番号は現状どおり「存在しない面」である。
5. The areka shall 画像を置く位置を面の左上（0,0）とし、`element` 行の位置の読み方は変えない。
6. The areka shall 同名の `.pna` を**読まない**（Boundary Context）。在っても無くても結果は変わらず、記録も出さない。
7. While `element` 行の `overlay` 以外の描画メソッドが読み飛ばされる既存の縮退（Boundary Context・`decode_elements`・担当 `areka-P0-shell-parse`）が残っている, the areka shall `overlay` 以外で書かれた `element0` を「`element0` なし」に数え、面 N の画像が在ればそれを土台にする（表のア・イ）。これは C4・C5（「element0が定義されている場合…破棄され」）からの既知のずれであり、3 検体に **0 件**である。ずれは縮退そのものに由来するので、縮退が直るとき（`overlay` 以外の行が値になるとき）に併せて正典どおりになる。本仕様はそのために解析の結果の型を変えない（開発者裁定 2026-09-19・要件 10.7 の物差し: テンプレートゴーストを動かすのに要らず、満たすには 25〜29 ファイルの追随が要る＝ギャップ分析 4 節）。この既知のずれは要件 9.3 ⑴ の備考と要件 9.5 に記す。

### Requirement 3: 面を引くすべての入口で同じ結果になる

**Objective:** 利用者として、起動時の立ち絵も、台本で替わる表情も、まばたきも、窓の大きさも、同じ規則で決まってほしい。そうすれば、「立ち絵は出るのにまばたきだけ出ない」「絵と窓の大きさが食い違う」が起きない。

#### Acceptance Criteria

1. When ゴーストが起動する, the areka shall 本体側に面 0・相方側に面 10 を、要件 1・2 で決まった絵で表示する。`defaultsurface` は読まない（0 本のまま）。`R_POST_and_KOMAINU` では本体側 236×462・相方側 140×160、`konnoyayame` では本体側 260×390・相方側 200×200 の外形になる。
2. When 台本の `\s[N]` が、画像だけで存在する面 N を指した, the areka shall その面を表示する（`R_POST_and_KOMAINU` の `\s[10]`・`\s[100]`・`\s[200]` と、波括弧を持つ `\s[1]`〜`\s[6]` のどちらも同じ規則で届く）。
3. If `\s[N]` が宣言も画像も無い番号を指した, then the areka shall 現状どおり `error!` を出して前の絵を残す（本仕様は変えない）。
4. When アニメーションまたは着せ替えのコマ（`animation*.pattern*`）の相手の面が、画像だけで存在する面である, the areka shall そのコマを、宣言のある面を相手にしたときと同じに描く（C7）。`konnoyayame` の面 0 は、まばたきのコマとして面 1031・1032・1033 の画像を位置 93,103 に重ねる。
5. If コマの相手が宣言も画像も無い番号である（停止を表す `-1`・`-2` は除く）, then the areka shall 現状どおりそのコマを描かずに続行し、コマを持つ側の面も起動も失敗させない。いま 0 件の記録を改め、同じ「面 → 相手」の組について `warn!` を出す。出す頻度は「コマを描くたび・毎フレーム」であってはならず、1 回の起動で同じ組について高々数回（面の表を組む回数以下）に収める。面の表は 1 回の起動で 3 回組まれる（2 スコープ＋採寸）ので、「起動で 1 回」にするか「面の表 1 つにつき 1 回」にするかは設計で定める。
6. The areka shall 窓の採寸と実際の表示で、同じ「番号 → 画像」の対応・同じ土台の絵・同じ透過の扱い（要件 4）を使う。採寸した窓の大きさと表示された絵の外形が食い違わない（バルーンで「列挙の規則が 2 つの実装に分かれると、採寸した窓寸と実際の枠が食い違い、実機でしか現れない欠陥になる」と分かっている＝`crates/areka/src/placement/measure.rs` の `measure_balloon_surface0` の説明）。
7. When `surface.append` が、画像だけで存在する面を対象に含む, the areka shall その面を「既にある面」と数えて追記を適用する（C6）。
8. If `surface.append` の対象が宣言も画像も無い番号である, then the areka shall 現状どおり新設せず、既存の `warn!`（「surface.append 対象 id が未存在: 新設せずスキップ」）を出す。
9. The areka shall どこからも引かれていない画像（`konnoyayame` の面 1030・1040〜1043）も、要件 1・2 の規則のまま「存在する面」として扱う。特別扱いも除外もしない（描かれる機会が無いだけで、`\s[1030]` と書けば出る）。

### Requirement 4: α チャンネルの無い絵は左上の色を抜いて描く

**Objective:** 利用者として、α チャンネルを持たない昔ながらの絵（両テンプレートの全部）が、背景の色の四角ごとではなく、キャラクターの形に抜かれて出てほしい。

#### Acceptance Criteria

1. When 絵が α チャンネルを持たず、透過の扱いとして抜き色が選ばれた（`Normalizer::select_source` が `AlphaSource::KeyColor` を返す場合＝常に `1` 相当で扱う areka では「α チャンネルが無い絵」のすべて）, the areka shall その絵の左上の 1 画素（座標 0,0）と同じ色の画素を**すべて完全に透明**にし、それ以外の画素を**すべて完全に不透明**にして描く（C9・C10・C11）。
2. The areka shall 「同じ色」を色の**完全な一致**（赤・緑・青の各成分が等しい）と定め、近い色を同じとみなす許容幅を持たない（許容幅 0）。正典は「同色」としか書かず許容幅に触れていない＝沈黙であり、C13（色がずれる形式は抜き色と相性が悪い）は完全一致を前提にした記述である。
3. The areka shall 抜く範囲を、左上の画素とつながった領域に限らず、絵の中の同じ色の画素**すべて**とする（C10「透過色と同じ色がキャラクター内にもあった場合、その部分も透過されてしまいます」）。
4. The areka shall 「α チャンネルを持つか」の判定を変えない（復号前の画素の形式で決める既存の判定＝`crates/areka-emo-atlas/src/decode/wic_arm.rs` の `pixel_format_has_alpha`）。α チャンネルを持つ絵は今日と同じく α をそのまま使い、結果は 1 画素も変わらない。
5. The areka shall この扱いを、透過の正規化を通るすべての絵に同じく適用する: 面の画像（要件 2）・`element` 行の部品・アニメーション／着せ替えのコマの相手の絵（`konnoyayame` の面 1031〜1033＝72×30 のまばたきの部品は、抜かれなければ目の周りに四角い地色が出る）・バルーンの面（C12）。
6. The areka shall 透明にした画素に元の色を残さない（絵の縁に地色のにじみや暗い縁を出さない）。
7. When 絵の全画素が左上の画素と同じ色である, the areka shall それを失敗にせず「全画素が透明な絵」として扱う。全画素が透明な絵の既存の扱い（描く命令には数えず、原寸は外形に数える＝`crates/areka-emo-compose/src/plan.rs` の `push_static_element_ops` と `flatten_extent`）は変えない。
8. The areka shall パレット形式で `tRNS`（色ごとの透明度）を持つ絵を抜き色で扱うときに、`tRNS` の透明度を生かすかどうかを設計で定め、沈黙ルール対応表に記す（正典は沈黙。C11 が名指しするのは「アルファチャンネル付きPNG」だけである）。3 検体でこの形の絵は `emo2` の `purple/a/null.png` の 1 枚だけで、全画素が左上と同じ色なので、どちらに定めても結果は「全画素が透明」で同じである。
9. The areka shall 抜き色以外の未実装の扱い（同名の `.pna` を使う・`full` で全面不透明にする）を今日のまま「未実装として、その絵だけを落として続行し、記録を出す」に保つ（変更 0）。透過の宣言（`seriko.use_self_alpha`／`use_self_alpha`）を読む経路は 0 本のままである。
10. While キャラクター窓の上にポインタがある, the areka shall 抜き色で透明になった場所を、α で透明な場所と同じく「キャラクターの外」として扱う: クリックは背後の窓へ抜け、撫でや当たり判定の対象にならない。当たりのマスクは合成し終えた絵の画素の不透明度から作られている（`crates/areka-emo-present/src/presenter/budget.rs` の `MaskRotation::regenerate` が wintf の `AlphaMask::from_pbgra32` を呼ぶ）ので、抜き色のために別の仕組みを足さない。`collision*` の矩形の読み方は変えない。

### Requirement 5: emo2 は画面に出る画素が 1 つも変わらない

**Objective:** 開発者として、この変更が既に動いているゴーストを壊さないことを、目視ではなくテストで言い切りたい。これは副作用の確認ではなく、本仕様の合否判定そのものである（開発者指示 2026-09-13）。

#### Acceptance Criteria

1. The areka shall `emo2` の面 0（`element0,overlay,surface0.png,0,0`・画像 `surface0.png` あり）と面 10（`element0,overlay,CityPop\surface0010.png,0,0`・画像 `surface10.png` あり）を、どちらも要件 2.1 の表のウとして扱い、画像由来の層を **0** にする。
2. The areka shall `emo2` の面 0 の外形 434×687・面 10 の外形 336×400 と、両面の合成結果（全画素）を、本仕様の適用前と同一に保つ。`crates/areka/src/placement/measure_tests.rs` の `SCOPE0_W`／`SCOPE0_H`／`SCOPE1_W`／`SCOPE1_H` を書き換えずに緑のままであること。
3. The areka shall `emo2` の面 0・面 10 以外のすべての面に、画像由来の層を **0** のまま保つ（直下の `surface*.png` は `surface0.png`・`surface10.png` の 2 枚しか無く、サブフォルダの画像は要件 1.3 で対象外）。
4. The areka shall `emo2` のシェルの PNG 58 枚のうち α チャンネルを持つ 57 枚の焼かれ方を 1 画素も変えない（要件 4.4）。
5. The areka shall `emo2` で変わるものを、次の **1 件だけ**にする: `purple/a/null.png`（382×547・α チャンネルなし・全 208,954 画素が左上の画素と同じ色）が、「焼く段で未実装として落ちる（失敗 1 件）」から「全画素が透明な絵として焼かれる（失敗 0 件）」へ変わる。これに伴い、
   - シェルを焼いたときの失敗は 1 件 → **0 件**、索引表に載る絵は 1 枚増える。
   - 起動時と採寸時の `warn!`「shell bake で脱落した element」（`build_boot_assets`・`build_shell_assets`）は、`emo2` では 1 回 → **0 回**になる。
   - 代わりに、焼く段の**既存の** `warn!`「element が全透明（α=0）でトリム後 0 寸です」（`crates/areka-emo-atlas/src/lib.rs` の `bake`）が、`null.png` について焼くたびに 1 回出るようになる（今日は落ちているので 0 回）。これは全画素が透明な絵に対する既存の扱いそのもので、本仕様は受け入れる。抜き色で全透明になった絵だけを警告から外す特例は足さない（要件 4.7「既存の扱いは変えない」と同じ向き）。
   - 面 1414（この絵だけを持つ）と、それを着せ替えのコマに引く面 1000 の**描かれる画素は変わらない**（今日は「落ちて描かれない」、適用後は「全画素が透明で描かれない」）。面 1000 の外形も変わらない（シェルには同じ原寸 382×547 の部品が他に 32 枚あり、面 1000 のコマ 37 行は全て位置 0,0 なので、`null.png` の原寸が外形に数えられるようになっても 382×547 を超えない）。
6. When 要件 5.5 の変化を既存のテストへ反映する, the 本仕様 shall この 1 件の脱落を留めているテスト（`crates/areka-emo-atlas/src/emo2_e2e.rs` の `emo2_shell_all_elements_baked` と定数 `SHELL_NORMALIZE_SEAM_KEY` の説明・シェルとバルーンを併せて焼く `emo2_balloon_same_bake_path_as_shell`・`crates/areka-emo-atlas/src/emo2_golden.rs` の `emo2_shell_bake_is_deterministic` の失敗集合の比較・同ファイルの `emo2_shell_matches_golden` と、その期待値 `crates/areka-emo-atlas/src/testdata/emo2_shell_golden.txt`＝今日は 54 行で `null.png` の行は **0 行**・適用後は `null.png` の 1 行が増える）を、**消さずに**「失敗 0 件・`null.png` は全画素が透明な絵として索引表に載る」を確かめる形へ書き換える。「抜き色は未実装として落ちる」を留めている `normalize.rs` の単体テストのうち `on_no_alpha_no_pna_selects_keycolor_seam` も同じく、抜かれた結果を確かめる形へ書き換える。もう 1 本の `off_no_pna_selects_keycolor_seam`（`UseSelfAlpha::Off`）は、`Off` を渡す経路が areka に **0 本**であり、`Off` かつ α チャンネル付きの絵は受け取る画素の形式の都合で正しく抜けないので、`Off` の下の抜き色をどこまで実装するかを設計で定め、その定めに合わせて書き換えるか据え置く（ギャップ分析 7 節の論点 8）。`.pna` と `full` の腕のテストは書き換えない（変更 0）。
7. The areka shall 「既知の α 無し `null.png` が落ちる」と説明している注記（`build_boot_assets`・`build_shell_assets`・`crates/areka/examples/` の 3 か所）を、事実に合わせて直す。
8. The areka shall 「`element0` が在れば画像を使わない」が壊れたらテストが赤になることを、面 10 の形（`element0` の画像 336×400 と、面の画像 427×463 が**別の絵**）で示す。面 0 の形（`element0` と面の画像が同じ不透明な絵）では、誤って二重に重ねても結果のバイトが変わらず検査にならないので、面 0 だけに頼らない。
9. The areka shall 実機で `emo2` を起動し、立ち絵・バルーン・撫で・メニュー・終了が適用前と同じに見えることを 1 度確かめる（要件 8）。

### Requirement 6: 記録

**Objective:** 開発者とシェル作者として、どの画像がどの面に解決され、どれが使われず、どの絵が抜き色で描かれたかをログから読み取りたい。

#### Acceptance Criteria

1. When シェルの画像の一覧を取り終えた, the areka shall `info!` で 1 行（認めた画像の数・土台に使った数・`element0` が在るため使わなかった数）を記録する。
2. When 面の画像を `element0` が在るために使わなかった, the areka shall 面の番号とファイル名を `debug!` で記録する（`emo2` では 2 件）。
3. When 絵を抜き色で扱った, the areka shall 絵の名前と抜いた色を `debug!` で記録する（`R_POST_and_KOMAINU` では面の画像 10 枚・`konnoyayame` では 18 枚・`emo2` では 1 枚）。
4. The areka shall 失敗と縮退の経路（一覧が取れない＝要件 1.7・同じ番号の重複＝要件 1.5・番号が大きすぎる＝要件 1.6・相手の無いコマ＝要件 3.5・未実装の透過の扱い＝要件 4.9）を各要件のとおり記録し、記録の無い失敗経路を持たない。
5. The areka shall 記録の書式を steering `logging.md`（構造化フィールド・スコープ接頭辞）に従わせる。

### Requirement 7: 決定論テスト

**Objective:** 開発者として、ファイル名の読み方・土台の絵の決まり方・抜き色を、実機に頼らず毎回のテストで確かめたい。

#### Acceptance Criteria

1. The areka shall 要件 1 の名前の判定を、ファイルの一覧（文字列の列）を入力にした純粋な関数として確かめる: 4 表記が同じ面 0 になること・要件 1.3 の各形が 0 件になること・大文字小文字・重複時の採り方・番号が大きすぎる名前。
2. The areka shall 要件 2.1 の表のア〜エの 4 通りを、画像の実寸の違いが外形に現れる入力で確かめる（アは画像の実寸・イは画像と `element1` の合わさった外形・ウは `element` だけの外形・エは現状の失敗）。
3. The areka shall 抜き色（要件 4.1〜4.3・4.6・4.7）を、画素を決め打ちした小さな絵で確かめる: 左上と同じ色は離れた場所でも透明になること・1 成分だけ 1 違う色は不透明のまま残ること・透明にした画素に色が残らないこと・全画素が同じ色の絵が失敗にならず全画素透明になること・α チャンネルを持つ絵は 1 バイトも変わらないこと（要件 4.4）。
4. The areka shall 検体 `R_POST_and_KOMAINU` を入力に、実物の絵を焼いて、面 0 の外形が 236×462・面 10 が 140×160 になること、宣言の無い面 10 が存在する面として引けること、面 0 の左上の画素が透明で、焼いたときの失敗が 0 件であることを確かめる。
5. The areka shall 検体 `konnoyayame` を入力に、実物の絵を焼いて、面 0 の外形が 260×390・面 10 が 200×200 になること、面 0 のまばたきのコマが面 1031・1032・1033 の画像を位置 93,103 に描く命令になること（0 件のまま素通りしないこと）、焼いたときの失敗が 0 件であることを確かめる。
6. The areka shall 要件 3.6（採寸と表示が同じ結果）を、同じ検体に対する 2 つの経路の外形の一致で確かめる。
7. The areka shall 要件 3.7・3.8（`surface.append` が画像だけの面には効き、どちらも無い番号には効かない）を確かめる。
8. The areka shall 要件 5（`emo2`）を、寸法と合成結果の既存のテストが書き換えなしで緑であること、要件 5.6 の書き換えたテストが緑であること、要件 5.8 の摂動（「`element0` が在れば使わない」を外すと赤になる）で示す。
9. The areka shall 上のテストのうち少なくとも、先頭の 0 の無視・表のウ・コマの相手の解決・抜き色の完全一致の 4 つについて、その判断を壊すと赤になることを摂動で示す。
10. The areka shall 要件 11（間隔の語）を、`sometimes` が `random,2` と・`rarely` が `random,4` と同じ再生の引き金になること、`always` など他の語が今日どおり再生の対象にならず `debug!` に元の語が残ることで確かめ、読み替えを外すと赤になることを摂動で示す。検体 `konnoyayame` を入力に、面 0 の `animation0` が再生の対象として採られること（今日は 0 件）を確かめる。
11. The areka shall 検体を共有ヘルパ（`sample_ghost_kit::SampleRoot::acquire`）経由で受け、`vendors/sample_ghost/<検体名>/` の直パスを書かない。
12. The areka shall 新しく足すテストを新しいファイルに置き、1 ファイル 1,000 行以内に収める。`crates/areka/src/placement/measure_tests.rs` は **981 行**・`crates/areka/src/emo2_boot/assets_tests.rs` は **976 行**（2026-09-19 実測）で、どちらにも追記の余地は無い。行数の検査（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表には触らない。

### Requirement 8: 実機確認

**Objective:** 開発者として、テストでは見えない「画面に出た絵」を、検体 3 体で 1 度ずつ目で確かめ、結果を記録に残したい。

#### Acceptance Criteria

1. The areka shall 実機確認を、絶対パスでの起動・32bit 補助プロセスの先行ビルド・`AREKA_APP_SMOKE_EXIT_MS` による有界の自動終了・`RUST_LOG` によるログの採取、の定石で行い、確認項目と結果を tasks の完了記録に残す。バルーンを同梱しない検体（`R_POST_and_KOMAINU`・`konnoyayame`）には `vendors/sample_ghost/StayseeBalloon` の絶対パスをバルーンの根として渡す。
2. The areka shall `R_POST_and_KOMAINU` について次を確かめる: ⑴ 本体側（236×462）と相方側（140×160）の絵が、地色の四角ではなくキャラクターの形に抜かれて出る ⑵ 起動挨拶がバルーンに出る ⑶ 絵の外（抜かれた場所）のクリックが背後の窓へ抜ける ⑷ 右クリックメニューの 1 項目目が既定名「説明書」でなく、辞書 `dic06_String.txt` の 3 候補（「現在のシェルについて(&R)」「取扱説明書(&R)」「Read me(&R)」）のどれかになり、`(&R)` に下線が付く（完了済み `areka-P0-popup-menu-minimal` の実機確認 9.3 ⑷ の引き受け。項目名の差し替えそのものは同仕様の決定論テストが留めているので、欠けているのは目視だけである）。
3. The areka shall `konnoyayame` について次を確かめる: ⑴ 本体側（260×390）と相方側（200×200）の絵が、キャラクターの形に抜かれて出る ⑵ 本体側がまばたきし、目の周りに四角い地色が出ない（コマの相手 3 枚が慣習で解決でき、抜き色が効き、`sometimes` が再生された証拠）⑶ 起動挨拶の文字が文字化けせずにバルーンに出る（SHIORI 通信の初期の文字コードは `Shift_JIS`（`charset_initial`・`source="default"`）・ゴーストの `descript.txt` は `charset,UTF-8` 。台本の段では 2026-09-19 の実走で文字化け 0 件を確かめてあり、残っているのは画面の上での目視である）⑷ シェルの `sakura.balloon.alignment,none`／`kero.balloon.alignment,none` が自動調整として効く（キャラクター窓が画面の右半分に居ればバルーンは左隣、左半分なら右隣）。
4. If 要件 8.2・8.3 の確認で、テンプレートゴースト 2 体の動作（起動・絵・起動挨拶・まばたき・クリック透過・メニュー・バルーンの位置・終了）を妨げる欠陥が見つかった, then the 本仕様 shall それを別件へ送らず、要件と設計を改訂して本仕様の中で直す（開発者裁定 2026-09-19・要件 10.7）。直すのに独立した仕様 1 本ぶんの規模が要ると分かったときだけ、黙って先送りせず開発者に諮る。検体 2 体が使っていない機能の欠陥は、この限りでない（引受先を確かめて起票する）。
5. The areka shall `emo2` について、適用前と同じに見えること（要件 5.9）と、起動時の `warn!`「shell bake で脱落した element」が 0 回であること、`null.png` の「全透明」の `warn!` が出ていること（どちらも要件 5.5）を確かめる。
6. When ログに「0 件」を根拠として書く（例: `EmptyComposition` が 0 件・脱落の `warn!` が 0 回）, the 本仕様 shall 同じ走行の中に `debug` の行が実在することを併せて示す（`RUST_LOG` は target 名で、未設定や書式の誤りは黙って `info` へ落ちるため）。

### Requirement 9: 網羅台帳と文書

**Objective:** 網羅調査とロードマップの読み手として、本仕様が正典のどこを引き受け、どこを引き受けていないか、引き受け手の居ない残りが何かが、文書で分かってほしい。

#### Acceptance Criteria

1. The 本仕様 shall 網羅台帳の担当欄（`owner`）へ新しく登記する項目を、間隔の語の **2 件**（`ukadoc:descript_shell_surfaces:sometimes:1`・`ukadoc:descript_shell_surfaces:rarely:1`。どちらも今日は担当が空欄）だけとし、実装の着地時に状態と担当を実測に合わせて改める。ファイル名の慣習と抜き色については **0 件**である。C1〜C3・C7・C10・C14 を持つページ `dev_shell` と、C8・C9・C13 を持つページ `manual_shell` は、台帳に「ページ 1 枚」の粒度でしか無く（`doc/ukadoc-coverage/ledger/assets.toml` の `ukadoc:dev_shell`・`ukadoc:manual_shell`）、台帳の id はカタログに実在しなければならないので、項目の行を本仕様が足すことはできない。
2. The 本仕様 shall `areka-P0-ukadoc-coverage-roadmap` への依頼「`dev_shell`・`manual_shell` の当該の文（ファイル名の慣習・抜き色）を項目へ割り、担当を本仕様にする」を、完了時の申し送りとして文書に残す。
3. When 実装が着地した, the 本仕様 shall 台帳の備考のうち本仕様で事実が変わる箇所を実測に合わせて直し、状態と担当は変えない。少なくとも次の 2 項目: ⑴ `descript_shell_surfaces` の `element*`（担当 `areka-P0-shell-parse`・縮退）＝「面の画像と `overlay` の `element0` の関係は正典どおりになった。`overlay` 以外の描画メソッドが読み飛ばされる縮退は残り、その間は `overlay` 以外の `element0` を持つ面で画像が土台に使われる（正典では破棄。要件 2.7 の既知のずれ）」 ⑵ `descript_shell` の `seriko.use_self_alpha,値`（未対応）＝「宣言は今も読まず常に `1` 相当。その下で α の無い絵は正典どおり抜き色で描かれるようになった。`.pna` と `full` は未実装のまま」。報告を作り直し、`cargo test -p ukadoc-survey` が緑であることを確かめる。
4. When 実装が着地した, the 本仕様 shall 正典が沈黙している点の裁量を `doc/COMPAT_ARCHITECTURE.md` §8 の沈黙ルール対応表へ追記する: 大文字小文字を区別しない（要件 1.4）・同じ番号の重複は辞書順で最小を採る（要件 1.5）・抜き色の許容幅は 0（要件 4.2）・パレットの `tRNS` の扱い（要件 4.8 で設計が定めたもの）。
5. When 実装が着地した, the 本仕様 shall steering `roadmap.md` に、**引き受け手の居ない残り**として次を登記する（2026-09-19 時点で `roadmap.md` にこれらを引き受ける spec は 0 本。実在しない spec の名前を引受先として書かない）: ⑴ `.pna`（開発者確認済みの方針は「非対応」）⑵ `seriko.use_self_alpha,full`／`use_self_alpha,full` の全面不透明 ⑶ 透過の宣言を読むこと（常に `1` 相当の固定）⑷ 全画素が透明になる面（C14 の単色の `surface10`）を表示の対象にしたときの扱いが、検体 0 件で未確認であること ⑸ `surfaces.txt` が無い／波括弧が 0 個で画像だけのシェル（C1 は成り立つ形とするが、起動の失敗のまま）⑹ `overlay` 以外の `element0` を持つ面で画像が使われる既知のずれ（要件 2.7。直る時機は `areka-P0-shell-parse` の縮退の解消と同じ）⑺ `sometimes`・`rarely` 以外の間隔の語。
6. The 本仕様 shall steering `roadmap.md` の本仕様の行にある「裁定 2 件（「element0 より下」の層表現と `surface.append` の順序）」が、どちらも正典の逐語（C4・C5／C6）で決まったことと、範囲に抜き色が加わったこと（要件 10.1）を、完了時に同じ行へ反映する。

### Requirement 10: 裁定の記録

**Objective:** 開発者と後続フェーズの担当として、brief から変わった点と、その根拠が 1 か所で読めてほしい。そうすれば、要件ディスカッションと設計が brief の古い読みを持ち込まない。

#### Acceptance Criteria

1. The 本仕様 shall 抜き色の透過（要件 4）を本仕様の範囲に**含める**（開発者裁定 2026-09-19・逐語:「抜き色形式は新しいゴーストでは使われない形式なので、当面サポートするつもりは無かったのですが、、、Aの対応可能ですか？実装スコープが変わりますが、この際対応した方がよいと判断しました。」）。brief の Scope はこれを In にも Out にも挙げていなかった。両テンプレートの絵が全て α チャンネルを持たず、抜き色が無ければ brief の第一の成果（絵が出る）に届かないことが要件段階の実測で分かったためである。触るクレートに `areka-emo-atlas` が加わり、規模は brief の見立て（M）の上の端になる。
2. The 本仕様 shall 抜き色の範囲を、透過の正規化の抜き色の腕（`AlphaSource::KeyColor`）**だけ**に限る。`.pna` の腕・`full` の腕・宣言の読取は範囲外で、変更 0 である（要件 4.9・要件 9.5）。
3. The 本仕様 shall 「`element0` を持つ面では画像を使わない」（要件 2.1 の表のウ）を、brief の読み（「`element` を持つ面にも下に 1 層増える」）に代えて採る。根拠は C4・C5 の逐語である（「正典の引き直し」の 1）。
4. The 本仕様 shall 「画像だけで存在する面にも `surface.append` は効く」（要件 3.7）を採る。根拠は C6 の逐語である（「正典の引き直し」の 2）。
5. The 本仕様 shall brief が「対象に含めるかは要件で決めてよい」とした、どこからも引かれていない画像（`konnoyayame` の面 1030・1040〜1043）を、特別扱いせず規則のまま面として扱う（要件 3.9）。
6. Where 開発者が上の裁定を覆した, the 本仕様 shall 該当する要件と設計を同時に改訂する。
7. The 本仕様 shall 範囲の物差しを「**テンプレートゴースト 2 体（`R_POST_and_KOMAINU`・`konnoyayame`）を動かすのに要る実装は、本仕様が巻き込む**」とする（開発者裁定 2026-09-19・逐語:「この際、テンプレートゴーストを動かすために必要な実装を、本specで巻き込んで対応する方向でスコープ拡大してもらえるか？その前提で他の議題も調整せよ。」）。「要る」とは、検体 2 体のどちらかが、起動から終了まで（要件 8.2・8.3 の確認項目）の道筋で実際に使っていることを指す。この物差しで、要件ディスカッションの 3 議題を次のとおり定めた: ⑴ `sometimes` の再生は**範囲内**（`konnoyayame` のまばたきが使う・要件 11）。`rarely` は検体に 0 件だが、`sometimes` と同じ読み替えの 1 語ぶんで済み、残せば引き受け手の居ない語が 1 つ増えるだけなので併せて入れる ⑵ `overlay` 以外の `element0` は**範囲外**（検体に 0 件・要件 2.7）⑶ `surfaces.txt` が無いシェルは**範囲外**（検体に 0 件・Boundary Context）。触るクレートに `areka-seriko`（または `areka-parsers` の間隔の語の読み取り）が加わり、規模は L の下の端になる。

### Requirement 11: まばたきの間隔の語 `sometimes`・`rarely`

**Objective:** 利用者として、テンプレートゴーストのキャラクターが、絵が出るだけでなく、まばたきをして生きて見えてほしい。

#### Acceptance Criteria

1. While 面が表示されている, the areka shall その面の `animationN.interval,sometimes` のアニメーションを、`interval,random,2` と同じ頻度（毎秒 2 分の 1 の確率）・同じ仕組みで再生する（C15・C17）。
2. While 面が表示されている, the areka shall その面の `animationN.interval,rarely` のアニメーションを、`interval,random,4` と同じ頻度（毎秒 4 分の 1 の確率）・同じ仕組みで再生する（C16・C17）。
3. The areka shall `random,数値`・`bind+random,数値`・`bind` の今日の扱いを変えない（変更 0）。
4. The areka shall `sometimes`・`rarely` 以外の間隔の語（`always`・`runonce`・`never`・`yen-e`・`talk,数値`・`periodic,数値` など）を、今日どおり値として保ち、再生の対象にせず、元の語を添えた `debug!` を残す（変更 0）。
5. When `konnoyayame` の面 0 が表示されている, the areka shall `animation0`（`pattern0`〜`5`・面 1031〜1033 を位置 93,103 に重ね、終端の `-1` で元へ戻す）を再生し、まばたきとして見せる。
6. The areka shall `emo2` の再生されるアニメーションの集合を変えない。`emo2` のシェルの `interval,sometimes`・`interval,rarely` は **0 件**である（2026-09-19 実測。`interval` の行は 37 件＝`bind` 30・`bind+random` 3・`random` 4。同じ数え方で `konnoyayame` は 1 件と出ることを確かめてある）。
7. The areka shall 読み替えをどの段（間隔の語の読み取りか、再生の表の組み立てか）で行うかを設計で定める。どちらで行っても、読み取った元の語（`sometimes`／`rarely`）が記録から読み取れること。

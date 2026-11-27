# Brief: balloon-directional-offset-keys

> 2026-11-27 `balloon-script-offset-align` の要件の討議（決まったことの 6）で `/kiro-discovery` で起票（対話なし・着手中の spec からの切り出し）。同 spec は接尾辞の無い descript のずらしだけを台本の向きと `@` の相対の指定の基準にし、方向つきのずらしと `surfaces.txt` のずらしは読まない（同 spec の `requirements.md` の Out of scope）。どちらも受け持ちの spec が無い。区分 **C**（ukadoc の拾い残し）。
>
> 同じ日の `balloon-script-offset-align` の設計の討議（議題 5）で、頭上・足元に出すときの縦の基準（0 の位置）の SSP との違いを**件 2** として足した（本文の下の節）。触る式（`resolver.rs` の `keyword_balloon_pos`）と ukadoc の段落（「0の位置(基準)が方向ごとに違う」）がこの spec と同じなので、別の spec に分けずにまとめた。ここまでの本文を件 1 とする。

## Problem

- シェルの作者は、バルーンを左・右・頭上・足元のどこに出すかで違うずらし量を書ける（descript の `balloon.offsetx` の後ろに方向の 1 文字）。また、サーフェスごとにバルーンのずらし量を変えられる（surfaces.txt の surfaceブレスの `balloon.offsetx`／`offsety`）。areka はどちらも読まないので、書いたシェルでは、バルーンが作者の決めた位置に出ない。記録も出ない（黙って壊れる）。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 10 行が `status = "absent"`・宛先が空・優先度 A2 のまま。descript の 4 行は「接尾辞の付いた形はどこからも引かれない」、surfaces.txt の 6 行は「areka-parsers の shell モジュールは surfaces.txt の中に書かれたバルーンのずらし量を、どの走査でも見ない」と書いている。
- `\![set,balloonoffset]` の `@` の相対の指定と `\![set,balloonalign]` の向きは、ukadoc では descript と surfaces.txt の両方の値を基準にする。`balloon-script-offset-align` が着地しても、基準は接尾辞の無い descript の値だけのまま残る。

## ukadoc の定め（逐語）

- descript.txt（シェル）の `sakura.balloon.offsetx`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#sakura.balloon.offsetx>・対応 Materia / SSP / CROW）:
  - 「offsetx / offsety の後ろに方向を表す1文字を付けると、バルーンをどこに出すかによって違う値を使わせることができる(SSPのみ)。」
  - 「offsetx / offsety: 方向指定なし。下記が書かれていない時に使われる既定値。」
  - 「offsetxl / offsetyl: バルーンをシェルの左に出す時。」
  - 「offsetxr / offsetyr: バルーンをシェルの右に出す時。」
  - 「offsetxc / offsetyc: バルーンをシェルの頭上・足元に出す時。中央寄せになる2通りで共通。」
  - 「offsetxb / offsetyb: バルーンをシェルの足元に出す時。頭上と足元で違う値にしたい場合に使う。」
  - 「offsetxb が無い時: offsetxc → offsetx の順に探す。」「offsetxc / offsetxl / offsetxr が無い時: offsetx を使う。」「X座標とY座標はそれぞれ独立に決まる。」
  - 「どちらに出すかは、バルーンのdescript.txtのwindowposition.x、 シェルのsakura.balloon.alignment、 さくらスクリプトの\![set,balloonalign,ID]で決まる。」
  - 「符号に注意。X座標の＋はどの場合もシェルに近づく向きで、左に出す時は右方向、右に出す時は左方向が＋になる (頭上・足元は右方向が＋)。Y座標の＋は常に下方向。」
  - 「ただし0の位置(基準)が方向ごとに違う。左右はバルーンの上端とシェル画像の上端が揃う位置、頭上はバルーンの下端がシェル画像の上端に接する位置、足元はバルーンの上端がシェル画像の下端に接する位置。」
  - 「ゴースト側のdescript.txtにも書ける。シェル側に書かれていればシェル側が優先され、書かれていなければゴースト側の値が使われる。」
- descript.txt の `kero.balloon.offsetx`／`offsety`・`sakura.balloon.offsety`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#kero.balloon.offsetx> ほか）: 「方向を表す1文字(l / r / c / b)を後ろに付ける書き方についてはsakura.balloon.offsetxを参照。」
- descript.txt の `char*.balloon.offsetx`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell.html#char-star-.balloon.offsetx>・対応 SSP）: 「\p[*]のバルーン位置X座標。char0はsakura、char1はkeroと同じ指定になる。」「座標の意味と、方向を表す1文字(l / r / c / b)を後ろに付ける書き方は、sakura.balloon.offsetxと同じ。」（`char*.balloon.offsety` も同じ）
- surfaces.txt の surfaceブレスの `sakura.balloon.offsetx`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#sakura.balloon.offsetx>・対応 Materia / SSP / CROW）:
  - 「そのサーフェスでのみの本体側バルーン位置X座標。」方向の 1 文字の 5 つの綴りは descript と同じ字面で並ぶ。
  - 「offsetxb が無ければ offsetxc、それも無ければ offsetx が使われる。 offsetxc / offsetxl / offsetxr が無い時は offsetx が使われる。X座標とY座標はそれぞれ独立に決まる。」
  - 「ここで書いた値は、シェルのdescript.txtに書いた同名の値より優先される。」
  - 「符号の向きはdescript.txtのsakura.balloon.offsetxと同じ。」
  - 既定値: 「左右に出す時：バルーンの左右端と、サーフェスの右左端が揃う位置」「頭上・足元に出す時：サーフェスの中央」
- surfaces.txt の `kero.balloon.offsetx`／`offsety`・`sakura.balloon.offsety`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#kero.balloon.offsetx> ほか）: 「そのサーフェスでのみの相方側バルーン位置X座標。」「方向を表す1文字(l / r / c / b)を後ろに付ける書き方についてはsakura.balloon.offsetxを参照。」
- surfaces.txt の `balloon.offsetx`（<https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#balloon.offsetx>・対応 SSP）: 「そのサーフェスでのみのバルーン位置X座標(本体・相方問わず)。」「offset.xとも記述可能。」「方向を表す1文字(l / r / c / b)を後ろに付ける書き方についてはsakura.balloon.offsetxを参照。」（`balloon.offsety` は「offset.yとも記述可能。」）
- `\![set,balloonalign,ID]`（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonalign-ID>・対応 SSP）: 「下記の位置を基準に、シェルのdescript.txtのballoon.offsetx及びballoon.offsetyの値、surfaces.txtのballoon.offsetx及びballoon.offsetyの値が反映される。」「方向ごとに違うオフセット値を使わせることもできる。 left/rightではoffsetxl/offsetxr、center・topではoffsetxc、bottomではoffsetxb(無ければoffsetxc)が優先される。」
- `\![set,balloonoffset,x,y]`（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonoffset-x-y>・対応 SSP / Ninix）: 「頭に@をつけるとdescript.txtやsurfaces.txt内のオフセット値からの相対座標指定となる。」

まとめると、読む綴りは次のとおり（ukadoc にある綴りだけ・新しい綴りは作らない）。

| ファイル | 接頭辞 | 綴り（x と y の両方） |
|---|---|---|
| シェル（とゴースト）の descript.txt | `sakura.`・`kero.`・`char*.` | `balloon.offsetx`／`offsety` と、後ろに `l`・`r`・`c`・`b` を付けた 4 つ |
| surfaces.txt の surfaceブレス | `sakura.`・`kero.` | 同じ 5 つ |
| surfaces.txt の surfaceブレス | 無し（本体・相方を問わない） | 同じ 5 つ。接尾辞の無い形は `offset.x`／`offset.y` とも書ける |

## Current State

- descript のずらしは `crates/areka-placement/src/config.rs:262-275` の `resolve_scope` が、接尾辞の無い `balloon.offsetx`／`balloon.offsety` だけを `cascade2`（`config.rs:195-202`＝シェル ＞ ゴースト）で引く。接頭辞は `scope_prefixes`（`config.rs:167-173`＝`sakura.`／`char0.`・`kero.`／`char1.`・`charN.`）が持つ。結果は `ScopeConfig::balloon_offset` と、右に出すときの符号の反転に使う `balloon_offset_descript_x`（`config.rs:66-71`）の 1 組だけで、方向ごとの値を持つ欄が無い。
- 置き方は `crates/areka-placement/src/resolver.rs:337-385`（P5）。同じ 1 組のずらしを、キーワードの頭上・足元（`keyword_balloon_pos`）にも左右にも足す。右に出すときは descript の x の分だけ符号を反転する（`resolver.rs:370-376`）。キーワードの位置の出し直しは `follow/keyword_base.rs`。倍率の換算は `crates/areka-placement/src/lib.rs:530` の `scale_then_merge`（`lib.rs:504` が `balloon_offset_descript_x` を換算値で置き直す）。
- surfaces.txt の読み手（`crates/areka-parsers/src/shell/decode.rs`）は、surfaceブレスの中から element・collision・animation などの決まった行だけを拾い、それ以外の行は値にしない（`decode.rs:24` のモジュールの説明）。`Surface`（`crates/areka-parsers/src/shell/model.rs:60-70`）に、バルーンのずらしの欄は無い。サーフェスが替わったときに置き方のずらしを選び直す道も無い。
- 網羅台帳の行（どれも `absent`・宛先が空）: descript＝`assets.toml` の 4673（`kero.balloon.offsetx`）・4693（`kero.balloon.offsety`）・5517（`sakura.balloon.offsetx`）・5537（`sakura.balloon.offsety`）、surfaces.txt＝6259（`balloon.offsetx`）・6275（`balloon.offsety`）・7137・7153（`kero.`）・7611・7627（`sakura.`）。descript の `char*.balloon.offsetx`／`offsety` の行は台帳に無い（写しが古い＝`ukadoc-catalog-refresh` の持ち物。着手のときに有無を見直す）。
- 手元のゴースト（emo2・ぱすた・どっとさくら）と試験の検体に、接尾辞の付いた綴りと surfaces.txt のずらしを書いたものは無い（`git grep` で 0 件）。

## Desired Outcome

- descript（シェルとゴースト）の方向つきの綴りと、surfaces.txt の今のサーフェスのずらしを、ukadoc の探す順と優先（surfaces.txt ＞ シェルの descript ＞ ゴーストの descript、方向つき ＞ 接尾辞なし、`b` → `c` → 接尾辞なし）で選び、バルーンをそこに出す。x と y は別々に決まる。
- 起動の配置・本体への追従・キーワードの位置の出し直し・倍率の変化・押し戻しの関門・サーフェスの切り替えのどれも、同じ選び方の値を読む。
- `\![set,balloonalign]` で向きを変えたときはその向きの値を、`\![set,balloonoffset]` の `@` は選んだ値を基準に使う（`balloon-script-offset-align` が作る台本の値の層の基準を、この spec の選び方に付け替える）。
- 網羅台帳の 10 行（と、在れば `char*.` の行）が実装済みになり、`doc/COMPAT_ARCHITECTURE.md` §8 に選び方の行が載る。

## Approach

- descript の読みは `resolve_scope` の中で、方向ごとの値（左・右・頭上と足元の共通・足元）を軸ごとに引き、選ぶのは向きが決まった所（P5 とキーワードの出し直し）で行う。向きの決まり（`balloon.alignment`・`windowposition.x` のキーワード・台本の向き）と右の符号の決まりは今のまま使う。
- surfaces.txt は読み手（`shell/{model,decode}.rs`）が surfaceブレスのずらしの行を値にし、提示が今のサーフェスの値を置き方へ渡す。サーフェスが替わったら選び直して置き直す（渡す道と、置き直しをどの相で行うかは設計の段）。
- 選び方は 1 つの純粋な関数にまとめ、方向・ファイルの層・接尾辞の探す順を表で固定する決定論のテストを置く。ukadoc が黙っている点は要件の段で SSP の MCP で測る（毎回開発者の許しを取る）。

## Scope

- **In**: descript（シェルとゴースト）の方向つきの 4 綴り×2 軸×3 接頭辞の読み・surfaces.txt の surfaceブレスのずらし（`sakura.`・`kero.`・接頭辞なし、方向つきと `offset.x`／`offset.y`）の読み・選び方の関数・置き方の全部の道への反映・サーフェスの切り替えでの選び直し・台本の値の層の基準の付け替え・記録の欄・決定論のテスト・網羅台帳の行と §8 の行・SSP の測り。件 2 の縦の基準の調べ（SSP の測り）と、合わせると決めたときの `keyword_balloon_pos` の基準の直し。
- **Out**: 向きの決め方そのもの（`balloon.alignment`・`windowposition.x`・`\![set,balloonalign]` の受け口）・右に出すときの符号の決まり・左右の並べ替え（`balloon-flip-side-rules`）・押し戻しの関門の画面の選び（`balloon-script-offset-align`）・倍率の値（`shell-scale-linkage`）・surfaces.txt の `point.basepos`（`surfaces-basepos`）。

## Boundary Candidates

- 綴りの読み（descript の方向つき・surfaces.txt の surfaceブレスの行）
- 選び方（方向・ファイルの層・接尾辞の探す順から 1 組を決める純粋な関数）
- 置き方への反映とサーフェスの切り替えでの選び直し

## Out of Boundary

- バルーン側の `windowposition.x`／`windowposition.y` の調整量（今のまま後から足す）。
- 3 体以上の窓のバルーン（`extra-character-windows`）。`char*.` の綴りは読むが、窓が無いスコープの置き方はこの spec では作らない。

## Upstream / Downstream

- **Upstream**: 着手中の `balloon-script-offset-align`（台本の値の層・`\![set,balloonalign]` の `center`・`bottom` の向き・`resolver.rs` を触る＝その着地の後に取る）・完了 `descript-window-keys`（descript の鍵の読み）・完了 `balloon-flip-side-rules`（右の符号と並べ替え）・完了 `shell-scale-linkage`（倍率の換算）。
- **Downstream**: 無し（使うシェルが現れたら効く）。

## Existing Spec Touchpoints

- **Extends**: なし（`balloon-script-offset-align` の Out of scope からの切り出し）。
- **Adjacent**: `balloon-script-offset-align`（`resolver.rs`・台本の値の層）・シェルの element の列の読み手の spec（`shell/{model,decode}.rs` を触る `draw-methods-canon`・`element-clipping-option`・`ghost-alias-file`・`collisionex-regions`＝同じウェーブに置かない）・`balloon-canon-residue`・`window-snap`（`resolver.rs`）・`defaultx-canon`（`config.rs` の `resolve_scope`）・`surfaces-basepos`（surfaceブレスの同じ読み手）。

## Constraints

- 読めない値は使わずに `warn!`（未検証の字は `?` で残す）。失敗の道は必ず記録（`error!`＋`Err`）。
- テストは兄弟のファイル `*_tests.rs`。`config.rs`（711 行）・`decode.rs`（726 行）は上限の近くへ寄る＝足して 1,000 を超えるなら新しいファイルへ置く。
- SSP を測るときは毎回開発者の許しを取る。試験の根・検体はワークツリーの `target\` の下だけ。

- **優先度の区分**: C（ukadoc の拾い残し・台帳の 10 行は A2）。
- **版**: いつか（区分 C でバグではない・今の版 v0.2.0 の wintf・dola の基幹の整備に当たらない・手元のゴーストと検体に使うものが無い）。上流の `balloon-script-offset-align`（v0.2.0）より後の版なので、版をまたぐ依存の向きの決まりに合う。
- **要件定義のモデル**: Opus。綴りの読みは既存の `resolve_scope` と surfaceブレスの読み手の型どおりで、選び方は ukadoc が探す順を書いている。残る決めは ukadoc が黙っている数点で、SSP の測りで決まる。
- **規模**: M〜L（14〜21 タスクの見込み・件 1＝12〜18・件 2＝2〜3・見積もりは要件の段）。
- **分割の案**: なし（descript と surfaces.txt の値は同じ選び方の関数で 1 組に決まる＝分けると探す順と優先の決まりを 2 か所に書くことになる）。
- **要件の段の議題**（答えで作業が変わるもの・ukadoc が黙っている点は SSP で測ってから）:
  1. ファイルの層と接尾辞の探す順の掛け合わせ（例: surfaces.txt に接尾辞なしだけ、シェルの descript に `offsetxr` があり右に出すとき、どちらを使うか）。
  2. surfaces.txt の接頭辞なしと `sakura.`／`kero.` が同じ surfaceブレスに両方あるときの優先と、`offset.x` に方向の 1 文字を付けた綴りを受けるか。
  3. 「そのサーフェス」はどのスコープのサーフェスか（本体の今のサーフェスか、そのバルーンの持ち主のスコープのサーフェスか）と、サーフェスが替わったときのドラッグのずらし・記憶の `BalloonOffset` の扱い。
  4. surfaces.txt の既定値の書き方（「バルーンの左右端と、サーフェスの右左端が揃う位置」）が、今の P5 の 0 の位置（シェルの窓の端）と同じか。
  5. 件 2: SSP の頭上・足元の縦の基準が何か（面の見える範囲か、当たりの矩形か、ほかか）と、areka をそれに合わせるか。

## 件 2: 頭上・足元に出すときの縦の基準が SSP と違う

### Problem

- `\![set,balloonalign,center]`（`top` も同じ）・`\![set,balloonalign,bottom]` と、バルーンの descript の `windowposition.x` の `center`・`top`・`bottom` で、areka はバルーンの下端（頭上）か上端（足元）を本体の窓の上端か下端に接して置く。SSP はそこに接して置かない。同じゴーストでも、バルーンの縦の位置が SSP と違う。
- SSP が何を基準にしているか（面の見える範囲か、当たりの矩形か、ほかか）は、まだ確かめていない。

### ukadoc の定め（逐語）

- `\![set,balloonalign,ID]`（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonalign-ID>・対応 SSP）:
  - 「center: シェルの中央上座標に、バルーンの中央下座標が接する。」
  - 「top: centerと同じ。」
  - 「bottom: シェルの中央下座標に、バルーンの中央上座標が接する。」
- バルーンの descript.txt の `windowposition.x`（<https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.x>・対応 Materia / SSP / CROW）: 「center(SSPのみ top でも同じ)でシェルの中央上に固定。」「bottom (SSPのみ)でシェルの中央下に固定。」
- バルーンの descript.txt の `windowposition.y`（<https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.y>・対応 Materia / SSP / CROW）: 「center,top……バルーン下端とシェル画像の上端が接する位置」「bottom……バルーン上端とシェル画像の下端が接する位置」
- シェルの descript.txt の `sakura.balloon.offsetx`（上の件 1 の節と同じ URL）: 「頭上はバルーンの下端がシェル画像の上端に接する位置、足元はバルーンの上端がシェル画像の下端に接する位置。」

ukadoc は「シェル」「シェル画像」とだけ書き、それが窓の矩形か、面の見える範囲か、当たりの矩形かは書いていない。

### Current State

- 幾何の式は `crates/areka-placement/src/resolver.rs:183-206` の `keyword_balloon_pos` の 1 か所。頭上は `char_pos.y − balloon_size.h`（`resolver.rs:194`）、足元は `char_pos.y + char_size.h`（`resolver.rs:196`）で、`char_pos`・`char_size` は本体の窓の位置と大きさ。使う所は P5（`resolver.rs:350`）と、表示の大きさが決まったときの出し直し（`crates/areka-placement/src/follow/keyword_base.rs:124`）。`windowposition.x` のキーワードの語の読みは `crates/areka-placement/src/windowposition.rs:92-93`。台本の `\![set,balloonalign]` の `center`・`bottom` も、`balloon-script-offset-align` の着地の後は同じ式を通る（同 spec の design は式を変えない）。
- SSP の測り（`balloon-script-offset-align` の `research.md` 8 節・SSP の MCP・ゴースト「窓際のぱすたさんDEBUG」の `\0`・DPI 144）: 本体の窓 `currentghost.scope(0).rect` = `-2581,833,-1750,1859`・バルーンの高さ 675。y のずらし 0 のとき、`center` のバルーンは `-2343,195,-1689,870`＝下端 870 が窓の上端 833 から 37 下。`bottom` は `-2343,1120,-1689,1795`＝上端 1120 が窓の下端 1859 より 739 上。`windowposition.x` のキーワードの側は SSP で測っていない。
- `balloon-script-offset-align` は今の基準のまま着地し、SSP との違いを `doc/COMPAT_ARCHITECTURE.md` §8 に記録する。§8 の `windowposition.x` のキーワードの行（「バルーン下端がシェル画像の上端に接する」）は、窓の上下の端を「シェル画像」の端と読んでいる。

### Desired Outcome

- 要件の段で、SSP の頭上・足元の縦の基準を SSP の MCP で測って確かめる（毎回開発者の許しを取る）。候補は面の見える範囲（透明でない画素の外接矩形）・当たりの矩形（collision）・ほか。面を替えたとき、ずらしを入れたとき、`windowposition.x` のキーワードのときも同じかを見る。
- 測った結果で、areka をそれに合わせるかを決める。合わせるなら `keyword_balloon_pos` の基準をその矩形にし、P5 と出し直しの両方が同じ値を読む。合わせないなら、理由を §8 の行に書いて閉じる。
- どちらでも、§8 の `balloon-script-offset-align` の違いの行と `windowposition.x` のキーワードの行が、決めた中身にそろう。

### Approach

- 測りは `balloon-script-offset-align` の 8 節と同じ型（`sakurascript` strict・`get_property` の矩形）で、面の見える範囲が窓より小さい検体を選ぶ。
- 合わせるときは、基準の矩形を `keyword_balloon_pos` の引数として渡す（式は 1 か所のまま）。その矩形を placement へ渡す道は設計の段で決める（件 1 のサーフェスの値を渡す道と同じ道に乗せられるかを見る）。
- 式の入出力を表で固定する決定論のテストを置く。

### Constraints

- 件 1 と同じ（読めない値は `warn!`・失敗は `error!`＋`Err`・兄弟のテストのファイル・SSP を測るときは毎回開発者の許し・試験の根はワークツリーの `target\` の下だけ）。
- 優先度の区分は C（バグではない・ukadoc の書き方はどちらとも読める）・版は件 1 と同じいつか。

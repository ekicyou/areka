# Requirements Document

## Project Description (Input)
ゴーストの作者が台本に `\![set,balloonoffset,x,y]`（バルーンの取り付け位置のずらし）と `\![set,balloonalign,ID]`（バルーンをシェルのどちら側へ付けるか）を書いても、areka はどちらも扱わない。網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の 2 行は `status = "absent"`（優先度 A2）で、`check_script` は受け手の無い命令（`unknown_command`）と答え、利用者から見るとタグが消えたのと同じになる（バルーンは descript の既定の位置・既定の側に出たまま）。

そのため、本体とそのバルーンを DPI の違う画面に分けて置く形を作れない。本体を画面の境にまたがせても、バルーンは本体の居る画面の作業領域の中へ押し戻される（`[balloon-limit] Clamp runtime 関門 … route=Some(BalloonFollow) … how="pushed"`）。完了 spec `shell-scale-linkage` の実機の確かめの段 4（ぱすた開発との約束＝分かれたときに倍率のずれの台詞を出さないこと・`currentghost.balloon.scope(0).currentmonitor.dpi` がバルーンの居る画面の DPI を答えること）は、2026-11-01 の開発者の裁定（案 A）で「今の areka では起きない形」として閉じたままである。

2 つの命令を ukadoc の意味どおりに受け、そのスコープのバルーンの置き方（起動の配置・追従・押し戻しの関門）を変える。台帳の 2 行を実装済みにし、作者が台本でバルーンを本体から離して置けば本体と別の画面にバルーンを出せるようにし、その形で `shell-scale-linkage` の段 4 を実機で確かめ直して結果をぱすた開発のセッションへ知らせる。

## Introduction

この spec は、台本からバルーンの置き方を変える 2 つの命令を受け、descript の既定の上に台本の値を重ねる 1 層を足すものである。版は **v0.2.0**（ウェーブ C28・区分 C＝ukadoc の拾い残し）。

ukadoc の正典（逐語の抜き書き）:
- `\![set,balloonoffset,x,y]`（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonoffset-x-y>・対応 SSP / Ninix）: 「バルーンのオフセット値をXY座標に変更。この値にユーザーがバルーンドラッグ移動で決定した座標が加算され、バルーン表示座標が決まる。」「頭に@をつけるとdescript.txtやsurfaces.txt内のオフセット値からの相対座標指定となる。」寿命には沈黙する。
- `\![set,balloonalign,ID]`（<https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonalign-ID>・対応 SSP）: ID は `left`（シェルの左上にバルーンの右上が接する）・`center` と `top`（シェルの中央上にバルーンの中央下が接する）・`right`（シェルの右上にバルーンの左上が接する）・`bottom`（シェルの中央下にバルーンの中央上が接する）・`none`（座標によって left と right が入れ替わる）・「上記以外: noneと同じ。」。初期値 none。「ゴースト終了まで有効。」その位置を基準に descript の `balloon.offsetx`・`balloon.offsety` が反映される。

用語:
- **台本の基準点**: `\![set,balloonoffset]` が決めた、そのスコープのバルーンのずらし。descript の `balloon.offsetx`・`balloon.offsety`（以下「descript のずらし」）の代わりに使う値。
- **台本の向き**: `\![set,balloonalign]` が決めた、そのスコープのバルーンをシェルのどちら側へ付けるか。
- **台本の値**: 台本の基準点と台本の向きの総称。
- **利用者のずらし**: 利用者がバルーンをドラッグして決めた分。
- **押し戻しの関門**: バルーンが作業領域からはみ出すとき、表示の位置を作業領域の端へ押し込む今の決まり（`doc/COMPAT_ARCHITECTURE.md` §8 の「【上書き】バルーンだけが作業領域からはみ出すとき」の行）。起動の配置・本体への追従・ドラッグを離したときの 3 か所にある。
- **スコープ**: 命令を書いた所の話し手（`\0`・`\1`・`\p[n]`）。

> **前提の記号**: 【仮定】は ukadoc で決まらず、SSP の測りで裏付ける項目である。要件の討議で決めたことは末尾の「決まったこと」に並べた。

## Boundary Context

- **In scope**:
  - 2 つの命令の受け付け（受け手の表への登記・引数の読み・壊れた値の扱い）と、`check_script` が受け手の無い命令と報告しなくなること。
  - 台本の値をそのスコープのバルーンの置き方へ反映すること（起動の配置・本体への追従・押し戻しの関門・ドラッグ・シェルの倍率の変化のどれも同じ値を読む）。
  - 台本の値の寿命（ゴーストの終わり・切り替え・読み直し、シェルとバルーンの切り替え、`\![execute,resetballoonpos]` との関係）。
  - 押し戻しの関門がどの画面の作業領域を見るか。
  - 記録の欄（受けた・捨てた・押し戻した）。
  - 直す前に赤になる決定論のテスト、網羅台帳の 2 行、`doc/COMPAT_ARCHITECTURE.md` §8 の行。
  - SSP の測り（毎回開発者の許しを取る）と、`shell-scale-linkage` の段 4 の実機の確かめ直し・ぱすた開発のセッションへの知らせ。
- **Out of scope**:
  - descript の鍵の読み（`descript-window-keys` の持ち物）。方向つきの `balloon.offsetxl`・`offsetxr`・`offsetxc`・`offsetxb` と `surfaces.txt` の `balloon.offsetx`・`offsety` は今の areka が読まない鍵であり、この spec は読む鍵を増やさない。
  - 左右の並べ替えの判定（`none` のときにどちら側へ出すか）と、右に出すときのずらしの符号の決まり（`balloon-flip-side-rules`）。
  - キャラクター窓の作業領域への寄せ（`work_area_clamp.rs`）・3 体以上の窓（`extra-character-windows`）。
  - 倍率・DPI の値そのもの（`shell-scale-linkage` の表）。この spec は窓の置き場所だけを変える。
  - バルーンの大きさ・面の選び。
  - 台本の値の保存（記憶への書き込み）。同じウェーブ C28 の約束で保存しない形で進める（要件 5.6）。
  - 方向つきのずらし（`balloon.offsetxl`・`offsetxr`・`offsetxc`・`offsetxb`）と `surfaces.txt` のバルーンのずらしの読み。別の spec `balloon-directional-offset-keys` が持つ。この spec は接尾辞の無い descript のずらしだけを台本の向きと相対の指定の基準にする。
- **Adjacent expectations**:
  - descript のずらし・`balloon.alignment` の読み（完了 `descript-window-keys`）と、バルーンの windowposition の調整量の足し（完了 `shell-scale-linkage` ほか）は今のまま使う。
  - 押し戻しは 4 辺の押し込みだけで、反対の側へ並べ替えず、相対の位置（配置のずらし・追従のずらし・記憶の `BalloonOffset`）に焼き付けない（§8 の行）。この決まりは保ち、見る画面だけを要件 6 で決める。
  - `\![lock,balloonmove]` の帳簿と寿命の決まり（§8 の行）は今のまま使う。
  - 倍率の表と `currentghost.balloon.scope(ID).currentmonitor.dpi` の読み手（完了 `shell-scale-linkage`）は窓ごとに値を持つので、この spec は読み手を変えない。
  - 同じウェーブ C28 の約束（`.kiro/steering/roadmap.md` のウェーブ編成）: この spec は台本の値を保存せず、`crates/areka-placement/src/persist.rs` と、`sylphya-sqlite-kvs` が直す試験の組み立て（`follow_test_support.rs`・`persist_finalize_save_tests.rs`・`crates/areka-runtime/src/frame_persist_test_support.rs`・`move_cue_apply_move_tests.rs`・`placement_tests/reset_balloon_pos_tests.rs`）に触らない。台帳と §8 は別々の行だけを足す。

## Requirements

### Requirement 1: 2 つの命令の受け付け
**Objective:** As a ゴーストの作者, I want `\![set,balloonoffset,x,y]` と `\![set,balloonalign,ID]` を書けば areka が受けること, so that 台本のタグが黙って消えず、置き方を台本から変えられる

#### Acceptance Criteria
1. When 台本の再生が `\![set,balloonoffset,x,y]` に達したとき, the areka shall その命令を書いた所のスコープのバルーンの台本の基準点を、その命令の値に置き換える。
2. When 台本の再生が `\![set,balloonalign,ID]` に達したとき, the areka shall その命令を書いた所のスコープのバルーンの台本の向きを、その ID の向きに置き換える。
3. When `check_script` が 2 つの命令のどちらかを含む台本を調べたとき, the areka shall その命令を受け手の無い命令（`unknown_command`）と報告しない。
4. The areka shall `\![set,balloonoffset]` の x と y を、それぞれ独立に「符号つきの整数＝絶対の指定」か「頭に `@` のついた符号つきの整数＝descript のずらしからの相対の指定」として読む（軸ごとに絶対と相対を混ぜてよい。例 `@-10,200`）。
5. The areka shall `\![set,balloonalign]` の ID を大文字と小文字を区別せずに読む（descript の `balloon.alignment` の読みと同じ）。
6. When 2 つの命令に決まりより多い引数があるとき, the areka shall 余分な引数を読まずに命令を当てる（`\![lock,balloonmove]` の先例と同じ）。
7. If `\![set,balloonoffset]` の x か y が欠けている、数として読めない、または `@` だけで数が無いとき, then the areka shall その命令を当てず（片方の軸だけが壊れていても両方の軸の台本の基準点を変えず）、受け取った字をそのまま残した警告を記録する。
8. If `\![set,balloonalign]` に ID が無いとき, then the areka shall その命令を当てず、警告を記録する。
9. If 命令を書いた所のスコープが数として読めない、またはキャラクター窓の無いスコープであるとき, then the areka shall その命令を当てず、スコープと命令の名前を残した警告を記録する。

### Requirement 2: 台本の基準点の反映
**Objective:** As a ゴーストの作者, I want 台本で決めたずらしでバルーンがシェルから離れて置かれること, so that バルーンを本体から離して置ける

#### Acceptance Criteria
1. When 絶対の指定で台本の基準点が決まったとき, the areka shall そのスコープのバルーンを、descript のずらしの代わりに台本の基準点を使った位置に置く。
2. When 相対の指定で台本の基準点が決まったとき, the areka shall そのスコープのバルーンを、descript のずらしにその値を足した位置に置く（descript のずらしが無い軸は 0 から足す）。
3. While 台本の基準点が決まっている間, the areka shall バルーンの位置を、台本の基準点に利用者のずらしを足した所に決める（ukadoc の「この値にユーザーがバルーンドラッグ移動で決定した座標が加算され」。命令の前に利用者が動かしていた分も足し続ける）。
4. The areka shall 台本の基準点を、descript のずらしと同じ座標の決まりで扱う（シェルの作者の座標で書かれた値として、シェルの倍率に合わせて換算する・右側に出すときの符号は descript のずらしと同じ決まりに従う＝左右どちらでも正の x が本体の側を向き、`center`・`bottom` では正の x が右を向く。SSP 2.9 系の測りと同じ＝`research.md` 8 節）。
5. The areka shall 台本の基準点が置き換えるのを descript のずらしだけにし、バルーンの windowposition の調整量と、面の `move` によるずれは今のまま足す（ukadoc が「無視される」と書くのは descript の offset 値だけ）。
6. When そのスコープのバルーンが画面に出ている間に台本の基準点が変わったとき, the areka shall そのバルーンを、次に本体を動かすのを待たずに新しい位置へ動かす。
7. When そのスコープのバルーンがまだ出ていない間に台本の基準点が決まったとき, the areka shall その値を覚え、そのバルーンを最初に置くときに使う。

### Requirement 3: 台本の向きの反映
**Objective:** As a ゴーストの作者, I want 台本でバルーンをシェルのどちら側に付けるかを選べること, so that 置きたい側にバルーンを出せる

#### Acceptance Criteria
1. When 台本の向きが `left` のとき, the areka shall そのスコープのバルーンを、シェルの左上にバルーンの右上が接する位置を基準に、ずらしを足して置く。
2. When 台本の向きが `right` のとき, the areka shall そのスコープのバルーンを、シェルの右上にバルーンの左上が接する位置を基準に、ずらしを足して置く。
3. When 台本の向きが `center` か `top` のとき, the areka shall そのスコープのバルーンを、シェルの中央上にバルーンの中央下が接する位置を基準に、ずらしを足して置く。
4. When 台本の向きが `bottom` のとき, the areka shall そのスコープのバルーンを、シェルの中央下にバルーンの中央上が接する位置を基準に、ずらしを足して置く。
5. When 台本の向きが `none` のとき, the areka shall そのスコープのバルーンの側を、descript で `none` を書いたときと同じ今の左右の決め方で決める（並べ替えの判定は `balloon-flip-side-rules` の持ち物で、この spec は変えない）。
6. When 台本の向きの ID が上の 6 語のどれでもないとき, the areka shall ukadoc の「上記以外: noneと同じ」どおり `none` として当て、受け取った字をそのまま残した記録を残す。
7. While 台本の向きが `left`・`right`・`center`・`top`・`bottom` のどれかである間, the areka shall シェルを動かしてもそのスコープのバルーンの側を入れ替えない。
8. When そのスコープのバルーンが画面に出ている間に台本の向きが変わったとき, the areka shall そのバルーンを、次に本体を動かすのを待たずに新しい向きの位置へ動かす。
9. The areka shall 台本の向きを変えたとき、台本の基準点と利用者のずらしを新しい向きの基準に足す（ukadoc の「下記の位置を基準に…offsetの値が反映される」。向きを変えると利用者のずらしの基準も新しい向きへ移る）。

### Requirement 4: どの道でも同じ台本の値を読むこと
**Objective:** As a 利用者, I want バルーンがどの場面でも台本で決めた置き方を保つこと, so that 本体を動かしたり倍率が変わったりしても置き方が勝手に戻らない

#### Acceptance Criteria
1. The areka shall 起動の配置・本体への追従・押し戻しの関門・バルーンのドラッグ・シェルの倍率の変化による置き直しのどれでも、そのスコープの同じ台本の値を読む。
2. When 利用者が本体をドラッグしたとき, the areka shall そのスコープのバルーンを、台本の値で決まるシェルとの相対の位置を保ってついていかせる。
3. When シェルの倍率が変わってバルーンを置き直すとき, the areka shall 台本の基準点を descript のずらしと同じく新しい倍率に合わせて換算する。
4. While 台本の値が決まっていないスコープでは, the areka shall 今と同じく descript の既定だけで置く（この spec の前と置き場所が変わらない）。
5. The areka shall あるスコープの台本の値を、ほかのスコープのバルーンの置き方に使わない。
6. While そのスコープに `\![lock,balloonmove]` が効いている間, the areka shall 利用者のドラッグだけを止め、台本の値の受け付けと置き方への反映は止めない。

### Requirement 5: 台本の値の寿命
**Objective:** As a ゴーストの作者, I want 台本の値がいつまで効くかが決まっていること, so that 切り替えや再起動の後の置き方を見通せる

#### Acceptance Criteria
1. The areka shall 台本の値を、ゴーストが終わるまで保つ（ukadoc の `\![set,balloonalign]` の「ゴースト終了まで有効」。台本の基準点も同じ寿命とする）。
2. When ゴーストの切り替えか読み直しが起きたとき, the areka shall 台本の値を捨て、descript の既定へ戻す（§8 の「台本で変えた吸着の向き」と `\![lock,balloonmove]` の行の先例と同じ）。
3. When シェルの切り替え（読み直しを含む）が起きたとき, the areka shall 台本の値を保ち、新しいシェルの descript の既定の上に重ねる（相対の指定は新しいシェルの descript のずらしからの相対になる）。
4. When バルーンの切り替えが起きたとき, the areka shall 台本の値を保つ。
5. When `\![execute,resetballoonpos]` かメニューの「バルーン位置初期化」でバルーンの位置を初期の位置へ戻すとき, the areka shall 利用者のずらしだけを捨て、台本の値は保ったまま、台本の値で決まる位置へ戻す。
6. The areka shall 台本の値を記憶へ保存せず、次の起動では descript の既定から始める（同じウェーブ C28 の約束）。
8. When 台本の基準点か台本の向きが効いているスコープで利用者がバルーンのドラッグを離したとき, the areka shall 記憶の `BalloonOffset` に、今のシェルとの相対の位置から台本の値の分を除いた値（台本の値が無いときの標準の位置に利用者のずらしを足した値）を書く（次の起動で台本の値の分まで戻らない。記憶の書き手の形は変えない）。
7. When SHIORI だけの読み直しが起きたとき, the areka shall 台本の値を保つ（§8 の先例と同じ）。

### Requirement 6: 押し戻しの関門が見る画面
**Objective:** As a ゴーストの作者, I want 台本で本体から離して置いたバルーンが、本体の居る画面へ押し戻されないこと, so that 本体と別の画面にバルーンを出せる

#### Acceptance Criteria
1. When 押し戻しの関門が、台本の基準点か台本の向きが効いているスコープのバルーンのはみ出しを調べるとき, the areka shall そのバルーンを置こうとする矩形の中心が入る画面の作業領域を使う（どの画面にも入らないときは最も近い画面）。
2. While 台本の値が効いているスコープのバルーンを置こうとする矩形がどれか 1 つの画面の作業領域に収まっている間, the areka shall そのバルーンを押し戻さない（本体と別の画面でも押し戻さない）。
7. While スコープに台本の値が効いていない間, the areka shall 押し戻しの関門で今と同じく本体の画面の作業領域を使う（台本を使わないゴーストの置き場所は変えない）。
3. The areka shall 押し戻しを 4 辺の押し込みだけにし、反対の側へ並べ替えず、台本の値・利用者のずらし・記憶の `BalloonOffset` に焼き付けない（§8 の今の決まりを保つ）。
4. Where スコープの `windowposition.limit` が 0 のとき, the areka shall 今と同じくそのスコープのバルーンを押し戻さない。
5. The areka shall 起動の配置・本体への追従・ドラッグを離したときの 3 か所の押し戻しで、同じ画面の選び方を使う。
6. When 本体とバルーンが DPI の違う画面に分かれたとき, the areka shall `currentghost.balloon.scope(ID).currentmonitor.dpi` にバルーンの居る画面の DPI を、`currentghost.scope(ID).currentmonitor.dpi` に本体の居る画面の DPI を答える（読み手は今のまま・この spec はその形を作れるようにするだけ）。

### Requirement 7: 記録
**Objective:** As a 開発者, I want 台本の値を受けた・捨てた・押し戻したことがログで分かること, so that 実機の確かめと不具合の調べで置き方の理由を辿れる

#### Acceptance Criteria
1. When 台本の値を受けて当てたとき, the areka shall スコープ・命令の名前・受け取った値（相対か絶対かを含む）を 1 行の情報の記録に残す。
2. If 命令の値が壊れていて当てなかったとき, then the areka shall スコープ・命令の名前・受け取った字（検証していない字はそのまま）・当てなかったわけを 1 行の警告に残す。
3. When 押し戻しの関門がバルーンを押し戻したとき, the areka shall 今の `[balloon-limit] Clamp` の行に、見た作業領域がどの画面のものか（バルーン自身の画面か本体の画面か）と、台本の値がそのスコープに効いていたかを欄として足す。
4. If 台本の値を置き方に当てる途中で、必要な材料（キャラクター窓・既定の配置の材料など）が無く当てられなかったとき, then the areka shall わけを残したエラーか警告を記録し、黙って捨てない。
5. When ゴーストの切り替え・読み直しで台本の値を捨てたとき, the areka shall 捨てたスコープを記録に残す。

### Requirement 8: テストと台帳
**Objective:** As a 開発者, I want 振る舞いが決定論のテストと台帳で固定されていること, so that 後の変更で黙って壊れない

#### Acceptance Criteria
1. The areka shall 要件 1〜6 の判断の分かれ目（絶対と相対・壊れた値・6 語と上記以外の向き・寿命の各場面・押し戻しの画面）ごとに、直す前に赤になる決定論のテストを持つ。
2. The areka shall 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![set,balloonoffset,x,y]` と `\![set,balloonalign,ID]` の 2 行を実装済みにし、持ち主をこの spec に、根拠の場所を書く。
3. The areka shall `doc/COMPAT_ARCHITECTURE.md` §8 に、正典が沈黙する決め（台本の基準点の寿命・シェルとバルーンの切り替え・`\![execute,resetballoonpos]` との関係・押し戻しの画面・軸ごとの相対と絶対）を別々の行で足し、今の「バルーンだけが作業領域からはみ出すとき」の行を画面の選び方に合わせて改める。
4. The areka shall 同じウェーブ C28 の約束（要件 5.6・Boundary Context の Adjacent expectations）で挙げたファイルを変えずに、テストを新しい兄弟のファイルに置く。

### Requirement 9: SSP の測りと段 4 の実機の確かめ直し
**Objective:** As a 開発者, I want SSP の振る舞いで未決の決めを裏付け、離して置いた形で `shell-scale-linkage` の段 4 を確かめ直すこと, so that ぱすた開発との約束を閉じられる

#### Acceptance Criteria
1. When 未決の議題（押し戻しの画面・寿命・`\![execute,resetballoonpos]` との関係・利用者のずらしの扱い・右側と `center`・`bottom` のときの台本の基準点の x の符号）を SSP で測るとき, the 開発の作業 shall 測るたびに開発者の許しを取り、試験の根と検体をワークツリーの `target\` の下に置く。
2. When 実装が終わったとき, the 開発の作業 shall 排他テストの机を持って、本体とそのバルーンを DPI の違う画面に台本で分けて置き、`currentghost.balloon.scope(0).currentmonitor.dpi` がバルーンの居る画面の DPI を答えることと、ぱすたの倍率のずれの台詞が出ないことを実機のログで確かめる。
3. When 段 4 の確かめが終わったとき, the 開発の作業 shall 結果（合否と根拠のログの行）をぱすた開発のセッションへ知らせる。
4. When SSP の測りの結果が要件 2〜5 の決めと食い違ったとき, the 開発の作業 shall その結果を `research.md` に記録し、食い違った決めを開発者と決め直す（SSP の実装の癖をそのまま写すかは決めごとに判断する・正典は ukadoc）。

## 決まったこと（要件の討議・2026-11-27）

1. **押し戻しの関門の画面**（要件 6.1・6.2・6.7）: 台本の値が効いているスコープだけ、バルーン自身の画面の作業領域で見る。台本の値の無いスコープは今のまま本体の画面で見る（台本を使わないゴーストの既定の置き場所を変えない）。
2. **台本の値の寿命**（要件 5.1〜5.4・5.7）: どちらの命令もゴーストが終わるまで。シェル・バルーンの切り替えと SHIORI だけの読み直しで保ち、ゴーストの切り替えと読み直しで捨てる。保存しない（§8 の台本の吸着の向き・`\![lock,balloonmove]` の先例）。
3. **利用者のずらしと記憶**（要件 2.3・3.9・5.8）: 命令の前のドラッグも足し続け、向きを変えたらドラッグの分も新しい向きの基準に乗せる。記憶へは台本の値の分を除いて書く（記憶の書き手の形は変えない）。
4. **引数の読み**（要件 1.4・1.5・1.7）: 軸ごとに `@` の有無を混ぜてよい。片方の軸でも壊れていれば命令ごと当てず警告。`@` だけで数が無い字も壊れた値。ID は大文字と小文字を区別しない。
5. **小さな確かめ**: `\![execute,resetballoonpos]` とメニューの初期化は利用者のずらしだけを捨て、台本の値は保つ（要件 5.5）。台本の基準点が置き換えるのは descript の `balloon.offsetx/y` だけ（要件 2.5）。`\![lock,balloonmove]` は利用者のドラッグだけを止める（要件 4.6）。
6. **方向つきのずらしと `surfaces.txt` のずらし**（Boundary Context）: この spec では読まない。別の spec `balloon-directional-offset-keys` として起票した。
7. **SSP の測り**（要件 9.1）: ukadoc が黙っている点（右側と `center`・`bottom` のときの x の符号・ドラッグとの重ね方・切り替えと初期化の後の寿命・`\![set,balloonoffset]` の寿命）を要件の段で SSP の MCP で測り、`research.md` に記録する。食い違ったら要件 9.4 で決め直す。

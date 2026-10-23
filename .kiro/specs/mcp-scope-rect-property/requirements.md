# Requirements Document

> 本文の実物の確認は **2026-10-23・本ブランチ**（main `15d8b0f3`）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> SSP の基準は、完了 spec `areka-P0-pasta-in-windows-verification` の `ssp-measurement.md` 2 節（SSP 2.9.23・2026-10-23 07:58〜08:13 の測り）。正典はライブの ukadoc（`https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html`・2026-10-23 に引き直した）。
> 「要件の段で残した議題」の節は、要件ディスカッションで開発者に諮る。答えしだいで要件 3 と要件 5 の文が変わりうる。

## Project Description (Input)
- **誰が困っているか**: 開発者と自動試験の道具。外から窓の位置を読む MCP の `get_property` に `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.balloon.scope(0).rect` を聞くと、areka は `NG:Cannot find such property name.` と答える（SSP は「左,上,右,下」の画面の座標を答える）。
- **今どうなっているか**: ぱすたの検証では窓とバルーンの位置を撮影と記録の `bounds=` の行から読み替えるしかなく、重なり・ドラッグ・画面をまたぐ移動の確かめを自動の試験に載せられない（v0.1.0 のゴール「SSP 並みの自動試験」の穴）。プロパティの値の置き場には、窓の位置のように刻々と変わる値を出す口が無い。
- **何を変えるか**: 刻々と変わる値を出す口を 1 つ足し、`currentghost.scope(ID).rect`（本体・相方・3 人目以降の窓）と `currentghost.balloon.scope(ID).rect` を、MCP の `get_property` とゴーストが読むプロパティの道で、今の窓の画面の座標「左,上,右,下」で読めるようにする。窓の無いスコープ・出ていないバルーンは値なし（捏造しない）。ぱすたの検証の手順書の「撮影と記録で読み替える」所を `get_property` で読む形へ戻す。詳細は `brief.md`。

## Introduction

### 誰が困っているか

- **AI エージェントで MCP の道具を使う人と、自動試験の道具**: 窓とバルーンが画面のどこに在るかを `get_property` で読めないので、重なり・ドラッグ・画面をまたぐ移動の結果を数で確かめられない。今は撮影と記録の行を人が読み替えている。
- **ゴースト**: ゴーストがプロパティの道で自分の窓の位置を聞いても「無い名前」になる（SSP では値が返る）。

### 正典（ukadoc・2026-10-23 に引き直した）

| 名前 | ukadoc の説明 | 入った版 |
|---|---|---|
| `currentghost.scope(ID).rect` | 「現在のゴーストのスコープIDのウインドウ矩形座標(左,上,右,下)。」 | （記載なし） |
| `currentghost.balloon.scope(ID).rect` | 「現在のバルーンの矩形領域。」 | 2.8.39 |

ukadoc は、窓やバルーンが出ていないとき・スコープが無いときに何を返すかを書いていない。座標の原点は、同じ表の `currentghost.scope(ID).x` の説明が「プライマリモニタ左上を原点とする仮想デスクトップ全体での位置」と書いている。物理の画素か論理の画素かは書いていない。

### SSP の答え（`ssp-measurement.md` 2 節の要約）

| 段 | 場面 | `scope(0)`・`scope(1)` の rect | `balloon.scope(0)` の rect |
|---|---|---|---|
| 1 | 起動の直後（150% の画面。画面の左上は `-2560,195`・大きさ `2560x1600`） | どちらも `-782,201,49,1227` | `-722,288,-68,963` |
| 4 | 同じ画面の中でドラッグした後 | どちらも `-1482,201,-651,1227`（2 つが揃って動く） | （測っていない） |
| 5 | 200% の主の画面へドラッグした後 | どちらも `818,261,1649,1287` | `878,348,1532,1023` |

ここから読めること:
- 4 つの数は「左,上,右,下」を `,` でつなぎ、空白を入れない。負の数がそのまま出る。
- 右 − 左＝幅（`49 − (−782) = 831`）、下 − 上＝高さ（`1227 − 201 = 1026`）＝右と下は窓の外の最初の画素（右端・下端を含まない）。
- 画面の左上 `-2560,195`・大きさ `2560x1600` は 150% の画面の物理の画素の値＝座標は**主の画面の左上を原点とする、物理の画素の仮想画面の座標**。
- SSP は拡大率の違う画面へ移っても窓の大きさを変えない（831x1026 のまま）。areka は画面の拡大率に合わせて大きさを変える方針（2026-07-18 開発者）なので、段 5 の後の大きさは SSP と違ってよい。合わせるのは**数の形と座標の系**で、値そのものは areka の窓の実際の矩形。

### 実物で引き直して確かめた点（2026-10-23）

1. **読む口は 2 つで、同じ値の置き場を読む。** MCP の `get_property`（`crates/areka/src/mcp/get_property.rs` の `handle`）と SHIORI の `GetProperty`（`crates/areka/src/shiori_host.rs` の `ShioriHostSink`）は、どちらも名前を手直しせず `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs`）へ渡す。値が無ければ前者は `NG:Cannot find such property name.`、後者は「プロパティが無い」の失敗で答える。読みは値の置き場の写しを待たずに取るだけで、別の手番へ問い合わせない。
2. **値を出す口は、変わらない値と SHIORI の答えの 2 つだけ。** 出す口 `SylphyaPublisher` の `publish_static`・`publish_shiori`（`crates/areka-sylphya/src/actor.rs`）と、保存へ書く `persist_put`。窓の位置のように刻々と変わる値を出す口は無い。出した値は値の置き場の持ち主のスレッドが受け取り、新しい写しに差し替える（`crates/areka-sylphya/src/mirror.rs` の冒頭の説明）＝出してから読めるまでに間がある作り。
3. **ゴーストの道の実機。** 本番のゴーストは SHIORI を 32bit の中継越しに呼び、その道には値の置き場へ届く `GetProperty` が無い。`ShioriHostSink` を使うのはデモとテストの入口だけ（`crates/areka/src/shiori_demo.rs`・`shiori_*_e2e_tests.rs`）＝ゴーストの道は決定論のテスト（要件 8）で確かめ、実機の確かめ（要件 9）は MCP だけで行う（前例: 完了 `areka-P0-property-name-case-fold` の要件 2.2）。
4. **areka の窓の位置は物理の画素で持っている。** 窓の移動の記録 `WindowMoveRecord`（`crates/areka/src/placement/diag.rs`）は窓ごとに物理の X・Y・幅・高さとスコープの番号を残す。
5. **網羅台帳の 2 行の持ち主は、もう本 spec。** `doc/ukadoc-coverage/ledger/property.toml` の `currentghost.scope_28ID_29.rect`・`currentghost.balloon.scope_28ID_29.rect` は `status = "vocabulary-only"`・`owner = "mcp-scope-rect-property"`。
6. **手順書の読み替えの所。** `doc/pasta-real-run.md` の「位置の読み方」の段落（本 spec の着地まで撮影と記録の `bounds=` の行で読む、と書いた所）と、表の段 1・4・5 の「`mcp-scope-rect-property`（位置を `get_property` で読む）」の欄。

## Boundary Context

- **In scope**:
  - `currentghost.scope(ID).rect`（本体・相方・3 人目以降の窓）と `currentghost.balloon.scope(ID).rect` の 2 つの名前を、MCP の `get_property` とゴーストが読むプロパティの道の両方で読めること。
  - 刻々と変わる値を出す口を 1 つ作ること（後の spec が同じ口に別の値を足せる形）。
  - 窓やバルーンが動いた・大きさが変わった・出た・消えた・ゴーストが替わった後に、読んだ値がついてくること。
  - 決定論のテスト・MCP の実機の確かめ・網羅台帳の 2 行・手順書 `doc/pasta-real-run.md` の読み替えの所の戻し。
- **Out of scope**:
  - `currentghost.*` の残りの名前（`currentghost-property-tree`・`currentghost-property-others` のまま）。
  - 倍率と DPI の名前（`currentghost.scope(ID).scaling`・`currentmonitor.dpi` など＝`shell-scale-linkage`）。
  - ゴーストが台本から読む道（`\![get,property]`＝`shell-scale-linkage`、`%property[]` などの残り＝`property-query-channels`）。
  - `currentghost.scope(ID).surface(ID).rect`（面ごとの、表示する前に見積もる矩形）。
  - 窓の置き方そのもの（`placement` 系の spec）。本 spec は窓の位置を読むだけで、位置を変えない。
  - 名前の英字の大小（✅ `property-name-case-fold` が済ませた。本 spec の 2 つの名前もその規則に乗る）。
- **Adjacent expectations**:
  - 刻々と変わる値は保存しない（保存の作り直し `sylphya-sqlite-kvs`〔v0.2.0〕に影響しない形にする）。
  - 後の spec（`shell-scale-linkage`・`currentghost-property-tree`・`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`）は、本 spec の口に値を足す。本 spec はその値を足さない。
  - 窓の位置を変える側（`window-link-directives`・`dpi-realign-remembered-chain`・ドラッグ・`\![move]`）が窓を動かした結果を、本 spec は読むだけ。

## Requirements

### Requirement 1: キャラクターの窓の矩形を読める
**Objective:** As a 自動試験の道具を使う開発者, I want `get_property` で各スコープの窓の画面の位置と大きさを読めること, so that 撮影や記録の行を読み替えずに、窓の重なりや動きを数で確かめられる

#### Acceptance Criteria
1. While ゴーストのスコープ ID の窓が画面に出ている, when MCP の `get_property` が `currentghost.scope(ID).rect` を聞いた, the areka shall その窓の今の画面の矩形を「左,上,右,下」の 4 つの整数で答える。
2. The areka shall スコープ 0（本体）・スコープ 1（相方）・スコープ 2 以降（3 人目以降）のどの窓にも、同じ名前の形 `currentghost.scope(ID).rect` で答える。
3. The areka shall 4 つの数を、主の画面の左上を原点とする物理の画素の仮想画面の座標で表す（主の画面より左や上に在る画面では負の数になる）。
4. The areka shall 4 つの数を `,` でつなぎ、空白・符号の `+`・前ゼロを入れない（SSP の `-782,201,49,1227` と同じ形）。
5. The areka shall 右の数を「左＋幅」、下の数を「上＋高さ」とする（右端・下端の画素を含まない数）。
6. The areka shall 答える矩形を、その窓が画面に占めている矩形（その窓の外枠の位置と大きさ）とし、面の画像の大きさから見積もった値や、置き方の予定の値で答えない。
7. While 2 つのスコープの窓が同じ位置と大きさで重なっている, the areka shall 両方の名前に同じ 4 つの数を答える。

### Requirement 2: バルーンの矩形を読める
**Objective:** As a 自動試験の道具を使う開発者, I want `get_property` で各スコープのバルーンの画面の位置と大きさを読めること, so that バルーンがキャラクターのどこに付いたかを数で確かめられる

#### Acceptance Criteria
1. While スコープ ID のバルーンが画面に出ている, when MCP の `get_property` が `currentghost.balloon.scope(ID).rect` を聞いた, the areka shall そのバルーンの今の画面の矩形を、要件 1 の 3〜6 と同じ座標の系・数の形で答える。
2. The areka shall `currentghost.balloon.scope(ID).rect` の ID を、そのバルーンが付いているスコープの番号として扱う（スコープ 0 の台詞のバルーンは `scope(0)`、スコープ 1 の台詞のバルーンは `scope(1)`）。
3. When バルーンの大きさが中身に合わせて変わった, the areka shall 変わった後の大きさを含む矩形を答える。

### Requirement 3: 値が無いときは値なしで答える
**Objective:** As a 自動試験の道具を使う開発者, I want 窓やバルーンが無いときは「無い」と答えること, so that 前の値や作った値を本当の位置と取り違えない

#### Acceptance Criteria
1. If 聞かれたスコープ ID の窓がゴーストに無い（そのゴーストが持たない番号）, the areka shall `currentghost.scope(ID).rect` に値なしで答える。
2. If 聞かれたスコープ ID の窓が画面に出ていない, the areka shall `currentghost.scope(ID).rect` に値なしで答える。
3. If 聞かれたスコープ ID のバルーンが画面に出ていない, the areka shall `currentghost.balloon.scope(ID).rect` に値なしで答える。
4. While ゴーストの起動の途中で窓がまだ画面に置かれていない, the areka shall 2 つの名前のどちらにも値なしで答える。
5. The areka shall 値なしを、MCP では今の「無い名前」と同じ `NG:Cannot find such property name.`、ゴーストの道では今の「プロパティが無い」の失敗で答える（新しい答えの文を作らない）。
6. The areka shall 値なしの場面で、最後に出ていたときの矩形・0 を並べた矩形・空の文字列を答えない。
7. If 名前の書式が読めない（例: `currentghost.scope().rect`・`currentghost.scope(x).rect`）, the areka shall 今の読み手と同じく値なしで答える。

### Requirement 4: 動いた後の値がついてくる
**Objective:** As a 自動試験の道具を使う開発者, I want 窓やバルーンが動いた・大きさが変わった後に読んだ値が新しい位置であること, so that 操作の直後に読んでも古い位置と取り違えない

#### Acceptance Criteria
1. When 利用者が窓のドラッグを終えた, the areka shall その後に受けた読み取りに、落とした後の矩形を答える。
2. When 台本の `\![move]` などで窓が動き終えた, the areka shall その後に受けた読み取りに、動いた後の矩形を答える。
3. When 窓が拡大率の違う画面へ移り、大きさが変わった, the areka shall その後に受けた読み取りに、移った先の画面での位置と大きさを答える。
4. When 面が替わって窓の大きさや位置が変わった, the areka shall その後に受けた読み取りに、替わった後の矩形を答える。
5. When 窓に付いて動くもう 1 つの窓やバルーンが一緒に動いた, the areka shall 一緒に動いたどの窓とバルーンの名前にも、動いた後の矩形を答える。
6. When 窓やバルーンの位置・大きさの変わりが画面に映った, the areka shall その画面の更新が終わった後に受けた読み取りで、変わる前の矩形を答えない。
7. While 窓が動いている途中（ドラッグの最中・時間をかけて動く `\![move]` の途中）, the areka shall その窓が実際に居たことのある矩形だけを答え、ある時点の左上と別の時点の右下を混ぜた矩形を答えない。

### Requirement 5: 読み取りは待たない
**Objective:** As a ゴーストと自動試験の道具, I want 位置の読み取りが窓を動かす側の都合を待たずに答えること, so that 読み取りが画面の更新を止めたり、ゴーストの処理が止まったりしない

#### Acceptance Criteria
1. The areka shall 2 つの名前の読み取りに、窓を持つ側の処理の終わりを待たずに答える（今のほかのプロパティの読み取りと同じく待たない）。
2. When ゴーストが自分の処理の途中でプロパティの道から 2 つの名前を読んだ, the areka shall 止まらずに答える。
3. The areka shall 読み取りのために、窓の移動・描画・画面の更新を遅らせない。
4. The areka shall 位置の値を出すたびに、通常の動作の記録を警告以上の段で残さない（ドラッグの最中に記録が流れない）。

### Requirement 6: ゴーストごとに分け、保存しない
**Objective:** As a 開発者, I want 位置の値がそのときのゴーストの物だけで、保存に残らないこと, so that 別のゴーストや前の起動の位置を今の位置と取り違えない

#### Acceptance Criteria
1. The areka shall 2 つの名前の値を、その窓とバルーンを持つゴーストの値として答え、MCP の `ghost_name` で選んだゴースト・プロパティの道で聞いたゴーストの窓の矩形を返す。
2. When ゴーストを替えた, the areka shall 前のゴーストの窓とバルーンの矩形を答えず、新しいゴーストの窓が画面に出てからその矩形を答える。
3. When ゴーストを読み直した（`reload`）, the areka shall 読み直しの後の窓とバルーンの矩形を答え、読み直しの前の値を残さない。
4. The areka shall 2 つの名前の値を保存のファイルに書かない。
5. When areka を起動し直した, the areka shall 前の起動の矩形を答えず、新しく置いた窓の矩形を答える。
6. The areka shall MCP の `get_property` とゴーストのプロパティの道で、同じ時点に同じ名前を読んだら同じ値を答える（どちらか一方だけが読める・違う値を返す、が無い）。

### Requirement 7: 刻々と変わる値を後から足せる口
**Objective:** As a 後の spec を作る開発者, I want 刻々と変わる値を足す口が 1 つ在ること, so that 倍率や DPI などの値を、読む道に手を入れずに足せる

#### Acceptance Criteria
1. The areka shall 刻々と変わる値を出す口を 1 つ持ち、本 spec の 2 つの名前をその口から出す。
2. Where 後の spec がこの口に別の名前の値を足した, the areka shall その名前を MCP の `get_property` とゴーストのプロパティの道の両方で、読む道を変えずに読めるようにする。
3. The areka shall この口から出した値を、変わらない値（`baseware.version` など）・SHIORI の答え・保存した値の読み取りと混ぜない（この口の値を出しても、ほかの値の答えが変わらない）。
4. The areka shall この口から出した値が消えた（窓が消えた・ゴーストが替わった）とき、その名前を値なしに戻す。

### Requirement 8: 決定論のテスト
**Objective:** As a 開発者, I want 上の約束を実の画面・実の時計なしで毎回同じ結果で確かめられること, so that 回帰をいつでも見つけられる

#### Acceptance Criteria
1. The areka shall 要件 1〜7 の各場面（窓が出た・ドラッグを終えた・`\![move]` で動いた・拡大率の違う画面へ移った・面が替わった・一緒に動いた・バルーンが出た・消えた・大きさが変わった・窓の無いスコープ・ゴーストを替えた・読み直した・起動し直した）を、実の窓・実の画面・実の時計を使わない決定論のテストで確かめる。
2. The areka shall 数の形（負の数・右と下の数・空白なし）を、主の画面より左や上に在る画面の座標を含むテストで確かめる。
3. The areka shall MCP の `get_property` とゴーストのプロパティの道の両方で同じ値・同じ値なしが返ることを、決定論のテストで確かめる。
4. The areka shall 動いている途中の読み取りが左上と右下を混ぜないことを、出す側と読む側を並べて動かす決定論のテストで確かめる。

### Requirement 9: 実機の確かめと文書
**Objective:** As a 開発者, I want 実機の MCP で位置が読めることを確かめ、台帳と手順書を直すこと, so that ぱすたの検証と自動試験を `get_property` で回せる

#### Acceptance Criteria
1. When 実機の確かめを行う, the 開発の手順 shall 排他テストの机を持ってから areka を起こし、終わったらすぐ机を返す。
2. When 実機でぱすたを起こした（手順書の段 1）・同じ画面でドラッグした（段 4）・拡大率の違う画面へ移した（段 5）, the 開発の手順 shall 各段で MCP の `get_property` が `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.balloon.scope(0).rect` に値を答え、その値が areka の記録に残る窓の位置と大きさと一致することを確かめる。
3. The 開発の手順 shall 実機の値を SSP の基準（Introduction の表）と並べ、数の形と座標の系が同じであること、大きさの違いが拡大率への追従の方針によるものであることを書き分けて残す。
4. When 本 spec の実装が終わった, the 開発の手順 shall 網羅台帳 `doc/ukadoc-coverage/ledger/property.toml` の 2 行を、実装済みを表す状態へ改め、根拠への道を添える。
5. When 本 spec の実装が終わった, the 開発の手順 shall 手順書 `doc/pasta-real-run.md` の「撮影と記録の `bounds=` の行で読む」所を `get_property` で読む形へ戻し、表の段 1・4・5 の本 spec の欄を「着地済み・確かめる」の形に改める。

## 要件の段で残した議題（要件ディスカッションで諮る）

| # | 議題 | 推奨 | 答えで変わる所 |
|---|---|---|---|
| 1 | 刻々と変わる値を「変わるたびに出す」か「読む時に窓を持つ側へ問い合わせる」か（brief の議題 1） | **変わるたびに出す**。今の値の置き場の作り（出す側が新しい写しに差し替え、読む側は待たずに取る）に口を 1 つ足すだけで、ゴーストの道の「待たずに即答する」約束（`shiori_host.rs` の再入の約束）をそのまま守れる。問い合わせる形は新しい待ち合わせの約束になり、要件 5 の 1・2 と食い違う | 要件 5（問い合わせる形を採るなら 5.1・5.2 を書き直し、要件定義のモデルを Fable へ戻す） |
| 2 | 出ていないバルーン・面を出していない（`\s[-1]`）スコープ・ゴーストに無いスコープに、SSP が何を答えるか（brief の議題 2） | **SSP の MCP で測ってから決める**。2026-10-23 のこの段では SSP が起きておらず測れなかった（ssp MCP が `Server not available`）。測るまでの既定は brief の決め（値なし・捏造しない）。SSP が最後の位置を答えると分かっても、ukadoc が黙っている所で前の位置を答えるのは「捏造しない」の決めと食い違うので、推奨は値なしのまま | 要件 3 の 2・3・6 |
| 3 | 答える矩形を「窓の外枠」とするか「面の画像の見える範囲」とするか | **窓の外枠**（ukadoc の「ウインドウ矩形座標」）。areka の窓が面の画像より広く取られている場面がギャップ分析で見つかったときだけ、どちらを答えるかを諮る | 要件 1.6 |
| 4 | 読み取りが新しい値になる時点の約束（brief の「見つけた穴」） | **窓の動きが画面に映った更新が終わった後に受けた読み取りは、新しい値を返す**（要件 4.6）。同じ更新の中で受けた読み取りまでは約束しない | 要件 4.6 |

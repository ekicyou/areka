# Requirements Document

> 本文の実物の確認は **2026-10-23・本ブランチ**（main `15d8b0f3`）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> SSP の基準は、完了 spec `areka-P0-pasta-in-windows-verification` の `ssp-measurement.md` 2 節（SSP 2.9.23・2026-10-23 07:58〜08:13 の測り）と、本 spec の `research.md` 7.1 節（同日 19:50 頃の測り）。正典はライブの ukadoc（`https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html`・2026-10-23 に引き直した）。
> 要件ディスカッション（2026-10-23）の決めは末尾の「要件ディスカッションで決めたこと」の節。

## Project Description (Input)
- **誰が困っているか**: 開発者と自動試験の道具。外から窓の位置を読む MCP の `get_property` に `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.balloon.scope(0).rect` を聞くと、areka は `NG:Cannot find such property name.` と答える（SSP は「左,上,右,下」の画面の座標を答える）。
- **今どうなっているか**: ぱすたの検証では窓とバルーンの位置を撮影と記録の `bounds=` の行から読み替えるしかなく、重なり・ドラッグ・画面をまたぐ移動の確かめを自動の試験に載せられない（v0.1.0 のゴール「SSP 並みの自動試験」の穴）。
- **何を変えるか**: プロパティの値の置き場（sylphya）に「読まれた時に、外から登録された読み手で一次情報源を見に行く」口を足し、`currentghost.scope(ID).rect`（本体・相方・3 人目以降の窓）と `currentghost.balloon.scope(ID).rect` を、MCP の `get_property` とゴーストが読むプロパティの道で、OS が持っている今の窓の矩形「左,上,右,下」で読めるようにする。ぱすたの検証の手順書の「撮影と記録で読み替える」所を `get_property` で読む形へ戻す。詳細は `brief.md`。

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

### SSP の答え

`ssp-measurement.md` 2 節の要約:

| 段 | 場面 | `scope(0)`・`scope(1)` の rect | `balloon.scope(0)` の rect |
|---|---|---|---|
| 1 | 起動の直後（150% の画面。画面の左上は `-2560,195`・大きさ `2560x1600`） | どちらも `-782,201,49,1227` | `-722,288,-68,963` |
| 4 | 同じ画面の中でドラッグした後 | どちらも `-1482,201,-651,1227`（2 つが揃って動く） | （測っていない） |
| 5 | 200% の主の画面へドラッグした後 | どちらも `818,261,1649,1287` | `878,348,1532,1023` |

`research.md` 7.1 節（ukadoc が黙っている所）の要約:

| 場面 | SSP の答え |
|---|---|
| 面を隠したスコープ（`\0\s[-1]` の後） | 隠す前と同じ矩形 |
| 出ていないバルーン | そのバルーンの窓の矩形 |
| ゴーストに無いキャラクターのスコープ（`scope(2)`・`scope(-1)`） | `0,0,0,0` |
| ゴーストに無いバルーンのスコープ（`balloon.scope(2)`） | `NG:Cannot find such property name.` |
| 番号でない（`scope(x)`） | `NG:Cannot find such property name.` |
| 前ゼロ（`scope(00)`） | `scope(0)` と同じ |

ここから読めること:
- 4 つの数は「左,上,右,下」を `,` でつなぎ、空白を入れない。負の数がそのまま出る。
- 右 − 左＝幅（`49 − (−782) = 831`）、下 − 上＝高さ（`1227 − 201 = 1026`）＝右と下は窓の外の最初の画素（右端・下端を含まない）。
- 画面の左上 `-2560,195`・大きさ `2560x1600` は 150% の画面の物理の画素の値＝座標は**主の画面の左上を原点とする、物理の画素の仮想画面の座標**。
- SSP は「画面に出ているか」で答えを変えない。OS の窓が在る限り、その窓の矩形を答える。
- SSP は拡大率の違う画面へ移っても窓の大きさを変えない（831x1026 のまま）。areka は画面の拡大率に合わせて大きさを変える方針（2026-07-18 開発者）なので、段 5 の後の大きさは SSP と違ってよい。合わせるのは**数の形・座標の系・無いときの答え**で、値そのものは areka の窓の実際の矩形。

### 実物で引き直して確かめた点（2026-10-23）

1. **読む口は 2 つで、同じ値の置き場を読む。** MCP の `get_property`（`crates/areka/src/mcp/get_property.rs` の `handle`）と SHIORI の `GetProperty`（`crates/areka/src/shiori_host.rs` の `ShioriHostSink`）は、どちらも名前を手直しせず `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs`）へ渡す。値が無ければ前者は `NG:Cannot find such property name.`、後者は「プロパティが無い」の失敗で答える。
2. **値の置き場には、読まれた時に値を取りに行く口が無い。** 値は出す口（`SylphyaPublisher` の `publish_static`・`publish_shiori`・`persist_put`・`crates/areka-sylphya/src/actor.rs`）から写しへ載ったものだけを読む。写しは値を 1 つ変えるたびに全区画を写し直す作り（`crates/areka-sylphya/src/mirror.rs` の `MirrorImage::successor`）。
3. **完了 spec `areka-P0-sylphya` の約束。** 要件 2.7 は「同期読み経路で他エンジン・他スレッドへのブロッキング照会を行わない・大域ロックにならない・遅延しうる値源を載せない」、要件 8.3 は「外部環境へのアクセスは注入可能な値源シームを経由する」。「OS を呼ばない」は要件に無く、設計（`design.md` の Invariants）とコードのコメント（`reader.rs` の冒頭・`mirror.rs` の `SharedMirror::load`）が言い過ぎている（2026-10-23 開発者「見に行ってはいけない、と契約したわけでは無い」）。
4. **ゴーストの道の実機。** 本番のゴーストは SHIORI を 32bit の中継越しに呼び、その道には値の置き場へ届く `GetProperty` が無い。`ShioriHostSink` を使うのはデモとテストの入口だけ（`crates/areka/src/shiori_demo.rs`・`shiori_*_e2e_tests.rs`）＝ゴーストの道は決定論のテスト（要件 8）で確かめ、実機の確かめ（要件 9）は MCP だけで行う（前例: 完了 `areka-P0-property-name-case-fold` の要件 2.2）。
5. **areka の窓は、OS の窓として起動時に全部作られる。** スコープから窓を引く表 `GhostWindows`（`crates/areka/src/placement/spawn.rs`）。`\s[-1]` は絵を隠すだけで OS の窓は同じ位置・同じ大きさで残る。プロセスはモニタごとの DPI 対応 v2（`crates/wintf/src/runtime/mod.rs`）なので、OS の窓の矩形は物理の画素・仮想画面の座標で読める。areka の `WindowPos` は OS の値の写しで、拡大率の違う画面へ移るときに補正した値を先に書くことがあり、一時的に OS の値とずれる（`crates/wintf/src/ecs/window_proc/window_pos.rs`）。
6. **網羅台帳の 2 行の持ち主は、もう本 spec。** `doc/ukadoc-coverage/ledger/property.toml` の `currentghost.scope_28ID_29.rect`・`currentghost.balloon.scope_28ID_29.rect` は `status = "vocabulary-only"`・`owner = "mcp-scope-rect-property"`。
7. **手順書の読み替えの所。** `doc/pasta-real-run.md` の「位置の読み方」の段落（本 spec の着地まで撮影と記録の `bounds=` の行で読む、と書いた所）と、表の段 1・4・5 の「`mcp-scope-rect-property`（位置を `get_property` で読む）」の欄。

## Boundary Context

- **In scope**:
  - `currentghost.scope(ID).rect`（本体・相方・3 人目以降の窓）と `currentghost.balloon.scope(ID).rect` の 2 つの名前を、MCP の `get_property` とゴーストが読むプロパティの道の両方で読めること。
  - sylphya に「外から登録された読み手で、読まれた時に値を取りに行く口」（プル）を 1 つ作ること（後の spec が同じ口に読み手を足せる形）。
  - 「スコープ → 窓」の表のように、まれにしか変わらない材料を変わった時点で sylphya へ出すこと（プッシュ）。
  - sylphya の「読みで OS を呼ばない」と言い過ぎているコメント（`reader.rs` の冒頭・`mirror.rs` の `SharedMirror::load`）を、完了 spec の要件 2.7・8.3 の言い方へ直すこと。
  - 決定論のテスト・MCP の実機の確かめ・網羅台帳の 2 行・手順書 `doc/pasta-real-run.md` の読み替えの所の戻し。
- **Out of scope**:
  - `currentghost.*` の残りの名前（`currentghost-property-tree`・`currentghost-property-others` のまま）。
  - 倍率と DPI の名前（`currentghost.scope(ID).scaling`・`currentmonitor.dpi` など＝`shell-scale-linkage`）。
  - ゴーストが台本から読む道（`\![get,property]`＝`shell-scale-linkage`、`%property[]` などの残り＝`property-query-channels`）。
  - `currentghost.scope(ID).surface(ID).rect`（面ごとの、表示する前に見積もる矩形）。
  - 窓の置き方そのもの（`placement` 系の spec）。本 spec は窓の位置を読むだけで、位置を変えない。
  - 名前の英字の大小（✅ `property-name-case-fold` が済ませた。本 spec の 2 つの名前もその規則に乗る）。
  - 完了 spec `areka-P0-sylphya` の文書（`design.md` の Invariants の言い過ぎ）の書き換え。直すのはコードのコメントだけ。
- **Adjacent expectations**:
  - 読みに行った値は保存しない（保存の作り直し `sylphya-sqlite-kvs`〔v0.2.0〕に影響しない形にする）。
  - 後の spec（`shell-scale-linkage`・`currentghost-property-tree`・`currentghost-property-others`・`system-property-values`・`property-catalog-lists`・`zorder-property`）は、本 spec の口に読み手や材料を足す。本 spec はその値を足さない。
  - 窓の位置を変える側（`window-link-directives`・`dpi-realign-remembered-chain`・ドラッグ・`\![move]`）が窓を動かした結果を、本 spec は読むだけ。
  - ウェーブ C11 の約束（`.kiro/steering/roadmap.md` の C11 の行の ④）で「触らない」とした `emo2_boot/mod.rs` などに、口をつなぐための行が要るときは、最小の行だけ足してよい（2026-10-23 開発者の承認・議題 5）。足すファイルと理由は設計に挙げ、⑥ `dpi-realign-remembered-chain` の持ち物（`placement/{spawn,persist,chain_realign,chain_finalize}.rs`・`emo2_boot/frame.rs`・`emo2_boot/frame/{dpi,drain_resnap}.rs`）に当たるときは、⑥ のセッションと調整してから触る。

## Requirements

### Requirement 1: キャラクターの窓の矩形を読める
**Objective:** As a 自動試験の道具を使う開発者, I want `get_property` で各スコープの窓の画面の位置と大きさを読めること, so that 撮影や記録の行を読み替えずに、窓の重なりや動きを数で確かめられる

#### Acceptance Criteria
1. While ゴーストがスコープ ID の窓を持っている, when MCP の `get_property` が `currentghost.scope(ID).rect` を聞いた, the areka shall その窓の今の画面の矩形を「左,上,右,下」の 4 つの整数で答える。
2. The areka shall スコープ 0（本体）・スコープ 1（相方）・スコープ 2 以降（3 人目以降）のどの窓にも、同じ名前の形 `currentghost.scope(ID).rect` で答える。
3. The areka shall 4 つの数を、主の画面の左上を原点とする物理の画素の仮想画面の座標で表す（主の画面より左や上に在る画面では負の数になる）。
4. The areka shall 4 つの数を `,` でつなぎ、空白・符号の `+`・前ゼロを入れない（SSP の `-782,201,49,1227` と同じ形）。
5. The areka shall 右の数を「左＋幅」、下の数を「上＋高さ」とする（右端・下端の画素を含まない数）。
6. The areka shall 答える矩形を、読まれた時点で OS がその窓に持っている矩形（窓の外枠）とし、areka の中の写し（`WindowPos` など）・面の画像から見積もった値・置き方の予定の値で答えない。
7. While 2 つのスコープの窓が同じ位置と大きさで重なっている, the areka shall 両方の名前に同じ 4 つの数を答える。
8. While スコープ ID の面が隠されている（`\s[-1]`）, the areka shall その窓の矩形を隠す前と同じ形で答える（SSP と同じ）。
9. When 前ゼロの付いた番号（`currentghost.scope(00).rect`）を聞かれた, the areka shall 前ゼロの無い番号と同じ窓の矩形を答える（SSP と同じ）。

### Requirement 2: バルーンの矩形を読める
**Objective:** As a 自動試験の道具を使う開発者, I want `get_property` で各スコープのバルーンの画面の位置と大きさを読めること, so that バルーンがキャラクターのどこに付いたかを数で確かめられる

#### Acceptance Criteria
1. While ゴーストがスコープ ID のバルーンの窓を持っている, when MCP の `get_property` が `currentghost.balloon.scope(ID).rect` を聞いた, the areka shall そのバルーンの窓の今の画面の矩形を、要件 1 の 3〜6 と同じ座標の系・数の形で答える。
2. The areka shall `currentghost.balloon.scope(ID).rect` の ID を、そのバルーンが付いているスコープの番号として扱う（スコープ 0 の台詞のバルーンは `scope(0)`、スコープ 1 の台詞のバルーンは `scope(1)`）。
3. When バルーンの大きさが中身に合わせて変わった, the areka shall 変わった後の大きさを含む矩形を答える。
4. While スコープ ID のバルーンが画面に出ていない, the areka shall そのバルーンの窓の矩形を答える（SSP と同じ）。

### Requirement 3: 窓が無いときの答え
**Objective:** As a 自動試験の道具とゴースト, I want 窓が無いときの答えが SSP と同じであること, so that SSP 向けに書かれたゴーストや試験が areka でも同じに動き、前の値や作った値を本当の位置と取り違えない

#### Acceptance Criteria
1. If 聞かれたスコープ ID のキャラクターの窓をゴーストが持たない（そのゴーストに無い番号・負の番号）, the areka shall `currentghost.scope(ID).rect` に `0,0,0,0` と答える（SSP と同じ）。
2. If 聞かれたスコープ ID のバルーンの窓をゴーストが持たない, the areka shall `currentghost.balloon.scope(ID).rect` に値なしで答える（SSP と同じ）。
3. While ゴーストの起動の途中で窓がまだ作られていない, the areka shall 2 つの名前に、窓を持たないスコープと同じ答え（要件 3.1・3.2）を返す。
4. The areka shall 値なしを、MCP では今の「無い名前」と同じ `NG:Cannot find such property name.`、ゴーストの道では今の「プロパティが無い」の失敗で答える（新しい答えの文を作らない）。
5. If 窓が壊された後に読まれた, the areka shall 壊れる前の矩形を答えず、窓を持たないスコープと同じ答えを返す。
6. If 名前の書式が読めない（例: `currentghost.scope().rect`・`currentghost.scope(x).rect`）, the areka shall 今の読み手と同じく値なしで答える。

### Requirement 4: 動いた後の値がついてくる
**Objective:** As a 自動試験の道具を使う開発者, I want 窓やバルーンが動いた・大きさが変わった後に読んだ値が新しい位置であること, so that 操作の直後に読んでも古い位置と取り違えない

#### Acceptance Criteria
1. When 利用者が窓のドラッグを終えた, the areka shall その後に受けた読み取りに、落とした後の矩形を答える。
2. When 台本の `\![move]` などで窓が動き終えた, the areka shall その後に受けた読み取りに、動いた後の矩形を答える。
3. When 窓が拡大率の違う画面へ移り、大きさが変わった, the areka shall その後に受けた読み取りに、移った先の画面での位置と大きさを答える。
4. When 面が替わって窓の大きさや位置が変わった, the areka shall その後に受けた読み取りに、替わった後の矩形を答える。
5. When 窓に付いて動くもう 1 つの窓やバルーンが一緒に動いた, the areka shall 一緒に動いたどの窓とバルーンの名前にも、動いた後の矩形を答える。
6. When OS の窓の矩形が変わった, the areka shall その後に受けた読み取りで、変わる前の矩形を答えない。
7. While 窓が動いている途中（ドラッグの最中・時間をかけて動く `\![move]` の途中）, the areka shall その窓が実際に居たことのある矩形だけを答え、ある時点の左上と別の時点の右下を混ぜた矩形を答えない。

### Requirement 5: 読み取りは待たず、動きを重くしない
**Objective:** As a ゴーストと自動試験の道具, I want 位置の読み取りが窓を動かす側の都合を待たずに答え、窓が動く間に余計な仕事を増やさないこと, so that 読み取りが画面の更新を止めたり、ドラッグが重くなったりしない

#### Acceptance Criteria
1. The areka shall 2 つの名前の読み取りに、窓を持つスレッドやほかのスレッドの処理の終わりを待たずに答える（完了 spec `areka-P0-sylphya` の要件 2.7 と同じ約束）。
2. When ゴーストが自分の処理の途中でプロパティの道から 2 つの名前を読んだ, the areka shall 止まらずに答える。
3. The areka shall 読み取りのために、窓の移動・描画・画面の更新を遅らせない。
4. The areka shall 窓が動くたび（ドラッグの毎フレームなど）に、矩形の値を作り直したり値の置き場へ出し直したりしない。
5. The areka shall 読み取りのたびに、通常の動作の記録を警告以上の段で残さない。

### Requirement 6: ゴーストごとに分け、保存しない
**Objective:** As a 開発者, I want 位置の値がそのときのゴーストの物だけで、保存に残らないこと, so that 別のゴーストや前の起動の位置を今の位置と取り違えない

#### Acceptance Criteria
1. The areka shall 2 つの名前の値を、その窓とバルーンを持つゴーストの値として答え、MCP の `ghost_name` で選んだゴースト・プロパティの道で聞いたゴーストの窓の矩形を返す。
2. When ゴーストを替えた, the areka shall 前のゴーストの窓とバルーンの矩形を答えず、新しいゴーストの窓の矩形を答える。
3. When ゴーストを読み直した（`reload`）, the areka shall 読み直しの後の窓とバルーンの矩形を答える。
4. The areka shall 2 つの名前の値を保存のファイルに書かない。
5. When areka を起動し直した, the areka shall 前の起動の矩形を答えず、新しく作った窓の矩形を答える。
6. The areka shall MCP の `get_property` とゴーストのプロパティの道で、同じ時点に同じ名前を読んだら同じ値を答える（どちらか一方だけが読める・違う値を返す、が無い）。

### Requirement 7: 値を取りに行く口と、材料を出す口
**Objective:** As a 後の spec を作る開発者, I want sylphya に「読まれた時に一次情報源を見に行く口」と「まれに変わる材料を出す口」が在ること, so that 倍率や DPI などの値を、読む道に手を入れずに、値ごとに正しい型で足せる

#### Acceptance Criteria
1. The sylphya shall 名前（またはその形）に対して、外から登録された読み手を持ち、その名前が読まれた時にその読み手で値を取りに行く口（プル）を 1 つ持つ。本 spec の 2 つの名前はこの口から答える。
2. The sylphya shall OS など外の環境を読むコードを自分では持たず、読み手は外（areka の側）から登録される（完了 spec `areka-P0-sylphya` の要件 8.3 の「注入可能な値源シーム」と同じ形）。
3. The sylphya shall この口に登録できる読み手を、窓を持つスレッドやほかのスレッドの返事を待たず、遅れもしない読み取りに限る（要件 5.1）。
4. Where 後の spec がこの口に別の名前の読み手を足した, the areka shall その名前を MCP の `get_property` とゴーストのプロパティの道の両方で、読む道を変えずに読めるようにする。
5. The areka shall この口から答えた値を、変わらない値（`baseware.version` など）・SHIORI の答え・保存した値の読み取りと混ぜない（この口に読み手を足しても、ほかの値の答えが変わらない）。
6. The areka shall 読み手が使う材料のうち、まれにしか変わらず、変わった時点が分かるもの（本 spec では「スコープ → 窓」の表）を、変わった時点で出す（プッシュ）。
7. The 開発の手順 shall 値ごとに型を次の基準で選び、設計に書く: いつ変わるか判断できない値はプル、「今この値に変わった」を簡単に受け取れる値はプッシュ、変わった時点が分かっても余りにも頻繁に変わりプッシュの負荷が馬鹿にできない値はプル（2026-10-23 開発者）。本 spec の窓の矩形は、ドラッグの間に毎フレーム変わるのでプル。
8. The areka shall sylphya の「読みで OS を呼ばない」と言い過ぎているコードのコメント（`reader.rs` の冒頭・`mirror.rs` の `SharedMirror::load`）を、要件 2.7・8.3 の言い方（他のスレッドを待たない・遅延しうる値源を載せない・外の環境は注入した値源を通す）へ直す。

### Requirement 8: 決定論のテスト
**Objective:** As a 開発者, I want 上の約束を実の画面・実の時計なしで毎回同じ結果で確かめられること, so that 回帰をいつでも見つけられる

#### Acceptance Criteria
1. The areka shall 要件 1〜7 の各場面（窓が在る・動いた・拡大率の違う画面へ移った・面が替わった・一緒に動いた・面を隠した・バルーンが出ていない・窓の無いスコープ・負の番号・前ゼロ・起動の途中・窓が壊れた・ゴーストを替えた・読み直した・起動し直した）を、実の窓・実の画面・実の時計を使わず、偽の読み手（偽の OS の窓の矩形）を登録した決定論のテストで確かめる。
2. The areka shall 数の形（負の数・右と下の数・空白なし・`0,0,0,0`）を、主の画面より左や上に在る画面の座標を含むテストで確かめる。
3. The areka shall MCP の `get_property` とゴーストのプロパティの道の両方で同じ値・同じ値なしが返ることを、決定論のテストで確かめる。
4. The areka shall 1 つの矩形の 4 つの数を、一次情報源への 1 回の問い合わせから組むことを、決定論のテストで確かめる（要件 4.7）。
5. The areka shall 本物の読み手（OS の窓の矩形を読む物）が、実の窓の矩形を物理の画素で返すことを、実の窓を 1 枚作る `#[ignore]` の実機テストで確かめる。

### Requirement 9: 実機の確かめと文書
**Objective:** As a 開発者, I want 実機の MCP で位置が読めることを確かめ、台帳と手順書を直すこと, so that ぱすたの検証と自動試験を `get_property` で回せる

#### Acceptance Criteria
1. When 実機の確かめを行う, the 開発の手順 shall 排他テストの机を持ってから areka を起こし、終わったらすぐ机を返す。
2. When 実機でぱすたを起こした（手順書の段 1）・同じ画面でドラッグした（段 4）・拡大率の違う画面へ移した（段 5）, the 開発の手順 shall 各段で MCP の `get_property` が `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.balloon.scope(0).rect` に値を答え、その値が areka の記録に残る窓の位置と大きさと一致することを確かめる。
3. The 開発の手順 shall 実機の値を SSP の基準（Introduction の表）と並べ、数の形・座標の系・無いときの答え（`0,0,0,0`・値なし）が同じであること、大きさの違いが拡大率への追従の方針によるものであることを書き分けて残す。
4. When 本 spec の実装が終わった, the 開発の手順 shall 網羅台帳 `doc/ukadoc-coverage/ledger/property.toml` の 2 行を、実装済みを表す状態へ改め、根拠への道を添える。
5. When 本 spec の実装が終わった, the 開発の手順 shall 手順書 `doc/pasta-real-run.md` の「撮影と記録の `bounds=` の行で読む」所を `get_property` で読む形へ戻し、表の段 1・4・5 の本 spec の欄を「着地済み・確かめる」の形に改める。

## 要件ディスカッションで決めたこと（2026-10-23）

| # | 議題 | 決め | 根拠 |
|---|---|---|---|
| 1 | 値の出し方 | **窓の矩形はプル**（読まれた時に、外から登録された読み手で OS の窓の矩形を見に行く）。**「スコープ → 窓」の表はプッシュ**（作る・壊す・替える時点で出す）。毎フレーム比べる仕組みは作らない | 開発者「システムパラメーターは要求があったときに、一次情報源へアクセスすべき」「いつ変わるのか判断できないならプル型、『今この値に変わった』が簡単に実装できるならプッシュ型」「余りにも頻繁に変わり、プッシュの負荷が馬鹿にできないならプル」。窓の矩形はドラッグの間に毎フレーム変わり、写しは 1 回ごとに全区画を写し直す作りなので、プッシュの負荷が読み（まれ）より桁違いに大きい |
| 2 | 新しい値になる時点（要件 4.6・4.7） | 議題から外れた。読んだ時点の OS の値を 1 回の問い合わせで返すので、作りの上で満たす | 議題 1 の決めの帰結 |
| 3 | 出ていない窓・無いスコープの答え | **SSP に合わせる**: 面を隠した窓・出ていないバルーンも窓の矩形を答える。窓の無いキャラクターのスコープは `0,0,0,0`、窓の無いバルーンのスコープは値なし | SSP の実測（`research.md` 7.1 節）。一次情報源（OS の窓）がそこに在る限り、その矩形が答え。`0,0,0,0` は SSP 向けのゴーストが 4 つの数を前提に分解しても壊れず、幅 0 なので本物の位置と取り違えない |
| 4 | 窓の外枠か、見えている絵の範囲か | **窓の外枠**（OS がその窓に持っている矩形） | 一次情報源が OS の窓。ukadoc も「ウインドウ矩形座標」。着せ替えのオフの部品の分まで窓が広いときは、その広さを答える |
| 5 | ウェーブの「触らない」約束 | 口をつなぐ最小の行は足してよい。理由とファイルは設計に挙げ、⑥ の持ち物に当たるときは ⑥ と調整する | 開発者の方針「触らない約束を理由に回避の形へ逃げない」 |
| 6 | sylphya の「読みで OS を呼ばない」 | 契約ではない。完了 spec の要件は「他のスレッドを待たない・遅延しうる値源を載せない・外の環境は注入した値源を通す」だけで、コメントが言い過ぎている。sylphya は OS のコードを持たず、読み手を外から登録してもらう | 開発者「sylphya は『どこにどんな情報があるのかを知っているし、更新できる情報なら更新できる、永続化対象の情報なら永続化する』ことが契約」「外部から読み込みハンドラを登録してもらう形にしてた方が、依存関係がきれいになる」 |

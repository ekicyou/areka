# Requirements Document

> 本文の実物の確認は **2026-10-31・本ブランチ**（main `9604677a`）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 正典はライブの ukadoc（`list_sakura_script.html`・`list_propertysystem.html`・`list_shiori_event.html`・`descript_ghost.html`・`descript_shell.html`・`spec_shiori3.html`・2026-10-31 に引き直した）。SSP の基準は、完了 spec `areka-P0-pasta-in-windows-verification` の `ssp-measurement.md` 2 節の段 5・3 節の表。
> ukadoc で読めず SSP でまだ測っていない点は、末尾の「要件討議の前に SSP で測る点と、討議で決める点」に挙げた。本文はそれまで ukadoc の字面と areka の先例で書いてある。

## Project Description (Input)
- **誰が困っているか**: ゴースト「窓際のぱすたさん」（ぱすた）。ぱすたは台本 `\![get,property,…]` で倍率と DPI を聞き、答えで窓の絵（面 10・11・12）を選ぶ。画面をまたいでドラッグされた後と倍率が変わった後（`OnShellScaling`）に、窓の位置と重なりの順を出し直す。descript に `balloon.syncscale,true` を書き、縦書きのバルーンの倍率をシェルに連動させる。
- **今どうなっているか**: areka では `\![get,property,…]` に受け手が無く、倍率の変わりのイベント（`OnShellScaling`・`OnBalloonScaling`）も送らず、ゴーストとシェルの descript の `balloon.syncscale` も読まない。どれも黙って捨てられる。DPI 追従でシェルとバルーンの倍率が揃うので見た目は崩れていないが、ぱすたの窓の絵の選び方と付け直しの合図は SSP と違う道を通る（聞いた答えが返らないので、起動の値 10 のまま）。
- **何を変えるか**: `\![get,property]` の受け手を 1 つ立て、指定のイベントを Reference に値を持たせて返す。`currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi` を MCP の `get_property` とゴーストの道の両方で読めるようにする。倍率が変わったときに `OnShellScaling`・`OnBalloonScaling` を正典の Reference で送る。`balloon.syncscale` を読んで値として持つ。範囲は狭く保つ（照会の道の残りは `property-query-channels`、利用者が倍率を変える道は `shell-balloon-scale-setting`）。詳細は `brief.md`。

## Introduction

### 誰が困っているか

- **ゴーストの作者（ぱすた）**: `\![get,property]` の答えが返らないので、窓の影の大きさを画面に合わせて選べない。倍率の変わりの合図が来ないので、倍率が変わった後の付け直しはドラッグの終わり（`OnMouseDragEnd`）だけが頼り。
- **SSP 向けに書かれたほかのゴースト**: 同じ道で値を聞くゴーストは、areka では答えを受け取れない。
- **自動試験の道具を使う開発者**: MCP の `get_property` で倍率と DPI を読めないので、画面をまたいだ後の倍率を数で確かめられない。

### ぱすたの使い方（取り寄せた書庫を展開した木 `target\pasta-verify\root\ghost\pasta-in-windows`・読むだけ）

| 場所 | 台本 | 答えを受けるイベントがすること |
|---|---|---|
| `ghost/master/dic/tategaki.pasta` の `＊重ね順`（毎秒のイベントで付け直すとき・辞書の読み込みの直後） | `\![get,property,OnPastaShellScale,currentghost.scope(0).scaling,currentghost.scope(0).currentmonitor.dpi]` | `OnPastaShellScale` が「シェルの拡大率 × 96 ÷ DPI」を計り、124 までは面 10・125〜174 は面 11・175 からは面 12 を窓の絵に選ぶ。選びが変わったら付け直しの印を付ける |
| 同じファイルの `OnShellScaling`・`OnBalloonScaling` の応答 | `\![get,property,OnPastaScaleCheck,currentghost.scope(0).scaling,currentghost.balloon.scope(0).scaling]` | `OnPastaScaleCheck` が 2 つの横の値を比べ、ずれていれば利用者に揃えてもらう台詞を出す。`OnShellScaling` は付け直しの印も付ける |
| `ghost/master/descript.txt` | `balloon.syncscale,true` | バルーンの倍率をシェルに連動させる（縦書きのバルーンは窓の絵に重ねてあるため） |

ぱすたのシェル（`shell/master`）とバルーン（`balloon/pasta_tategaki`）は基準の DPI を書いていない（どちらも 96）。

### 正典（ukadoc・2026-10-31 に引き直した）

| 名前 | ukadoc の説明 |
|---|---|
| `\![get,property,イベント名,プロパティ名,…]` | 「イベント名で指定したイベントが発生し、そのreference0以降にひとつずつプロパティシステムの値が入る」。記述例のイベント名は `SSPからバルーン情報取得`（`On` で始まらない） |
| `SenderType` の `property` | 「プロパティシステムの参照命令の結果起こされたイベント。」 |
| `currentghost.scope(ID).scaling` | 「現在のゴーストのスコープIDの実効拡大率。ユーザー設定×タグ設定×SERIKO設定の結果なので、小数点以下が出る場合あり。」（2.7.89） |
| `currentghost.balloon.scope(ID).scaling` | 「現在のバルーンの実効拡大率。」（以下同じ・2.7.89） |
| `currentghost.scope(ID).currentmonitor.dpi` | 「現在のゴーストのスコープIDのウインドウが表示されているモニターのDPI。」（2.8.16） |
| `OnShellScaling` | 「シェル倍率が変更された際に発生。」Reference0＝横の後・1＝横の前・2＝縦の後・3＝縦の前（パーセント） |
| `OnBalloonScaling` | 「バルーン倍率が変更された際に発生。」Reference は `OnShellScaling` と同じ並び（2.5） |
| ゴーストの descript `balloon.syncscale,true` | 「バルーン拡大縮小率のゴーストとの連動を強制的にONにする(trueのみ有効)。」（2.7 RC1） |
| シェルの descript `sakura.`・`kero.`・`char*.balloon.syncscale,true` | 本体側・相方側・2 人目以降のバルーンについて同じ。既定は「ghostのdescript設定(balloon.syncscale)」 |

### SSP の基準（`ssp-measurement.md` 2 節の段 5・3 節の表）

150% の画面から 200% の画面へ移っても、SSP の `currentghost.scope(0).scaling` は `150,150`、`currentghost.scope(0).currentmonitor.dpi` は 192、ぱすたの大きさは 831×1026 のまま。`OnShellScaling` は届かなかった（SSP の倍率は利用者の設定で、画面を移っても変わらないため）。値は「横,縦」を `,` でつないだ形。

### areka の倍率の意味（本 spec の決め・討議で確かめる）

- areka の基本の設計は**画面の DPI への追従**（2026-07-18 開発者・SSP の「画面が替わっても大きさを変えない」には合わせない・`verification.md` 5 節の 8）。窓ごとに「窓の居る画面の DPI ÷ 作者の基準の DPI」（シェルは `seriko.dpi`、バルーンは `dpi`・無ければ 96）の倍率で絵を描く（`crates/areka/src/emo2_boot/frame/dpi.rs` の `AuthorDpis`・`run_dpi_phase`）。利用者が倍率を変える道（`shell-balloon-scale-setting`・v0.4.0）・倍率のタグ `\![set,scaling]`・`surfaces.txt` の `scaling` はまだ無い（掛ける残りはどれも 100%）。
- ukadoc は `scaling` を「実効拡大率」＝実際にかかっている倍率の積と書く。ゆえに本 spec は、areka の `scaling` に**画面の DPI への追従の倍率 × 表示の倍率（今は 100%）**を出す（brief の議題 1 の推奨・枠は 2026-07-18 の開発者の決め）。
- この意味では、ぱすたの「拡大率 × 96 ÷ DPI」は作者の基準が 96 のとき常に 100 になり、ぱすたはどの画面でも面 10（影を縮めない絵）を選ぶ。Windows の窓の影も、ぱすたの絵の影も、同じく画面の DPI で大きくなるので、これが見た目の上で正しい選び。
- 拡大率の違う画面へ移ると areka の実効の倍率は変わるので、areka は `OnShellScaling`・`OnBalloonScaling` を送る（SSP は送らない場面だが、SSP では倍率がそもそも変わらない。正典の「倍率が変更された際」に従う）。

### 実物で引き直して確かめた点（2026-10-31）

1. **受け手が無い。** `\![get,property]` は汎用の運び手で受け手の表（`crates/areka/src/emo2_boot/consumer_ledger.rs`・登記 42 件）まで届き、誰も拾わない。
2. **出どころの語はもうある。** `SenderType` の `property` は `crates/areka-talk/src/origin.rs` の `SenderWord::Property` にあり、どこも使っていない。完了 spec `areka-P0-script-security-level` の要件は「照会の道は `SenderType: internal,property` を送る」と書いている。
3. **名前の自由なイベントの受理の先例は 2 つ。** 選択肢の ID は `On` で始まる名前だけ（`crates/areka-kanade/src/schedule/events.rs` の `is_allowed_choice_event`）。外から頼まれた名前は「空や空白だけでない・名前と Reference に改行（CR・LF）と NUL を含まない」（`crates/areka-kanade/src/sstp.rs` の `validate_notify`）。ukadoc の記述例が `On` で始まらないので、本 spec は後者に揃える。
4. **値を読む口は 2 つで、同じ置き場を読む。** MCP の `get_property` とゴーストのプロパティの道（✅ `mcp-scope-rect-property` の要件 6.6・7.4）。本番のゴーストの道は 32bit の中継越しで値の置き場へ届かない＝ゴーストの道は決定論のテストで確かめる（同じ spec の確かめた点 4）。
5. **網羅台帳の行。** `doc/ukadoc-coverage/ledger/property.toml` の 3 行（`scaling` の 2 つと `currentmonitor.dpi`）・`shiori.toml` の `OnShellScaling`・`OnBalloonScaling`・`sakura-script.toml` の `\![get,property]`・`assets.toml` のゴーストの `balloon.syncscale`（持ち主は本 spec）と、シェルの 3 行（持ち主が空）。

## Boundary Context

- **In scope**:
  - `\![get,property,イベント名,プロパティ名,…]` の受け手と、返しのイベント（`SenderType: internal,property`）。読める値なら名前を問わず返す。
  - `currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi` の 3 つの値（MCP の `get_property` とゴーストの道）。
  - `OnShellScaling`・`OnBalloonScaling` を送る場面と Reference。
  - ゴーストの descript の `balloon.syncscale` と、シェルの descript の `sakura.`・`kero.`・`char*.balloon.syncscale` の読み取りと、その値を持つこと。
  - 台本の命令の受け手の一覧に 1 行（`check_script` の判定に載る）・決定論のテスト・網羅台帳の行・ぱすたの実機の確かめ（200% と 150% の画面をまたぐドラッグ）。
- **Out of scope**:
  - `\![set,property]`・`%property[…]`・`\![embed]` など照会の道の残り（`property-query-channels`）。
  - 利用者が倍率を変える道と設定・`syncscale` の強制を利用者の切り替えに効かせること（`shell-balloon-scale-setting`・v0.4.0）。
  - `\![set,scaling]` のタグ・`surfaces.txt` の描画メソッド `scaling`・ほかの `currentghost.*` の名前（`currentghost-property-tree`・`currentghost-property-others`）。
  - 画面をまたぐドラッグの置き方（`dpi-realign-remembered-chain`）・窓の組の連れ動き（`window-link-directives`）。本 spec は倍率を読み、合図を送るだけで、窓の置き方と大きさの決め方を変えない。
  - DPI 追従の基本の設計の変更（SSP の「画面が替わっても大きさを変えない」には合わせない）。
- **Adjacent expectations**:
  - 3 つの値は ✅ `mcp-scope-rect-property` が作った「外から登録された読み手」の口に載せる。倍率と DPI は変わる時点が分かる（DPI 追従の相が新しい倍率を決める）ので、変わった時点で読み手の材料へ出す（プッシュ・2026-10-23 開発者の基準）。
  - 返しのイベントは ✅ `mcp-kanade-tools` の参照つきのイベントの送り方と、✅ `areka-P0-script-security-level` の出どころの運び方に乗る。
  - 後の spec（`property-query-channels`）は同じ受け手の表と読み手を使う。`shell-balloon-scale-setting` は、利用者が表示の倍率を変えたときにも本 spec の 2 つのイベントを同じ Reference で送り、本 spec が持つ `syncscale` の値を利用者の切り替えに効かせる。
  - ウェーブ C15 の約束（`.kiro/steering/roadmap.md` の C15 の行の ①）: 受け手の表 `consumer_ledger.rs` に行を足すのは本 spec の 1 行だけ（42→43・`balloon-communicatebox` は C16）。`placement/mod.rs` は 1〜2 行だけ足す。同じウェーブの ③ `alert-dialog-per-monitor-dpi` は `emo2_boot/frame/dpi.rs` に触らず、⑤ `emo-bake-log-overlap` は `placement/mod.rs`・`ghost_session.rs` に触らない約束なので、本 spec がこれらを触ってよい。

## Requirements

### Requirement 1: `\![get,property]` で値を聞き、指定のイベントで受け取れる
**Objective:** As a ゴーストの作者, I want 台本 `\![get,property,イベント名,プロパティ名,…]` で聞いた値が、指定したイベントの Reference に入って返ること, so that SSP と同じ道で倍率や DPI を聞き、答えで振る舞いを選べる

#### Acceptance Criteria
1. When 再生中の台本が `\![get,property,イベント名,プロパティ名1,プロパティ名2,…]` に届いた, the areka shall 各プロパティ名の値を、届いた時点の値として、MCP の `get_property` と同じ名前の規則（英字の大小の扱いを含む）で引く。
2. When 値を引き終えた, the areka shall イベント名のイベントをその台本のゴーストの SHIORI へ送り、Reference0 から順に、台本に書かれた順のプロパティ名の値を 1 つずつ入れる（n 番目の名前の値を Reference(n−1) に入れる）。
3. The areka shall 返しのイベントの要求の `SenderType` を `internal,property` とする。
4. If プロパティ名の値が無い（無い名前・読めない書式・窓の無いスコープなど）, the areka shall その番号の Reference を空の文字列にし、後ろの名前の番号をずらさない。
5. The areka shall 受け手を特定のプロパティ名に縛らず、MCP の `get_property` が答えられる名前ならどれでも返す（本 spec の 3 つの値に限らない）。
6. The areka shall 台本の再生を `\![get,property]` で止めず、タグの後ろの台本を続けて再生する。
7. When 返しのイベントの応答が台本を返した, the areka shall その台本を、areka が送るほかのイベントの応答と同じ規則で再生する。
8. When MCP の `check_script` が `\![get,property,…]` を含む台本を調べた, the areka shall その命令を「受け手の無い命令」として報告しない。

### Requirement 2: 返しのイベントの名前の受理
**Objective:** As a ゴーストの作者, I want 返しのイベントの名前を自分で決められ、書き損じたときは記録で分かること, so that ukadoc の記述例どおりの名前を使え、壊れた要求が黙って SHIORI へ流れない

#### Acceptance Criteria
1. The areka shall 返しのイベントの名前を、`On` で始まる名前や areka の決まったイベントの表に載る名前に限らない（ukadoc の記述例 `SSPからバルーン情報取得` も受ける）。
2. If イベント名が空、または空白だけ, the areka shall イベントを送らず、警告の段で理由とタグを記録に残し、台本の残りの再生を続ける。
3. If イベント名か引いた値に改行（CR・LF）または NUL が含まれる, the areka shall イベントを送らず、警告の段で理由とイベント名を記録に残し、台本の残りの再生を続ける。
4. If プロパティ名が 1 つも書かれていない（`\![get,property,イベント名]`）, the areka shall Reference を持たないイベントとして送る。
5. The areka shall 返しのイベントを送ったことを、イベント名と値の数を添えて記録に残す（通常の動作なので警告より下の段）。

### Requirement 3: 倍率と DPI の値が読める
**Objective:** As a ゴーストの作者と自動試験の道具を使う開発者, I want 各スコープの窓とバルーンの実効の拡大率と、窓の居る画面の DPI を読めること, so that 画面をまたいだ後の倍率を数で確かめ、ぱすたが窓の絵を正しく選べる

#### Acceptance Criteria
1. When `currentghost.scope(ID).scaling` が読まれた, the areka shall そのスコープのキャラクターの窓の絵にその時点で実際にかかっている倍率を、百分率の「横,縦」で答える。
2. When `currentghost.balloon.scope(ID).scaling` が読まれた, the areka shall そのスコープのバルーンの窓にその時点で実際にかかっている倍率を、同じ形で答える。
3. The areka shall 実効の倍率を「画面の DPI への追従の倍率（窓の居る画面の DPI ÷ 作者の基準の DPI）× 表示の倍率」とし、表示の倍率を、利用者が倍率を変える道ができるまで 100% とする（シェルは `seriko.dpi`、バルーンはバルーンの `dpi` を作者の基準とし、書かれていなければ 96）。
4. The areka shall SSP のように「拡大率の違う画面へ移っても倍率の値を変えない」答え方をせず、移った先の画面での実効の倍率を答える（DPI 追従の基本の設計）。
5. When `currentghost.scope(ID).currentmonitor.dpi` が読まれた, the areka shall そのスコープのキャラクターの窓が居る画面の DPI を 10 進の整数で答える（96 が等倍）。
6. The areka shall 倍率の横と縦に同じ数を入れ、`,` でつなぎ、空白・符号の `+`・前ゼロを入れない（SSP の `150,150` と同じ形）。
7. The areka shall 倍率が整数の百分率で割り切れるときは整数で、割り切れないときは小数点以下を付けて答える（ukadoc「小数点以下が出る場合あり」）。
8. While ぱすたのように作者の基準の DPI が 96 で、窓が 150%（DPI 144）の画面に居る, the areka shall `currentghost.scope(0).scaling` に `150,150`、`currentghost.scope(0).currentmonitor.dpi` に `144` を答える（200%〔DPI 192〕の画面なら `200,200` と `192`）。
9. If 聞かれたスコープの窓をゴーストが持たない（そのゴーストに無い番号・負の番号・起動の途中で窓がまだ無い）, the areka shall 3 つの名前に値なしで答える（MCP では今の `NG:Cannot find such property name.`、ゴーストの道では今の「プロパティが無い」の失敗・新しい答えの文を作らない）。
10. The areka shall MCP の `get_property` とゴーストのプロパティの道で、同じ時点に同じ名前を読んだら同じ値を答える。

### Requirement 4: 値は変わった時点で新しくなり、ゴーストごとに分かれ、保存しない
**Objective:** As a ゴーストと自動試験の道具, I want 画面を移った直後に読んでも古い倍率と取り違えず、読み取りが動きを重くしないこと, so that 倍率の変わりの合図を受けてすぐ聞き直しても、新しい値が返る

#### Acceptance Criteria
1. When 窓が拡大率の違う画面へ移り、新しい倍率が絵にかかった, the areka shall それより後に受けた読み取りに、新しい倍率と移った先の画面の DPI を答える。
2. The areka shall 新しい倍率が絵にかかる前に新しい値を答えず、かかった後に古い値を答えない（答える値と描かれた絵の倍率を食い違わせない）。
3. The areka shall 3 つの値の読み取りに、窓を持つスレッドやほかのスレッドの処理の終わりを待たずに答える（完了 spec `areka-P0-sylphya` の要件 2.7 と同じ約束）。
4. The areka shall 窓が動くたび（ドラッグの毎フレームなど）に、倍率と DPI の値を作り直したり値の置き場へ出し直したりせず、倍率か DPI が変わった時点でだけ出す。
5. When ゴーストを替えた・読み直した・areka を起動し直した, the areka shall 前のゴーストや前の起動の値を答えず、新しい窓の値を答える。
6. The areka shall 3 つの値を保存のファイルに書かない。

### Requirement 5: 倍率が変わったときの `OnShellScaling`・`OnBalloonScaling`
**Objective:** As a ゴーストの作者, I want シェルとバルーンの倍率が変わったときに正典のイベントが届くこと, so that ぱすたが倍率の変わりで窓を付け直し、シェルとバルーンの倍率のずれを確かめられる

#### Acceptance Criteria
1. When 本体（スコープ 0）のキャラクターの窓にかかる実効の倍率が変わった（拡大率の違う画面へ移った）, the areka shall そのゴーストの SHIORI へ `OnShellScaling` を送り、Reference0 に横の変わった後、Reference1 に横の変わる前、Reference2 に縦の変わった後、Reference3 に縦の変わる前の倍率を入れる。
2. When 本体（スコープ 0）のバルーンの窓にかかる実効の倍率が変わった, the areka shall `OnBalloonScaling` を、`OnShellScaling` と同じ並びの Reference で送る。
3. The areka shall 2 つのイベントの Reference の倍率を、要件 3 の `scaling` の横・縦の数と同じ数の形（百分率・割り切れないときは小数）で入れる。
4. The areka shall 2 つのイベントを、新しい倍率が絵にかかったのと同じ時点で送り、1 フレーム遅らせて送らない。そのイベントの応答の台本から `\![get,property]` で聞いた `scaling` には、変わった後の倍率が返る。
5. If 窓が同じ DPI の画面の間を移った・倍率の百分率が変わる前と同じ, the areka shall 2 つのイベントを送らない。
6. The areka shall ゴーストの起動・ゴーストの切り替え・読み直し・シェルとバルーンの切り替えで倍率が初めて決まるときには、2 つのイベントを送らない（倍率が変わったのではなく、絵の組が新しくなったため）。
7. The areka shall スコープ 1 以降の窓だけの倍率が変わったときには、2 つのイベントを送らない（ukadoc の 2 つのイベントはスコープを Reference に持たないので、本体の窓を代表とする）。
8. When 本体の窓とバルーンの窓の倍率が同じ時点で変わった, the areka shall `OnShellScaling` と `OnBalloonScaling` をそれぞれ 1 回ずつ送る。
9. When ぱすたを拡大率の違う画面へドラッグでまたがせた, the areka shall 今も送っているドラッグの終わり（`OnMouseDragEnd`）に加えて倍率の変わり（`OnShellScaling`）を届け、ぱすたがどちらからも付け直しの印を付けられるようにする。

### Requirement 6: `balloon.syncscale` を読み、値として持つ
**Objective:** As a ゴーストの作者, I want descript に書いた `balloon.syncscale,true` が読まれ、バルーンの倍率をシェルに連動させる指定として持たれること, so that 利用者が倍率を変える道ができたときに、作者の指定が働く

#### Acceptance Criteria
1. When ゴーストを起こした, the areka shall ゴーストの descript の `balloon.syncscale` を読み、値が `true` のとき、すべてのスコープについて「バルーンの倍率をシェルに連動させる」指定を持つ。
2. When シェルを読み込んだ, the areka shall シェルの descript の `sakura.balloon.syncscale`（スコープ 0）・`kero.balloon.syncscale`（スコープ 1）・`char*.balloon.syncscale`（`*` の番号のスコープ）を読み、値が `true` のスコープについて同じ指定を持つ。
3. The areka shall あるスコープの指定を、シェルの側の鍵が `true` か、ゴーストの側の鍵が `true` のときに「連動する」とする（シェルの側の既定はゴーストの設定・どちらも `true` だけが有効）。
4. If 鍵の値が `true` 以外（`false`・空・ほかの文字）, the areka shall その鍵を「指定なし」として扱い、起動を止めない。
5. When ゴースト・シェルを切り替えた、または読み直した, the areka shall 新しい descript から指定を読み直し、前の指定を残さない。
6. The areka shall 指定を持つだけで、今のバルーンの倍率の働き（バルーンの DPI への追従・表示の倍率 100%）を変えない（指定を利用者の倍率の設定に効かせるのは `shell-balloon-scale-setting`）。
7. The areka shall 読んだ指定を、スコープごとに記録に残す（通常の動作なので警告より下の段）。

### Requirement 7: 決定論のテスト
**Objective:** As a 開発者, I want 上の約束を実の画面・実の時計なしで毎回同じ結果で確かめられること, so that 回帰をいつでも見つけられる

#### Acceptance Criteria
1. The areka shall 要件 1・2 の各場面（値が並ぶ順・値の無い名前の空の Reference・名前を問わない受け手・`On` で始まらない名前・空の名前・改行と NUL・プロパティ名なし・`SenderType`・台本の再生が続くこと・`check_script` の判定）を、決定論のテストで確かめる。
2. The areka shall 要件 3・4 の値（倍率の式・小数の形・DPI・窓の無いスコープ・新しい倍率がかかる前と後・ゴーストの切り替え）を、偽の DPI と偽の作者の基準を注入した決定論のテストで、MCP の `get_property` とゴーストのプロパティの道の両方について確かめる。
3. The areka shall 要件 5 の送る場面と送らない場面（本体の倍率が変わった・同じ DPI の画面・起動と切り替え・スコープ 1 以降だけ・シェルとバルーンが同じ時点で変わった）と Reference の並びを、決定論のテストで確かめ、イベントが新しい倍率のかかったのと同じフレームで送られることを固定する。
4. The areka shall 要件 6 の読み取り（ゴーストの側だけ・シェルの側だけ・両方・`true` 以外の値・切り替え）を、決定論のテストで確かめる。

### Requirement 8: 実機の確かめと網羅台帳
**Objective:** As a 開発者, I want 実機のぱすたが SSP と同じ道で倍率を聞き、付け直しが走ることを確かめ、台帳を直すこと, so that ぱすたの検証（`pasta-in-windows-verification` の回し直し）で本 spec の分を閉じられる

#### Acceptance Criteria
1. When 実機の確かめを行う, the 開発の手順 shall 排他テストの机を持ってから areka を起こし、終わったらすぐ机を返す。
2. When 実機でぱすたを起こし、200% と 150% の画面の間をドラッグでまたがせた, the 開発の手順 shall `OnPastaShellScale` が Reference に倍率と DPI を持って届くこと、`OnShellScaling` が届くこと、ぱすたが窓の位置と重なりの順を出し直すことを、areka の記録で確かめる。
3. When 8.2 の確かめの各段（起動の直後・同じ画面の中でのドラッグの後・拡大率の違う画面へ移した後）に達した, the 開発の手順 shall MCP の `get_property` で 3 つの値を読み、窓の居る画面の拡大率と一致すること、ぱすたが倍率のずれの台詞（`＊バルーン倍率のずれ`）を出さないことを確かめる。
4. The 開発の手順 shall 実機の値を SSP の基準（Introduction の表）と並べ、数の形が同じであること、倍率の値と `OnShellScaling` の届き方の違いが DPI 追従の方針によるものであることを書き分けて残す。
5. When 本 spec の実装が終わった, the 開発の手順 shall 網羅台帳の `sakura-script.toml` の `\![get,property]`・`shiori.toml` の `OnShellScaling`・`OnBalloonScaling`・`property.toml` の 3 行・`assets.toml` のゴーストとシェルの `balloon.syncscale` の 4 行を実装済みを表す状態へ改め、根拠への道を添える（シェルの 3 行の持ち主に本 spec を書く）。

## 要件討議の前に SSP で測る点と、討議で決める点

brief の議題 ⑴ ⑵ と、要件を書くときに ukadoc で決まらなかった点。本文は下の「今の書き方」で書いてある。測りの結果や決めで変わるときは、該当の受け入れ基準を直す。

| # | 点 | 今の書き方（根拠） | 変わる受け入れ基準 |
|---|---|---|---|
| 1 | areka の `scaling` に何を出すか（brief の議題 1） | 画面の DPI への追従の倍率 × 表示の倍率（ukadoc「実効拡大率」・2026-07-18 の DPI 追従の決め） | 3.3・3.4・3.8・5.1・5.5 |
| 2 | `\![get,property]` の往復の Reference・`SenderType`・`SecurityLevel`（SSP の MCP で測る・brief の議題 2） | Reference0 から順・`internal,property`（完了 spec `areka-P0-script-security-level` の書き方） | 1.2・1.3 |
| 3 | 無い名前・プロパティ名なし・窓の無いスコープの `scaling`・`currentmonitor.dpi` のときの SSP の答え（SSP で測る） | 空の Reference で番号を保つ・Reference なしで送る・値なし（`rect` は `0,0,0,0` だったが、倍率と DPI は測っていない） | 1.4・2.4・3.9 |
| 4 | SSP が `OnShellScaling` を送る場面（シェルの倍率の設定を変えたとき・画面を移ったとき・シェルの切り替え）（SSP で測る・brief の議題 2） | 本体の窓の実効の倍率が変わったときだけ・起動と切り替えでは送らない | 5.1・5.6 |
| 5 | 割り切れない倍率の小数の桁（SSP で測る） | 小数点以下を付ける（桁は設計で決める） | 3.7・5.3 |
| 6 | 2 つのイベントの代表のスコープ（areka は窓ごとに倍率を持つ・SSP はゴーストに 1 つ） | 本体（スコープ 0）の窓 | 5.1・5.2・5.7 |

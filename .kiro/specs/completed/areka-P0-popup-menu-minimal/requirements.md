# Requirements Document

> 本文の file:line と行数は**本ブランチでの実測値（2026-09-18）**である。設計・実装の着手時に引き直すこと。

## Introduction

### 誰が困っているか

第三者の利用者。ゴーストを替える・バルーンを替える・更新する・終了する、という**操作の入口が画面に無い**。里々や YAYA で書かれた第三者のゴーストは「ベースウェアが右クリックメニューを持っている」前提で作られており、ゴースト側に自前のメニューを持たないものが多い。

### いま何が起きているか（本ブランチで実測・2026-09-18）

- キャラクター窓の右クリックは、**ダブルクリックとしてしか SHIORI に届かない**（`crates/areka/src/input_events/mod.rs` の `on_char_pointer_pressed`＝`DoubleClick::Right` → `OnMouseDoubleClick`）。単発クリックは意図して送っていない（同関数の doc「単発クリック → 送出しない（7.3）」）。「owner-draw 右クリックメニューは実装しない（7.4）」と明記されている。
- `CreatePopupMenu`／`TrackPopupMenu`／`HMENU`／`WM_CONTEXTMENU`／`AppendMenuW`／`ShellExecuteW` は `crates/` に **0 件**。ルート `Cargo.toml` の `windows` 機能一覧には `Win32_UI_WindowsAndMessaging` と `Win32_UI_Shell` が既に有効＝**依存追加 0** で呼べる。
- 終了の入口は **Ctrl＋左ダブルクリック**だけ（同関数・`MouseWiring::send_close_request(CloseReason::User)` → `KanadeMsg::CloseRequest`）。第三者には見つけられない。
- 終了の握手 `OnClose` は Ref0（`user`／`system`）だけを載せる（`crates/areka-kanade/src/schedule/events.rs` の `on_close`・doc「Ref1/2（スコープ番号・SSP）は単一スコープの M1 では省略する」）。
- SHIORI リソースの `GET` は `username` の 1 つしか引けない（`crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`）。`*button.caption`・`popupmenu.visible`・`popupmenu.type` は語彙台帳（`crates/areka-sylphya/src/vocab/shiori_resource.rs`）に名前があるだけで、引きに行く場所が無い。網羅台帳では 13 項目とも `vocabulary-only`・担当 `""`（`doc/ukadoc-coverage/ledger/shiori.toml`）。
- `readme` は `crates/areka`・`crates/areka-ghost`・`crates/areka-parsers` に **0 件**。ゴースト定義の `readme,ファイル名` キーも読んでいない（台帳 `assets.toml`＝`absent`）。`\![open,readme]` の消費者も無い（台帳 `sakura-script.toml`＝`absent`）。検体の実配置は、emo2（`crates/pilot/examples/shiori-host-32/fixtures/emo2/readme.txt`）も `R_POST_and_KOMAINU`（`vendors/sample_ghost/R_POST_and_KOMAINU/readme.txt`）も**ゴーストのフォルダ直下**（`ghost/`・`shell/`・`install.txt` と同じ階層）に `readme.txt` を置き、どちらも `descript.txt` に `readme` キーを持たない。
- 右ボタンは wintf のポインタ経路で押下・解放とも捕まえている（`crates/wintf/src/ecs/window_proc/mouse_click.rs` の `WM_RBUTTONUP`）が、areka へ配られるのは押下のみで、**解放はまだ届いていない**（`crates/wintf/src/ecs/pointer/dispatch/mod.rs` の `dispatch_pointer_events` は `OnPointerPressed`／`OnPointerMoved` だけを発火する）。
- M1 の E2E で「メニュー」と呼んでいたものは、ゴースト側が用意した**選択肢バルーン**（`OnMouseDoubleClick` → 辞書 `menu.pasta`）であり、ベースウェアのメニューではない。

### 何を変えるか

キャラクター窓を右クリックすると、**OS 標準（Win32）のポップアップメニュー**が出る。α の第 1 スライスとして本仕様が着地させるのは、「説明書」（ゴーストの readme を既定のアプリで開く）と「終了」（既存の `OnClose` の握手へ）の 2 項目、SHIORI リソース `*button.caption` による項目名の差し替え、`popupmenu.visible` による表示の抑止、そして後続の spec が自分の項目を足すための**登記式の口**（`MenuRegistry`）である。ゴースト／シェル／バルーンのサブメニュー（列挙は `areka-P0-baseware-root-layout`・切替は `areka-P0-ghost-shell-balloon-switch`）と「インストール…」「ネットワーク更新」（`areka-P0-ghost-install`・`areka-P0-network-update`）は、それぞれの spec がこの口へ登記する。本仕様は口の形（枠・名前・有効／無効・チェック・子項目・選ばれたときの動作）を要件で確定し、「登記した項目が正しい位置に現れる」を決定論テストで検証する。

見た目は OS に任せる（DPI・フォント・高コントラストは OS が面倒を見る）。オーナードロー（`menu_*.png`・`menu.font.*`）は M2 予約のまま持ち込まない。

## Boundary Context

- **In scope**（利用者・ゴースト作者・後続 spec の実装者から見える範囲）:
  - キャラクター窓（本体側スコープ 0・相方側スコープ 1）の右クリックで OS 標準のポップアップメニューが出ること。
  - 項目の枠 7 種（ゴースト・シェル・バルーン・ネットワーク更新・インストール…・説明書・終了）と、その並び順。
  - 「説明書」と「終了」の 2 項目の動作。
  - 項目名の差し替え（`*button.caption`）と、表示の抑止（`popupmenu.visible`）・種類（`popupmenu.type`＝α では差を付けない）。
  - 登記式の口 `MenuRegistry`（後続 spec が枠へ項目・サブメニューを足す）。
  - `\![open,readme]`（引数なし）＝「説明書」と同じ関数を呼ぶ台本側の入口。
  - 終了の握手 `OnClose` に Ref1・Ref2（スコープ番号）を載せること。
  - メニュー表示中の並行動作（SHIORI の死活監視が止まらない・閉じた後に再生が続く）。
  - 網羅台帳の担当欄への登記（15 項目）と、それに連なる文書の追随。
- **Out of scope**:
  - オーナードロー（見た目の全て）・`menu.*` の descript キー・`menu_*.png`（M2 予約・ロードマップ仮裁定 3）。
  - 着せ替えメニュー（`sakura.menuitem*`・`bindgroup*.name`・α 後）。
  - 設定ダイアログ・バージョン情報・使用率グラフ・ポータル・おすすめ・「全て終了」（`quitbutton.caption`）・「他のゴーストを呼ぶ」など、7 種の枠に無い項目（α 後）。α は単一ゴーストなので「終了」と「全て終了」を区別しない。
  - トレイアイコン（`Shell_NotifyIcon`・裁定候補 ⑷・含めない）。
  - 右クリック（単発）の `OnMouseClick`（Ref5＝1）送出（裁定候補 ⑶・送らない現状を維持）。
  - バルーン窓の右クリック（現状のまま変えない）。
  - ゴースト／シェル／バルーンのサブメニューの**中身**（列挙・現在のチェック・`menu,hidden` の除外）と「インストール…」「ネットワーク更新」の**動作**。それぞれの spec が登記する。
  - `\![open,readme,種類,名前]`（引数付き）の実行。列挙が要るため α では語彙だけ持ち、警告して何もしない。
  - `readme.charset`。既定のアプリに渡すだけなので areka は中身を読まない。
  - 「終了」以外の理由（`system`）の終了。既存のまま。
- **Adjacent expectations**:
  - **前提（完了済み）**: `areka-P0-input-events`（ポインタ経路・`MouseWiring`）・`areka-P0-collision-dpi-hittest`（右クリック座標の窓相対化）・`areka-P0-host32-window-thread-pump`（SHIORI アクターの待機中の死活監視）。
  - **下流（登記する側）**: `areka-P0-baseware-root-layout`（ゴースト／バルーンの列挙のサブメニュー）・`areka-P0-ghost-shell-balloon-switch`（選択時の `SwitchRequest`・シェルの列挙）・`areka-P0-ghost-install`（「インストール…」）・`areka-P0-network-update`（「ネットワーク更新」）・`areka-P0-alpha-release-signoff`（第三者の手順として右クリックメニューを載せる）。これらは本仕様の口の形を変えず、項目を足すだけ。
  - **並走（A0）**: `areka-P0-nar-install`・`areka-P0-default-balloon-bundle`。本仕様が触るのは `crates/areka/src/input_events/`・wintf のポインタ配送・新規ファイルで、検体パスを参照する既存テスト・example は触らない（共有ファイル 0）。ただし網羅台帳と `doc/ukadoc-coverage/roadmap-draft.md` は 3 本とも触りうる共有文書であり、後着側が数を数え直す。
  - **完了済みの隣接**: `areka-P0-ukadoc-coverage-roadmap`（台帳の検査 `cargo test -p ukadoc-survey`＝担当欄・`[[spec]]` 行・`owner_count`・報告の全文一致を見張る）。

## Requirements

### Requirement 1: 右クリックでメニューが出る

**Objective:** 第三者の利用者として、キャラクターを右クリックしたら操作の一覧が出てほしい。そうすれば、説明書も終了も、後から増える切替や更新も、同じ場所から見つけられる。

#### Acceptance Criteria

1. When キャラクター窓（スコープ 0 または 1）の上で右ボタンが押されて離された, the areka shall 離された位置に OS 標準のポップアップメニューを表示する。
2. The areka shall メニューの描画・DPI・フォント・高コントラスト・キーボード操作（矢印・Enter・Esc）・メニュー外クリックによる閉じ方を OS の標準動作に委ね、自前で描画しない。
3. While メニューが表示されている, the areka shall メニュー外へのクリックまたは Esc でメニューを閉じ、その操作をゴーストへの入力（撫で・ダブルクリック）として扱わない。
4. When メニューの項目が選ばれた, the areka shall メニューを閉じてからその項目の動作を 1 回だけ行う。
5. When 右ボタンが押されて離された（メニューを出す操作）, the areka shall SHIORI へ `OnMouseClick` を送らない（裁定候補 ⑶・現状維持）。
6. The areka shall バルーン窓の右クリックの振る舞いを変えない。
7. If メニューの表示そのものに失敗した（OS の API がエラーを返した）, then the areka shall 失敗を `error!` で記録して何も表示せず、ゴーストの動作を続ける。
8. While ゴーストの起動が完了する前（入力の結線が済む前）, the areka shall 右クリックでメニューを出さず、`trace!` で記録して何もしない（既存の自己抑止と同じ扱い）。
9. While キャラクター窓を左ボタンでドラッグしている, the areka shall 右ボタンの操作でメニューを出さない（ドラッグを終えてからの右クリックで出る）。
10. When 右ボタンのダブルクリックが検出された, the areka shall 右クリックがメニューの引き金として有効なとき（`popupmenu.visible` が `0` でない）は `OnMouseDoubleClick`（Ref5＝1）を SHIORI へ送らず、`popupmenu.visible` が `0` でメニューを出さないときだけ既存どおり送る（正典 [OnMouseDoubleClick](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseDoubleClick:1) の Ref5＝1 はメニューを抑止したゴーストのためのもの）。1 度目の解放と 2 度目の押下が同じ処理単位に入ると「メニューが出る」と「ダブルクリックが届く」が同時に起きうるため、どちらか一方に定める（裁定候補 ⑸・要件 11.3）。

### Requirement 2: メニューの構成と並び

**Objective:** 利用者と後続 spec の実装者として、メニューのどこに何が並ぶかが決まっていてほしい。そうすれば、後から足される項目が勝手な位置に現れず、説明書と終了はいつも同じ場所にある。

#### Acceptance Criteria

1. The areka shall メニューの枠を次の 7 種・この並び順と定める: ①ゴースト ②シェル ③バルーン ④ネットワーク更新 ⑤インストール… ⑥説明書 ⑦終了。正典は並びを定めていないので、この並びは本仕様の裁定であり、SSP の並びを写したものではない。
2. The areka shall 枠 ⑥説明書と ⑦終了を本仕様で登記し、常にメニューに載せる。
3. While 枠 ①〜⑤に項目が登記されていない, the areka shall その枠をメニューに**出さない**（無効表示にもしない）。
4. When 枠に子項目の列（サブメニュー）が登記されている, the areka shall その枠をサブメニュー付きの項目として出し、子項目を登記された順に並べ、「現在」と示された子項目にチェックを付ける。
5. When 登記された項目が「無効」と示されている, the areka shall その項目を選べない状態（灰色）で出す。
6. The areka shall 枠の間の区切り線の有無を設計に委ねる（要件が定めるのは並び順だけである）。
7. The areka shall 1 回の表示の中で各項目に一意の識別子を与え、選ばれた識別子から登記された動作を取り違えなく引く。

### Requirement 3: 項目名は SHIORI リソースで差し替わる

**Objective:** ゴースト作者として、里々や YAYA の辞書に書いた `*button.caption` の文言がそのままメニューに出てほしい。そうすれば、SSP 向けに書いたゴーストの「ネットワーク更新(&U)」「新しい私に生まれ変わる」といった名前が areka でも見える。

#### Acceptance Criteria

1. The areka shall 枠と SHIORI リソースと既定名の対応を次のとおり定める（リソース名は正典 [SHIORI Resource リスト](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html) のもの・既定名は正典が定めていないので本仕様の裁定）:

   | 枠 | SHIORI リソース | 既定名 |
   |---|---|---|
   | ①ゴースト | `ghostrootbutton.caption` | ゴースト |
   | ②シェル | `shellrootbutton.caption` | シェル |
   | ③バルーン | `balloonrootbutton.caption` | バルーン |
   | ④ネットワーク更新 | `updatebutton.caption` | ネットワーク更新 |
   | ⑤インストール… | `ghostinstallbutton.caption` | インストール… |
   | ⑥説明書 | `readmebutton.caption` | 説明書 |
   | ⑦終了 | `closebutton.caption` | 終了 |

2. When メニューを出す操作が行われた, the areka shall そのとき出す枠のリソースをそれぞれ `GET` で SHIORI に問い合わせ、返った値が空でなければその文言を項目名に使う。問い合わせはメニューを出すたびに行い、前回の値を使い回さない。
3. If リソースの問い合わせが「値なし（204）」または空文字列を返した, then the areka shall 既定名を使い、そのことを `debug!` で記録する。
4. If リソースの問い合わせが失敗した（SHIORI が応答できない・応答が壊れている）, then the areka shall 既定名を使い、失敗を `warn!` で 1 回記録し、メニューは出す。
5. When 文言に `&` が含まれている, the areka shall それを OS のアクセラレータ記法としてそのまま渡す（`(&U)` が下線付きの U として出る）。エスケープや除去は行わない。
6. When メニューを出す操作が行われた窓のスコープが n である, the areka shall `popupmenu.visible` の問い合わせ先を n＝0 なら `sakura.popupmenu.visible`・n＝1 なら `kero.popupmenu.visible` とする（正典 [sakura.popupmenu.visible](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.visible:1)・[kero.popupmenu.visible](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#kero.popupmenu.visible:1)）。α のキャラクター窓はスコープ 0 と 1 だけ（Boundary Context・`char_scope` の前提）なので、[char*.popupmenu.visible](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#char_2a.popupmenu.visible:1)（n≧2）は語彙として持つだけで問い合わせ先を作らない（多キャラクターの窓が生まれる spec が引き受ける）。
7. If `popupmenu.visible` の問い合わせが `0` を返した, then the areka shall メニューを出さず、そのことを `info!` で 1 回記録する。値なし・空・失敗・`0` 以外の値はいずれも「表示する」と扱う。
8. The areka shall `popupmenu.type`（同じスコープ接頭辞・正典 [sakura.popupmenu.type](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.type:1)）を**問い合わせない**。値 `1`（省略メニュー）の中身を正典が定めていないため、α ではどの値でも本体側メニューと同じものを出す（値で中身を変えないなら問い合わせる意味が無く、往復を増やさない）。警告も記録も出さない。
9. The areka shall 登記された項目が自分の文言に使うリソース名を登記時に指定できるようにし、指定が無い項目は登記された既定名をそのまま使う（サブメニューの子項目＝列挙されたゴースト名などは通常リソースを持たない）。
10. While ゴーストの起動が完了して SHIORI と会話できる状態になる前, the areka shall SHIORI へ問い合わせを送らず（運行側が「値なし」を即座に返す）既定名でメニューを出す（起動待ちでメニューを止めない）。

### Requirement 4: 説明書を開く

**Objective:** 第三者の利用者として、ゴーストの説明書（readme）をメニューから開きたい。そうすれば、作者の注意書きや配布元を探し回らずに済む。

#### Acceptance Criteria

1. The areka shall 説明書のファイルを次の順で決める: ⑴ ゴースト定義 `ghost/master/descript.txt` の `readme` キーの値（正典 [descript_ghost readme,ファイル名](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#readme_2c_30d5_30a1_30a4_30eb_540d:1)）、⑵ キーが無ければ正典の既定 `readme.txt`。ファイル名はゴーストのフォルダ（`ghost/`・`shell/` を含む階層＝起動引数で渡した根）の直下で解決する。
2. When 「説明書」が選ばれた, the areka shall 決めたファイルを OS の既定のアプリで開き、areka の中に表示用の窓は作らない。
3. While 決めたファイルが存在しない, the areka shall 「説明書」を選べない状態（灰色）で出し、初回に `debug!` で記録する。
4. If ファイルは存在するが既定のアプリで開けなかった, then the areka shall 失敗を `error!`（パスとエラーコード付き）で記録し、ゴーストの動作を続ける。
5. When 台本に `\![open,readme]`（引数なし・正典 [\![open,readme]](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bopen_2creadme_5d:1)）が現れた, the areka shall 「説明書」を選んだときと同じ動作を行う（既存の汎用コマンド運搬で名前 `open` を消費者が選別する）。
6. If `\![open,readme,種類,名前]`（引数付き）が現れた, then the areka shall 何もせず `warn!` で 1 回記録する（列挙が要るため α では語彙だけ持つ）。
7. The areka shall `readme.charset` を読まない（既定のアプリに渡すだけで中身を解釈しない）。

### Requirement 5: 終了

**Objective:** 第三者の利用者として、メニューから終了したい。そうすれば、Ctrl＋左ダブルクリックという隠し操作を知らなくても、ゴーストが終了の挨拶をして正しく閉じる。

#### Acceptance Criteria

1. When 「終了」が選ばれた, the areka shall 既存の終了指示の経路（`CloseRequest{User}`・完了済み `areka-P0-input-events` が Ctrl＋左ダブルクリックのために敷いたもの）で終了の握手を始め、窓を直接閉じない。
2. When 終了の握手の `OnClose` を SHIORI へ送る, the areka shall Ref0 を `user`、Ref1 と Ref2 をメニューを出した窓のスコープ番号（本体側 0・相方側 1）にする（正典 [OnClose](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:1)）。`popupmenu.type` に差を付けない α では Ref1＝Ref2 である。
3. The areka shall 結線済みの Ctrl＋左ダブルクリックによる終了指示の入口を**取り除く**（開発者裁定 2026-09-19・実機確認でメニューの「終了」が働くことを確かめたうえで: 同じ役目の隠し操作は要らない）。結線済みで Shift を伴わない Ctrl＋左ダブルクリックは、Ctrl の無い左ダブルクリックと同じに扱う（`OnMouseDoubleClick` を送る）。利用者起因の終了の入口はメニューの「終了」だけになる。
4. If 終了の握手が SHIORI に拒まれた（終了挨拶の台本が返らず握手が `Steady` へ戻る既存の経路）, then the areka shall メニューからの終了でも同じく終了せず、既存の記録をそのまま出す。
5. The areka shall 強制退避の入口（結線前の Ctrl＋左ダブルクリック、および Ctrl＋Shift＋左ダブルクリック）を残す。メニューは結線後のキャラクター窓にしか出ず、応答しない SHIORI は終了の握手を拒むので、起動に失敗した・固まったゴーストから抜ける口はこれだけである（2026-09-19 の実機確認でも、絵を出せないゴーストは検証用ダミー窓へ落ちてメニューを出せなかった）。

### Requirement 6: 登記式の口（MenuRegistry）

**Objective:** 後続 spec（列挙・切替・インストール・更新）の実装者として、メニュー本体を触らずに自分の項目やサブメニューを足したい。そうすれば、4 本の spec が同じファイルを競って書き換えずに済み、切替が着地した日にメニューへ切替が現れる。

#### Acceptance Criteria

1. The areka shall 登記の単位として次を受け取る口を用意する: 枠（①〜⑦のどれか）・既定名・文言のリソース名（任意）・有効／無効・チェック（任意）・子項目の列（サブメニューにするとき）または選ばれたときの動作（そのどちらか一方）。
2. When メニューを出す操作が行われた, the areka shall 登記された各項目の**そのときの**内容（既定名・有効／無効・チェック・子項目）を求めてから組み立てる（登記時に固定した値を使い回さない）。そうすれば、列挙の結果や「現在の選択」が最新のものになる。
3. When 既に項目が登記されている枠へ別の項目が登記された, the areka shall 後から登記されたものに置き換え、そのことを `warn!` で記録する。
4. The areka shall 登記の取り消し（枠を空にする）を受け付ける。取り消した枠は要件 2.3 のとおり出なくなる。
5. When 子項目が選ばれた, the areka shall その子項目に登記された動作を、メニューを閉じた後に 1 回だけ行う。
6. The areka shall 登記された動作の実行を台本の操作と同じ経路に流す（切替は `SwitchRequest`・終了は `CloseRequest`・インストールと更新はそれぞれの spec が定める要求）。メニュー固有の抜け道を作らない。
7. The areka shall 「登記した項目が定めた枠の位置に、登記した順で、チェック・有効／無効が写されて現れる」ことを、OS の API に触れない純粋な構造（枠→項目→子項目・識別子の払い出し）で確かめられる形にする（要件 9 の決定論テストの前提）。

### Requirement 7: メニュー表示中の並行動作

**Objective:** 利用者として、メニューを出している間や閉じた後に、ゴーストが固まったり壊れたりしてほしくない。

#### Acceptance Criteria

1. While メニューが表示されている, the areka shall SHIORI の待機中の死活監視（完了済み `areka-P0-host32-window-thread-pump` が定めたもの）を止めない。メニューが表示のために待っている間も、SHIORI の停止は同じ時間内に検出される。
2. While メニューが表示されている, the areka shall 会話の再生（文字送り・待ち）・SERIKO のアニメーション（まばたき等）・バルーンの描画を止めずに続ける（開発者裁定 2026-09-18: デスクトップマスコットとしての挙動を優先し、「表示中に描画が止まる」形は採らない）。
2a. When メニューが閉じた, the areka shall 表示中に届いた入力を取りこぼさず、表示中と同じ時間の流れのまま再生を続ける（閉じた瞬間に止まっていた分を一気に追いつくような飛びを起こさない＝表示中に止めていないので追いつく分は生じない）。
3. If メニューの表示中に SHIORI の停止が検出された, then the areka shall メニューが閉じた後に既存の停止経路を通し、落ちない。
4. If メニューの表示中に窓が閉じられた（SHIORI からの終了指示や強制退避）, then the areka shall メニューを閉じて落ちない。表示中もゴーストが動き続ける（7.2）ため、この経路は表示中に実際に起こりうる。閉じた後に窓が既に無ければ、選ばれていた項目の動作を行わず `debug!` で記録する。
5. When メニューを出して閉じた, the areka shall 既定の IME 窓（ある種の窓を作ると OS が黙って添える見えない窓）をキャラクター窓やバルーンの上に残さない（既知の罠＝記憶 windows-default-ime-window-sits-above-owner。避け方は設計・確かめ方は実機確認＝要件 9.9）。

### Requirement 8: 記録

**Objective:** 開発者として、メニューが出なかった・名前が既定に落ちた・説明書が開かなかった理由をログから読み取りたい。

#### Acceptance Criteria

1. When メニューを出した, the areka shall `info!` で 1 行（スコープ・項目数）を記録する。
2. When 項目が選ばれた, the areka shall `info!` で 1 行（枠・識別子）を記録する。
3. When メニューが何も選ばれずに閉じた, the areka shall `debug!` で 1 行を記録する。
4. The areka shall 失敗の経路（表示失敗・リソース問い合わせ失敗・既定アプリで開けない・登記の置き換え）を要件 1.7・3.4・4.4・6.3 のとおり記録し、記録の無い失敗経路を持たない（記憶 areka-log-first-no-silent-failure）。
5. The areka shall 記録の書式を steering `logging.md`（構造化フィールド・スコープ接頭辞）に従わせる。

### Requirement 9: 決定論テストと実機確認

**Objective:** 開発者として、OS のメニュー API に触れずに構造・名前・動作の分岐を毎回のテストで確かめ、API に触れる部分だけを実機で 1 度確かめたい。

#### Acceptance Criteria

1. The areka shall 「登記 → 枠と並び → 識別子」の構造を純粋関数として持ち、次を決定論テストで確かめる: 7 枠の並び順・未登記の枠が出ないこと・サブメニューの子項目の順とチェック位置・無効の写し・識別子の一意性と逆引き。
2. The areka shall 偽の SHIORI（台本どおりに `GET` へ答える既存の仕組み）で、`*button.caption` が値を返す／空を返す／204 を返す／失敗する の 4 通りで項目名が要件 3.2〜3.4 のとおりになることを確かめる。
3. The areka shall `popupmenu.visible` が `0`／`1`／値なし のときの表示可否（要件 3.7）と、`0` のときだけ右ダブルクリックが `OnMouseDoubleClick`（Ref5＝1）として届くこと（要件 1.10）を確かめる。
4. The areka shall 説明書のファイルの決め方（`readme` キーあり／なし・ファイルあり／なし）と、その結果としての有効／無効（要件 4.1・4.3）を確かめる。
5. The areka shall `\![open,readme]` が「説明書」と同じ関数へ届くこと、引数付きが警告して何もしないこと（要件 4.5・4.6）を確かめる。
6. The areka shall 「終了」の選択が `CloseRequest{User}` を 1 件送ること、および `OnClose` の Ref1・Ref2 がスコープ番号になること（要件 5.1〜5.3）を、既存の終了テスト（`input_events_tests.rs`）と同じ観測方法（送られた指示を受信側で数える）で確かめる。
7. The areka shall 上のテストが判断分岐を壊すと赤になることを、少なくとも 1 つの分岐で摂動して示す（記憶 cage-must-walk-the-reachable-path・checks-must-judge-not-just-print）。
8. The areka shall 新設・改変したファイルを 1 ファイル 1,000 行以内に収める（`input_events/mod.rs` は現 475 行・番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`）。
9. The areka shall OS の API に触れる部分（表示・位置・閉じ方・既定アプリで開く・IME 窓の罠）を、現行の argv 起動（emo2 の実 pasta と `R_POST_and_KOMAINU` の里々）による実機確認で 1 度確かめ、確認項目と結果を tasks の完了記録に残す: ⑴ 右クリックで出る ⑵ 「説明書」で readme.txt が既定アプリで開く ⑶ 「終了」で終了挨拶が再生されて閉じる ⑷ 里々の `readmebutton.caption` の文言（`(&R)` の下線）が写る（`R_POST_and_KOMAINU` の `dic06_String.txt` は「現在のシェルについて(&R)」「取扱説明書(&R)」「Read me(&R)」の 3 候補から毎回選ぶので、開き直すたびに変わりうる＝要件 3.2 の「毎回問い合わせる」の実機観察でもある。同ファイルの `updatebutton.caption` は枠 ④ が α で未登記のため観察できない）⑸ 表示中に落ちない・閉じた後に会話が続く ⑹ 閉じた後に見えない窓（IME 窓）がキャラクター窓の上に残らない。（2026-09-19 追記: ⑷ は `R_POST_and_KOMAINU` が絵を出せずキャラクター窓が生えないため本仕様では観察できず、`areka-P0-shell-implicit-surface` の実機サインオフへ引き渡した。項目名の写しそのものは要件 9.2 の決定論テストが留めている。）

### Requirement 10: 網羅台帳への登記

**Objective:** 網羅調査の読み手として、本仕様が正典のどの項目を引き受けたかが台帳で分かってほしい。そうすれば、束「メニュー」の残りが何かを数え直さずに済む。

#### Acceptance Criteria

1. The 本仕様 shall 網羅台帳の担当欄（`owner`）に本仕様の名前を登記する項目を次の **15 項目**と定める（零も明示する）:
   - `doc/ukadoc-coverage/ledger/shiori.toml`（13）: `ghostrootbutton.caption`・`shellrootbutton.caption`・`balloonrootbutton.caption`・`updatebutton.caption`・`ghostinstallbutton.caption`・`readmebutton.caption`・`closebutton.caption`・`sakura.popupmenu.visible`・`kero.popupmenu.visible`・`char*.popupmenu.visible`・`sakura.popupmenu.type`・`kero.popupmenu.type`・`char*.popupmenu.type`
   - `doc/ukadoc-coverage/ledger/sakura-script.toml`（1）: `\![open,readme]`
   - `doc/ukadoc-coverage/ledger/assets.toml`（1）: `descript_ghost` の `readme,ファイル名`
   - 担当に**しない**もの（理由付き）: `quitbutton.caption`（「全て終了」は α に無い）・`OnMouseClick`（裁定候補 ⑶・送らない）・`readme.charset`（中身を読まない）・`menu,hidden`／`char*.menu`（シェルの列挙側＝`areka-P0-ghost-shell-balloon-switch` か `areka-P0-baseware-root-layout` が登記時に除外する）・`OnClose`（既に実装済み・Ref1/2 を足しても状態は変わらない）・束「メニュー」の残り（オーナードロー・着せ替え・その他の `*button.caption`＝α 後）。
2. When 担当欄を登記する, the 本仕様 shall 同じコミットで `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に本仕様の行（`owner_count`＝台帳の数え直し）を足し、`[briefs].count` と本文の手書きの数を実数えで直し、`report`・`report-summary` を走らせて報告を作り直し、`cargo test -p ukadoc-survey` が緑であることを確かめる（担当欄の非空の宛先は `[[spec]]` の名前でなければ赤になる＝腕 f）。
3. When 実装が着地した, the 本仕様 shall 15 項目の状態を実測に合わせる（実装済みは「areka が正典どおりに動く」項目だけ・`README.md` §1）:
   - 実装済み（6）: `readmebutton.caption`・`closebutton.caption`・`sakura.popupmenu.visible`・`kero.popupmenu.visible`・`readme,ファイル名`・`\![open,readme]`（引数なし。引数付きは縮退として備考に書く）。
   - 語彙のみのまま担当だけ登記（9・備考に理由と引受先を書く）: `ghostrootbutton.caption`・`shellrootbutton.caption`・`balloonrootbutton.caption`・`updatebutton.caption`・`ghostinstallbutton.caption`（引く仕組みは本仕様が置くが、枠 ①〜⑤が未登記の間は誰も引かない。枠を登記する spec が着地した日に実装済みへ改める）・`char*.popupmenu.visible`（α に n≧2 の窓が無い・要件 3.6）・`sakura.popupmenu.type`・`kero.popupmenu.type`・`char*.popupmenu.type`（問い合わせない裁定・要件 3.8）。
4. The 本仕様 shall 15 項目の定義箇所に正典 URL のコメントを 1 行ずつ置く（既存の慣行・`doc/ukadoc-coverage/README.md`）。
5. If 並走する A0 の spec が先に `roadmap-draft.md` を書き換えて数が食い違った, then the 本仕様 shall 引き算で合わせず、台帳を引いて数え直した値を書く。

### Requirement 11: 裁定候補と既定

**Objective:** 開発者として、答えで作業が変わる分かれ目だけを渡され、それぞれに既定が置かれていてほしい。そうすれば、覆さない限り実装は止まらない。

#### Acceptance Criteria

1. The 本仕様 shall 裁定候補 ⑶「右クリックで `OnMouseClick`（Ref5＝1）も SHIORI へ送るか」を**送らない**（現状維持）で進める。送ると里々の標準テンプレートが「右クリック」の台詞を返し、メニューと同時に喋る（brief）。
2. The 本仕様 shall 裁定候補 ⑷「トレイアイコンを α に含めるか」を**含めない**で進める（`windowposition.limit` で画面内へ戻る構造が既にある）。
3. The 本仕様 shall 裁定候補 ⑸「右ダブルクリックの `OnMouseDoubleClick`（Ref5＝1）」を、**既存の経路を残し、メニューが有効なときは送らず、`popupmenu.visible`＝0 でメニューを抑止したときだけ送る**（要件 1.10）で進める。右ボタンを離した時点でメニューが出れば以後の入力はメニューが受けるので通常は到達しないが、1 度目の解放と 2 度目の押下が同じ処理単位に入る速いダブルクリックでは両方が起きうる。到達不能に頼らず規則として定め、正典の Ref5＝1 が意味を持つ「メニューを抑止したゴースト」でだけ届ける。完了済み `areka-P0-input-events` の右ダブルクリック送出に条件を 1 つ足す改変になる。
4. The 本仕様 shall 裁定候補 ⑹「説明書のファイルが無いとき」を**項目を灰色で出す**（要件 4.3）で進める。出さない案（枠を消す）は「説明書」の場所が変わるので採らない。
5. Where 開発者が上の既定を覆した, the 本仕様 shall 該当する要件（1.5・1.10・4.3・11.3）と設計を同時に改訂する（記憶 revise-design-not-just-requirements）。

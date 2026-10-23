# Brief: areka-P0-popup-menu-shell-look

> 2026-10-18 棚卸㉗で起票（開発者の指示「もう一つの目標は、ゴースト「ぱすた」がSSPと同じレベルで動くこと、です。」＝v0.1.0 のゴール）。証拠はこの棚卸のぱすたの抜けの調べと、ゴースト「窓際のぱすたさん」のシェルの descript（読むだけ・`menu.*` と `sakura.menu,auto`）。roadmap の予約「owner-draw 右クリックメニュー（`popup-menu-minimal` の構造の上に見た目を被せる）・着せ替えメニュー」を引き取る。正典は ukadoc MCP で 2026-10-18 に引き直した。

## Problem

- **ゴーストの作者**: シェルの descript に書いた右クリックメニューの見た目（`menu.*`）と、着せ替えのメニュー（`sakura.menu,auto`）が areka で出ない。
  - ぱすたは着せ替えを `sakura.bindgroup*` で 20 個近く持ち（口・目・眉・うで・まばたき・閉じる・ボール）、`sakura.menu,auto` でメニューに自動で並べる。areka の右クリックメニューに着せ替えの項目は無い＝利用者が着せ替えを選べない。
  - ぱすたの `menu.*` は「あえて普通の Windows のメニューに見せる」色と絵（Windows 11 のライトテーマの標準のメニューの実測の色）。areka の OS 標準のメニューでも、ライトテーマならほとんど同じに見える。違いが出るのはダークテーマのときと、左の帯の絵（SSP の標準の絵を消す幅 1px の帯）くらい。

## Current State

- **正典（ukadoc・要旨）**:
  - シェルの descript の `menu.background.bitmap.filename`・`menu.foreground.bitmap.filename`・`menu.sidebar.bitmap.filename`（既定はベースウェアの標準の絵）と、それぞれの `*.alignment`（位置。`lefttop`・`repeat-x`・`repeat-y` など）・`menu.background.font.color.*`・`menu.foreground.font.color.*`・`menu.disable.font.color.*`・`menu.separator.color.*`・`menu.frame.color.*`（2.5.41）・`menu.font.name`・`menu.font.height`（オーナードローのメニューの文字・ゴーストの descript とシェルの descript の両方）。絵の既定の名前は `menu_background.png`・`menu_foreground.png`・`menu_sidebar.png`（ukadoc「シェル」のファイルの一覧）。
  - `sakura.menu,auto` または `hidden`: 本体側の着せ替えのメニューを自動で並べる・隠す（既定は手書き）。`sakura.menuitem*,ID`・`sakura.menuitemex*,メニュー名,ID`: 手で並べる（`-` で仕切り線）。プロパティ `sakura.bind.menu`。
- **areka**:
  - 右クリックメニューは OS 標準（`crates/areka/src/menu/win32.rs`・137 行・`CreatePopupMenu`＋`TrackPopupMenuEx`。持ち主はキャラクターの窓そのもので、メニューのための窓は作らない＝完了 `popup-menu-minimal` の要件 7.5）。
  - steering `tech.md` に「自前の窓もオーナードローも持たない（描画・キーボード操作・外側クリックで閉じる作法は OS 任せ＝α の表現力据え置き方針）」と書いてある。roadmap は owner-draw と着せ替えのメニューを予約に置いている。
  - メニューの組み立ては枠 11 種の登記（`crates/areka/src/menu/mod.rs` の `Frame::ORDER`）と計画（`menu/plan.rs` の `MenuPlan`）。着せ替えの枠は無い。
  - オーナードローの描画の知らせ（`WM_MEASUREITEM`・`WM_DRAWITEM`）は、キャラクターの窓の手続き（`crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message`）に腕が無い。
  - 着せ替えの状態は seriko の部品が持つ（`crates/areka-seriko/src/state.rs` の `apply_bind`・`apply_bind_exclusive`）。`bindgroup` の名前と既定は読み込みで拾われている（`crates/areka-parsers/src/package/resolve_shell.rs` ほか）。
  - 網羅台帳 `assets.toml`: `menu.*` の行・`sakura.menu`・`sakura.menuitem*`・`sakura.menuitemex*` は absent・持ち主なし。
- **windows-reactor の危うさ**: roadmap は設定画面・タスクトレイ・インストーラーを「windows-reactor 待ち」（WinUI3 に任せる）に置いた。右クリックメニューはそこに入っていない。ただし、後で右クリックメニューも WinUI3 のメニューへ移すなら、Win32 のオーナードローの描画は捨てることになる。残るのは、descript の `menu.*` の読み取り・絵の読み込み・着せ替えの項目の計画（`MenuPlan` の側）で、捨てるのは Win32 の描画（`WM_MEASUREITEM`・`WM_DRAWITEM` の受け答え）だけ。オーナードローは `popup-menu-minimal` の計画の上に見た目を被せる形なので、計画の作りとはぶつからない。

## Desired Outcome

- 着せ替えのメニュー: `sakura.menu,auto` のシェルで、右クリックメニューに着せ替えのサブメニューが出る（カテゴリごと・`mustselect` のカテゴリは 1 つだけ選べる・今の着衣に印）。選ぶと `\![bind]` と同じ道で着せ替えが変わる。`hidden` なら出ない。`sakura.menuitem*`・`menuitemex*` の手書きの並びも効く。
- 見た目: `menu.*` を書いたシェルでは、右クリックメニューがその色と絵で出る。書いていないシェルは今の OS 標準のまま。
- 網羅台帳の行が実物と合う。steering `tech.md` の「オーナードローを持たない」の行が新しい決まりに合う。

## Approach

- 着せ替えのメニュー: 計画（`menu/plan.rs`）に着せ替えの枠を 1 つ足す。項目は descript の `bindgroup` と `menu,auto`／`menuitem*` から組み、今の着衣は seriko の部品から写しを受け取る。選んだら台本の `\![bind]` と同じ受け手へ流す。
- 見た目: 計画の項目に「オーナードローで描く」印を足し、wintf の窓の手続きに `WM_MEASUREITEM`・`WM_DRAWITEM` の口を足して、areka が用意した描き方（色・絵・文字）で描く。描き方の材料はメニューを出す前に全部そろえ、OS のメニューの待ちのあいだに World を借りない。
- **採らなかった案**: メニューのための自前の窓（`popup-menu-minimal` の要件 7.5 と steering に反し、キーボード操作と外側クリックの作法を自前で持つことになる）。

## Scope

- **In**: 着せ替えのサブメニュー（`sakura.`・`kero.`・`char*.menu`・`menuitem*`・`menuitemex*`・`mustselect`・選んだときの着せ替え）・オーナードローの見た目（`menu.*` の色・3 枚の絵と位置・`menu.font.*`・選べない項目・仕切り線・枠）・拡大率の追従・記録・決定論のテスト・実機の確かめ・網羅台帳・steering の行。
- **Out**: メニューの項目そのものの増減（`popup-menu-residue`・各 spec）・WinUI3 のメニュー（windows-reactor）・プロパティ `sakura.bind.menu` の読み書き（`currentghost-property-tree`）・`OnDressupChanged`（`script-small-tags` の議題）・タスクトレイのメニュー。

## Boundary Candidates

- 着せ替えの枠（`menu/` の計画と供給・seriko からの着衣の写し）
- 見た目の読み取り（descript の `menu.*`・絵）
- オーナードローの描画（`menu/win32.rs` と wintf の窓の手続きの新しい口）

## Out of Boundary

- 右クリックの引き金と待ち（`menu/trigger.rs`）
- 着せ替えの合成（`areka-emo-compose`）

## Upstream / Downstream

- **Upstream**: 完了 `popup-menu-minimal`（計画と表示の作り）・完了 `ghost-shell-balloon-switch`（サブメニューの枠の前例）。
- **Downstream**: `pasta-in-windows-verification`。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は直さない。steering の行を直す）。
- **Adjacent**: `popup-menu-residue`（`menu/win32.rs`・`trigger.rs`・件 11 の黒い四角・件 12 の見えずに閉じる＝オーナードローの描画と同じ所）・`stay-on-top-directives`（メニューの「常に手前に表示」を作るなら）・`inputbox-user-input`・`link-context-copy`（`menu/win32.rs` を挙げる）・`extra-character-windows`（`char*.menu`）。

## Constraints

- 失敗（絵が読めない・描けない）は記録を残し、OS 標準の見た目へ落ちてメニューは出す。
- オーナードローの描画は OS のメニューの待ちの中で同期して呼ばれる＝その中で World を借りない・待たない。
- 拡大率 200% の画面で、絵と文字の大きさが合う（DPI 追従は基本設計）。
- 新しいコードは新しいファイルへ。テストは兄弟のファイル。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka/src/menu/plan.rs`（135）・`menu/mod.rs`（368・枠を 1 つ登記）・`menu/win32.rs`（137）・新しい `menu/dressup_frame.rs` と `menu/shell_look.rs`（印: 新）と兄弟のテスト・`crates/wintf/src/ecs/window_proc/mod.rs`（380・口を 2 つ）と新しいファイル（印: 新）・seriko の着衣の写しを渡す所（`crates/areka-seriko/src/state.rs` か areka の側の写し）・descript の読み取り（`crates/areka-parsers/src/package/resolve_shell.rs` か新しい転記のファイル）・網羅台帳 `assets.toml`・`.kiro/steering/tech.md` の 1 行。
- 規模: 14〜18 タスク（着せ替えのメニュー 6〜8・オーナードローの見た目 8〜10）。20 を超えそうなら、**オーナードローの見た目**と**着せ替えのメニュー**の 2 本に分ける（触る所は `menu/plan.rs`・`menu/mod.rs` だけが共通で、wintf を触るのはオーナードローの側だけ）。
- 先に要るもの: 働きの上では無し。ファイルの上では `popup-menu-residue`（`menu/win32.rs`・`trigger.rs`）と同じウェーブに置かない。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。
- 版: v0.1.0。
- 要件定義のモデル: Fable（基準＝先例の無い境目の公開の約束: wintf の窓の手続きが、OS のメニューの待ちの中で同期して `WM_MEASUREITEM`・`WM_DRAWITEM` を areka へ渡す口を新しく持つ。場所は `crates/wintf/src/ecs/window_proc/mod.rs` と `crates/areka/src/menu/win32.rs`）。着せ替えのメニューだけに分けた側は Opus で足りる（先例＝ゴースト・シェル・バルーンのサブメニュー）。
- 要件の段の議題:
  1. **windows-reactor との関係**（開発者の決め）: 右クリックメニューも後で WinUI3 へ移すつもりか。移すなら、v0.1.0 では着せ替えのメニュー（計画の側＝移しても残る）だけを作り、オーナードローの描画は WinUI3 の形が決まるまで待つ案を推す（ぱすたのメニューはライトテーマでは OS 標準とほぼ同じに見えるので、ぱすたのゴールの線にはほとんど効かない）。移さないなら両方を作る。
  2. **steering の「オーナードローを持たない」**（2026-09-19 の方針）を改めてよいか（開発者の決め）。
  3. **着衣の写し**: メニューを組む時点で seriko の部品の着衣を、areka の側でどう読むか（写しを持つか・聞きに行くか）。
  4. **SSP で測る**: 着せ替えのメニューの並び（カテゴリの順・仕切り・`mustselect` の印）と、選んだときに `OnDressupChanged` が来るか。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 10-18 から触るファイルは動いていない（`crates/areka/src/menu/plan.rs` 135・`menu/mod.rs` 368〔枠は 11 種のまま・着せ替えの枠は無い〕・`menu/win32.rs` 137・wintf の `crates/wintf/src/ecs/window_proc/mod.rs` 380〔`WM_MEASUREITEM`・`WM_DRAWITEM` の腕は今も無い〕・着衣を持つ `crates/areka-seriko/src/state.rs` 728・`crates/areka-parsers/src/package/resolve_shell.rs` 127）。生きている決まり 6（オーナードローと着せ替えメニューは予約）を解くかの開発者の裁定はまだ出ていない＝ウェーブの外のまま。
- 新しく起票された相手: v0.4.0 の `menu-thumbnail-preview`（10-20）が同じ `menu/{mod,win32,plan}.rs` と `window_proc/mod.rs` を挙げる（向こうはメニューとは別の小さな窓で出すと決め、決まり 6 を待たない）＝同じウェーブに置かない相手が 1 本増えた（版が違うので当面は並ばない）。
- 触るファイル: 棚卸㉗の見立てのとおり。
- 規模: 14〜18 のまま。
- 先に要るもの: 開発者の裁定（決まり 6 を解くか・右クリックメニューを後で WinUI3 へ移すか）。同じウェーブに置けない相手: `popup-menu-residue`（`menu/win32.rs`・版に縛らない）。v0.1.0 の他とは 0（`ghost-inner-balloon` は `menu/balloon_frame.rs` で別のファイル・`mouse-click-wheel-events` が触りうるのは `menu/trigger.rs` で本 spec の外）。1 本だけの席は取らない。実機の確かめ（メニューを開いて撮る）は、クリックを通す窓へ入力を送る許し＝人の待ちになりうる。議題 4 は SSP で測る。
- 優先度の区分: A のまま（開発者の指示＝v0.1.0 のゴール・ぱすたの `menu.*` と `sakura.menu,auto`）。
- 要件定義のモデル: Fable のまま（オーナードローの側だけ。基準＝先例の無い境目の約束: wintf の窓の手続きが OS のメニューの待ちの中で同期して描画の知らせを areka へ渡し、その中で World を借りない決まりを新しく引く。場所は `window_proc/mod.rs` と `menu/win32.rs`。今ある `WM_SETCURSOR` の口は wintf の中で閉じた診断で、先例にならない）。着せ替えのメニューだけなら Opus（先例＝ゴースト・シェル・バルーンのサブメニュー）。
- 分割の案: 前のまま（オーナードローの見た目／着せ替えのメニュー）。裁定が「WinUI3 へ移す」なら、v0.1.0 には着せ替えのメニューだけ（6〜8・Opus・wintf に触らない）を残し、オーナードローの側は別の 1 本として後へ回す。
- 見つけた穴・古くなった記述: `popup-menu-residue` の件 11（メニューを開いている間の黒い四角）・件 12（メニューが見えずに閉じる）とどちらが先に直すかは未決のまま。

## 本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表。件の番号は表の番号）と、SSP の測りの記録 `ssp-measurement.md`（SSP 2.9.23・書庫の sha256 `ad473d5d…a43a6`＝`pasta.toml` 1.0.0）。撮った物と記録は git の外の `<w>\target\pasta-verify\`（本走行 run1。`logs\`・`shots\run1\`・SSP の撮った物は `ssp\`）・`<w>\target\pasta-verify-alt\`（補いの走り alt2。縦書きのバルーンを根の `balloon\` へ写した置き方）・`<w>\target\pasta-verify-rerun\`（直しの後の回し直し rerun1。alt2 と同じ置き方）に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。日本時間は 9 時間足す。
- 開発者の決め（2026-10-23）「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。この spec は v0.1.0 なので、下の件はこの spec の中で片づける。
- 着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md`（5 節の 7 段の表）で回す。
- **件 14（着せ替えの項目が無い）**: 右クリックメニューに「着せ替え」の項目が無く、着せ替えを選ぶ操作を回せなかった。項目はゴースト・シェル・バルーン・シェル位置初期化・ネットワーク更新・インストール…・置き場の 3 項目・説明書・終了で、シェルの下はパスタだけ。
  - 根拠: run1 の `2026-10-22T23:23:44.627931Z` `[menu] shown event="menu_shown" scope=0 items=13`（`logs\run1.stage6-excerpt.log`）・`shots\run1\6-menu.png`・`6-menu-shell-sub.png`。
  - 原因: `crates/areka/src/menu/mod.rs:49`。メニューの枠 `Frame` に着せ替えの枠が無い。シェルの `sakura.menu,auto` を読む所も無い。
- **件 15（シェルの `menu.*` を使わない普通のメニュー）**: 右クリックメニューが、シェルの `menu.*` の絵と色を使わない普通の Windows のメニューで出た。SSP もシェルの絵と色で「普通の Windows のメニューに見える」ので、見た目が違うかは開発者の判定がまだ無い。
  - 根拠: `shots\run1\6-menu.png`。SSP は `ssp\stage6-menu-shot.png`。
  - 原因: `crates/areka/src/menu/mod.rs:49` の枠のまま普通のメニューを組み、シェルの `menu.*` の 21 の鍵を読む所が無い（網羅台帳で absent）。
- **SSP の基準・測りの答え**（`ssp-measurement.md` 2 節の段 6・3 節の 8 行目）:
  - メニューはシェルの絵と色で、普通の Windows のメニューに見えた（`ssp\stage6-menu-shot.png`）。
  - 着せ替えのメニューに、口・目・眉・うで・まばたき・ボール・「閉じる - 赤」が並んだ（`ssp\stage6-dress-menu-shot.png`）。目 → びっくり を選ぶと面が変わり（`stage6-surface0-after-pick1.png` は起動したときの写しと、顔の範囲 66,216〜299,445 で 5742 画素違う）、同じ項目をもう一度選ぶと外れて戻った（`stage6-surface0-after-pick2.png`）。
  - ぱすたは `OnSecondChange` の応答で着せ替えの `\![bind,…]` を毎分返し直すので、選んだ着せ替えは長くは残らない。areka で比べるときは、選んだ直後に撮る。
  - 切り出した記録に着せ替えの通知（`OnDressupChanged`）の要求は無い。在るのはメニューを組むための `ID: sakura.recommendsites`（日本時間 08:09:52・08:10:59）。リソース `sakura.recommendsites`（台帳では vocabulary-only・宛先なし）が SSP と同じに見えるかは、件 15 と一緒に判定する（`verification.md` 6 節の「申し送りの漏れが無いことの確かめ」）。
- 段 6 でメニューが見えずに閉じる件（`popup-menu-residue` の件 12）は、run1 では起きなかった。
- 網羅台帳の宛先: 2026-10-23 に 22 行の `owner` をこの spec にした（`verification.md` 10 節の「直したことと残り」）。
- **着地で変わるはずの所**（手順書の段 6）: メニューの色と絵がシェルの指定どおりになり、着せ替えのメニューが出て、選ぶと着せ替えが替わる。

# Brief: popup-menu-shell-look

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

## 2026-10-31 `popup-menu-residue` からの写し（件 11・12 と見直す条件）

- 出どころ: `popup-menu-residue` の brief の 2026-10-16 追記（件 11）と 2026-10-18 追記（件 12）。C14 の約束（棚卸㉛）により、`popup-menu-residue` は件 11・12 を直さず、この brief へ渡す。引き受けるか・どちらが直すかは、この spec の要件の段で決める（ここでは決めない）。
- **件 11（メニューを開いている間、うにゅう側〔kero〕のバルーンの窓が黒い四角で塗られて見えることがある）**:
  - 見つけた所: `areka-P0-baseware-root-list` の実機確認（タスク 7.3・2026-10-16）。メニューを開いた直後に画面を撮ると、2 回とも kero のバルーンの位置が黒い四角だった（ゴースト emo2・バルーン emo2-kakukaku・debug ビルド・`TrackPopupMenuEx` の標準メニュー）。さくら側のバルーンは普通に描かれていた。記録は `.kiro/specs/completed/areka-P0-baseware-root-list/tasks.md` の Implementation Notes。
  - まだ分かっていないこと: メニューを閉じると元に戻るか・メニューを開いた瞬間に毎回なるか・release ビルドでもなるか・kero のバルーンが喋っている途中かどうかで変わるか。
  - 見立て（未確認）: 標準メニューのモーダルの間は UI スレッドのメッセージの回り方が変わる。その間に kero のバルーンの窓の中身（合成の面）が出し直されず、黒く見えている疑い。触る場所は `menu/trigger.rs`（メニューを開く所）と、窓の表示（wintf の合成）のどちらか。原因が wintf の合成の側と分かったら、その持ち主へ渡す。
  - 再現の測り方: 実機でメニューを開く前後の画面を撮り、記録と並べる。人の待ち（入力を送る許し）になりうる。
- **件 12（メニューが何も選ばれないまま閉じる・「出した」の記録はあるのに画面に見えないことがある）**:
  - 見つけた所: えみりの実機の 2 回目の走行（2026-10-17 23:56〜23:58・`.kiro/specs/completed/areka-P0-emily-ghost-verification/verification.md` の崩れた所 6）。メニューの「よんでね」でメモ帳を開いて閉じた後、道具のクリックで「終了」を選ぶと、メニューは何も選ばれずに閉じた。続く右クリック 3 回は `menu_shown` の記録が出るのに画面に見えず、対になる `menu_selected` も無かった。SendInput だけで出し直すと見えて、選べた。ぱすたの run1（2026-10-23）では起きなかった（上の申し送りの節）。
  - まだ分かっていないこと: 何が起きると見えなくなるか（メモ帳を閉じた直後・別のアプリが前に出た後のどれか）・見えずに閉じたのか、見えないまま開いているのか。
  - 見立て（未確定）: メニューの持ち主の窓が前面を取れないと、Win32 のメニューが見えずに閉じる。areka の窓は前面に出ない作りなので、`TrackPopupMenu` の前の前面の取り方（`SetForegroundWindow`）が効いていない疑い。
  - 記録の穴: 選ばれずに閉じたときの記録 `menu_dismissed`（`crates/areka/src/menu/trigger.rs` 502 行・`debug!`）は `info` の水準では見えない。このため、見えずに閉じたのか、見えないまま開いているのかを `info` の記録では区別できない（`popup-menu-residue` はコードを変えない）。
  - 再現の測り方: `RUST_LOG` に `areka::menu=debug` を足し、メモ帳を開いて閉じた後などに右クリックして、`menu_shown`・`menu_selected`・`menu_dismissed` の並びと画面を見比べる。実機でメニューを開いて撮る仕事なので、件 11 と同じく人の待ちになりうる。
- **網羅台帳の 3 行の見直す条件**: `sakura.popupmenu.type`・`kero.popupmenu.type`・`char*.popupmenu.type` の 3 行は、`popup-menu-residue` が「省略した形のメニューを作り分けない」と裁定した（正典は値 1 を「省略メニュー」と名前だけ挙げ、中身を定めていない）。この spec がオーナードローのメニューを入れたら（正典はこの値を「オーナードローメニューの変更」と書く）、3 行の裁定を見直す起票をする。

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
- **前提の変化**: 開発者の決め（2026-10-31・`popup-menu-extra-resources` について）「メニュー関係はreactor解放されたら大改造するので、内容によっては後回し」＝議題 1（右クリックメニューを後で WinUI3 へ移すか）に「移す」の向きの答えが出た。生きている決まり 6（オーナードローと着せ替えメニューは予約）を解くかの裁定そのものはまだ。✅ `popup-menu-residue`（10-31）が `menu/win32.rs`（137→151・組み立ての失敗の記録 `menu_build_failed`）・`menu/trigger.rs`（583）に触って着地し、件 11・12 をこの brief へ写した。✅ `popup-menu-extra-resources`（11-01）が `menu/captions.rs` と kanade の `schedule/resources.rs` に触って着地し、⑵（おすすめ・ポータルなどの項目）を windows-reactor 待ちの段へ渡した。その渡り物に「件 15（SSP がメニューを組むときに `sakura.recommendsites` を問う）との分け方（未決）」が載る（`completed/popup-menu-extra-resources/brief.md` の完了の棚卸の節）。✅ `script-small-tags` が seriko の着せ替えの受け口を `crates/areka-seriko/src/actor_bind.rs`（新・109）へ分けた（`actor.rs` 828・`bind.rs` 655）＝議題 3（着衣の写し）の読み先が動いた。`menu/mod.rs` 386（枠は 12 種のまま）・`menu/plan.rs` 139・wintf の `window_proc/mod.rs` 382・`tick_wake.rs` 375・`tech.md` 161 行目（「自前の窓もオーナードローも持たない」）は動いていない。
- **触るファイル**: 棚卸㉗の見立て＋`tick_wake.rs` に、着衣の読み先 `crates/areka-seriko/src/{actor_bind.rs,bind.rs}` を足したもの。
- **規模**: 14〜18 のまま（着せ替えのメニューだけなら 6〜8）。
- **先に要るもの**: 開発者の裁定（決まり 6 を解くか）。10-31 の決めに沿う案＝v0.1.0 には着せ替えのメニュー（件 14・wintf に触らない・6〜8・Opus）だけを残し、オーナードローの見た目（件 15）は windows-reactor 待ちの段へ送る。ただし着せ替えの項目を `menu/` に組む部分も作り直しで捨てる側に入る（作り直しで残るのは seriko の着衣の読みと、選んだときに着せ替えを替える道）＝どこまでを v0.1.0 でやるかを、残る部分と捨てる部分を分けて開発者に示して決める。同じウェーブの縛り: `popup-menu-residue`・`popup-menu-extra-resources` は着地して解けた。残る相手は `balloon-communicatebox`（入口をメニューに足すと決めたときだけ `menu/`）・後の版の `menu-thumbnail-preview`（`menu/{mod,win32,plan}.rs`・`window_proc/mod.rs`）・`tasktray-icon`（`menu/mod.rs`・windows-reactor 待ち）。1 本だけの席は取らない。人の待ち: メニューを開いて撮る実機の確かめ（排他テストの机）。
- **優先度の区分**: A のまま（開発者の指示＝v0.1.0 のゴール・申し送りの件 14・15）。
- **要件定義のモデル**: 着せ替えのメニューだけなら Opus（先例＝ゴースト・シェル・バルーンのサブメニュー）・オーナードローまでなら Fable（基準＝OS のメニューの中でその場で描いて答える口を wintf と areka の間に新しく引く・場所は `window_proc/mod.rs` と `menu/win32.rs`。✅ `shell-mouse-cursor` の `CursorQuery`／`plan_unborrowed` の形に乗ると設計で決まれば Opus もありうる）。
- **分割の案**: 前のまま（オーナードローの見た目／着せ替えのメニュー・共通は `menu/plan.rs`・`menu/mod.rs` だけ・wintf を触るのはオーナードローの側だけ）。上の案なら、オーナードローの側は windows-reactor 待ちの段の残りにする。
- **見つけた穴・古くなった記述**: 件 11（黒い四角）・件 12（見えずに閉じる）とどちらを先に直すかは未決のまま。本文 Current State の「枠 11 種」は 12 種。申し送りの `sakura.recommendsites` の判定（件 15 と一緒に判定する）は、`popup-menu-extra-resources` の ⑵ の渡り物と二重に持たれている＝件 15 を後へ送るなら、一緒に windows-reactor 待ちの段へ寄せる。

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者「popup-menu-shell-lookもだいぶ怪しい。そもそもオーナードローメニューはやらないと裁定したし、メニューで凝ったことをしたって、リアクターが来れば作り直し」＝v0.1.0 から windows-reactor 待ちの段へ移した。生きている決まり 6（オーナードローと着せ替えメニューは予約のまま）は解かない。作り直しの後に、着せ替えのメニュー（ぱすたの申し送りの 14）で要るところだけを測り直す。

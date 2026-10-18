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

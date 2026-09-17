# Brief: areka-P0-popup-menu-minimal

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。開発者の指示「オーナードローは不要ですが、最低限のメニューも必要」。`doc/ukadoc-coverage/roadmap-draft.md` 段階 A 順位 11 の束「メニュー」（候補名 `areka-P0-ownerdraw-menu-canon`）のうち、**見た目（オーナードロー）を除いた「項目と操作」**だけを引き受ける。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: 第三者の利用者。ゴーストを替える・バルーンを替える・更新する・終了する、という**利用者の操作の入口が画面に無い**。

今日の右クリックは**ダブルクリックとしてしか SHIORI に届かない**（`crates/areka/src/input_events/mod.rs:444-450`＝`MouseEventKind::DoubleClick{Left|Right}` → `OnMouseDoubleClick`）。シングルクリックは意図して送っていない（`input_events/mod.rs:396`）。オーナードローメニューは「実装しない（7.4）」と明記されている（`input_events/mod.rs:398`）。`TrackPopupMenu`／`CreatePopupMenu`／`HMENU`／`WM_CONTEXTMENU` は `crates/` に 0 件。

M1 の E2E で「メニュー」と呼んでいたものは**ゴースト側が用意した選択肢バルーン**である（`OnMouseDoubleClick` → `MAIN_MENU_TALK`・`crates/areka/src/emo2_boot/spine_conformance_script.rs:540`・辞書 `fixtures/emo2/ghost/master/dic/menu.pasta`）。里々や YAYA の第三者ゴーストは、ベースウェアの右クリックメニューがあることを前提に書かれており、ゴースト側のメニューを持たないものが多い。

終了は Ctrl＋左ダブルクリック（`input_events/mod.rs:420-441`）で、第三者には見つけられない。

## Current State

- 窓は wintf の Win32 窓（`crates/wintf/src/api.rs:375` の `CreateWindowExW`）。`Win32_UI_WindowsAndMessaging` は有効（ルート `Cargo.toml` の `windows` 機能一覧）＝`CreatePopupMenu`／`AppendMenuW`／`TrackPopupMenuEx` は**依存追加 0** で呼べる。
- 右クリックの入力はポインタ経路（`wintf/src/ecs/pointer`）で捕まえている。メニューを出す場所は「右ボタンが上がった座標」。
- メニュー項目の名前は、正典では SHIORI リソースで差し替えられる（`ghostrootbutton.caption`・`shellrootbutton.caption`・`balloonrootbutton.caption`・`updatebutton.caption`・`readmebutton.caption`・`closebutton.caption`・`quitbutton.caption`・`ghostinstallbutton.caption` 等。束「メニュー」の `members` に列挙）。`popupmenu.visible`／`popupmenu.type` で表示と種類を変えられる。
- メニューを出したことを SHIORI に知らせるイベントは正典に無い（`OnMenuExec` は PLUGIN 側）。閉じる操作は `OnClose`（Ref0＝`user`・Ref1／Ref2＝スコープ番号）。

## Desired Outcome

完了時に次が真になっている。

1. **キャラクター窓の右クリックで Win32 標準のポップアップメニューが出る。** 項目（α の最小）:
   - ゴースト（サブメニュー＝`baseware-root-layout` の列挙・現在にチェック）
   - シェル（サブメニュー＝現ゴーストのシェル・`menu,hidden` を除く・現在にチェック）
   - バルーン（サブメニュー＝列挙・現在にチェック）
   - ネットワーク更新（`network-update` が項目を登記するまでは出ない）
   - インストール…（ファイル選択ダイアログ・`ghost-install` が登記するまでは出ない）
   - 説明書（ゴーストの `readme` を既定のアプリで開く・`ShellExecuteW`）
   - 終了（`OnClose` の握手＝既存の Ctrl＋左ダブルクリックと同じ経路）
2. **項目名は SHIORI リソースで差し替わる。** 上の 7 項目に対応する `*button.caption` を `GET` して空でなければその名を使う。`popupmenu.visible` が 0 なら出さない。`popupmenu.type` の省略メニュー（1）は α では本体側と同じ扱い（差を付けない・`warn!` なし＝正典が「省略」の中身を定めていない）。
3. **右クリック（シングル）も SHIORI に届く。** SSP は右クリックでメニューを出しつつ `OnMouseClick`（Ref5＝1）も送る。α では「メニューを出す」を優先し、`OnMouseClick` は送らない現状を維持する（既存の裁定 `input_events/mod.rs:396`）。裁定候補として登記（下記）。
4. **項目は登記式。** `ghost-install`・`network-update` が後から自分の項目を足せるよう、メニューは「項目の一覧を受け取って組み立てる」形（薄い拡張シーム。項目＝ラベル・有効／無効・選ばれたときの `MenuAction`）。
5. **メニューの操作は台本の操作と同じ経路を通る。** ゴースト／シェル／バルーンの選択は `ghost-shell-balloon-switch` の `SwitchRequest` を送る。終了は既存の `CloseRequest{User}`。

## Approach

**選んだ形**: Win32 標準メニュー（`HMENU`）を wintf の窓に 1 枚。描画は OS に任せる（DPI・フォント・高コントラストは OS が面倒を見る＝オーナードローの理由がまるごと消える）。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 出す | 右ボタン up でメニューを組み立てて `TrackPopupMenuEx`（`TPM_RETURNCMD`）。返った ID → `MenuAction` | 実窓: メニューが出て「終了」が握手を始める（既存の終了テストの入口を 1 つ増やす）。決定論: 項目一覧 → メニュー構造（ID と親子）の純関数 |
| ② 項目 | 列挙 3 種のサブメニュー・説明書・終了。現在の選択にチェック | 決定論: 列挙の fixture から構造を作り、チェック位置と `menu,hidden` の除外を突き合わせる |
| ③ 名前 | `*button.caption` の `GET`（空／204 は既定名）。`popupmenu.visible` | 決定論: 偽 SHIORI が返す caption で項目名が変わる／`visible=0` で出ない |
| ④ 登記 | `MenuRegistry` に項目を足す口。`ghost-install`／`network-update` が使う | 決定論: 登記した項目が正しい位置に現れる |

**取らない形**: オーナードロー（`menu_background.png`・`menu.font.*`・`menu_sidebar.png`）。M2 予約のまま（roadmap「M2 以降」）。項目の**位置**は SSP の並びを写さない（正典は並びを定めていない）。

**説明書を開く**: `readme.txt`（または `readme.md`）を `ShellExecuteW` で既定アプリに渡す。areka の中で表示する窓は作らない（束「作り付けの窓」の `\![open,readme]` は同じ関数を呼ぶ S の追加として本仕様に含める）。

## Scope

- **In**:
  - 右クリック → Win32 ポップアップメニュー（`Win32_UI_WindowsAndMessaging`＝依存追加 0）
  - 項目 7 種（ゴースト・シェル・バルーン・ネットワーク更新・インストール・説明書・終了）と登記式の `MenuRegistry`
  - `*button.caption` リソースによる項目名・`popupmenu.visible`
  - 現在の選択のチェック・`menu,hidden` の除外
  - `\![open,readme]`（説明書を開く関数の台本側の入口・S）
  - 終了項目＝既存の握手へ
- **Out**:
  - オーナードロー（見た目の全て）・`menu.*` の descript キー・`menu_*.png`
  - 着せ替えメニュー（`sakura.menuitem*`・`bindgroup*.name`＝束「メニュー」の着せ替え側・α 後）
  - 設定ダイアログ・バージョン情報・使用率グラフ・ポータル・おすすめ（`*rootbutton.caption` の残り・α 後）
  - トレイアイコン（`Shell_NotifyIcon`・α 後の裁定候補）
  - `OnMouseClick`（シングルクリック）の送出（裁定候補）

## Boundary Candidates

- **入力**（右ボタン up をメニューの引き金にする。ポインタ経路の 1 分岐）
- **構造**（項目一覧 → HMENU の純関数・ID の払い出し）
- **名前**（SHIORI リソース `GET` の薄い読み手）
- **操作**（`MenuAction` → `SwitchRequest`／`CloseRequest`／登記した閉包）

## Out of Boundary

- 切替・インストール・更新の実行（それぞれの spec）
- 列挙の実体（`baseware-root-layout`）

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-layout`（列挙）／`areka-P0-ghost-shell-balloon-switch`（`SwitchRequest`）／完了仕様 `areka-P0-input-events`（ポインタ経路）／完了仕様 `areka-P0-collision-dpi-hittest`（右クリック座標の窓相対化）。
- **Downstream**: `ghost-install`（「インストール…」の登記）・`network-update`（「ネットワーク更新」の登記）・`alpha-release-signoff`。α 後: オーナードロー（M2 予約）は本仕様の構造の上に見た目を被せる。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-choice-marker-styling`／`anchor-tag-canon`（α 後・選択肢バルーン）——ゴースト側の「メニュー」（選択肢）とは別物。共有ファイル 0。
  - `areka-P0-property-catalog-lists`（α 後）——束「メニュー」の依存先として 4 件を持つ。本仕様はプロパティを出さない。

## Constraints

- 新規の外部依存 0。
- メニューは UI スレッドで出す（render/window は UI スレッド固定＝記憶 areka-concurrency-model）。`TrackPopupMenuEx` はモーダルにメッセージを回すので、その間も SHIORI アクターの死活監視（`host32-window-thread-pump`）が止まらないことを確かめる。
- 既定の IME 窓の罠（記憶 windows-default-ime-window-sits-above-owner）＝メニューの親は所有者鎖の実窓にする。
- 1 ファイル 1,000 行。`input_events/mod.rs` の現在行数を測ってから足す（新規ファイル `menu.rs` に置く）。
- **裁定候補 ⑶（開発者）**: 右クリックで `OnMouseClick`（Ref5＝1）を SHIORI にも送るか。SSP は送る。送ると里々の標準テンプレートが「右クリック」の台詞を返し、メニューと同時に喋る。α は**送らない**（現状維持）を推し、要望が出たら足す。
- **裁定候補 ⑷（開発者）**: トレイアイコン（`Shell_NotifyIcon`）を α に含めるか。キャラクター窓が画面外へ行ったときの復帰手段として役立つが、`windowposition.limit`（完了仕様 `windowposition-limit`）で画面内へ戻る構造が既にある。α は**含めない**を推す。

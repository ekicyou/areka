# Brief: play-package

> 決まりの正本: `doc/ideas/play-layer-folders-2026-11-28.md`（1 章・2.1〜2.3・2.6・2.8）。

## Problem

- **劇の作者**: ノベルゲーム（劇）を、ゴーストと同じ形の入れ物として配り、areka で遊ばせたい。いまの areka には、劇を入れる種別も、劇の置き場も、劇を始める入口も無い。
- **遊ぶ人**: デスクトップのゴーストを動かしたまま、劇を専用の窓で遊びたい。

## Current State

- 書庫の種別は `ghost`・`shell`・`supplement`・`balloon` の 4 つだけを受け、ほかは断る（`crates/areka-nar/src/manifest.rs` の `parse_kind`）。
- 根の最上位は `ghost/`・`balloon/`。根は並びで何本でも登録できる（完了 `areka-P0-baseware-root-list`）。
- 1 つのプロセスで動くゴーストは 1 体（`multi-ghost-runtime` が何体も動かす土台を作る）。
- 起動の入口は、起動の引数・右クリックのメニューの「ゴースト」・台本の切り替えのタグ。

## Desired Outcome

- play 層は ghost 層の上位で、ukadoc に無い areka の拡張である。劇は、設計の上でゴーストと同じ入れ物で、違いは「既定がウィンドウモード」と「配下にゴーストを持てる」の 2 つだけである。
- `type,play` の書庫は `play/<directory>/` に入る。SSP には断られてよい。
- 劇の中は `master/`・`ghost/`・`shell/`・`balloon/`。master は `master-folder-resolve` の関数で解く。
- 配下の `ghost/` は、そのゴーストのシェルとバルーンを劇と一緒に配る置き場で、配下のゴーストの SHIORI は起こさない。台詞は全部、劇の SHIORI が書く。主の流れに入る台本は送り手を持つ形のまま扱い、「劇の SHIORI 1 つだけ」と決め打ちしない（配下の SHIORI を後から足せるように）。
- 劇は専用の窓で始まり、デスクトップのゴーストと並んで動く。劇を始めても、デスクトップのゴーストは降ろさない。
- 別の劇なら同時に何本でも開ける。同じ劇は 1 つだけで、2 つ目を開こうとしたら、開いている窓を前に出す。
- 劇の窓を閉じたら、劇の SHIORI へ正典の `OnClose`（Reference0＝`user`）を送り、終わりの台本を流してから降ろす。右クリックのメニューの「終了」と同じ道。
- 劇も「1 体」に数える。最後の 1 体（ゴーストでも劇でも）が止まったとき・「すべて終了」・OS のシャットダウンで、`OnCloseAll` → `OnClose` で全員を止め、アプリも終わる。
- 劇を始める入口は 3 つ: 右クリックのメニューの「劇」の一覧・`type,play` の書庫を入れ終えた直後・起動の引数（劇の根を渡す）。
- 劇の記憶は `play/<名>/master/profile/areka/`。

## Approach

ゴーストの入れ物と同じ部品を使い、劇だけの違い（種別・置き場・配下・既定の窓・入口）を足す。何体も動かす土台は `multi-ghost-runtime`、窓と窓の中の演出は `shell-multi-compose`（ghost 層の働き）に任せる。

## Scope

- **In**: `type,play` のインストール・`play/` の目録・劇の master と配下の `ghost/` の読み・既定がウィンドウモードで始まること・同じ劇を 1 つに限ること・窓を閉じたときの終わり方・劇を「1 体」に数えること・入口 3 つ・決定論テスト。
- **Out**: ゴーストの台本から劇を開く入口（`play-open-directive`）・全画面の許可（`fullscreen-consent`）・窓の中の演出とウィンドウモードの出入り（`shell-multi-compose`）・素材の探し方と識別子（`ghost-material-metadata`）・命令の名前と形（pasta の `novel-areka-contract`）・配下のゴーストの SHIORI を起こすこと。

## Boundary Candidates

- 劇の書庫と置き場（インストール・目録）。
- 劇の起動と終わり（入口・同じ劇は 1 つ・窓を閉じたとき・アプリの終わり方への参加）。

## Out of Boundary

- 何体も動かす土台そのもの（`multi-ghost-runtime`）。
- 劇の中の素材を借りる決まり（`ghost-material-metadata`）。

## Upstream / Downstream

- **Upstream**: `master-folder-resolve`・`multi-ghost-runtime`・`ghost-material-metadata`・`shell-multi-compose`（ウィンドウモード）。
- **Downstream**: `play-open-directive`・`fullscreen-consent`（劇の設定の置き場）・`areka-usage-stats`（演劇の信号）・pasta の劇。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `install-package-type`（`type,package`・書庫の種別を読む所が同じ）・`multi-ghost-runtime`・`ghost-call`。

## Constraints

- 既定の動作を変えない（ゴーストの起動・切り替え・終了は今と同じ）。
- メッセージボックスは使わない。
- 区分 A・ノベルゲーム基盤の段。

> **pasta の取り決めとのすり合わせの決め**（正本は `doc/ideas/play-layer-folders-2026-11-28.md` の 2.10）: areka 向けの劇は `type,play` の `.nar`、SSP でも動かす台本は `type,ghost` の `.nar` で配る。`type,ghost` で配ったものも、areka では命令でウィンドウモードに入れば同じ演出が使える。利用者の設定（窓の形・全画面の許可・役者の差し替え）は areka が持ち、物語の状態（セーブデータ）は劇の SHIORI が持つ。

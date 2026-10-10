# Brief: areka-P0-dev-folder-alias

> 2026-10-05 `/kiro-discovery` で起票（開発者「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利。開発中は git リポジトリを適当な場所に展開して開発用ゴーストをいじることが多いので」）。「ゴーストフォルダの複数管理」の 2 本の 2 本目（roadmap の同名の節が分け方の正本）。`baseware-root-list` が作る「上から探す」関数の上に乗る。

## Problem

- **利用者**: ゴースト・シェル・バルーンの作者。開発中の作品を git のリポジトリとして好きな場所に置いて作業する。
- 今は、開発中のゴーストを areka で動かす道が 2 つしかない。
  - 根の下へ写す。写すたびに作業ツリーとずれる。
  - 起動の引数にパスを渡す。この場合は「目録の外」の扱いで、メニューにも切替先にも `LastGhost` にも載らない（`doc/COMPAT_ARCHITECTURE.md` の目録の外の行）。
- シェルとバルーンには、引数で渡す道すら無い（シェルはゴーストのフォルダの下でしか探さない）。

## Current State

- `baseware-root-list` の着地の後は、根の並びと「フォルダ名 → 実際の場所」を引く関数が 1 つある前提。今の呼び手はその関数を通る。
- シェルの場所: `list_shells(ghost_dir)`（`crates/areka-ghost/src/catalog.rs`）が `<ゴースト>/shell/<名>/` を走査する。切替・メニュー（`menu/shell_frame.rs`）・`LastShell` の復元もゴーストのフォルダ基準。
- 記憶の書き込み: areka 自身の記憶は、ゴーストのフォルダの中（`ghost/master/profile/areka/`・`sylphya_wiring.rs` の `profile_areka_root`）とシェルのフォルダの中（`<shell>/profile/areka/`）に書く。
- インストール（`install/`）とネットワーク更新（`update/desk.rs`）は、見つかったフォルダへ書き込む。

## Desired Outcome

1. **名前 → フォルダの表を持てる。**
   - ゴースト: 「フォルダ名 → フォルダ」。例: `emo2 = D:\repos\emo2`。
   - バルーン: 「フォルダ名 → フォルダ」。
   - シェル: 「ゴーストのフォルダ名＋シェル名 → フォルダ」。シェルはゴーストの下で探すものなので、ゴーストごとに持つ。
   - 名前は、今フォルダ名が担っている役（`LastGhost`・切替・メニューの印）をそのまま担う。descript の `id` は使わない。
2. **探す順のいちばん上。** 探す順は「エイリアス → 根の並びの上から」。エイリアスと同じ名前のものが根にあれば、根の方は隠れる。だから、入っている `emo2` の代わりに開発中の `emo2` を同じ名前で動かせる。記憶・メニュー・`\![change,ghost,…]` もそのまま効く。
3. **エイリアスの先には書き込まない**（作業ツリーを守る）。
   - インストールの宛先にしない。同じ名前の `.nar` が来たら根の方（`baseware-root-list` の決め方）へ入れる。入れたものはエイリアスに隠れる。
   - ネットワーク更新はエイリアスのゴーストでは行わない。行わなかったことを記録に残し、ゴーストへ失敗を知らせる。知らせ方は ukadoc の更新の失敗イベントから要件の段で選ぶ。
   - ただし areka 自身の記憶（`profile/areka/`）は今どおりフォルダの中に書く。作業ツリーには `profile/` の `.gitignore` が要る（SSP の `profile/` と同じ扱い）。このことを説明書に書く。
4. **登録は手で書く。** メニューは作らない（開発者向けの機能）。
   - 表は、根の並びと同じ App の記憶（`sylphya.toml`）に置く。
   - 環境変数（`AREKA_` の名前空間・名前は設計で決める）で表を上書きできるようにする。
5. **無いときに黙らない。** 登録したフォルダが無い・形が違う（ゴーストなら `ghost/master/descript.txt` が無い等）ときは、記録を残してそのエイリアスを飛ばす。根の方に同じ名前があれば、そちらが見える。

## Approach

- `baseware-root-list` の「フォルダ名 → 実際の場所」を引く関数の手前に、エイリアスの表を引く段を 1 枚足す。列挙の和にもエイリアスを先頭の層として混ぜる。
- 各項目は「エイリアスから来た」ことを持つ。インストール（`install/`）と更新（`update/desk.rs`）は、それを見て書き込みを断る。
- シェルの探し方（`list_shells` とその呼び手）に、ゴーストごとのシェルのエイリアスを足す。

## Scope

- **In**:
  - ゴースト・バルーン・シェルのエイリアスの表（App の記憶・環境変数の上書き）。
  - 探す順の先頭の段と、和の列挙への混ぜ込み。
  - シェルの探し方の拡張。
  - インストール・更新で書き込みを断る分岐と、その記録と知らせ。
  - 説明書（`dist/README.txt`）の書き方と `.gitignore` の注意。
  - 決定論テスト（隠れる・断る・無いフォルダを飛ばす）。
- **Out**:
  - 根の並び（`baseware-root-list`）。
  - メニューからの登録・設定画面。
  - 作業ツリーの変更を見張って、自動で読み直すこと。今の読み直しの道（切替・`\![reload,…]`）を使う。
  - areka 自身の記憶をフォルダの外へ逃がすこと。

## Boundary Candidates

- エイリアスの表（保存と環境変数）。
- 探す関数の先頭の段（純粋な判断）。
- 書き込みを断る分岐（インストール・更新）。
- シェルの探し方。

## Out of Boundary

- 根の並びの解決・メニューの根の 2 項目（`baseware-root-list`）。
- ネットワーク更新そのものの手順（`network-update-canon-order`・`update-check-options`）。

## Upstream / Downstream

- **Upstream**: `baseware-root-list`（探す関数・和の列挙・インストール先の決め方）。
- **Downstream**: なし。`mcp-reload` が先に入っていれば、エイリアスのゴーストの読み直しにも使える。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `emo2_boot` の結線の列（`ghost_switch.rs`・`shell_balloon_switch.rs`）。
  - `network-update-canon-order`・`update-check-options`（`update/`）。
  - `install-live-target-hazards`（`install/`）。

## Constraints

- 鍵（名前）は今のフォルダ名と同じ扱いにする。新しい見分けの鍵は作らない。
- 段: その他。規模の見込み M（8〜12）。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 前提の `baseware-root-list` は未着手のまま＝本 spec は待ち。10-05 の後に変わった点は 2 つ。
  - シェルの列挙に、メニューに出さないシェルも数える `list_all_shells` が在る（`crates/areka-ghost/src/catalog.rs`。中身は `scan_shells` の 1 か所）＝シェルのエイリアスはこの 1 か所に足せば両方に効く。
  - 名前でフォルダを引く呼び手に `\![open,explorer,…]`（`crates/areka/src/readme/opener.rs` の `named_folder`）が増えた。エイリアスの先のフォルダを開く動きになる（書き込みではないので断らない）。
- 触るファイル: `baseware-root-list` が作る「フォルダ名 → 場所」の関数（`catalog.rs`）・`crates/areka/src/boot_config.rs`（環境変数）・`install/`・`update/desk.rs`（書き込みを断る分岐）・`menu/shell_frame.rs`・`crates/areka-sylphya/src/persist/`・`dist/README.txt`。正確な一覧は `baseware-root-list` の着地の後に引き直す。
- 規模: 8〜12 タスク。
- 先に要るもの: `baseware-root-list`（働きの依存）。
- 優先度の区分: A（開発者「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利」）。
- 要件定義のモデル: Opus（決めごとは起票のときに済んでいる。残る議題は、更新を断ったときに送るイベントを ukadoc から選ぶ 1 件）。
- 分割の案: なし。
- 見つけた穴・古くなった記述: なし。

# Brief: dev-folder-alias

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

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**: 棚卸㉙の後に `readonly-mode`（10-24）と `default-balloon-selfmade` が着地し、v0.1.0 で重なる相手は `popup-menu-shell-look`（`package/resolve_shell.rs`・裁定待ち）だけになった。`readonly-mode` は、更新を断ってゴーストへ知らせる形を `crates/areka/src/update/readonly_refusal.rs` に入れた（成否の欄が空の `OnUpdateResultEx` → `OnUpdateResult`）＝エイリアスのゴーストの更新を断るときの知らせ方の前例。要件の議題「ukadoc の更新の失敗イベントから選ぶ」は、この形に寄せるかの 1 件に縮む。`install-before-boot`（10-31）の起動前のインストールも `install/destination.rs` の `install_into` を通る＝書き込みを断る分岐をそこに置けば起動前の道にも効く。
- **触るファイル**: `crates/areka-ghost/src/catalog_roots.rs`（307 行・`union` 223 行目・`balloons_for` 156 行目）・`catalog.rs`（487・`RootMark` 34 行目・`scan_shells` 193 行目）・`crates/areka/src/roots/mod.rs`（54）と新しい読み手・`crates/areka-sylphya/src/persist/{mod,format}.rs`（496・595）・`crates/areka/src/install/destination.rs`（215・`running_root` 68 行目）・`update/desk.rs`（706）か `update/readonly_refusal.rs` の隣・シェルの場所を組む 3 か所（`scan_shells`・`crates/areka/src/boot_resolve.rs` の `decide_boot_shell` 416 行目・`crates/areka-parsers/src/package/resolve_shell.rs` 29 行目の `SHELL_ROOT`）・`Temporary` の守りの 5 か所（印を並べるとき）・`dist/README.txt`（自分の行）。1,000 行に近いものは無い。
- **規模**: 10〜14 タスクのまま。
- **先に要るもの**: 働きの上では無し（置き場の列で前に居た `user-data-root` は着地済み＝列の先頭）。人の手・静かな机・前後の全体テストの比べ・依存の追加・emo-text の新しいファイルは要らない。
- **重なり**: `shell-scale-linkage` とは 0。v0.1.0 では `popup-menu-shell-look`（`resolve_shell.rs`）だけ。版に縛らない列では `config-parse-diagnostics`（`catalog.rs`・`package/`）。余力の枠の `multi-ghost-runtime` とは `persist/`・`roots/`・`boot_resolve.rs`。
- **版の席**: 見立ての上限 14 が空き席の上限 10 を超える＝v0.1.0 の間は隙間に入れない。
- **優先度の区分**: A（開発者「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利」）。
- **要件定義のモデル**: Opus（今ある口の手前に段を 1 枚足すだけ・隠す形〔`balloons_for` の `Hidden`〕・書かない守り〔`Temporary`〕・更新を断る形〔`readonly_refusal.rs`〕の前例が在る）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - バルーンを探す順が 3 段（エイリアス・ゴーストの中・置き場の並び）になり、エイリアスとゴーストの中のどちらを上にするかが決まっていない＝要件の議題（棚卸㉙から持ち越し。`balloons_for` は置き場の和 `list_balloons` を呼ぶので、和の先頭に混ぜると「ゴーストの中が上」になる）。
  - 本文 Problem の 2 つ目（起動の引数のパスは目録の外）は古い（`--ghost <パス>` は親を `RootMark::Temporary` の置き場として先頭に足す・棚卸㉘）。本文 15 行目の「着地の後は…前提」は今は事実。

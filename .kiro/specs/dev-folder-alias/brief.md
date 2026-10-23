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

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）
- 前提の変化: 先に要る `baseware-root-list` が 10-16 に着地した（PR#317）＝待ちは解けた。
  - 「フォルダ名 → 場所」を引く口は `crates/areka-ghost/src/catalog_roots.rs` に集まった（`list_ghosts`・`list_balloons`・`find_ghost`・`find_balloon`・`install_root`・`survey`。どれも同じファイルの中の和を組む関数 `union` を通る）。エイリアスの段は `union` の手前に 1 枚足せば、メニュー・切替・`\![open,…]`（`crates/areka/src/readme/opener.rs` の `named_folder`）に全部効く。
  - 一覧の各項目は自分の根を持つ（`catalog.rs` の `GhostEntry.root`・`BalloonEntry.root`）。シェルの項目（`ShellEntry`）は持たない。
  - 根を決める環境変数と記憶を読む口は `crates/areka/src/roots/mod.rs` だけ。記憶の鍵の前例は `crates/areka-sylphya/src/persist/{mod,format}.rs` の `BasewareRoots` の行。
- 気を付ける所: 起動中のゴーストへ入れ直すときの宛先は、ゴーストの場所の親の親を根とみなして組む（`install/destination.rs` の `running_root` 41〜63 行と `install_into` 135〜140 行）。エイリアスの先ではこれが成り立たないので、書き込みを断る分岐はこれより前に置く。
- シェルの場所を自分で組む所は 3 つ: `catalog.rs` の `scan_shells`（150 行）・起動のシェルを決める `crates/areka/src/boot_resolve.rs` の `decide_boot_shell`（364〜382 行）・ゴーストを載せる読み手 `crates/areka-parsers/src/package/resolve_shell.rs`（28 行の `SHELL_ROOT`）。シェルのエイリアスはこの 3 つに要る。
- 触るファイル: `catalog_roots.rs`・`catalog.rs`・`roots/mod.rs` と新しい読み手・`persist/{mod,format}.rs`・`install/destination.rs`・`update/desk.rs`（書き込みを断る）・上のシェルの 3 か所・`dist/README.txt`（自分の行）。棚卸㉓の「`boot_config.rs`（環境変数）」は古い。`menu/shell_frame.rs` は一覧を受け取るだけなら触らない。
- 規模: 10〜14 タスク（シェルの場所の 3 か所が見えた分）。分割の案: 切らない。
- 先に要るもの: 働きの上では無い＝先頭。
- 重なり:
  - 今の版: `user-data-root` と `roots/{mod,resolve}.rs`・説明書の「■ 置き場」で重なりうる＝同じウェーブに置かない。`write-failure-notice` とは `boot_resolve.rs` だけで、関数が別（向こうは 391〜579 行の印と記録、こちらは `decide_boot_shell`）。
  - ほか: `config-parse-diagnostics`（記録を出す所を `package/` に選んだとき）・`property-name-case-fold`（`persist/` を畳むとき）・`install-live-target-hazards`（`install/`）・`network-update-canon-order`・`update-check-options`（`update/`）。
- 版の席: 版に縛らない列。`user-data-root` とはファイルが重なり、`write-failure-notice` だけのウェーブでは見立ての上限 14 が向こうの下限 7 を超える＝今の版の間は隙間に入らない。
- 人の手: 要らない（作業ツリーを `target\` の下に置いて確かめられる）。静かな机も全体テストの比べも要らない。依存も足さず、emo-text にも触らない。
- 優先度の区分: A（開発者「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利」）。要件定義のモデル: Opus（`baseware-root-list` の口の上に段を 1 枚足すだけ）。
- 古くなった記述: 本文 15 行目の「着地の後は…前提」は今は事実。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化（`startup-options-core`・10-19 着地）: `--ghost <フォルダのパス>` で起こすと、そのフォルダの親を「この起動のあいだだけの置き場」として置き場の並びの先頭に足すようになった（`crates/areka/src/boot_config.rs` 256〜279 行目・印は `crates/areka-ghost/src/catalog.rs` 33 行目の `RootMark::Temporary`）。先頭なので、上から探す決まりで同じ名前の入ったゴーストより先に見つかる。一方で、置き場の並びの記憶へ書かない・インストールの宛先にしない守りが 5 か所に入った（`roots/edit.rs`・`menu/roots_frame.rs`・`roots/resolve.rs`・`catalog_roots.rs` の `install_root`・`install/destination.rs`）。
- これで本文 Problem の 2 つ目（「起動の引数にパスを渡すと目録の外」）は古い。ゴーストについては「1 回の起動のあいだだけ・親のフォルダごと」の形で近いことが既にできる。本 spec に残る値打ちは ⑴ 名前で覚えて起動のたびに効く ⑵ シェルとバルーンにも効く ⑶ 更新を断る、の 3 つ。エイリアスの印は `Temporary` と同じ守りの 5 か所に足す形が先例になる（要件の段で、`Temporary` と並ぶ印にするかを決める）。
- 触るファイル: `crates/areka-ghost/src/catalog_roots.rs`（270 行・和を組む `union` は 189 行目）・`catalog.rs`（475 行・`scan_shells` 192 行目）・`crates/areka/src/roots/mod.rs`（54 行）と新しい読み手・`crates/areka-sylphya/src/persist/{mod,format}.rs`（451・566 行）・`crates/areka/src/install/destination.rs`（215 行・`running_root` 68 行目）・`update/desk.rs`（704 行・書き込みを断る）・シェルの場所を組む 3 か所（`catalog.rs` の `scan_shells`・`boot_resolve.rs` の `decide_boot_shell` 400 行目・`crates/areka-parsers/src/package/resolve_shell.rs` 28 行目の `SHELL_ROOT`）・印を足すなら `Temporary` の守りの 5 か所・`dist/README.txt`（自分の行）。
- 規模: 10〜14 タスクのまま（守りの先例がある分、下の側に寄る見込み）。
- 先に要るもの: 働きの上では無い＝先頭。人の手・静かな机・全体テストの比べは要らない。依存は足さない・emo-text に触らない。
- v0.1.0 の spec との重なり（隙間に入れられるか）: 版の段の v0.1.0 の行の 23 本のうち 5 本と重なる＝`ghost-inner-balloon`（`catalog.rs`・`catalog_roots.rs`・`boot_resolve.rs`・`update/desk.rs`）・`readonly-mode`（`roots/`・`persist/`・`update/desk.rs`・`boot_resolve.rs`）・`default-balloon-selfmade`（`catalog.rs`・`boot_resolve.rs`）・`property-name-case-fold`（`persist/` を畳む案のとき）・`popup-menu-shell-look`（`resolve_shell.rs`）。余力の枠の `multi-ghost-runtime` とも `persist/mod.rs`・`boot_resolve.rs` で重なる。見立ての上限 14 も v0.1.0 のたいていの spec の下限を超える＝**v0.1.0 の間は隙間に入れない**のが既定（入れるなら、この 5 本がどれも同じウェーブに居ず、そのウェーブで一番大きい v0.1.0 の spec の下限が 14 以上のときだけ）。
- 優先度の区分: A のまま。
- 要件定義のモデル: Opus のまま（今ある口の手前に段を 1 枚・書かない守りの先例も在る）。
- 分割の案: なし。
- 見つけた穴・古くなった記述: 本文 Problem の 2 つ目は上のとおり古い。棚卸㉖の節の `decide_boot_shell`（364〜382 行）は 400 行目へ、`running_root`（41〜63 行）は 68 行目へ動いた。見直しの spec（v0.1.0 の後）がこれらのファイルを 1 本ずつ受け持つ＝v0.1.0 の後は、見直しとの順も棚卸で照らす。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: `ghost-inner-balloon`（10-23 着地）が「ゴーストのフォルダの中のバルーンが、同じフォルダ名の置き場のバルーンを隠す」形を入れた（`crates/areka-ghost/src/catalog_roots.rs` の `balloons_for` と、隠した項目の記録 `Hidden`・`GhostBalloons`）。名前が同じものを上の段が隠し、隠したことを値で返す前例が 1 つ増えた＝エイリアスの「根の方は隠れる」はこの形に寄せられる。同じ spec がゴーストの読み手 `crates/areka-parsers/src/package/` を割った（`resolve.rs` から `resolve_bind.rs` を出し、`resolve_shell.rs` も 71 行動いた）。`property-name-case-fold`（10-23 着地）は保存の `persist/` に触らなかった。
- **触るファイル**: `crates/areka-ghost/src/catalog_roots.rs`（307 行・和を組む `union` は 223 行目）・`catalog.rs`（487・印 `RootMark` は 34 行目・`scan_shells` は 193 行目）・`crates/areka/src/roots/mod.rs`（54）と新しい読み手・`crates/areka-sylphya/src/persist/{mod,format}.rs`（451・566）・`crates/areka/src/install/destination.rs`（215・`running_root` は 68 行目）・`update/desk.rs`（706）・シェルの場所を組む 3 か所（`scan_shells`・`crates/areka/src/boot_resolve.rs` の `decide_boot_shell` 400 行目・`package/resolve_shell.rs` 29 行目の `SHELL_ROOT`）・`Temporary` の守りの 5 か所（印を並べるとき）・`dist/README.txt`（自分の行）。1,000 行に近いものは無い。
- **規模**: 10〜14 タスクのまま。
- **先に要るもの**: 働きの上では無し。人の手・静かな机・前後の全体テストの比べ・依存の追加・emo-text の新しいファイルは要らない。
- **v0.1.0 との重なり（測り直し）**: 着地した 2 本（`ghost-inner-balloon`・`property-name-case-fold`）が外れ、残るのは 3 本＝`readonly-mode`（`catalog*.rs`・`roots/`・`persist/`・`install/destination.rs`・`update/desk.rs`・`boot_resolve.rs`）・`default-balloon-selfmade`（`catalog.rs`・`boot_resolve.rs`）・`popup-menu-shell-look`（`resolve_shell.rs`・裁定待ち）。`startup-install-option` は `boot_config.rs` の場所を引く関数を呼ぶだけで、説明書も別の行＝重ならない。新しく足された v0.1.0 の 9 本（`shiori2-protocol` ほか）とは 0。余力の枠の `multi-ghost-runtime` とは `persist/`・`roots/`・`boot_resolve.rs` で重なる。
- **版の席**: `readonly-mode` と `default-balloon-selfmade` は次のウェーブの先頭の見込みで、見立ての上限 14 も v0.1.0 のたいていの spec の下限を超える＝**v0.1.0 の間は隙間に入れない**のが既定のまま。
- **優先度の区分**: A（開発者「特定のディレクトリを、特定 ID のゴースト・シェル・バルーンとしてエイリアス出来たら便利」）。
- **要件定義のモデル**: Opus（今ある口の手前に段を 1 枚足すだけ・隠す形と書かない守りの前例が在る）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - バルーンを探す順が 3 段になる（エイリアス・ゴーストの中・置き場の並び）。エイリアスとゴーストの中のどちらを上にするかが決まっていない＝要件の議題に足す（`balloons_for` は置き場の和 `list_balloons` を呼ぶので、エイリアスを和の先頭に混ぜると「ゴーストの中が上」に自然になる）。
  - 棚卸㉘の節の行の番号は動いた（`union` 189→223・`scan_shells` 192→193・`SHELL_ROOT` 28→29）。本文 Problem の 2 つ目（起動の引数のパスは目録の外）が古いのは棚卸㉘のとおり。

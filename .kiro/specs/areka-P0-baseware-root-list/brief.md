# Brief: areka-P0-baseware-root-list

> 2026-10-05 `/kiro-discovery` で起票（開発者「ゴーストフォルダの複数管理の仕組みが欲しい。SSP の仕様に従う必要はない。今どきの、良い感じの管理方式」）。「ゴーストフォルダの複数管理」の 2 本の 1 本目（roadmap の同名の節が分け方の正本）。後の `dev-folder-alias` はこの spec が作る「上から探す」関数の上に乗る。

## Problem

- **利用者**:
  - SSP から乗り換える人・SSP と併用する人。SSP に入れてあるゴーストやバルーンを、移さずに areka でもそのまま使いたい。
  - ゴーストが多く、別のドライブに置きたい人。
- 今の areka は根（ベースウェアの格納フォルダ）を 1 つしか持てない。`<根>/ghost/<名>/`・`<根>/balloon/<名>/` の外にあるものは、起動の引数で直接渡す以外に使えない。引数で渡したものは「目録の外」の扱いで、メニューにも切替先にも記憶にも載らない。
- SSP は「本体設定 → フォルダ」で格納フォルダを種類ごとに複数登録できる（SSP ヘルプ `ssphelp:config-folder`）。順番・一時的な除外・フォルダごとの個別設定・インストールのたびに行き先を選ぶ、などを持つ。**開発者裁定で、これには従わない。**

## Current State

- **根の決め方**: `crates/areka/src/boot_config.rs` の `resolve_root_from`。環境変数 `AREKA_ROOT` → exe の本当の場所（`exe_location`）の隣、の 2 段で 1 つに決まる。結果は `BootContext.root: BasewareRoot` に 1 つだけ入る。根が無いときの告知（`crates/areka/src/alert.rs`）の文面は `AREKA_ROOT` を名指しする。
- **値型と列挙**: `crates/areka-ghost/src/catalog.rs` の `BasewareRoot`（`ghost_store`・`balloon_store`・`ghost_dir(folder)`・`balloon_dir(folder)`）と `list_ghosts`・`list_balloons`。並びはフォルダ名のバイト順。`list_shells(ghost_dir)` はゴーストのフォルダ基準で、根に依存しない。
- **根を 1 つと決め打ちしている本番の箇所**: 約 14 ファイル・35〜40 か所（2026-10-05 の下調べ）。
  - 起動の解決: `boot_config.rs`・`boot_resolve.rs`（既定の定数 `DEFAULT_GHOST_FOLDER`＝`emo2`・`DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon`）・`main.rs`（致命時の既定ゴースト・起動中の印の名前）。
  - 切替: `emo2_boot/ghost_switch.rs`（目録・`name` とフォルダ名の照合・`random`／`sequential`／`lastinstalled`・既定へ戻す・切替先のバルーン）・`emo2_boot/shell_balloon_resolve.rs`・`emo2_boot/shell_balloon_switch.rs`。
  - メニュー: `menu/ghost_frame.rs`・`menu/balloon_frame.rs`。
  - インストール: `install/desk.rs`（`GhostFacts.root`・インストール後の記録）・`install/overwrite.rs`・`install/procedure.rs`。`areka-nar` は根を `&Path` で受けるだけで、場所は決めない（作業フォルダは `<根>/.nar-work`）。
  - ネットワーク更新: `update/desk.rs`。対象は今のフォルダそのもので、根を引くのは `updateother` の `--balloon=` だけ。
- **記憶の鍵はフォルダ名だけ**: `LastGhost`（App スコープ）・`LastBalloon`・`LastShell`（Ghost スコープ）・切替の `GhostSpec::Folder`・メニューの印。どの根かは持たない（`crates/areka-sylphya/src/persist/mod.rs` の `PersistKey`）。
- **App の記憶の置き場**: `default_app_profile_dir`（`AREKA_PROFILE_DIR` → `<exe のフォルダ>/profile/areka/`）。本体の設定ファイル（`areka.toml`）は無い（`baseware-root-layout` 要件で「新設しない」）。
- **先送りの記録**: 完了 spec `baseware-root-layout` の brief の Out に「複数の根（ポータブル運用と `%APPDATA%` の併用）。α は 1 つ」。

## Desired Outcome

1. **根を並びで持てる。** 例: 「`c:\areka` → `c:\ssp`」。各根は今と同じ形（`<根>/ghost/`・`<根>/balloon/`）で、SSP のフォルダもそのまま根になる。種類ごと（ゴースト・バルーン）の登録はしない。
2. **上から探す。** ゴースト・バルーンをフォルダ名で引くときは、並びの上の根から探して最初に見つかったものを使う（Windows の `PATH` と同じ）。同じフォルダ名が下の根にもあれば、下のものは隠れる。隠れたものは起動のときに記録へ 1 行ずつ残す。
3. **見分ける鍵はフォルダ名のまま。** 隠れる規則によって、有効なフォルダ名は並び全体で 1 つに決まる。なので `LastGhost` などの記憶・切替・メニューの印は今の鍵のままで動く（鍵の作り直しはしない）。
4. **列挙は並び全体の和。** メニュー・`random`／`sequential`・`\![change,ghost,…]` の照合は、隠れたものを除いた和の上で今どおりに動く。
5. **インストール先**:
   - 新しく入れるものは、並びの先頭の根へ入れる。
   - すでにどこかの根にあるもの（同じフォルダ名）への上書きは、今あるその根で行う。
   - ネットワーク更新は今どおり、そのフォルダの場所で行う。
   - インストールのたびに行き先を尋ねることはしない。
6. **並びの持ち方**:
   - 既定は今と同じで、exe の隣の 1 つだけ。
   - 追加した根は App の記憶（`sylphya.toml`）に並びとして残す。
   - `AREKA_ROOT` は `;` 区切りで並び全体を渡せるようにし、渡したときは記憶の並びを上書きする（開発・テスト・実機サインオフの入口）。1 つだけ渡す今の使い方はそのまま動く。
7. **メニューで足す・外す。** 右クリックメニューに次の 2 つを置く。設定画面は作らない。
   - 「根を追加…」: OS 標準のフォルダ選択ダイアログで選び、並びの末尾に足す。
   - 「根を外す ▸」: 登録済みの根から選んで外す。フォルダそのものは消さない。

   並べ替えの画面は作らない（外して足し直す）。
8. **無いときに黙らない。** 登録した根が見つからなければ、記録を残してその根を飛ばす。並びのどれも使えないときは、今の「根が無い」告知へ倒す。告知の文面は並びに合わせて直す。

## Approach

- 「根の並び」を表す値型を `areka-ghost/src/catalog.rs` に置く。中身は `BasewareRoot` の並び。
  - 列挙（`list_ghosts`・`list_balloons`）は並びを受けて、隠れたものを除いた和を返す。各項目は自分がどの根にあるかを持つ。
  - 「フォルダ名 → 実際の場所」を引く関数を 1 つ置く。今 `ghost_dir(folder)`・`balloon_dir(folder)` でパスを組んでいる呼び手は、全部この関数へ寄せる。
  - **この関数が `dev-folder-alias` との境目になる。** 後の spec は、この関数の手前にエイリアスの段を 1 枚足すだけで済む形にしておく。
- 根の解決（`boot_config.rs`）は並びを返す形にする。`BootContext.root` は並びになる。インストールの「先頭の根」も、ここから取る。
- 記憶の鍵を変えないので、`boot_resolve.rs` の判断（`listed: &[String]`＝フォルダ名の列）はほぼそのまま使える。
  - **2026-10-07 の注記**（`ghost-standard-balloon` の完了時の棚卸）: バルーンの鎖の一覧は `ghost-standard-balloon` で `BalloonInputs::listed: &[areka_ghost::catalog::BalloonEntry]`（フォルダ名と descript の `name` の組）に変わった。`&[String]` のままなのはゴーストの一覧（`GhostInputs::listed`）だけ。バルーンの根を並びにするときは、`BalloonEntry` の列をどの根から集めたかを持たせる形で読み直すこと。
- メニューの 2 項目は `menu/` に足す。フォルダ選択は Win32 の `IFileOpenDialog`（`FOS_PICKFOLDERS`）を使う。workspace の `windows` の機能に `Win32_UI_Shell`・`Win32_System_Com` はすでにある見込みで、要件の段で確かめる。

## Scope

- **In**:
  - 根の並び（値型・解決・`AREKA_ROOT` の `;` 区切り・App の記憶への保存）。
  - 上から探す関数と、和の列挙（隠れたものの記録）。
  - 根を引く本番の呼び手を全部その関数へ寄せる（起動・切替・メニュー・インストール・更新の `--balloon=`）。
  - インストール先の決め方（新規は先頭・既存はその場）。
  - メニューの「根を追加…」「根を外す ▸」。
  - 告知の文面と、利用者向けの説明（`dist/README.txt`）。
  - 決定論テスト（並び・隠れ・インストール先・環境変数の上書き）。
- **Out**:
  - ゴースト・シェル・バルーン 1 つずつを別の場所に結び付けること（`dev-folder-alias`）。
  - SSP の個別の設定（一時的な除外のチェック・フォルダごとのランダム切替・自動切替・自動更新・ヘッドラインの除外・インストールのたびに行き先を選ぶ・起動時に使うフォルダを選ぶ）。
  - 並べ替えの画面・設定画面・本体の設定ファイル。
  - 同じゴーストを SSP と areka で同時に起動したときの SHIORI の保存ファイルの書き合い。防がず、説明書の注意書きだけにする。
  - プロパティの `ghostlist`・`balloonlist`（`property-catalog-lists` の持ち物）。この spec の列挙が、その日の供給源になる。

## Boundary Candidates

- 根の並びの解決と保存（`boot_config.rs`・App の記憶）。
- 上から探す関数と和の列挙（`catalog.rs`・純粋な判断）。
- 呼び手の付け替え（起動・切替・メニュー・インストール・更新）。
- メニューの 2 項目とフォルダ選択ダイアログ。

## Out of Boundary

- エイリアス（`dev-folder-alias`）。
- インストールの中身（`areka-nar` の展開・`install.txt` の意味論）。
- 多重ゴースト（同時に起動するのは今どおり 1 体）。

## Upstream / Downstream

- **Upstream**: 完了 spec `baseware-root-layout`（根と列挙）・`ghost-shell-balloon-switch`（切替）・`popup-menu-minimal`（メニュー）・`ghost-install`／`nar-install`（インストール）・`network-update`（更新）。
- **Downstream**: `dev-folder-alias`（上から探す関数の上に乗る）・`property-catalog-lists`（列挙を供給源にする）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `baseware-root-layout` の Out「複数の根」を引き取る）。
- **Adjacent**:
  - `emo2_boot` の結線の列（`ghost_switch.rs`・`shell_balloon_switch.rs` を分け合う）。
  - `install-live-target-hazards`（`install/` を触る）。
  - `popup-menu-residue`（`menu/`）。
  - `mcp-reload`（`ghost_switch.rs`・`shell_balloon_switch.rs`）。

## Constraints

- 記憶の置き場は exe の隣のまま（`baseware-root-layout` の裁定 1）。インストーラー版の `%APPDATA%` は予約の行のまま。
- 実機サインオフの「絶対パス起動」（argv の上書き）は残す。
- **確かめること（要件の段）**: winget の portable で更新したとき、exe の隣のフォルダが入れ替わるか。入れ替わるなら、exe の隣に入れたゴーストを守る手（先頭の根を exe の外に置く案内など）を要件に足す。
- App の記憶の `PersistKey` は 1 つの値を持つ鍵の族で、並びを持つ鍵は無い。入れ物の形（鍵の族を足すか・TOML の表の形）と、areka が動いている間に利用者が `sylphya.toml` を手で書き換えたときの扱いは設計で決める。
- 段: その他。規模の見込み M〜L（14〜18）。

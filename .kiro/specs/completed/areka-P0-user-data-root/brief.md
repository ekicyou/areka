# Brief: areka-P0-user-data-root

> 2026-10-10 `/kiro-discovery` で起票。出どころは `areka-P0-winget-manifest-submission` のタスク 3.6（手元の確かめで見つかった件の起票。同 spec の要件 4.6・4.8・6.3）。根拠の記録は同 spec の `verification/winget-local-check.md`（「上げ直しの実測」「外し方の実測」「機械の全員向けの実測」「後片付けの確かめ」と、「見つかった件と起票」の 2〜4 行目）。区分 A: 同 spec の要件の討議で、利用者の物が消えるおそれについて開発者「消えるなら winget は使えない。アプリをインストーラー形式にするとか、ファイル置き場を指定できるようにするとか、整備が必要」。

## Problem

- **困る人**: winget で areka を入れた利用者。それと、`areka.exe` の在るフォルダをインストーラーが管理している人・そのフォルダに書き込めない人。
- areka は、利用者の物をすべて `areka.exe` の隣（入れ先のフォルダの中）に置く。
  - 後から入れたゴーストとバルーン: `<根>\ghost\<名>\`・`<根>\balloon\<名>\`
  - `.nar` を入れるときの作業フォルダ: `<根>\.nar-work\`
  - areka の記憶（前回使ったゴーストなど）: `<exe のフォルダ>\profile\areka\`
  - ゴーストの記憶と、ゴースト自身の書き残し（SHIORI の保存データなど）: `<根>\ghost\<名>\ghost\master\profile\…`
  - シェルの記憶: `<根>\ghost\<名>\shell\<シェル>\profile\areka\`
- winget は、配布 zip の最上位に在った物（`ghost\`・`balloon\` と 6 つのファイル）を、**上げ直すときにも外すときにも**消す。フォルダは、中に後から置かれた物ごと消える。winget が出す文は、利用者の物を消したことを告げない。
  - ＝利用者が後から入れたゴースト・バルーンと、ゴーストの記憶（同梱のゴーストの分も）が、上げ直しのたびに黙って消える。
- 機械の全員向け（`winget install --scope machine`）に入れると、入れ先（`C:\Program Files\WinGet\Packages\…`）の直下に普段の権限では何も作れない。areka の記憶は書けず、ゴーストを後から入れることもできない。
- この状態が直るまで、説明書に winget での入れ方を載せられず、winget-pkgs へ次の版も出せない（`areka-P0-winget-release-automation` が待つ）。

## Current State

### コードで確かめたこと（2026-10-10・この枝の木）

- **根の決め方**: `crates/areka/src/boot_config.rs` の `resolve_root_from`。環境変数 `AREKA_ROOT` が在ればそれ、無ければ起動した exe の本当の場所（`exe_location`）の親フォルダ。根は 1 つだけ（`BootContext.root`）。
- **areka の記憶の置き場**: 同じファイルの `default_app_profile_dir`。環境変数 `AREKA_PROFILE_DIR` が在ればそれ、無ければ exe のフォルダの `profile\areka\`。**`AREKA_ROOT` を置いても、こちらは動かない**（根ではなく exe の場所から決める）。
- **ゴーストの記憶とシェルの記憶の置き場**: `crates/areka-ghost/src/sylphya_wiring.rs` の `profile_areka_root` が、渡されたフォルダの下の `profile\areka\` を返す。呼び手（`crates/areka-ghost/src/runtime.rs`）が渡すのは、ゴーストの SHIORI のフォルダ（`<ゴースト>\ghost\master`）と、シェルのフォルダ。＝どちらも**ゴーストのフォルダの中**に決まる。ゴースト自身の書き残し（同梱の えも？？ では `ghost\master\profile\pasta\` の下）は、SHIORI が自分のフォルダへ書く物で、areka は場所を決めていない。
- **作業フォルダ**: `crates/areka-nar/src/install.rs` の定数 `WORK`（`.nar-work`）。1 回の展開が `<根>\.nar-work\<プロセス識別子>-<連番>\` を使う。
- **今でも、2 つの環境変数を両方置けば全部を入れ先の外へ出せる**（`AREKA_ROOT` でゴースト・バルーン・作業フォルダ・ゴーストとシェルの記憶、`AREKA_PROFILE_DIR` で areka の記憶）。ただし次の 4 つが足りない。
  - winget で入れた形では測っていない。
  - 本体の設定ファイルは無い（完了 `baseware-root-layout` の範囲外に「設定画面・設定ファイルの新設」）。利用者が環境変数を自分で置くことになる。
  - `AREKA_ROOT` を置くと、根がその 1 つになるので、exe の隣の同梱のゴーストとバルーンは探されなくなる。
  - 外へ出す先を、利用者が自分で決めて作ることになる。
- **前の裁定**: 完了 `baseware-root-layout` の裁定 1（2026-09-23）は、根の既定を exe の隣にした。理由は「書き込み権限の問題が起きない場所へ利用者が置く」。`%APPDATA%\areka\` は α では採らなかった。winget は、利用者が置き場を選ばない入れ方なので、この前提が当てはまらない。
- **説明書**: `dist/README.txt` の「■ 記憶の置き場」は、上の 3 か所（areka・ゴースト・シェル）を exe の隣からの場所で書いている。シェルの記憶は、0.0.2 には書く操作が無く、実測で作れなかった（記録の「既知の制限」）。

### 測った事実（記録から。どれも 0.0.2・開発機 1 台・手元のマニフェストで入れた形）

| 操作 | 消えた物 | 残った物 |
|---|---|---|
| 上げ直し（利用者向け・`winget upgrade`） | **6**（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）＝208 ファイル | areka の記憶（中身は、消えたゴーストを指したまま）。次の起動は同梱のゴーストで立った |
| 外す（利用者向け・`--purge` も `--preserve` も付けない） | 同じ **6**（利用者の物 207 ファイル） | areka の記憶 1 ファイルと空の `.nar-work\`。このため入れ先のフォルダが残った。利用者の PATH の項目 1 件も残った |
| 外す（利用者向け・`--purge`。前の残りの上へ入れ直した物） | — | 入れ先のフォルダと、利用者の PATH の項目 1 件 |
| 外す（機械の全員向け・`--purge`） | — | 入れ先のフォルダは消えた。機械の PATH の項目 1 件が残った |

- 見えた形: zip の中に在る直下の物（`ghost\`・`balloon\`・最上位の 6 ファイル）は、フォルダなら中に後から置かれた物ごと消された。zip の中に無い直下の物（`profile\`・`.nar-work\`）は触られなかった。
- 機械の全員向けに入れた形（普段の権限で起動）:
  - ゴーストは立った（終了コード 0）。
  - areka の記憶は書けなかった（1 回の起動につき 3 回、「アクセスが拒否されました」）。
  - メニューの「インストール…」は、作業フォルダを作れずに止まった。
  - ゴーストごとの記憶は、この機械では書けた（入れた管理者と普段の利用者が同じアカウントで、入れ先の下の階層にそのアカウントの権限の行が在ったため）。

### 分かっていないこと

- winget-pkgs から入れた形（入れ先のフォルダの名前と `winget list` の ID が、手元のマニフェストで入れた形と違う）。
- 入れた管理者と普段の利用者が別のアカウントの機械（権限の一覧からは、ゴーストごとの記憶も書けず、起動もできないおそれが読める。測っていない）。
- 利用者向けで、何も残っていない所へ入れて `--purge` で外す形。
- arm64 の実機。
- 中身の違う新しい版への上げ直し（確かめは、版の欄だけを上げたマニフェストで行った）。
- winget の利用者の設定で「外すときに入れ先を丸ごと消す」をオンにした機械。

## Desired Outcome

1. winget の上げ直しと外し方（`--purge` を付けない）で、利用者が後から入れたゴースト・バルーン、areka の記憶、ゴーストの記憶、シェルの記憶が消えない。**消えた物が 0** であることを、出どころの spec と同じ実測のやり方で確かめる。
2. 機械の全員向けに入れた形でも、普段の権限の利用者が areka の記憶を書け、ゴーストを後から入れられる。
3. zip を展開して exe の隣で使う今の使い方は、今と同じに動く。すでに exe の隣に在る利用者の物を見失わない。
4. 利用者の物が入れ先の中に無くなるので、winget で外したときに、入れ先のフォルダを winget が消せる。
5. 説明書の「■ 記憶の置き場」が、実際の置き場と合う。
6. 出どころの spec が雛形に足した注意書き（`InstallationNotes`＝上げ直しで消える物の知らせ）を外してよい状態になる。

## Approach

**推す向き: 利用者の物を、入れ先のフォルダの外の「書ける根」に置く。**

- 同梱の物（配布 zip に入っているゴーストとバルーン）は exe の隣に残し、根の並びの下の段にする。
- 新しく入れる物・記憶・作業フォルダは、書ける根へ行く。
- 土台は `areka-P0-baseware-root-list` の「根を並びで持つ・新しく入れる物は先頭の根へ」。その上に、既定の並びと記憶の置き場を足す形にする。

この向きは起票のときの推しで、**開発者の裁定はまだ受けていない**。裁定はこの spec の要件の討議で受ける。

**推さなかった向き**

- **インストーラー版を作る**（winget の名乗り `Areka.Areka`。置き場と後片付けをインストーラーが持つ）。大きい。ポータブル版（`Areka.Areka.Portable`）で入れた人は今のまま残る。インストーラー版は roadmap の「予約」に 1 行在るだけで、brief が無い。
- **zip の並びだけを変える**（winget が `ghost\`・`balloon\` を「自分が入れた物」と覚えない形にする）。小さい。ただし、機械の全員向けの「書けない」は直らない。0.0.2 を入れた人の最初の上げ直しも救えない見込み（何を消すかは、入っている版の控えで決まるため。下の議題 5 のとおり、これは推す向きでも同じ見込み）。

## Scope

- **In**:
  - 利用者の物（後から入れるゴーストとバルーン・作業フォルダ・areka の記憶・ゴーストの記憶・シェルの記憶）の置き場の決め方。
  - 同梱の物を exe の隣から探し続けること。
  - すでに exe の隣に在る利用者の物の扱い（探し続けるか、移すか）。
  - 説明書 `dist/README.txt` の「■ 記憶の置き場」の直し（シェルの記憶の行を含む）。
  - 決定論テストと、winget で入れた形での実測（上げ直し・外す・機械の全員向け）。
- **Out**:
  - winget が外した後に残す PATH の項目（winget の側の動き。areka からは直せない）。
  - インストーラー版（名乗り `Areka.Areka`）。
  - winget のマニフェストの雛形（`dist/winget/**`）の書き替え。注意書きを外すのは、雛形の持ち主の仕事。
  - 書けないときに利用者へ伝えること（`areka-P0-write-failure-notice`）。

## Boundary Candidates

- 書ける根の場所の決め方（純粋な判断。環境変数の上書きを含む）。
- areka の記憶の置き場（`default_app_profile_dir`）。
- ゴーストの記憶・シェルの記憶・ゴースト自身の書き残しの置き場（同梱のゴーストの分＝入れ先の中に在る物をどうするか）。
- 作業フォルダの置き場（`areka-nar` へ渡す根）。
- すでに在る利用者の物の引き継ぎ。
- 説明書の直し。

## Out of Boundary

- 根の並びそのもの（値の形・上から探す関数・メニューの「根を追加…」）＝`areka-P0-baseware-root-list`。
- 開発用のエイリアス＝`areka-P0-dev-folder-alias`。
- ゴーストの中のバルーン＝`areka-P0-ghost-inner-balloon`。
- `.nar` の展開の中身（`areka-nar` の検査と手続き）。
- 起動中のゴーストへ入れるときの危険＝`areka-P0-install-live-target-hazards`。

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-list`（brief だけ。根を並びで持つ・新しく入れる物は先頭の根へ・既定は「exe の隣だけ」）。この spec はその上に作るか、少なくともその後に置く（同じ `crates/areka-ghost/src/catalog.rs`・`crates/areka/src/boot_config.rs`・`boot_resolve.rs`・`install/` を触る）。完了 `baseware-root-layout`（根と記憶の置き場の裁定）。
- **Downstream**: `areka-P0-winget-release-automation`（この spec が着地するまで、説明書に winget の行を載せず、winget-pkgs へ次の版を出さない）。

## Existing Spec Touchpoints

- **Extends**: なし。完了 `baseware-root-layout` の裁定 1（根と記憶は exe の隣）を、winget で入れた形について改める。
- **Adjacent**:
  - `areka-P0-baseware-root-list`: 同 brief の「確かめること」（winget の portable で更新したとき、exe の隣のフォルダが入れ替わるか）の答えは「入れ替わる」と出た。同 brief は、その場合に「exe の隣に入れたゴーストを守る手を要件に足す」と書く。**持ち分が重なる**ので、着手のときにどちらが持つかを決める。
  - `areka-P0-dev-folder-alias`・`areka-P0-ghost-inner-balloon`・`areka-P0-shell-companion-balloon`（同じ `catalog.rs`・`boot_resolve.rs`・`boot_config.rs`）。
  - `areka-P0-winget-manifest-submission`（出どころ。雛形の `InstallationNotes` と、設計の「Revalidation Triggers」に「利用者の物の置き場を変える spec が着地する」が在る）。
  - `areka-P0-install-live-target-hazards`（`install/`）。
  - `areka-P0-write-failure-notice`（同じ日に起票。この spec が着地しても、書き込みはほかの理由で失敗しうるので、別の仕事）。

## Constraints

- zip を展開して exe の隣で使う形（ポータブル）を壊さない。
- 実機サインオフの「絶対パス起動」（引数での上書き）と、環境変数 `AREKA_ROOT`・`AREKA_PROFILE_DIR` の今の役目を残す。テストと実機の確かめでは、書ける根の場所を環境変数でワークツリーの `target\` の下へ向けられること（実機の根・検体・一時フォルダは `target\` の下だけ）。
- メッセージボックスを足さない。設定画面は作らない。SSP のフォルダ設定は写さない。置き場は OS の決まった場所（利用者ごとのフォルダ）を使う。
- winget で入れた形の確かめは、出どころの spec と同じ決まりで行う（winget と OS の設定を変えるのは開発者の手だけ・変えた設定は戻す・確かめ専用のマニフェストは `target\` の下）。
- 一度出した名乗り `Areka.Areka.Portable` は変えない。
- 区分: A。規模の見込み: M〜L（要件で測る。`baseware-root-list` の着地の形で変わる）。要件定義のモデル: Fable を勧める（置き場の裁定・すでに在る物の引き継ぎ・ゴーストのフォルダの中の記憶にまたがる）。

## 要件の討議へ持ち込む議題（起票では決めていない）

1. **向きの裁定**: 上の「推す向き」でよいか。
2. **いつ外の根を使うか**: いつも／exe のフォルダに書けないときだけ／winget で入れたときだけ。どれを選んでも、exe の隣で使う今の形は今と同じに動かす。
3. **どこに置くか**: 利用者ごとの場所（例: `%LOCALAPPDATA%` の下・ドキュメントの下）。機械の全員向けでも書ける場所であること。
4. **すでに exe の隣に在る利用者の物**を、探し続けるか、移すか。
5. **0.0.2 を winget で入れた人の、最初の上げ直し**: winget が物を消すのは、新しい版の areka が動く前。入っている版の控えに従って消す見込みなので、どの向きを選んでも、この 1 回は areka の側からは守れない見込み（測っていない）。守る手が説明（今の `InstallationNotes`）しか無いなら、そう決めて書く。
6. **areka の記憶**（`profile\areka\`）をどこへ置くか。上げ直しと外し方では残ったが、機械の全員向けでは書けなかった。
7. **同梱のゴーストの記憶**: ゴーストの記憶とゴースト自身の書き残しは、ゴーストのフォルダの中（同梱のゴーストでは、winget が消す木の中）に書かれる。ここがいちばん難しい。areka が決めているのは `profile\areka\` の場所だけで、SHIORI の書き残しの場所は SHIORI が決める。
8. **説明書**の「■ 記憶の置き場」をどう書き直すか（シェルの記憶の行は、0.0.2 では書く操作が無い）。
9. **雛形の注意書きを外す時機と持ち主**（この spec の着地の後。`dist/winget/**` はこの spec の範囲外）。
10. **次の版を、自分たち以外が winget-pkgs へ出しうること**（2026-10-10 `winget-manifest-submission` の最後の検証で出た一般論・確かめていない）: 初回の `0.0.2` が取り込まれた後は、第三者や自動の道具が、次の版の PR を winget-pkgs へ出すことがありうる。「置き場が直るまで次の版を出さない」は、自分たちの側の決めでしかない。そうなったときに、`0.0.2` を入れた人の上げ直しで物が消えるのを、どこまで受け入れ、どう知らせるか（雛形の注意書き・Release の説明）。


## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）
- 前提の変化: 無し（10-10 の起票の後、関係のファイルは変わっていない）。winget-pkgs への初回の提出は、リポジトリの記録の上では人の承認待ちのまま。
- 触るファイル（呼び手を数えた）:
  - 根の決め方と areka の記憶の置き場: `crates/areka/src/boot_config.rs`（`resolve_root_from`・`default_app_profile_dir`）。後者の呼び手は `main.rs`（956 行・上限 1,000）・`ghost_session.rs`（873）・`shiori_host.rs`。
  - ゴーストとシェルの記憶の置き場を動かす場合（議題 7）: `crates/areka-ghost/src/{sylphya_wiring,runtime}.rs`・`crates/areka/src/boot_resolve.rs`（ゴーストの記憶の根を組む所）・`placement/persist.rs`。
  - 作業フォルダ: `crates/areka/src/install/desk.rs`（展開器へ根を渡す所）・要れば `crates/areka-nar/src/install.rs`（定数 `WORK`）。
  - 既定の並び: `crates/areka-ghost/src/catalog.rs`（`baseware-root-list` が作る並びの型）。説明書 `dist/README.txt`「■ 記憶の置き場」。
- 規模: 12〜18 タスク（議題 7 で同梱のゴーストの記憶を動かすなら上の方。winget で入れた形の実測 3 種を含む）。切る案は無し。
- 先に要るもの: `baseware-root-list`（根の並び・「新しく入れる物は先頭の根へ」）。
- カタログの鎖の順の案: `baseware-root-list` → `user-data-root` → `ghost-inner-balloon` → `write-failure-notice` → `shell-companion-balloon`＝この spec は 2 番目。理由は、`winget-release-automation` と説明書の winget の行がこの spec を待つこと。
- ファイルの重なり:
  - カタログの鎖のほかの 4 本（`boot_resolve.rs`・`boot_config.rs`・`catalog.rs`）。`write-failure-notice` とは `main.rs` も。
  - `tasktray-icon`（`main.rs`）・`install-live-target-hazards`（`install/`）。
  - `placement/persist.rs`・`ghost_session.rs` を触るほかの列の spec は、着手の前に照らす。
- 優先度の区分: A。要件定義のモデル: Fable（どちらも起票のまま）。
- 見つけた穴・古くなった記述: 本文の触る所に `main.rs`・`ghost_session.rs`・`shiori_host.rs`・`placement/persist.rs` が無い。`baseware-root-list` の「確かめること」（exe の隣が入れ替わるときに守る手）は、この spec が持つ案。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 版の段で **v0.0.3** の 2 本目（カタログの鎖）。触るファイルは変わっていない。winget-pkgs への初回の提出（PR 450070）は 10-16 に読んでも開いたまま（取り込まれていない）。
- 触るファイル: 棚卸㉔のまま。足す分: ネットワーク更新の作業フォルダ（`crates/areka-update/src/paths.rs` の `WORK_DIR`＝`<対象のゴースト>\.update-work\`・`work.rs`）は、ゴーストのフォルダそのものの中に作られ、更新もその場で書く。＝同梱のゴースト（exe の隣）の更新は、機械の全員向けでは書けず、利用者向けでは次の上げ直しで更新の中身ごと消える。範囲に入れるかを議題 7（同梱のゴーストの記憶）と一緒に決める。
- 規模: 12〜18 タスク（上の件を入れるなら上の方）。
- 先に要るもの: `baseware-root-list`（根の並び・「新しく入れる物は先頭の根へ」）。
- 同じウェーブに置けない相手（24 本の中）:
  - `baseware-root-list`・`write-failure-notice`（`main.rs`・`boot_resolve.rs`）・`default-balloon-selfmade`（`boot_resolve.rs` の定数 1 行）。
  - `multi-ghost-runtime`（`main.rs`・`boot_config.rs`・`boot_resolve.rs`・`ghost_session.rs`・`placement/persist.rs`）・`dpi-realign-remembered-chain`（`placement/persist.rs`）・`placement-measure-bake-once`（案 A を選ぶと `ghost_session.rs`）。
  - 重なり 0: `app-icon`・`balloon-text-bugs`・`winget-release-automation`（`dist/README.txt` は別の節）。
- 優先度の区分: A（開発者「消えるなら winget は使えない」）。
- 要件定義のモデル: Opus に改める。置き場の向きは開発者の裁定（議題 1〜3）で決まり、先例（完了 `baseware-root-layout` の裁定・`AREKA_PROFILE_DIR` の上書き）の延長。新しい境界・並行の保証・先例の無い骨格の分かれ目のどれにも当たらない。
- 分割の案: 切らない。
- 見つけた穴・古くなった記述:
  - 本文の触る所に `areka-update` の作業フォルダが無い（上）。
  - 雛形の注意書き（`dist/winget/**` の `InstallationNotes`＝上げ直しで消えるという知らせ）を外す持ち主がいない。この spec は「雛形の持ち主の仕事」と書き、`winget-manifest-submission` は完了し、`winget-release-automation` は `dist/winget/**` を範囲外にしている＝要件の段で持ち主を決める（`winget-release-automation` が持つのが自然）。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）
- 前提の変化: `baseware-root-list` が 10-16 に着地した（PR#317）。根は並びになり、決め方は `crates/areka/src/roots/` へ移った（環境変数と記憶を読む口は `roots/mod.rs` の `resolve_roots`、並びを決める本体は `roots/resolve.rs` の `resolve_roots_from`。何も無いときの既定＝exe のフォルダ 1 つは同じ関数の 117〜129 行）。本文の「`boot_config.rs` の `resolve_root_from`」「根は 1 つだけ（`BootContext.root`）」は古い（今は `BootContext.roots`）。
- インストールの側は済んだ所が多い: 新しく入れる物は見つかるいちばん上の根へ入り（`crates/areka/src/install/destination.rs`）、展開の作業フォルダもその根ごとに作る（`crates/areka-nar/src/install.rs` の作業フォルダの型）。＝書ける根を既定の並びの先頭に置けば、新しく入れる物と作業フォルダは付いてくる。`install/desk.rs` を触る見込みは消えた。
- 残る穴（この spec が持つ・議題 7 と一緒に決める）:
  - すでにある物の入れ直しは「見えている根でその場で上書き」なので、exe の隣の同梱のゴーストを入れ直すと exe の隣へ書く（`catalog_roots.rs` の `install_root`）。
  - ネットワーク更新の作業フォルダ（`crates/areka-update/src/paths.rs` の `WORK_DIR`・`work.rs` の 39・156 行）は今も対象のゴーストの中。`baseware-root-list` は `areka-update` に触っていない。
  - 利用者がメニューの「置き場を外す」で書ける根を外すと、新しく入れる物が exe の隣へ戻る（外せないようにするかを議題に足す）。
- 触るファイル（今の見立て）:
  - 必ず: `crates/areka/src/boot_config.rs` の `default_app_profile_dir`（373〜381 行と説明の注記）・`roots/resolve.rs` の `resolve_roots_from`・`roots/mod.rs` の `resolve_roots`・`dist/README.txt` の「■ 置き場」「■ 記憶の置き場」の行。
  - 選び方しだい: 書ける根を守るなら `roots/edit.rs` の `decide_remove`（67〜89 行）。記憶に書く綴りに `<exe>` のような印を足すなら `roots/resolve.rs` の `decode_memory`・`encode_memory`。
  - 議題 7 でゴーストとシェルの記憶を動かすなら: `crates/areka-ghost/src/{sylphya_wiring,runtime}.rs`（`profile_areka_root` と 591〜596 行）・`crates/areka/src/boot_resolve.rs` の `ghost_roots`（352〜357 行だけ）・`placement/persist.rs`（282〜284 行）。
  - `main.rs`（971 行）は触らない見込み（220 行が `default_app_profile_dir` を呼ぶだけ）。`emo2_boot/ghost_switch.rs`・`install/judge.rs` も触らない。
- 規模: 11〜18 タスク（インストールの側が済んだ分だけ下がる。入れ直しと更新の作業フォルダを入れると上の方）。分割の案: 切らない。
- 先に要るもの: 無し（`baseware-root-list` は着地）＝今の版の先頭。
- `write-failure-notice` と同じファイルを触るか（棚卸㉕の開発者への問い ⑴）: 上の見立てなら、確実に重なるファイルは 0。重なりうるのは次の 3 つで、どれも関数が別。
  - `boot_resolve.rs`: こちらは議題 7 を選んだときの `ghost_roots`（352〜357 行）、向こうは 391〜579 行の印と記録の関数。
  - `roots/edit.rs`: こちらは書ける根を守るときの `decide_remove`、向こうは並びを記憶へ書けないときの `apply`（157〜218 行）。
  - `main.rs`: こちらが置き場を `main` の中で 1 度だけ決めて渡す形を選んだときだけ（171〜231 行）。向こうは `boot_first_ghost`・`settle_session_mark` ほか。
  - 説明書と `doc/COMPAT_ARCHITECTURE.md` の §8 は別の行（許す重なり）。
- ほかの重なり: `dev-folder-alias`（`roots/{mod,resolve}.rs`・説明書の「■ 置き場」）・`ghost-inner-balloon`・`shell-companion-balloon`（`boot_config.rs` の別の関数）・`multi-ghost-runtime`・`dpi-realign-remembered-chain`（議題 7 のとき `placement/persist.rs`）・`network-update-canon-order`・`update-check-options`（更新の作業フォルダを動かすとき）・`install-live-target-hazards`（入れ直しの宛先を変えるとき）。
- 人の手: winget で入れた形の実測（上げ直し・外す・機械の全員向け）に開発者の手が要る。静かな机は要らない。依存は足さない（利用者ごとの場所は環境変数 `LOCALAPPDATA` で取れる。前例は中継の `crates/areka-mcp-bridge/src/config.rs` の 69 行）。
- 優先度の区分: A（開発者「消えるなら winget は使えない」）。要件定義のモデル: Opus（前例の延長で、新しい境界も並行の保証も無い）。

## 2026-10-16 棚卸㉖のウェーブの約束（C8・`write-failure-notice` と同じウェーブ）
- 測り直しで、`write-failure-notice` と確実に重なるファイルは 0、重なりうる 3 つ（`boot_resolve.rs`・`roots/edit.rs`・`main.rs`）は選び方しだいと分かった。ウェーブ C8 では次の約束で同じファイルに触らないことにして、2 本を並べる（v0.0.3 の着地を 1 ウェーブで済ませる）。
- この spec が触らないもの: `crates/areka/src/main.rs`・`crates/areka/src/boot_resolve.rs`・`crates/areka/src/roots/edit.rs`（3 つとも向こうが触る）。
  - 書ける根を外せないようにするなら、`roots/edit.rs` の `decide_remove` ではなく、並びを決める `roots/resolve.rs` の側で書ける根をいつも含める形にする。
  - 置き場は `main` の中で決めて渡す形を取らず、`boot_config.rs` の `default_app_profile_dir` の中身を替える。
  - 議題 7 でゴーストの記憶を動かすときは、まず `crates/areka-ghost/src/{sylphya_wiring,runtime}.rs` の側で済ませる。`boot_resolve.rs` の `ghost_roots` に手を入れないと済まないと分かったら、そのタスクを最後に置き、`write-failure-notice` の着地を待って main を取り込んでから直す。
- この spec が持つと決めたもの: 中継 `crates/areka-mcp-bridge/src/config.rs`（82〜98 行・控えの置き場が areka の記憶の置き場の決め方を写している）。areka の記憶を exe の隣から動かすなら、中継の側も同じ置き場へそろえる（そろえないと中継の控えだけが exe の隣に残り、winget の上げ直しで消える）。同じウェーブの `mcp-stdio-bridge-residue` はこのファイルに触らない約束。

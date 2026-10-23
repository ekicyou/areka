# Brief: startup-install-option

> 2026-10-18 棚卸㉗で起票（開発者の指示「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします」＝v0.1.0 のゴール）。根拠は棚卸㉗の調べ（SSP の起動オプションと areka の今の起動の比べ・main `2d86dbff`）。正典は SSP ヘルプ「起動オプション一覧」の `--install`・`--option callghost`・`--option deletesource`・`initial_install.nar`（ukadoc MCP の `ssphelp:option`・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）。区分 **A**。版 **v0.1.0**。

## Problem

- **開発者・AI のエージェント**: 作ったばかりの NAR（例: ぱすたの `pasta_check release` が作る `pasta-in-windows.nar`）を areka に入れて確かめるには、今は areka を立てて窓へ投げ込むか、右クリックメニューから選ぶしかない。自動の試験では窓への投げ込みを真似る（`SendInput` や computer-use）ことになり、遅く、ほかの試験と画面を取り合う。
- SSP には `ssp.exe --install a.nar b.nar` があり、起動の 1 行で順に入れてから起動を続ける。`--option deletesource` で入った NAR を消し、`--option callghost` で入れた後にそのゴーストを呼ぶ。exe の隣に `initial_install.nar` を置くと、起動のときに確認なしで上書きで入れて消す（ゴーストを同梱して配る使い道）。
- 「入れる → 起こす → MCP で確かめる」を 1 本の試験で回すには、インストールを起動の引数から始められる必要がある。

## Current State

- インストールの手続きは揃っている（完了 `ghost-install`・`file-drop`・`nar-install` ほか）。依頼の口は `install::submit`（`crates/areka/src/install/mod.rs`・`InstallOrder { archives: Vec<PathBuf>, origin }`＝書庫を何本も並べられる）と、台本と投げ込みから使う送り口 `install::desk::raw_sender`（`crates/areka/src/install/desk.rs` 141 行目）。依頼の出どころの語は `InstallOrigin`（`Menu`・`Script`・`WindowDrop`）で、手続きは出どころで分かれない（記録の語だけ）。
- インストールの窓口は窓のアプリの中（World）に置かれている＝窓のアプリが立った後でないと使えない。
- 入れた後は `OnInstallComplete`／`OnInstallCompleteEx` をゴーストへ送る（`crates/areka/src/install/judge.rs`）。**入れたゴーストへ切り替えるのはゴースト自身か利用者のメニューの選択だけ**（下の裁定）。
- ゴーストが 1 体も無いときは、起動の解決が「ゴーストが見つからない」として既定のゴーストの台詞の道（`alert`）で止まる。SSP の「何とかして下さい」の画面に当たる「インストールしてから起動を続ける」道は無い。
- 起動オプションを読む口は無い（`startup-options-core` が作る）。`initial_install.nar` を読む所も無い。

### SSP の正典（2026-10-18 に ukadoc MCP で読んだ `ssphelp:option`）

- `--install NARファイル名`: NAR を入れる。起動中の SSP があればそちらに任せ、無ければ自分で入れてから起動を続ける。空白区切りで複数を順に入れる。任せる先の SSP が「ゴーストが入っていない」の警告を出している状態でも受ける。そのとき `--option callghost` も付いていれば、警告の「起動を続ける」を押したことにして続ける。
- `--option callghost`: `--install` と組むと、インストールが終わった後にゴーストを自動で呼び出す。
- `--option deletesource`: `--install` と組むと、NAR のインストールに成功したときに NAR を消す。
- `initial_install.nar`（2.7.52〜）: 起動オプションではないが、`ssp.exe` と同じフォルダに置くと、起動のときに確認のダイアログなしで上書きで入れ、終わったら消す。
- `--option readonly` のときは「手動でのネットワーク更新・NAR のインストールも行わない」「`initial_install.nar` のインストールを行わない」。

### 先にある裁定（2026-09-28 `ghost-install` の要件ディスカッション 議題 2・開発者）

> 「インストールと切り替えは異なるイベントです。『ゴーストが自分から切り替えに行く』挙動はありですが、arekaは主導しません。ゴーストが何も言わない場合、ユーザーが明示的にメニューから新しいゴーストを呼び出さない限り切り替わりません。」

`--option callghost` は、利用者（起動の行を書いた人）が「入れた後に呼べ」と明示する指示である。この裁定と両立するかを要件の段の最初の議題にする（下の議題 1）。

## Desired Outcome

- `areka.exe --install <NAR> [<NAR> …]` で、起動の後（または前・議題 2）に並べた順に入れ、入ったものと失敗したものが記録とゴーストへの知らせ（今のインストールと同じ `OnInstallComplete`／失敗の知らせ）に残る。インストールの手続きそのものは今の窓への投げ込みと同じ道を通る。
- `--option deletesource` のとき、入ったことが確かめられた NAR だけを消す。失敗した NAR は残す。
- `--option callghost` の扱いは議題 1 の答えのとおり。
- exe の隣の `initial_install.nar` を、起動のときに確認なしで上書きで入れ、入ったら消す（入らなければ残し、記録とゴーストへの知らせに残す）。
- `--option readonly` のときは、`--install` と `initial_install.nar` を行わず、断ったことを記録に残す（`readonly-mode` と同じ印を見る）。
- `dist/README.txt` に節が加わる。

## Approach

- `startup-options-core` が作る起動オプションの値に `install`（NAR の並び）と `callghost`・`deletesource` を足す（新 `crates/areka/src/cli/install_args.rs`）。
- 窓のアプリが立ち、最初のゴーストが定常に着いた所で、並びを 1 本の `InstallOrder`（出どころの語に起動の引数を表す 1 語を足す）として `install::submit` へ渡す。`deletesource` は手続きの結果（入ったか）を受けて消す。
- `initial_install.nar` も同じ道に、確認なし・上書き・成功で消す、の印を付けて流す。
- ゴーストが 1 体も無いときの扱いは議題 2。

## Scope

- **In**: `--install`（複数・順に）・`--option deletesource`・`--option callghost`（議題 1 の答えの形）・`initial_install.nar`・readonly のときに断ること・出どころの語の追加・決定論のテスト（並びの順・消す／消さない・readonly で断る）・実機の確かめ（NAR を入れて MCP の `get_active_ghost_list` で確かめる・`real-run-mcp-kit` の足場で）・`dist/README.txt` の節。
- **Out**: 起動中の areka へインストールを任せる仕組み（SSTP の受け口・二重起動の確かめの後）・`--ukagaka-link`・インストールの手続きそのものの変更・NAR を作る道具（`--offline-tool`・`pasta_check` が持つ）。

## Boundary Candidates

- 起動オプションの値（`startup-options-core`）と、インストールの窓口（`install::submit`）の境＝定常に着いた所の 1 か所。
- インストールの結果を受けて NAR を消す所（`deletesource`・`initial_install.nar`）。

## Out of Boundary

- インストールの判定・置き場・上書きの手続き（完了 `ghost-install`・`nar-install`・`nar-install-hardening`）。
- 切り替えの手続き（完了 `ghost-shell-balloon-switch`）。

## Upstream / Downstream

- **Upstream**: `startup-options-core`（起動オプションの値）・`readonly-mode`（readonly の印）。
- **Downstream**: `pasta-in-windows-verification`（ぱすたの NAR を起動の 1 行で入れて確かめる）・`real-run-mcp-kit`（入れる試験の足場）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `ghost-install` の手続きを新しい入口から呼ぶ）。
- **Adjacent**: `readonly-mode`（`install/desk.rs` の守り）・`install-live-target-hazards`（使っている最中のフォルダへの上書き＝`initial_install.nar` の上書きでも起こりうる）・`multi-ghost-runtime`（`callghost` を「2 体目として呼ぶ」形にするなら）・`default-balloon-selfmade`（`dist/README.txt`）。

## Constraints

- 2026-09-28 の裁定（上）を守る。areka が入れた後の切り替えを自分から始める形は、利用者の明示の指示（`callghost`）があるときだけ、議題 1 で開発者が認めた形に限る。
- 失敗はメッセージボックスで告げない（既定のゴーストの台詞）。ただしインストールの警告はダイアログでもよい（2026-10-17 の例外）。
- 失敗の道は必ず記録（`error!`＋`Err`）。
- テストは兄弟のファイル `*_tests.rs`。`main.rs` は 1,000 行の上限＝定常に着いた所の結線は `main.rs` の外。
- 試験の根・NAR の写しはワークツリーの `target\` の下だけ。`deletesource` の試験で本物の NAR を消さない（写しを使う）。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: 新 `crates/areka/src/cli/install_args.rs`＋兄弟のテスト・`crates/areka/src/install/mod.rs`（出どころの語 1 つ）・`install/desk.rs`（524 行・起動の引数の依頼と結果を受ける所）・`install/judge.rs` か `procedure.rs`（成功を受けて NAR を消す・読むだけで済むかは設計）・定常に着いた所の結線（`crates/areka/src/ghost_session.rs` 885 行＝上限の近く・足す分は新しいファイルへ）・`initial_install.nar` を探す所（`boot_config.rs` の exe の場所 `exe_location` を使う）・`dist/README.txt`。
- 規模: S〜M（6〜9 タスク）。ゴーストが 1 体も無いときの起動の前のインストール（議題 2）を入れると 9〜12。
- 先に要るもの: `startup-options-core`・`readonly-mode`（readonly の印・`install/desk.rs` の守りを先に作る）。ファイルの上では `readonly-mode`（`install/desk.rs`）・`install-live-target-hazards`（インストールの道）と同じウェーブに置かない。
- 優先度の区分: **A**（開発者の指示「SSPなみの起動オプション」「SSP並みの自動試験」）。
- 版: **v0.1.0**。
- 要件定義のモデル: **Opus**。インストールの手続きは今ある窓口へ並びを渡すだけで、`callghost` の扱いは開発者の裁定に問う議題（骨格の分かれ目ではあるが、答えは開発者が出す）。新しい境界・並行の設計には当たらない。
- 要件の段の議題（答えで作業が変わるもの）:
  1. **`--option callghost` と 2026-09-28 の裁定**（「インストールと切り替えは異なるイベント・areka は主導しない」）。`callghost` は起動の行を書いた利用者の明示の指示なので、裁定の「利用者が明示的に呼び出す」に当たるとして、入れた後にそのゴーストへ切り替える（または 2 体目として呼ぶ）か。裁定どおり受け取るだけで何もしない（記録に残す）か。推し: 利用者の明示の指示として認め、メニューの「ゴースト」から選んだのと同じ切り替えの道を通す（areka が自分から決めるのではなく、利用者が起動の行で選んだ）。ただし裁定の文言に触れるので、開発者に問う。答えで、切り替えの道を呼ぶ 2〜3 タスクが入るか外れるかが変わる。SSP で測る（`callghost` で入れたとき、`OnGhostChanging` などの切り替えの知らせが出るか・「呼び出し」が切り替えか 2 体目か）。
  2. ゴーストが 1 体も無いとき（初めての起動・空の置き場）の `--install`: SSP は「何とかして下さい」の画面の代わりに入れてから起動を続ける。areka はインストールの窓口が窓のアプリの中にあるので、起動の解決の前に入れるには窓口の外から手続きを呼ぶ道が要る（3〜4 タスク増）。入れるか、「ゴーストが居るときだけ」に絞るか。
  3. `initial_install.nar` の確認なしの上書き: 今のインストールの手続きに利用者へ確かめる段（`crates/areka/src/install/terms.rs`・`overwrite.rs` の起動中のゴーストへ入れる一周）がどこにあるかを要件の段で確かめ、あればその段を飛ばす印を足すか。
  4. 起動の引数のインストールが終わるまで、ゴーストのトークを待たせるか（SSP は「インストール処理を行った後起動を続行する」）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化（`startup-options-core`・10-19 着地）: `--install`（短い形 `/i`）は「値を空白区切りで何本も取る・まだ扱わない」行として読まれ（`crates/areka/src/cli/parse.rs` 61 行目）、`callghost`・`deletesource` も「まだ扱わない語」として記録して無視される（263 行目）。本 spec はその行と語を効かせる側へ替える。`readonly-mode` はまだ着手されていない＝readonly の印はまだ無い。
- 前提の変化（定常に着いた所）: インストールの窓口に「定常に着いた」知らせの受け口 `install::desk::on_steady`（`crates/areka/src/install/desk.rs` 127 行目・ゴーストの切り替えの `emo2_boot/ghost_switch.rs` 513 行目から呼ばれる）が既に在る。起動の引数の依頼はここから出せる見込み＝`ghost_session.rs`（910 行＝上限の近く・10-18 の 885 から増えた）に触らずに済む。最初の起動の定常でもここが呼ばれるかは設計で確かめる。
- 前提の変化（`mcp-reload`・10-22 着地）: `install/desk.rs` に窓口が忙しいかを答える `is_busy`（150 行目）が足され、読み直しの入口がこれを見て断る＝起動の引数で並べた書庫を扱う間は読み直しが断られる（試験の手順に書く）。`install/desk.rs` は 524→540 行。依頼の出どころの語 `InstallOrigin`（`install/mod.rs` 40 行目・`Menu`・`Script`・`WindowDrop`）は変わっていない。
- 前提の変化（`real-run-mcp-kit`・10-19 着地）: 実機の試験の窓口（`crates/real-run-kit/`）は起動の引数を渡せる（`launch.rs` の `arg`）＝「入れる → 起こす → MCP で確かめる」の実機の確かめはこの窓口の上で書ける。
- 触るファイル: `cli/mod.rs`・`cli/parse.rs`・新しい兄弟のテスト（`cli/install_args.rs` を分けるかは設計）・`install/mod.rs`（出どころの語 1 つ）・`install/desk.rs`（依頼と結果を受ける所・`on_steady`）・`install/judge.rs` か `procedure.rs`（成功を受けて NAR を消す）・`initial_install.nar` を探す所（`boot_config.rs` 145 行目の `exe_location` を呼ぶだけ）・`dist/README.txt`（「■ 起動オプション」の 59 行目の文）。
- 規模: 6〜9（議題 2 を入れると 9〜12）のまま。
- 先に要るもの: `readonly-mode`（readonly の印と `install/desk.rs` の守り）。同じウェーブに置かない相手: `readonly-mode`・`offline-surface-dump`（`cli/` と説明書の同じ文）・`install-live-target-hazards`（測ることが本体・インストールの道）。依存は足さない・emo-text に触らない。静かな机・全体テストの比べは要らない。議題 1 は開発者の裁定（2026-09-28）に触れる＝要件の段で開発者の答えを待つ。`callghost` の振る舞いは SSP で測る。
- 優先度の区分: A のまま。
- 要件定義のモデル: Opus のまま（今ある窓口へ並びを渡すだけ・`callghost` の扱いは開発者が答える議題）。
- 分割の案: なし。
- 見つけた穴・古くなった記述: 棚卸㉗の節の「定常に着いた所の結線（`ghost_session.rs` 885 行＝上限の近く）」は、今は 910 行で、`install::desk::on_steady` を使えば触らない見込み。`install/desk.rs` の行数は 540。Downstream の `real-run-mcp-kit` は 10-19 に着地済み（足場として使う側になった）。

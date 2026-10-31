# Brief: install-before-boot

> 2026-10-30 `startup-install-option` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・完了した spec からの引き渡し）。起票の条件: `startup-install-option` の要件 10.2（要件の討議 議題 2 の決め）で「SSP のように起動の解決の前に入れてから起動を続ける形は、完了の段で別の spec として起票する」と決まっている。開発者の指示「SSPなみの起動オプション」（2026-10-18・v0.1.0 のゴール）の続き。正典は SSP ヘルプ「起動オプション一覧」の `--install` と `initial_install.nar`（ukadoc MCP の `ssphelp:option`・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）。区分 **A**。

## Problem

- **利用者・配る人**: exe の隣に `initial_install.nar` を置いてゴーストを同梱して配っても、置き場にゴーストが 1 体も無いと、areka はそれを入れずに止まる。空の置き場から最初のゴーストを入れて起こす使い方ができない。
- **開発者・AI のエージェント**: 空の試験の根に `--install a.nar` を渡して「入れる → 起こす → MCP で確かめる」を回すことができない。今は根に既定のゴースト（emo2）を先に写しておく必要がある。
- SSP は `--install` を「インストール処理を行った後起動を続行する」と書き、ゴーストが入っていないときも、入れてから起動を続ける。

## Current State

- `startup-install-option` は、`--install` と `initial_install.nar` を最初のゴーストが定常に着いた後に入れる（`crates/areka/src/install/desk.rs` 127 行目の `on_steady`）。インストールの窓口は窓のアプリの中（World）にあり、窓のアプリが立った後でないと使えない。
- 起動の持ち物は `crates/areka/src/install/startup.rs` の `StartupInstall`（472 行）。`main` が起動の解決の前に作り、`install::startup::install` で World の資源へ移す（`crates/areka/src/main.rs` 185 行目と 259 行目）。
- 起動の解決（根 → ゴースト → バルーン → 構成の入力）は `main.rs` 213 行目の `resolve_boot`。ゴーストが見つからなければ既定のゴーストの台詞の道（`alert::raise`）で伝えて終了コード 1 で終わる。
- そのとき `StartupInstall` は落とされ、`Drop` が `startup_install_not_submitted reason=boot_stopped` の warn を 1 件残す（`startup.rs` 115 行目・`startup-install-option` 要件 7.1）。入れる物があっても入れない。
- 手続きの本体 `install::procedure::run_order`（`procedure.rs` 119 行目）は `InstallOrder` と `InstallPorts`（トレイト）を受ける形で、World そのものは受けない。窓口の外から呼べるかは、ports の中身（利用条件の画面・警告・ゴーストへの知らせ・記録）しだい。
- readonly の印は窓のアプリの外からも `readonly::is_on()` で見られる。
- `main.rs` は 984 行（1,000 行の上限の近く）。足す分は `main.rs` の外に置く。

## Desired Outcome

- `--install` の書庫と `initial_install.nar` を、起動の解決（ゴースト・バルーンを選ぶ）の前に順に入れてから起動を続ける。置き場が空でも、入れたゴーストで起動できる。
- 入れた結果（入った・失敗・断った）は今と同じ語で記録に残る。`--option deletesource` は「入った」書庫だけを消す（今の決まりのまま）。
- 起動の解決は、入れた後の置き場で行う。`--option callghost` で起こすゴーストの選び方、`bootghost`（`install-package-type`）との関係は要件の段で決める。
- readonly のときは今どおり断る。

## Approach

- 起動の解決の前に、窓口の外から `run_order` を呼ぶ道を作る（ports を起動の前の形で用意する）。利用条件の画面・インストールの警告（ダイアログでよい＝2026-10-17 の例外）は窓のアプリが無くても出せるかを、要件の段で確かめる。
- ゴーストが居るときも起動の前に入れる形にそろえるか、ゴーストが居ないときだけ起動の前に入れるか（今の定常の後の道を残すか）は要件の段で決める。SSP の書き方は前者に読める。
- 起動中のゴーストへ知らせる段（`OnInstallComplete` など）は、起動の前にはゴーストが居ないので送れない。SSP がどうしているかを SSP の MCP で測る（打つ前に許し）。

## Scope

- **In**: 起動の解決の前に入れる道・起動の前の ports・入れた後の置き場での起動の解決・`boot_stopped` の記録の扱いの見直し・決定論のテスト（空の根に入れて起動の解決が当たる・失敗の並び）・実機の確かめ（空の根に `--install` で入れて MCP の `get_active_ghost_list` で確かめる）・`dist/README.txt` の行。
- **Out**: 起動中の areka へインストールを任せる仕組み（SSTP の受け口）・インストールの手続きそのもの（判定・置き場・上書き）の変更・`callghost` を 2 体目として呼ぶ形（`ghost-call`）。

## Boundary Candidates

- 起動の前の入れる道（`install/startup.rs` と新しい兄弟のファイル）
- 起動の解決を呼ぶ所（`main.rs` 213 行目の前後・足す分は `main.rs` の外）
- 定常の後の道（`install/desk.rs` の `on_steady`）を残すか・どこまで寄せるか

## Out of Boundary

- `--install` の値の読み方（`cli/` は `startup-install-option` で済んだ）
- 切り替えの手続きそのもの

## Upstream / Downstream

- **Upstream**: 完了 spec `startup-install-option`（`.kiro/specs/completed/startup-install-option/`・要件 7.1 と 10.2）。完了の `ghost-install`・`nar-install`（手続き）。
- **Downstream**: 空の根から回す実機の試験（`real-run-mcp-kit` の足場を使う試験）。`install-package-type`（`bootghost` で起こすゴーストの名前を渡す先）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の要件 10.2 の続き）。
- **Adjacent**: `install-package-type`（`bootghost`・同じ起動の入口）・`install-live-target-hazards`（使っている最中のフォルダへの上書き）・`session-module-gather`／`session-crate-split`（`main.rs` のシームと `boot_config` を `session/` へ移す＝同じウェーブに置かない・パスは roadmap の「本体の crate 分け」の対応表で読み替える）・`multi-ghost-runtime`（`boot_config.rs`・`ghost_session.rs`）・`background-result-tick-wake`・`actor-thread-log-capture`（`install/desk.rs`）。

## Constraints

- 失敗はメッセージボックスで告げない（既定のゴーストの台詞）。ただしインストールの警告はダイアログでもよい（2026-10-17 の例外）。
- 失敗の道は必ず記録（`error!`＋`Err`）。
- テストは兄弟のファイル `*_tests.rs`。`main.rs` は 1,000 行の上限。
- 試験の根・NAR の写しはワークツリーの `target\` の下だけ。
- 2026-09-28 の裁定（インストールと切り替えは別のイベント・areka は主導しない）を守る。

- **優先度の区分**: A（開発者の指示「SSPなみの起動オプション」の続き・`startup-install-option` 要件 10.2）。
- **版**: v0.1.0（`startup-install-option` と同じ版のゴール「SSP並みの自動試験」の続き。版の門を増やすかは次の棚卸で決める）。
- **要件定義のモデル**: Opus。今ある手続き `run_order` を起動の前から呼ぶ形で、新しい境界・並行の設計には当たらない。
- **規模**: S〜M（6〜10 タスクの見込み・見積もりは要件の段）。
- **分割の案**: なし。
- **要件の段の議題**（答えで作業が変わるもの）:
  1. ゴーストが居るときも起動の前に入れるか、居ないときだけか。
  2. 起動の前に入れたゴーストへ `OnInstallComplete` などを送るか（SSP で測る）。
  3. 利用条件の画面と警告を、窓のアプリの前に出せるか。

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: 起票（10-30・`a1bf0556`）の後に触る所へ入ったのは `window-work-area-clamp`（`main.rs` の `restore_merged_placements` の戻り値を 1 つ足した）と `balloon-scroll-arrows`（`ghost_session.rs` 7 行）だけ。本文の位置は今も当たる（`install/desk.rs` 127 行目の `on_steady`・`install/startup.rs` 115 行目の `boot_stopped`・`main.rs` 185・213・259 行目・`install/procedure.rs` 119 行目の `run_order`・`readonly.rs` 33 行目の `is_on`）。ports の本番の中身は `install/worker.rs` 123 行目の `DeskPorts`（World の側）＝起動の前の形は別に要る。起動の解決は `boot_config.rs` 200 行目の `resolve_boot` が根からゴーストまでを 1 つの関数で決める＝根を決めてから入れ、その後でゴーストを選ぶには、根の段とゴーストの段を分けるか、先に解いた根を渡す形が要る（設計で決める）。
- **触るファイル**: `install/startup.rs`（472）・新しい兄弟のファイル（起動の前に入れる道と ports・印: 新）と兄弟のテスト・`install/worker.rs`（276）・`install/desk.rs`（565・定常の後の道を残すか寄せるか）・`boot_config.rs`（662）・`main.rs`（**986**・呼ぶ所だけ・足す分は外へ）・`callghost` の扱いしだいで `emo2_boot/ghost_switch.rs`・実機の結合テスト `crates/areka/tests/startup_install_real_run.rs`（111）か新しい兄弟・`dist/README.txt` の起動オプションの行。`ghost_session.rs` は触らない見込み（`startup-install-option` も触っていない）。
- **規模**: S〜M（6〜10）のまま。
- **先に要るもの**: 働きの上では無い（`startup-install-option` は着地済み）。同じウェーブに置かない相手（ファイル）: `config-parse-diagnostics`・`dev-folder-alias`・`shell-companion-balloon`・`multi-ghost-runtime`（`boot_config.rs`）・`session-module-gather`／`session-crate-split`（`main.rs`・`boot_config.rs`・`install/desk.rs`）・`install-live-target-hazards`（`install/` の手続き）・`actor-thread-log-capture`（`install/desk_overwrite_tests.rs`）。v0.1.0 の残りの `script-small-tags`・`placement-measure-bake-once`・`shell-scale-linkage`・`balloon-communicatebox` とは 0（あちらは `ghost_session.rs`・`emo2_boot/` の側）。1 本だけの席は取らない。実機の確かめは本物の窓が出る（排他テストの机）。議題 2 は SSP の MCP で測る（打つ前に許し）。
- **優先度の区分**: A のまま（開発者の指示「SSPなみの起動オプション」・`startup-install-option` 要件 10.2）。
- **版の門（推奨）**: v0.1.0 の版の門に**数える**。訳: v0.1.0 のゴールの 1 つ目「SSP並みの自動試験」に直に当たる（Problem の 2 つ目＝空の試験の根に `--install` を渡して「入れる → 起こす → MCP で確かめる」を回せない）。ぱすたの手順書 `doc/pasta-real-run.md` も、新しい書庫は起こした areka の窓へ投げ込んで入れる手順のまま（248 行目）＝着地すれば空の根に `--install` で入れる 1 行で済む。
- **要件定義のモデル**: Opus のまま（今ある手続き `run_order` を起動の前から呼ぶ形・新しい境界や並行の設計に当たらない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文 Current State の「`main.rs` は 984 行」は今 986 行。roadmap の台帳の行も「984 行」と書く。

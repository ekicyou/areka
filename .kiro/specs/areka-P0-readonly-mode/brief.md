# Brief: areka-P0-readonly-mode

> 2026-10-18 棚卸㉗で起票（開発者の指示「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします」＝v0.1.0 のゴール）。根拠は棚卸㉗の調べ（SSP の起動オプションと areka の今の起動の比べ・areka が書く場所の数え上げ・main `2d86dbff`）。正典は SSP ヘルプ「起動オプション一覧」の `--option readonly`（ukadoc MCP の `ssphelp:option`・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）。区分 **A**。版 **v0.1.0**。

## Problem

- **開発者・AI のエージェント**: 自動の試験で areka を立てると、areka は起動中の印・最後に使ったゴースト・最後のシェルとバルーン・窓の位置・ゴーストのプロパティ・置き場の並びを記憶に書き、利用者ごとの置き場（`%LOCALAPPDATA%\areka`）を作って exe の隣の記憶を写しうる。試験のたびに記憶が変わるので、次の試験の起こり方が前の試験に左右される。今は `AREKA_PROFILE_DIR` と `AREKA_USER_ROOT` を `target\` の下へ向けて逃がしているが、逃がし忘れると開発者の本物の記憶を書き換える。逃がす環境変数の組は試験ごとに書き足していて（実機の試験 3 本がそれぞれ持つ）、1 つ足りないだけで本物へ書く。
- SSP には `--option readonly` がある。「ユーザー環境に対する永続的な変更を一切行わずに起動する」。画面の振る舞いとゴーストとの対話はふだんどおりで、設定を変えることもできるが、終わるときにファイルへ書き戻さない。`--sstp-listen` と組んで「起動中のほかの SSP と干渉しない試験用の SSP を立てる」使い方が正典に書いてある。
- 利用者にも、借りた PC や試しの起動で設定を汚さずに動かす使い道がある。

## Current State

- 起動オプションを読む口は無い（`startup-options-core` が作る）。
- areka が書く場所（2026-10-18 に数えた）:
  - 記憶の 13 種（`crates/areka/src/write_failure_sites_tests.rs` の `KINDS`）: アプリの記憶＝起動中の印を書く・降ろした直後・定常の印・印を消す・最後に使ったゴースト・置き場の並び、ゴーストごとの記憶＝最後に使ったバルーン・最後に使ったシェル・最後のシェルだけ・最後のバルーンだけ・入れたバルーン・窓の位置・ゴーストのプロパティ。どれも sylphya の保存の口 `save_scope`（`crates/areka-sylphya/src/persist/mod.rs` 276 行目）を通る。呼び手は 3 か所（`crates/areka-sylphya/src/actor.rs` の保存の流し込み・`crates/areka/src/boot_resolve.rs`・`crates/areka/src/roots/edit.rs`）で、書き込みは `PersistIo` の形（`crates/areka-sylphya/src/persist/io.rs` の `FsPersistIo`）を通る。
  - 起動のときの書けるかの確かめ（`crates/areka/src/write_failure.rs` の `check_boot_dirs`＝試しに書いて消す）。
  - 利用者ごとの置き場を作り exe の隣の記憶を写す（`crates/areka-user-data/src/establish.rs`・`copy.rs`）と、書けないゴーストの写し（`crates/areka/src/roots/user_root.rs` の `writable_ghost`）。
  - 右クリックメニューの置き場の並びの編集（`crates/areka/src/roots/edit.rs`）。
  - インストール（`crates/areka/src/install/`・`crates/areka-nar/src/install.rs`）・ネットワーク更新（`crates/areka/src/update/desk.rs`・`crates/areka-update/`）・URL から落とす書庫の置き場（`crates/areka/src/install/fetch_url.rs`）。
- SHIORI（pasta.dll など）が自分で書く保存は areka の外（SSP の readonly も対象にしない）。
- 書けないときの伝え方は完了 `write-failure-notice` が作った（書けなければ既定のゴーストの台詞で伝えて終わる）。readonly はこの道へ入ってはいけない（書かないのは失敗ではない）。

### SSP の正典（2026-10-18 に ukadoc MCP で読んだ `ssphelp:option` の `--option readonly`・2.8.96〜）

- `--option bootunlock,standalone` が有効になる（起動中の SSP へ任せると、そちらが書いてしまうため）。二重起動の確かめに一切干渉しない。
- 本体・ゴースト・シェル・バルーン・プラグインの設定を一切保存しない。起動の履歴・使った数・キャッシュ・vanish の記録・SSTP の設定などを保存しない。
- レジストリに書かない。関連付け・ショートカット・ジャンプリスト・最近使った項目を作らない。壁紙を変えない。ごみ箱を空にしない。vanish してもゴーストのフォルダを消さない。
- 自動の更新をしない。**手動のネットワーク更新・NAR のインストールもしない**。`initial_install.nar` を入れない。時刻合わせをしない。
- 設定の値は変えられるが、終わるときに書き戻さないので次の起動に引き継がない。動くのに要る一時ファイル（一時フォルダ）は作り、終わるときに捨てる。
- `--option kiosk,readonly` と組める（kiosk とは別のもの）。

## Desired Outcome

- `areka.exe --option readonly …` で起こした areka が、走る間も終わるときも、記憶の置き場・利用者ごとの置き場・ゴーストのフォルダ・バルーンのフォルダに 1 つもファイルを作らず、変えず、消さない（一時フォルダを除く）。
- 画面の振る舞いとゴーストとの対話はふだんどおり。記憶に書くはずだった値（窓の位置・プロパティなど）は、走る間は今までどおり読み書きでき、終わるときに捨てる。
- インストール・ネットワーク更新・URL からの書庫の取り込みは、断ったことを記録に残して行わない（ゴーストへの知らせは SSP に合わせる・議題）。
- 書けないときの伝え方（`write-failure-notice`）の道には入らない。
- 「13 種の記憶がどれも書かれない」「置き場に 1 つもファイルが増えない」を決定論のテストで固定する（`write_failure_sites_tests.rs` の 13 種の表を裏返して使う）。

## Approach

- `startup-options-core` が作る起動オプションの値に `readonly` を足し、起動のはじめに「読み取りだけ」の印をプロセスに 1 つ持つ。
- 記憶の 13 種は、sylphya の保存の口の 1 か所（`save_scope` が使う `PersistIo` を「書かない」形に差し替える）で止める。呼び手 3 か所には手を入れない。
- そのほかの直接の書き込み（書けるかの確かめ・利用者ごとの置き場を作る・ゴーストの写し・置き場の並びの編集・インストール・更新・書庫の取り込み）は、それぞれの入口で印を見て、書かずに記録を残す。
- 書かない形は失敗ではないので、`write-failure-notice` の判定（書けたか）には「書かなかった」を成功として返す（議題）。

## Scope

- **In**: `--option readonly`（`bootunlock,standalone` を含む扱い）・プロセスの印・sylphya の保存の口の差し替え・直接の書き込みの各入口の守り・インストールと更新と書庫の取り込みを断る扱い・`write-failure-notice` の判定との合わせ・決定論のテスト（13 種の表の裏返し・置き場のファイルの数が変わらない）・`dist/README.txt` の節。
- **Out**: SHIORI が自分で書く保存・一時フォルダ・`--option kiosk`・SSP の SSTP・POP・メッセンジャー・カレンダー・セーブデータ共有（areka に無い）・`initial_install.nar` の受け取りそのもの（`startup-install-option`。readonly のときに入れないことはこちらで守る）。

## Boundary Candidates

- プロセスの印（起動オプション → 印）と、印を見る所の境。
- sylphya の保存の口（`PersistIo` の差し替え＝1 か所で 13 種を止める）。
- 直接の書き込みの入口（write_failure・roots・install・update・user-data）。

## Out of Boundary

- 記憶の形と置き場の決め方（完了 `user-data-root`・`baseware-root-list`）。
- 書けないときの伝え方そのもの（完了 `write-failure-notice`）。

## Upstream / Downstream

- **Upstream**: `startup-options-core`（起動オプションの値）。
- **Downstream**: `real-run-mcp-kit`（環境変数で逃がす代わりに `--option readonly` で起こせる）・`startup-install-option`（readonly のときに `initial_install.nar` と `--install` を断る）・`dev-folder-alias`（別名のフォルダへ書かない守りを同じ印で作れる）・`pasta-in-windows-verification`（開発者の本物の記憶を汚さずに検体を回せる）。

## Existing Spec Touchpoints

- **Extends**: 完了 `write-failure-notice`（13 種の表）・完了 `user-data-root`（置き場を作る・写す）。完了 spec は直さず、本 spec が新しい決まりを持つ。
- **Adjacent**: `multi-ghost-runtime`（`persist/mod.rs` の `LastGhost`・`LastRunning`）・`property-name-case-fold`（保存の読み戻しまで畳む案のときだけ `persist/`）・`startup-install-option`（`install/desk.rs`）・`dev-folder-alias`（書かない守り）・`install-live-target-hazards`（インストールの道）・`update-check-options`（更新の道）。

## Constraints

- 失敗はメッセージボックスで告げない。断ったことは記録（info か warn）に残す。失敗の道は必ず記録（`error!`＋`Err`）。
- テストは兄弟のファイル `*_tests.rs`。`main.rs` は 1,000 行の上限＝印を立てるのは `startup-options-core` の作った口の中。
- 試験の根はワークツリーの `target\` の下だけ。開発者の本物の `%LOCALAPPDATA%\areka` を測りに使わない。
- 時刻の扱いと並行の作りは変えない（書くか書かないかだけを変える）。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: `crates/areka/src/cli/`（`startup-options-core` が作る・項目を足す）・新 プロセスの印の小さなモジュール（例 `crates/areka/src/readonly.rs`＋兄弟のテスト）・`crates/areka-sylphya/src/persist/io.rs`（書かない形）・`persist/mod.rs`（451 行・差し替えの口）・`crates/areka/src/write_failure.rs`（書けるかの確かめ）と `write_failure_sites_tests.rs`（13 種の表の裏返し）・`crates/areka/src/boot_resolve.rs`・`boot_config.rs`（置き場を作る・写すの呼び出し）・`crates/areka/src/roots/{user_root,edit}.rs`・`crates/areka-user-data/src/establish.rs`・`crates/areka/src/install/{desk,fetch_url}.rs`・`crates/areka/src/update/desk.rs`・`dist/README.txt`。
- 規模: M〜L（12〜16 タスク）。上限 20 の内。
- 先に要るもの: `startup-options-core`。ファイルの上では `multi-ghost-runtime`（`persist/mod.rs`・`boot_resolve.rs`）・`startup-install-option`（`install/desk.rs`）・`dev-folder-alias`（`boot_config.rs`・`boot_resolve.rs`・`update/desk.rs`・`persist/`）・`mcp-reload`（`update/desk.rs` の同じゴーストへの載せ替えを読む）と同じウェーブに置かない。`startup-install-option` より先（readonly で断る所を先に作る）。
- 優先度の区分: **A**（開発者の指示「SSPなみの起動オプション」「SSP並みの自動試験」）。
- 版: **v0.1.0**。
- 要件定義のモデル: **Opus**。書く所の数え上げは `write-failure-notice` の 13 種の表が先例で、止める所は今ある保存の口 1 か所。新しい境界・並行の設計・先例で決まらない骨格の分かれ目のどれにも当たらない。
- 要件の段の議題（答えで作業が変わるもの）:
  1. 起動中の印（前回きれいに終わらなかった印）を readonly で書かないと、走る間に落ちても次の起動は気付かない。SSP の readonly は二重起動の確かめに一切干渉しない＝印も書かない読み。areka も書かない、でよいか。推し: 書かない（SSP どおり）。
  2. 利用者ごとの置き場がまだ無い（初めての起動）とき、readonly でどこから読むか（exe の隣の記憶を読むだけ・写さない）。
  3. インストールと更新を断ったとき、ゴーストへ何を知らせるか（`OnInstallFailure` などの理由の語・何も知らせない）。SSP で測る（readonly の SSP へ NAR を投げ込む・`\![updatebymyself]` を流す）。
  4. 書かなかった保存を `write-failure-notice` の判定で「成功」と返すか、判定の手前で道を分けるか（どちらでも利用者には何も出ないが、記録の行が変わる）。
  5. ゴーストのフォルダの中へ areka が書くもの（シェルの `profile` など）が残っていないかの数え直し（13 種の表の外）。要件の段で 1 度数える。

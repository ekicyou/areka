# ギャップ分析: areka-P0-user-data-root

> 実測は 2026-10-16・この枝（`claude/areka-P0-user-data-root-12e486`、main `0bb51d42` の上に spec の初期化 1 コミット）。コードは関数名・型名＋ファイルパスで指す。
> 入力: `requirements.md`（要件 1〜10・議題 1〜11）・`brief.md`（10-16 の再測定とウェーブの約束を含む）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）。
> この文書は情報と選択肢を並べるもので、決めるのは要件の討議と設計。

## 1. 要約

- **土台はほぼそろっている。** 根の並び（`crates/areka/src/roots/`）・「新しく入れる物は見つかるいちばん上の根へ」（`areka_ghost::catalog_roots::install_root`）・作業フォルダは宛先の根ごと（`areka-nar` の `WorkArea`）・上の根が下の同じフォルダ名を隠す規則（`catalog_roots::union`）が在る。書ける根を並びの先頭に置けば、新しく入れる物・`.nar-work\`・起こしたゴーストの記憶とシェルの記憶・ゴーストとシェルのネットワーク更新は、**コードを足さずに**書ける根へ行く。
- **足りないのは 4 つ。** ⑴ 書ける根を使う形かどうかの判断（winget の入れ先の見分け・書けるかの見分け・環境変数）、⑵ 書ける根の場所と、並びの先頭への差し込み（`resolve_roots_from` の入力が 1 つ増える）、⑶ areka の記憶の置き場の切り替えと引き継ぎ（`default_app_profile_dir`）と中継の追随（`areka-mcp-bridge` の `Config::from_values`）、⑷ **同梱の物を書ける根へ写す仕組み**（今は無い。要件 4.3〜4.9）。
- **要件 4（写してから使う）は実現できる。** フックの場所は、最初の起動＝`boot_config::resolve_boot_from`（`boot_config.rs`）、切替・既定ゴーストへの戻し・`\![change,ghost,…]`＝`emo2_boot/ghost_switch.rs` の `boot_into` の 2 か所で全部の起こし方を覆え、**ウェーブの約束の 3 ファイル（`main.rs`・`boot_resolve.rs`・`roots/edit.rs`）に触らずに済む見込み**。入れ直し・supplement は `install/destination.rs`、バルーンのネットワーク更新は `update/desk.rs` に別のフックが要る。
- **注意する点**: 切替の写しは UI スレッドの上で起きる（大きいゴーストで固まる）・写しの途中の失敗で半端な物を残さない形（作業フォルダで組んで `rename`）・同梱のゴーストが新しくなっても写しが隠し続ける（要件 4.9 の代償。今の記録 `root_hidden_item` は info で毎回出る）。
- **調べること**: winget の入れ先の見分け方（入れ先に winget が置く `<フォルダ名>.db`・ARP のレジストリの値・`--location` と `portablePackage*Root` の付け替えでの振る舞い）、書けるかの見分け方（試し書きか権限の問い合わせか）。

## 2. 今のコード（関係する所）

### 2.1 根の並び（完了 `baseware-root-list`）

| 物 | 場所 | 今の振る舞い |
|---|---|---|
| 並びの決め方（純粋） | `crates/areka/src/roots/resolve.rs` の `resolve_roots_from(env, memory, exe_dir)` | `AREKA_ROOT` → 記憶の `[baseware] roots` → 既定（実行ファイルのフォルダ 1 つ）。重複は上を残す。見つからない根も並びに残し、`status.found` は `is_dir` で決める。どれも見つからなければ `RootError::NoneFound` |
| 薄い読み口 | `roots/mod.rs` の `resolve_roots(app_profile_dir)` | プロセスの環境変数・記憶のファイル・`exe_location` を読んで上へ渡す（読むのはここだけ） |
| 記憶の綴り | `roots/resolve.rs` の `decode_memory`・`encode_memory` | 実行ファイルのフォルダは `<exe>`、ほかは絶対パス。区切りは `\|` |
| 並びの変更 | `roots/edit.rs` の `decide_add`（末尾に足す）・`decide_remove`（1 つだけなら外せない）・`apply`（`BootContext.roots` と記憶を書き換える） | **ウェーブの約束で触らない** |
| メニュー | `crates/areka/src/menu/roots_frame.rs` の `remove_item` | 並びの全部の根を子に並べ、根が 2 つ以上ならどれも選べる |
| 記録 | `roots/report.rs` の `log_roots`（`root_listed`・出どころ `origin`）・`log_hidden`（`root_hidden_item`・**info**） | 隠れは起動と並びの変更のたびに info で残る |

### 2.2 和・隠れ・入れる根（`areka-ghost`）

- `crates/areka-ghost/src/catalog_roots.rs`: `list_ghosts`／`find_ghost`／`list_balloons`／`find_balloon` は「見つかる根を上から走査し、同じフォルダ名（`fold_key`）は上を残す」。`GhostEntry` は自分の `root` を持つ＝**ある項目が実行ファイルの隣の根から来たかは `entry.root` と実行ファイルのフォルダの `same_root` で分かる**。
- `install_root(roots, kind, folder)`: 見えている物が在ればその根（＝入れ直しはその場で上書き）、無ければ見つかるいちばん上の根。`head_skipped` は先頭の根が見つからなかったとき。

### 2.3 インストール

- `crates/areka/src/install/destination.rs` の `CatalogDestinations`（`areka_nar::InstallRoots` の実装）: `ghost_root`／`balloon_root` は `install_root` の答え。起動中のゴーストと同じフォルダ名なら起動中のゴーストの根（`running_root`）。答えの型は `PathBuf` で、**失敗を返せない**。
- `install_into(roots, archive, target_ghost, running)`: 見つかる根が無ければ `install_no_root`。入れた後に `install_placed`。
- `crates/areka-nar/src/install.rs`: 宛先の木を作業フォルダ（`<根>\.nar-work\<pid>-<連番>\`）へ下敷きとして写し（`copy_existing`・`Overlay`／`Replace{keep}`）、書庫の中身を重ね、最後に `rename` で入れ替える。**下敷きの読み元と書き先は同じ `placement.destination`**。`WorkArea::create` は根が実在しないと失敗する（根は作らない）。

### 2.4 記憶の置き場

- `crates/areka/src/boot_config.rs` の `default_app_profile_dir()`: `AREKA_PROFILE_DIR` → `exe_location()` の親の `profile\areka\` → `"."`。**呼ぶたびに環境変数を読む**。呼び手は `boot_config::resolve_boot`・`ghost_boot_options`・`ghost_session::GhostBootInputs::production`・`shiori_host.rs`・`main.rs`（`BootContext.app_profile_dir` を詰める所）。
- `exe_location()`: `OnceLock` で 1 回だけ解いて覚える（リンクを辿る `follow_exe_links`）。**「1 回だけ決めて覚える」前例**。
- ゴーストの記憶とシェルの記憶: `areka_ghost::sylphya_wiring::profile_areka_root(dir)`＝`dir\profile\areka\`。呼び手は `areka-ghost/src/runtime.rs`（`mount.shiori.dir`・`mount.shell.dir`）・`boot_resolve.rs` の `ghost_roots`・`placement/persist.rs`。**どれも渡されたゴーストのフォルダから導く＝写しから起こせば写しの中になる**（要件 7.3 はそのまま満たせる）。
- シェルの記憶（`PersistScope::Shell`）: 本番のコードで書く呼び手は見当たらない（`areka-sylphya` の読み込みの列にだけ現れる。`LastShell` 等の鍵の書き先の層は設計で確かめる）。＝要件 8.3 の「作る操作が無い」の見込み。

### 2.5 ネットワーク更新

- `crates/areka/src/update/desk.rs` の `resolve`: ゴースト＝`GhostSession::ghost_dir()`（起こしたフォルダ）・シェル＝起こしたゴーストのシェル・バルーン＝`BootContext.current.balloon.dir`・`updateother` のバルーン＝`list_balloons(roots)` から名前で。
- `crates/areka-update/src/paths.rs` の `WORK_DIR`（`.update-work`）・`work.rs` の `WorkArea::create(target)`: 対象のフォルダの中に作業場所を作り、その場で書く。
- ＝**ゴーストとシェルは、写しから起こせば写しが対象になる**。**バルーンは、表示だけなら写さない（要件 4.5）ので、同梱のバルーンの更新は実行ファイルの隣を対象にしてしまう**（要件 4.4 のフックが要る）。

### 2.6 中継

- `crates/areka-mcp-bridge/src/config.rs` の `Config::from_values(port, profile_dir, exe, areka_beside, local_app_data)`: 控えは `AREKA_PROFILE_DIR\mcp-bridge` → 隣に `areka.exe` が在れば `<exe>\profile\areka\mcp-bridge` → `%LOCALAPPDATA%\areka\mcp-bridge` → なし。`from_env` だけが環境を読む。exe の場所は `current_exe` のまま（リンクを辿らない）。
- 中継の依存は `areka-mcp` だけ（`areka`・`areka-ghost` には依存しない）。`areka-mcp` は areka と中継の**共有の定義**の前例（`PORT_ENV`・`candidates_from_env_value`）。

### 2.7 winget で入れた形の事実（完了 `areka-P0-winget-manifest-submission` の `verification/winget-local-check.md`）

- 利用者向けの入れ先は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\`（手元のマニフェスト。winget-pkgs から入れると後ろの綴りが違う見込み）。機械の全員向けは `%ProgramFiles%\WinGet\Packages\…`。
- `areka.exe` は入れ先の実ファイル（リンクではない。`Links\areka.exe` は作られず、PATH に入れ先が足される＝`ArchiveBinariesDependOnPath`）。
- **入れ先の直下に、winget が置いた隠しファイル `Areka.Areka.Portable__DefaultSource.db`（16384 バイト）が在る**（zip の中には無い）。フォルダ名と同じ綴り＋`.db`。
- winget の利用者の設定 `installBehavior.portablePackageUserRoot`／`portablePackageMachineRoot` で入れ先の根を付け替えられる（測った机では付け替えなし）。

### 2.8 決まりごと（steering・記憶）

- 1 ファイル 1,000 行以下（`structure.md`）。今の行数: `main.rs` 971・`emo2_boot/ghost_switch.rs` 917・`ghost_session.rs` 876・`boot_resolve.rs` 615・`boot_config.rs` 450。**`ghost_switch.rs` は余裕が 83 行**。
- テストは兄弟ファイル（`*_tests.rs`）。実機の根・一時フォルダはワークツリーの `target\` の下だけ（`temp-path-kit` クレートが在る）。
- 新しい外部依存は 0（要件 7.6）。利用者ごとの場所は環境変数 `LOCALAPPDATA` で取れる（前例は中継）。`windows` クレートはワークスペースに在る（レジストリを読むなら機能の追加だけで、新しいクレートは増えない）。
- 本番の環境変数は `AREKA_` で始める・ドメインの語で名付ける。

## 3. 要件ごとの対応表

凡例: **在る**＝今のまま使える／**拡張**＝今の物を伸ばす／**欠け**＝新しく要る／**不明**＝調べが要る／**制約**＝縛りが在る。

| 要件 | 今ある物 | ギャップ |
|---|---|---|
| 1.1 winget の入れ先の見分け | なし | **欠け＋不明**: 見分け方を決める（§5 調べ 1） |
| 1.2 書けないときの見分け | なし（今は書いて失敗するだけ） | **欠け＋不明**: 試し書きか権限の問い合わせか（§5 調べ 2）。zip で書ける形で試し書きを残さないこと（1.4） |
| 1.3 環境変数での上書き | `AREKA_PROFILE_DIR`・`AREKA_ROOT` の前例 | **欠け**: 新しい名前（設計） |
| 1.4 どれでもないときは 0 の変化 | 今の振る舞いそのもの | **制約**: 判断の分岐を足しても今の経路を通る（決定論テストで固定） |
| 1.5 形と理由の記録 | `root_listed` などの記録の前例 | **欠け**: 起動に 1 行 |
| 1.6 尋ねない | — | 在る（何も足さない） |
| 2.1〜2.3 書ける根の場所・作る | 中継の `%LOCALAPPDATA%\areka` の前例 | **欠け**: 場所の決め方（純粋）と作る口。**作るのは並びを決める前**（`status.found` が `is_dir` で決まるため。作る前に並びを組むと書ける根が「見つからない」になり、新しく入れる物が実行ファイルの隣へ行く＝`head_skipped`） |
| 2.4 決められない・作れない | — | **欠け**: 今と同じ形へ倒す（error） |
| 2.5 ほかの根と同じ形 | `BasewareRoot` | 在る |
| 3.1 既定の 2 つの並び | `resolve_roots_from` の既定の腕（実行ファイル 1 つ） | **拡張**: 入力に「書ける根（任意）」を足し、既定・記憶の腕で先頭へ差し込む |
| 3.2 記憶の並びの先頭へ（重ねない） | 重複は上を残す処理 | **拡張**: 差し込んでから既存の重複除去を通せば「2 回並べない」は付いてくる。ただし**記憶の中の書ける根を先頭へ動かす**（記憶の順を変える）ので、重複除去の前に取り除いてから先頭に置く |
| 3.3 外した後に先頭へ戻す | 起動のたびに解く | **在る（3.2 の結果）**。ただしその回のうちは `BootContext.roots` に書ける根が無い＝新しく入れる物が実行ファイルの隣へ行く＝**要件 4.1 と食い違う**（議題 11）。`roots/edit.rs` に触らずに塞ぐ案は `menu/roots_frame.rs` で書ける根の子を選べなくする（§4 の決めること 10） |
| 3.4 `AREKA_ROOT` のときは足さない | 環境変数の腕 | **拡張**（腕を変えない）。**`AREKA_ROOT` と書ける根を使う形が重なるときの写しの扱いが未定**（§4 の決めること 11） |
| 3.5 記憶に書かない | 今も既定のときは書かない | 在る |
| 3.6 並びの記録に出どころ | `RootsOrigin`・`log_roots` | **拡張**: 根ごとの出どころ（既定の書ける根・環境変数）。`RootsOrigin` は並びで 1 つなので、根ごとの印を `ResolvedRoots` に足すか別の行にするか（設計） |
| 4.1 実行ファイルの下に 0 | 下の 4.2〜4.4 の合計 | **制約**: 抜け穴は ⑴ 4.3〜4.4 のフックの無い経路 ⑵ 3.3 のその回 ⑶ 試し書き（1.2）⑷ 32bit の helper の書き込み（下の注） |
| 4.2 新しく入れる物は書ける根へ | `install_root` の「いちばん上」 | 在る（書ける根が先頭に在り、実在していれば） |
| 4.3 起こす前に写す | なし | **欠け**: 写す部品＋フック 2 か所（§3.1） |
| 4.4 書く前に写す（入れ直し・supplement・更新） | なし | **欠け**: `install/destination.rs`（入れ直し・シェル・supplement）・`update/desk.rs`（バルーンの更新）にフック。ゴーストとシェルの更新は 4.3 で付いてくる |
| 4.5 表示だけのバルーンは写さない | 今の振る舞い | 在る |
| 4.6〜4.7 記録・半端を残さない | `areka-nar` の「作業フォルダで組んで `rename`」の型 | **欠け**: 写す部品に同じ型を持たせる |
| 4.8 書ける根に在れば写さない | `union` の隠れ | 在る（`find_ghost` が書ける根の物を返す） |
| 4.9 新しい同梱で上書きしない | 隠れの規則 | 在る（何もしなければそうなる）。**代償の扱いは議題**（§4 の決めること 8） |
| 5.1〜5.3 記憶の置き場 | `default_app_profile_dir` | **拡張**: 中身を替える（`main.rs` を触らない形＝brief の約束） |
| 5.4〜5.5 引き継ぎ | なし | **欠け**: 1 回だけ写す。「記憶が在る」の定義（`sylphya.toml` の有無か、フォルダの中身か）を決める |
| 5.6 1 つに決める | 呼ぶたびに環境変数を読む関数 | **拡張**: 1 回だけ決めて覚える（`exe_location` の `OnceLock` の型）。テストでは注入できる純粋な判断と分ける |
| 6.1〜6.3 中継 | `Config::from_values` | **拡張**: 入力を足す。**判断を areka と中継で 2 か所に持つか、共有するか**（§4 の決めること 3） |
| 7.1〜7.6 守る | — | **制約**: 決定論テストで「書ける根を使う形でない」の全部の経路が今と同じであることを固定 |
| 7.4 argv の起動は写さない | `GhostRoute::Argv`・`BootContext.argv_session` | 在る（フックで経路を見て飛ばす） |
| 8.1〜8.5 説明書 | `dist/README.txt` の「■ 置き場」「■ 記憶の置き場」・`doc/COMPAT_ARCHITECTURE.md` §8 の「ベースウェアの格納フォルダ（根）を複数持つ形」の行 | **拡張**（文書） |
| 9.1〜9.3 申し送り | — | 文書と実測（開発者の手） |
| 10.1〜10.7 決定論テスト | `resolve_tests.rs`・`destination_tests.rs`・`config_tests.rs`・`temp-path-kit` | **拡張**: 判断の分岐だけ |
| 10.8〜10.10 実測 | 出どころの spec の手順と道具（`target\winget-check\`） | 開発者の手 |

注（4.1 の抜け穴 ⑷）: SHIORI は 32bit の helper（`shiori-host32-helper.exe`）の中で動く。マニフェストを持たない 32bit のプロセスは、`Program Files` の下への書き込みを `%LOCALAPPDATA%\VirtualStore\` へ黙って逸らされることがある（UAC の仮想化）。写しから起こせば SHIORI は書ける根の中へ書くので本 spec の形では効かないが、**書けるかの見分け（1.2）を helper の側の結果で判断しない**こと、実測で `VirtualStore` に何も出来ていないことを数えるかは設計で決める。

### 3.1 要件 4（写してから使う）の詳しい見立て

**どこで写すか（起こす経路の全部）**

| 起こし方 | 通る所 | ファイル | 約束の 3 ファイルか |
|---|---|---|---|
| 最初の起動（記憶・唯一・既定・無作為・前回がきれいに終わらなかったときの既定） | `boot_config::resolve_boot_from` の `resolve_ghost` の後（argv の経路を除く） | `boot_config.rs` | いいえ |
| メニュー・`\![change,ghost,…]`・`\![call,ghost]` 等の切替 | `ghost_switch::switch_to` → `boot_into` | `emo2_boot/ghost_switch.rs` | いいえ |
| 切替先が失敗したときの既定への戻し | `ghost_switch::switch_to_default` → `boot_into` | 同上 | いいえ |
| ネットワーク更新の後の読み直し | `update/desk.rs` の `reload` → 切替の経路 | 同上 | いいえ |
| 起動の失敗の告知の場所（`main.rs` の `find_ghost(DEFAULT_GHOST_FOLDER)`） | 表示だけ・書かない | `main.rs` | 触らない（写す必要なし） |

- `resolve_boot_from` で写すと、`GhostDecision.dir`・`ConfigInputs.ghost_root` が最初から写しを指すので、`BootContext.current`・`GhostSession::ghost_dir()`・更新の対象・入れ直しの `running_root` がそろって写しになる。`boot_resolve.rs` の `ghost_roots`・`read_last_balloon`・`decide_boot_shell` は渡されたフォルダを読むだけなので、写しを渡せば変えずに済む。
- 代わりに `ghost_session::GhostBootInputs::production`（全部の起こし方の共通の入口）で写す案もあるが、`BootContext.current`・`GhostDecision` が同梱の場所のまま残り、告知・更新・記憶の場所が食い違う。
- **起動の前に写すのは UI の前（窓を開く前）だが、切替の写しは UI スレッドの上**。同梱のゴーストは小さい（0.0.2 の実測で入れ先の `ghost\` は同梱分 143 ファイル程度）が、「書けない所へ zip を展開した」形では実行ファイルの隣に大きいゴーストが置かれうる。記憶「ゴーストの台本の大きさを甘く見ない・UI スレッドの外へ出す」に照らすと、切替の写しは背景で行い、済んでから起こす形が要る見込み（切替の段の状態を 1 つ増やす。§4 の決めること 5）。

**どう写すか（半端を残さない）**

- 書ける根の下の作業フォルダ（例: `<書ける根>\.nar-work\` と同じ置き方の別の棚）へ丸ごと写し、`<書ける根>\ghost\<フォルダ名>` へ `rename`（同じボリュームの中なので 1 手で入れ替わる＝要件 4.7 の「同じフォルダ名のものが現れない」）。写し元は別のボリュームでもよい（写すのは `copy`、入れ替えだけ `rename`）。
- 写す部品は `areka-nar` の `copy_existing` と同じ仕事（読み取り専用の属性を落とす点も同じ）だが非公開。⑴ `areka-nar` から公開して借りる ⑵ 標準ライブラリだけで小さく書く、のどちらか。リンク（ジャンクション）の中を辿るかは決める。
- 写した後の `rename` が「すでに在る」で失敗したら（別のプロセスが先に写した）、写しを捨てて書ける根の物を使う（4.8 と同じ）。

**入れ直し・supplement（要件 4.4）**

- 今は `install_root` が見えている根（実行ファイルの隣）を答えるので、そこへ上書きする。書ける根を答えるだけ（写さない）にすると、シェルや supplement だけを書ける根へ置いた「中身の欠けたゴースト」が同梱の物を隠す＝壊れる。**写してから書ける根を答える必要がある**。
- 置き場の候補: ⑴ `install_into` の前段で、書庫の宛先のフォルダ名（本体の `directory`・`target_ghost`・同梱のバルーン）を引いて先に写す（失敗を返せる） ⑵ `CatalogDestinations::answer` の中で写す（答えの型が `PathBuf` で失敗を返せない） ⑶ `areka-nar` の計画で「下敷きの読み元」と「書き先」を分ける（同梱を読み、書ける根へ書く。写しと入れ替えが 1 回で済み、半端が残らない。ただし `.nar` の展開の中身は範囲外＝境界を広げる判断が要る）。
- インストールは背景のスレッド（`install/worker.rs`）で走るので、写しが重くても UI は止まらない。起動中のゴーストへ入れる一周（`run_overwrite_between`）は起動中のゴースト＝すでに写しなので、写す場面は来ない。

**ネットワーク更新（要件 4.4）**

- ゴースト・シェルは写しが対象（4.3 で付いてくる）。
- バルーン（今のバルーン・`updateother`）は同梱の物が対象になりうる。`update/desk.rs` の `resolve` で写してから写しを対象にし、さらに今表示しているバルーン（`BootContext.current.balloon`）を写しへ替える（更新の後の読み直しの流れとの兼ね合いは設計で確かめる）。

**同梱のゴーストが新しくなったとき（要件 4.9）**

- 何もしなければ、写しが下の同梱を隠し続ける（`root_hidden_item` が起動のたびに info で残るだけ）。areka の版が上がって同梱のゴーストの台本が直っても、winget で入れた利用者には届かない（ゴースト自身のネットワーク更新か、利用者の入れ直しまで）。
- 選べる形（§4 の決めること 8）。

## 4. 実装の形の候補

### 案 A: 今の部品を伸ばす（判断を `boot_config.rs`・`roots/resolve.rs` に足す）

- 書ける根を使う形の判断・場所・記憶の引き継ぎを `boot_config.rs` に置き（`exe_location` と同じく `OnceLock` で 1 回）、`default_app_profile_dir` がそれを読む。`roots/mod.rs` の `resolve_roots` が書ける根を `resolve_roots_from` へ渡す。写す部品も `boot_config.rs` か `install/` の中に置く。
- ✅ ファイルが増えない・前例（`exe_location`・`AREKA_PROFILE_DIR`）の延長。
- ❌ `boot_config.rs`（450 行）に判断・I/O・写しが混ざり、1,000 行に近づく。中継は別に同じ判断を持つ（重複）。

### 案 B: 新しい部品を作る（「書ける置き場」の小さなモジュール）

- 例: `crates/areka/src/user_data/`（判断＝純粋 `decide(入力の値)`・場所・記憶の引き継ぎ・写す部品＝`materialize`）。`boot_config.rs`・`roots/mod.rs`・`ghost_switch.rs`・`install/destination.rs`・`update/desk.rs` はそれを呼ぶ 1〜数行だけ。
- 判断の純粋な部分（「winget の入れ先か」「環境変数」「書けるか」の値を受けて形と理由を返す・場所を返す）は、中継と共有するなら `areka-mcp` か、依存の無い小さなクレートに置く。
- ✅ 判断の分岐を値だけで赤にできる（要件 10.1〜10.5）・`ghost_switch.rs` の余裕（83 行）を食わない。
- ❌ 部品が 1 つ増える・共有先の選び方（`areka-mcp` は MCP の部品なので意味がずれる／新しいクレートは数が増える）。

### 案 C: 混ぜる（推しの形の候補）

- 判断と場所と記憶の引き継ぎは案 B の小さなモジュール（areka 側）＋中継と同じ入力の組で確かめるテスト。中継の判断は、同じ純粋な関数を共有するか、写しを持って「同じ入力の組」のテストで食い違いを赤にする。
- 並びへの差し込みは案 A（`resolve_roots_from` に入力を 1 つ足す）。
- 写す部品は案 B のモジュールに置き、フックは 4 か所（`resolve_boot_from`・`boot_into`・`install_into` の前段・`update/desk.rs` の `resolve`）。
- ✅ ウェーブの約束の 3 ファイルに触らない・既存の型（`OnceLock`・作業フォルダと `rename`）に沿う。
- ❌ 切替の写しを背景へ出すなら、切替の段に「写し中」を足す設計が要る（`ghost_switch.rs` の行数に注意）。

### ウェーブの約束（`main.rs`・`boot_resolve.rs`・`roots/edit.rs`）との照合

| 要件 | 触らずに済むか | 根拠 |
|---|---|---|
| 1・2（判断・場所） | 済む | `default_app_profile_dir` と `roots/mod.rs` の中で閉じる |
| 3.1〜3.2・3.4〜3.6（並び） | 済む | `roots/resolve.rs`・`roots/mod.rs`・`roots/report.rs` |
| 3.3（外した後） | 済む（次の起動から） | 起動のたびに `resolve_roots_from` が差し込む。**その回のうちに効かせるなら `menu/roots_frame.rs` で書ける根の子を選べなくする**（`edit.rs` は不要）。書ける根の場所は `BootContext` に足すと `main.rs`（`BootContext` を組む所）に触るので、`OnceLock` の口から読む |
| 4.3（起こす前に写す） | 済む | `boot_config::resolve_boot_from` と `ghost_switch::boot_into` |
| 4.4（入れ直し・更新） | 済む | `install/destination.rs`・`update/desk.rs` |
| 5（記憶） | 済む | `BootContext.app_profile_dir` は `main.rs` が `default_app_profile_dir()` を呼んで詰める＝関数の中身を替えれば付いてくる（brief の約束どおり） |
| 7.3（ゴーストとシェルの記憶） | 済む | 写しのフォルダから導かれる。`boot_resolve.rs` の `ghost_roots` は変えない |

＝**要件のどれも 3 ファイルに触ることを強いない見込み**。ただし「`BootContext` に書ける根を持たせたい」「最初の起動の写しを `main` の中で背景に出したい」と設計で決めると `main.rs` に触ることになる（そのときはタスクを最後に置く）。

## 5. 調べること（設計へ持ち越す）

1. **winget の入れ先の見分け方**（要件 1.1）。候補:
   - ⑴ 綴り: 実行ファイルのフォルダが `…\WinGet\Packages\<名>\` の形か（既定の根 `%LOCALAPPDATA%\Microsoft\WinGet\Packages`・`%ProgramFiles%\WinGet\Packages`・`%ProgramFiles(x86)%\WinGet\Packages`）。単純。利用者が `portablePackageUserRoot`／`portablePackageMachineRoot` を付け替えた機械や `winget install --location` を外す。
   - ⑵ winget の控え: 入れ先の直下の `<フォルダ名>.db`（0.0.2 の実測で在った・zip に無い）。付け替えにも効く見込み。**名前の規則（`<名乗り>_<出どころ>` か、`--location` のときどうなるか）は winget-cli の作りで確かめる**。
   - ⑶ ARP のレジストリ: `HKCU`／`HKLM` の `…\CurrentVersion\Uninstall\<名乗り>_<出どころ>` の入れ先の値（`winget list` の ID が `ARP\User\X64\Areka.Areka.Portable__DefaultSource` だった）。`windows` クレートの機能の追加が要る（新しいクレートは 0）。値の名前は確かめが要る。
   - ⑷ ⑴〜⑶ の組み合わせ。どれで見分けたかを記録の理由に載せる（要件 1.5）。
   - 中継も同じ見分けを持つ（要件 6.1）。中継は exe の場所を `current_exe` のまま使う（リンクを辿らない）ので、areka と食い違わないかも確かめる。
2. **書けるかの見分け方**（要件 1.2）: ⑴ 試し書き（作ってすぐ消す。消せなかったときに実行ファイルの隣に物が残る・要件 1.4 の「変わる物 0」とどう両立するか）⑵ 権限の問い合わせ（`AccessCheck` 等。正確だが手数が多い）⑶ 「今の記憶のフォルダを作れるか・開けるか」で見る。どの物（実行ファイルのフォルダ・`profile\`・`ghost\`）で見るかも決める。
3. **winget-pkgs から入れた形の入れ先の綴り**（`_Microsoft.Winget.Source_8wekyb3d8bbwe` の見込み）と、そのときの `.db` の名前。
4. **シェルの記憶を書く呼び手が本当に 0 か**（要件 8.3）。`PersistKey` の各鍵の書き先の層を確かめる。
5. **バルーンの更新の後の読み直し**が、写しへ替えた今のバルーンで正しく効くか（`update/desk.rs` の `reload`）。
6. **`.nar-work` を写しの作業に使い回してよいか**（`areka-nar` の片付けが、自分の物でない作業フォルダを消しにいく＝`prepare_shelf` は他の走行の物も消す）。別の棚の名前にするなら説明書の「■ 置き場」に足す。

## 5.1 決めること（要件の討議へ）

番号は §6 の報告と同じ。

1. **winget の入れ先の見分け方**（調べ 1）: 綴り／winget の控え `.db`／レジストリ／組み合わせ。付け替えた入れ先と `--location` をどこまで救うか。
2. **書けないときの見分け方**（調べ 2）: 試し書きを許すか（zip で書ける形で一瞬だけ物を作る）、権限の問い合わせにするか。
3. **判断を areka と中継で 1 つにする置き場**: `areka-mcp` に置く／小さなクレートを作る／中継に写しを持ち同じ入力の組のテストで縛る。
4. **書ける根の形の判断を持つ所**: `OnceLock` で 1 回だけ決めて覚える（`main.rs` と `BootContext` に触らない）か、`BootContext` に持たせる（`main.rs` に触る＝ウェーブの最後へ）。
5. **切替のときの写しをどのスレッドで行うか**: UI スレッドで済ませる（同梱は小さいので許す・上限を置かない）／背景に出して済んでから起こす（切替の段が 1 つ増える）。最初の起動は窓の前なので同期でよいか。
6. **写しの作業フォルダ**: `.nar-work` を使い回すか、別の棚（例 `.copy-work`）を足すか。写す部品を `areka-nar` から借りるか、標準ライブラリで小さく書くか。リンクの中を辿るか。
7. **入れ直し・supplement の写しの置き場**: `install_into` の前段で先に写す／`CatalogDestinations::answer` の中／`areka-nar` の計画で読み元と書き先を分ける（`.nar` の展開の中身に手を入れる＝境界を広げる）。
8. **同梱のゴーストが新しくなったときの扱い**（要件 4.9 の代償）: ⑴ 何もしない（今の隠れの info の行だけ）⑵ 写したときの同梱の版（`descript.txt` の値や中身の印）を覚えておき、新しくなったら info／warn で残す（上書きはしない）⑶ 後の spec へ起票する。説明書に書くか。→ **2026-10-16 要件の討議で決着**: 写しは上書きしない（写しが無いときだけ写す）。説明書に「新しい版はネットワーク更新で」と書く（要件 4.9・8.2）。⑵ の版の記録は要件にしない。
9. **最初に全部写すか、起こすときに 1 体ずつ写すか**: 書ける根を使う形の最初の起動で同梱の全部（ゴースト・バルーン）を写せば、フックは 1 か所で済み、要件 4.4 のフックも要らなくなるが、要件 4.5（表示だけのバルーンは写さない）と 4.3 の字面（起こすゴーストだけ）から外れる。要件の書き直しが要る代わりの案として並べる。
10. **書ける根を「置き場を外す ▸」で外したその回**（議題 11）: 要件どおり次の起動から戻す（その回の新しく入れる物は実行ファイルの隣へ行きうる＝要件 4.1 の例外として書く）／`menu/roots_frame.rs` で書ける根の子を選べなくする（`edit.rs` は触らない・その回の穴も塞がる）。
11. **`AREKA_ROOT` と書ける根を使う形が重なるとき**（要件 3.4 と 4.3）: `AREKA_ROOT` のときは書ける根が並びに無いので、写しても見えない。写すのをやめる（実行ファイルの隣の根が `AREKA_ROOT` に在っても今どおりそこへ書く）か、`AREKA_ROOT` のときは書ける根を使う形にしない（1.3 の環境変数だけは別）か。記憶の置き場（5.2）だけは書ける根の下へ行くのか。
12. **書ける根の場所の環境変数の名前**（要件 2.3）: 例 `AREKA_USER_ROOT`・`AREKA_DATA_DIR`。`AREKA_PROFILE_DIR` との関係（両方在るとき、記憶は `AREKA_PROFILE_DIR`＝要件 5.1）。
13. **記憶の引き継ぎの「記憶が在る」の定義**（要件 5.4）: `sylphya.toml` の有無か、`profile\areka\` の中身の有無か。写すのは `profile\areka\` 丸ごとか、`mcp-bridge\` を含むか（中継の控えも一緒に引き継ぐか）。
14. **中継の控えの置き場の 2 つの形**（要件 6.2）: 隣に `areka.exe` が無い中継（開発の机で `cargo install` した物）は今どおり `%LOCALAPPDATA%\areka\mcp-bridge\`＝書ける根の直下に `mcp-bridge\` が並ぶ。書ける根の中のフォルダの並び（`ghost\`・`balloon\`・`.nar-work\`・`profile\`・`mcp-bridge\`）としてよいか、中継の既定も `profile\areka\mcp-bridge\` へそろえるか。
15. **並びの記録の根ごとの出どころ**（要件 3.6）: `RootsOrigin` は並びで 1 つ。根ごとの印を足すか、書ける根の行を別に出すか。

## 6. 規模と危うさ

- **規模: M〜L**（12〜16 タスクの見込み）。判断と場所・並びの差し込み・記憶と引き継ぎ・中継は前例の延長（S〜M）。写す部品と 4 か所のフックに、切替の背景化を足すと L 寄り。実測 3 種（上げ直し・外す・機械の全員向け）と zip の形の確かめは開発者の手。
- **危うさ: 中**。前例の型（`OnceLock`・作業フォルダと `rename`・上の根が隠す規則）に乗るので骨格は低いが、⑴ winget の入れ先の見分けは winget の側の作りに頼る（付け替え・`--location`・控えの名前）、⑵ 写しの経路の漏れが要件 4.1 の「0」をすぐ壊す（経路の全部を表で縛る必要）、⑶ 切替の写しを背景に出すなら切替の段の設計が要る。

## 7. 設計への申し送り

- 推しの形の候補は案 C（判断と写す部品は小さなモジュール・並びは `resolve_roots_from` の入力を 1 つ足す・フックは 4 か所）。ウェーブの約束の 3 ファイルには触らない見込み。
- 要件 4.1 の「0」は、経路の表（§3.1）を設計の決定論テストの入力の組にして縛る。
- 調べ 1・2（見分け方）は設計の最初に片付ける（答えで判断の入力の型が変わる）。
- 次の段: 要件の討議で §5.1 の決めることを裁定 → `/kiro-design areka-P0-user-data-root`。

### 出典

- 完了 `areka-P0-winget-manifest-submission` の `verification/winget-local-check.md`（入れ先の綴り・`.db` の控え・リンクでないこと・消えた物 6）。
- [winget の設定（Settings.md）](https://github.com/microsoft/winget-cli/blob/master/doc/Settings.md)・[winget settings（learn.microsoft.com）](https://learn.microsoft.com/windows/package-manager/winget/settings): `portablePackageUserRoot`／`portablePackageMachineRoot` で portable の入れ先の根を付け替えられる（絶対パス・portable の入れ方だけに効く）。
- [winget-pkgs の FAQ（ARP の項）](https://github.com/agfeo/winget-pkgs/blob/master/FAQ.md): winget は ARP の記録で入っている物を見分ける。


---

# 設計の段の調べと決定（2026-10-16・`/kiro-spec-design -y`）

## Summary
- **Feature**: `areka-P0-user-data-root`
- **Discovery Scope**: Extension（今の根の並び・記憶の置き場・インストールと更新の宛先を伸ばす）。軽い発見（`design-discovery-light.md`）と、winget の作りの外部調べ 1 件。
- **使った手引き**: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、steering の `logging.md`（ライブラリは `tracing`・構造化の欄）・`structure.md`（1,000 行・兄弟テスト）。
- **Key Findings**:
  - winget は zip の入れ先へ隠しファイル `<名乗り>_<出どころ>.db` を置く（入れ先の根を付け替えても、`--location` でも置く）。名前の頭 `Areka.Areka.Portable_` を見れば、フォルダの一覧を読むだけで見分けられる。
  - 「書けるか」は、フォルダのハンドルを `FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY` の権限で開くだけで、何も作らずに確かめられる（標準ライブラリの `OpenOptionsExt`）。
  - `crates/areka/src` には `lib.rs` が無く、モジュールは `main.rs` で宣言する。＝新しいモジュールを `roots/` の下に置けば `main.rs` に触らずに済む。`ResolvedRoots` に欄を足すと、`roots/edit.rs` の `apply`（構造体を組む所）が壊れるので、欄は足さない。

## Research Log

### winget の portable の入れ先の見分け（ギャップ分析の調べ 1・3）
- **Sources Consulted**:
  - microsoft/winget-cli の `src/AppInstallerCLICore/PortableInstaller.{h,cpp}`・`Workflows/PortableFlow.cpp`・`src/AppInstallerCommonCore/Runtime.cpp`・`PortableARPEntry.cpp`・`src/AppInstallerRepositoryCore/Microsoft/PortableIndex.cpp`・`src/AppInstallerSharedLib/AppInstallerStrings.cpp`・`doc/Settings.md`（サブエージェントが GitHub の API で読んだ）。
  - 完了 `areka-P0-winget-manifest-submission` の `verification/winget-local-check.md`。
- **Findings**:
  - 入れ先の根の既定: 利用者向けは `%LOCALAPPDATA%\Microsoft\WinGet\Packages`、機械の全員向けは `%ProgramFiles%\WinGet\Packages`（x86 の入れ方は `%ProgramFiles(x86)%\…`）。
  - `portablePackageUserRoot`／`MachineRoot` は根そのものを替え、その下に製品コードのフォルダを足す。`--location` はその値をそのまま使う（下に足さない）。
  - 製品コード＝`MakeSuitablePathPart(<名乗り> + "_" + <出どころ>)`（`<>:"/\|?*` を `_` に替える）。
    - 手元のマニフェストでは `Areka.Areka.Portable__DefaultSource`（実測と合う）。
    - winget-pkgs の出どころの綴り `Microsoft.Winget.Source_8wekyb3d8bbwe` は実機での観察で、ソースでは確かめていない。
  - 書庫（zip）の入れ方では、入れ先に `<製品コード>.db`（SQLite・隠し属性）を作る。
    - 1 つの exe だけの portable では作らない（areka は zip なので当たる）。
    - `--location` でも作る。そのとき名前の頭はフォルダ名と一致しないが、名乗りの頭は一致する。
  - 最も確かなのは ARP の記録の `InstallLocation`・`WinGetInstallerType=portable`・`WinGetPackageIdentifier`。鍵は 3 つの見方に分かれる（`HKCU`、`HKLM` の 64 ビット、x86・arm64 の `HKLM\…\Wow6432Node`）。
- **Implications**: 見分けは「実行ファイルのフォルダの直下に `Areka.Areka.Portable_*.db` が在るか」にした（下の決定 1）。

### 書けるかの見分け（調べ 2）
- **Findings**:
  - 試し書きは、zip の書ける形で一瞬だけ物を作る（要件 1.4 の「変わる物 0」と食い違う）。消せなければ物が残る。
  - `AccessCheck` は正確だが、手数が多く `windows` の機能も要る。
  - フォルダを `FILE_ADD_FILE`（0x2）`| FILE_ADD_SUBDIRECTORY`（0x4）の権限と `FILE_FLAG_BACKUP_SEMANTICS`（0x0200_0000・フォルダを開くための指定）で `CreateFileW` すると、開くときに OS が権限を確かめる。何も作らずに答えが出る。
  - 標準ライブラリの `std::os::windows::fs::OpenOptionsExt::{access_mode, custom_flags}` で書ける。
- **Implications**:
  - 決定 2。
  - 32 ビットの helper の UAC の仮想化（`VirtualStore`）は、確かめる側の areka（64 ビット）には効かないので、判断を狂わせない。
  - 実測では、`VirtualStore` に物ができていないことを数える。

### 写す部品（調べ 6）
- **Findings**:
  - `areka-nar` の `copy_existing` は非公開。作業の棚 `.nar-work` の片付け（`prepare_shelf`）は、他の走行の作業フォルダも消す。
  - `std::fs::rename` は、入れ替え先のフォルダが在ると失敗する（＝先に写した方が勝つ）。
  - `FileType::is_symlink` は、Windows ではジャンクションでも真になる。
- **Implications**: 決定 6。

### 呼び手の形（コードの確かめ）
- `crates/areka/src/main.rs` がモジュールを宣言する（`lib.rs` は無い）。`roots/mod.rs` が宣言するのは `desk`・`edit`・`pick`・`report`・`resolve`。
- `roots/edit.rs` の `apply` は `ResolvedRoots { roots, origin, status, skipped, memory_fell_back }` を組んで `log_roots` を呼ぶ。＝`ResolvedRoots` に欄を足すと、約束のファイルが壊れる。
- `areka-nar` の `InstallRoots` は、配置ごとに 1 回だけ答えを尋ねる（`plan.rs` の `body_placement`・`companion_placement`・`existing_target_ghost`）。＝答えの中で写しても、写すのは 1 配置につき 1 回。
- 切替（`ghost_switch::switch_to` → `boot_into`）は、UI スレッドの上の同期の手続き（降ろす・窓を閉じる・厳格な起動）。
- 更新の対象は UI スレッドの `update/desk.rs` の `resolve` で解く。背景の走り手は根を持たない。
- `areka-sylphya` の `PersistScope::Shell` は読み込み（`build_initial_image`）にだけ現れる。＝シェルの記憶を書く本番の呼び手は 0 の見込み（要件 8.3 は実装のタスクで確かめて書く）。
- ログは標準エラーと記憶の中だけで、ファイルには書かない。＝実行ファイルの隣にログの物は増えない。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 案 A | 判断を `boot_config.rs` に足す | ファイルが増えない | 中継に同じ判断を写す（食い違う）・`boot_config.rs` が膨らむ | 採らない |
| 案 B | areka の中の新しいモジュールに全部置く | 判断を値で赤にできる | 中継が使えない（中継は areka に依存できない） | 一部採用 |
| **案 C** | 判断・場所・記憶・写す手続きは共有の葉のクレート。プロセスの形と写す判断は `roots/user_root.rs`。並びは `resolve_roots_from` に入力を 1 つ足す。フックは 4 か所 | 中継と 1 つの規則・約束の 3 ファイルに触らない（最後の 1 タスクを除く）・前例の型に沿う | クレートが 1 つ増える | **採用** |

## Design Decisions（ギャップ分析 §5.1 から持ち越した件と、要件の討議で残った B 区分の答え）

### 決定 1: winget の入れ先の見分け（§5.1 の 1）
- **Alternatives**: ⑴ 綴り `…\WinGet\Packages\<名>` ⑵ winget の控え `.db` ⑶ ARP のレジストリ ⑷ 組み合わせ。
- **Selected**: ⑵ の形。実行ファイルのフォルダの直下に、`Areka.Areka.Portable_` で始まり `.db` で終わるファイルが在るかを見る（大文字と小文字は区別しない）。
- **Rationale**:
  - 利用者向け・機械の全員向け・根の付け替え・`--location`・winget-pkgs の出どころの、どれでも当たる。
  - ファイルの一覧 1 回で済み、`windows` の機能を足さない。
  - ⑴ は、根の付け替えと `--location` で外れる。
  - ⑶ は最も確かだが、3 つの鍵の見方（64 ビット・32 ビット）とレジストリの機能の追加が要る。⑵ で外れる場面も無い。
- **Trade-offs**:
  - 入れ先のフォルダを丸ごと別の場所へ写して使うと、控えも付いて来るので「winget の入れ先」と見なす。書ける根を使う形になるだけで、害は無い。
  - winget が名前の規則を変えると外れる（Revalidation Triggers）。機械の全員向けは決定 2 で拾える。
- **Follow-up**: 実測で、winget-pkgs から入れた形の `.db` の名前を記録する。

### 決定 2: 書けるかの見分け（§5.1 の 2）
- **Selected**: 実行ファイルのフォルダのハンドルを `FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY` で開き、すぐ閉じる。
  - `PermissionDenied` → 書けない。
  - ほかの失敗 → 分からない（今の形のままにして warn）。
  - 見るのは実行ファイルのフォルダだけ。
- **Rationale**: 何も作らない（要件 1.4・4.1）。権限の確かめは OS に任せる。
- **Trade-offs**: 「フォルダには書けるが `profile\` だけ書けない」のような部分の権限は拾わない。その失敗は `write-failure-notice` の確かめが拾う。

### 決定 3: 判断を areka と中継で 1 つにする置き場（§5.1 の 3）
- **Selected**: 新しい葉のクレート `crates/areka-user-data`（依存は `std`・`tracing` だけ・`publish = false`）。中に置く物:
  - 判断（`decide`）・場所・記憶の置き場（`app_profile_dir`）。
  - プロセスのはじめの準備（`establish`）: 書ける根を作る・記憶を引き継ぐ・記録を残す。
  - 写す手続き（`copy_tree_atomic`）。
- **Rationale**:
  - 中継は `areka` に依存できない。`areka-mcp` は MCP の部品なので、置くと意味がずれる。
  - 2 か所に写すと、今の中継の `from_values` のように黙って食い違う。
  - 記録の文は ASCII にする（中継が標準エラーへ出すため・道具が端末へ出す文は ASCII の決まり）。`event` の欄で areka の記録とそろえる。
- **Trade-offs**: クレートが 1 つ増える（ワークスペースは `crates/*` なので自動で入る）。

### 決定 4: 形を持つ所（§5.1 の 4）
- **Selected**: `crates/areka/src/roots/user_root.rs` の `OnceLock`（`exe_location` と同じ型）。`BootContext` には持たせない。
- **Rationale**:
  - `BootContext` を組むのは `main.rs`（約束のファイル）。
  - `default_app_profile_dir` の中身を替えれば、全部の読み手がそろう（要件 5.6）。
  - モジュールを `roots/` の下に置くのは、`main.rs` でモジュールを宣言しないため。

### 決定 5: 切替のときの写しのスレッド（§5.1 の 5）
- **Selected**:
  - 切替: UI スレッドで同期に写す（`boot_into` の中）。
  - 最初の起動: 窓を開く前に写す（`resolve_boot_from`）。
  - インストール: 背景のスレッドで写す（`areka-nar` が答えを尋ねる所）。
- **Rationale**:
  - 切替はすでに「降ろす → 窓を閉じる → ゴーストを読んで起こす」の同期の手続き。写しは、それと同じ大きさの読みを 1 回足すだけ。
  - 写すのは、同梱の物 1 つにつき一生に 1 回。
  - 背景に出すと、切替の段に「写し中」の状態が 1 つ増え、`ghost_switch.rs`（917 行）の余裕を食う。
- **Trade-offs**:
  - 書けない所へ展開した大きいゴーストでは、初回の切替が写す間だけ固まる。
  - `user_data_copied` の `ms` で実測し、長ければ背景へ出す（design の Open Questions 1・設計の討議へ）。
- **2026-10-16 設計の討議で改めた（議題 2・推奨案で承認）**:
  - 切替: 予約の時点（`request_ghost_switch_with` の受理）で背景のスレッドで写し始める。前のゴーストが止まった時点で写し終えていなければ、段「写し待ち」で待ち、写し終えたら写しから起こす。段は `emo2_boot/switch_copy.rs` に置き、`ghost_switch.rs` は数行だけ。
  - バルーンの更新: 写しを更新の作業スレッド（`update/worker.rs`）へ移す。`update/desk.rs` の `resolve` は値の関数のまま。
  - 最初の起動（窓の前）・既定ゴーストへの戻し（非常口・emo2 は大きさが分かり、ほとんど初回に写し済み）は同期のまま。
  - 理由: 開発者の方針「ゴーストの大きさを甘く見ない・上限で断らず UI スレッドの外へ出す」（10-05）。後から背景へ出すと `ghost_switch.rs` の段を組み替えることになる。

### 決定 6: 写しの作業フォルダと写す部品（§5.1 の 6）
- **Selected**:
  - 棚は `<書ける根>\.copy-work\<pid>-<連番>\`。`.nar-work` は使い回さない。
  - 部品は標準ライブラリで書く（`areka-nar` から借りない）。
  - リンク（シンボリックリンク・ジャンクション）は辿らない。在れば写しを失敗にして、元を使う。
  - 読み取り専用の属性は落とす。
  - 入れ替えの先が在れば負け（`Raced`）として自分の作業を捨て、在る方を使う。
- **Rationale**:
  - `areka-nar` の棚の片付けは、他の走行の作業フォルダを消す。
  - 写す部品を公開して借りると、`.nar` の手続きの境界が広がる。
  - リンクを辿ると、書ける根の外の物まで写しうる。
- **Trade-offs**: プロセスが写す途中で落ちたときの残骸は、自動では片付けない（ponytail: 要るなら `establish` で古い項目を消す）。

### 決定 7: 入れ直し・supplement の写しの置き場（§5.1 の 7）
- **Selected**: `CatalogDestinations::answer`（`InstallRoots` の答え）の中で写す。失敗したら元の根を答える。要件 4.7 の「元を使って続ける」と一致するので、答えの型は `PathBuf` のままでよい。
- **Rationale**:
  - `areka-nar` は配置ごとに 1 回だけ尋ねるので、写すのも 1 回。シェル・supplement の宛先ゴースト（`existing_target_ghost`）も同じ答えを通る。
  - `install_into` の前段で書庫の宛先を引き直す案は、`areka-nar` の計画の規則（同梱のバルーンの名前など）を areka の側に写すことになる。
  - `areka-nar` の計画で読み元と書き先を分ける案は、`.nar` の展開の中身に手を入れる（範囲外）。
- **Trade-offs**: 利用条件で「いいえ」を選ぶなどして入れなかったときも、計画の時点で写しは済んでいる。写しは起こすときにどのみち作られる物なので、害は無い。

### 決定 8: 写しの判断の条件（§5.1 の 9 と要件 3.7 の組み合わせ）
- **Selected**:
  - 最初に全部は写さず、1 体ずつ書く直前に写す（要件 4.3・4.5 の字面）。
  - 写すのは、実行ファイルの隣の根の置き場の直下の物で、並びの上で書ける根が実行ファイルの隣の根より上に在るとき。
  - `AREKA_ROOT` のときは並びに書ける根が無いので、写さない（要件 3.4）。
- **Rationale**:
  - 写しが上の根として下の同梱の物を隠すときだけ、写しと一覧・切替・更新が食い違わない。
  - 利用者が「置き場を先頭へ ▸」で実行ファイルの隣を上へ動かしたら、その選びのとおり実行ファイルの隣を使う（要件 3.7 と同じ向き）。
- **2026-10-16 設計の討議で改めた（議題 1・推奨案で承認）**: 書ける根を使う形では「置き場を先頭へ ▸」の実行ファイルの隣の子を選べない形にする（要件 3.8 に 1 文を足した）。足すのは末尾・書ける根は外せない・先頭へも上げられない、で実行ファイルの隣が書ける根より上に来る道が無くなり、要件 4.1 の「0」を例外なしで守る。`plan_copy` の「書ける根が上」の条件は、記憶を手で書き替えた場合の守りとして残す。

### 決定 9: 書ける根の場所の環境変数の名前（§5.1 の 12）
- **Selected**: `AREKA_USER_ROOT`。
  - 在れば、要件 1.1・1.2 に関わらず書ける根を使う形にする（要件 1.3）。
  - 空・相対の値は「決められない」として error を残し、今の形で続ける。
  - `AREKA_PROFILE_DIR` も在るとき、記憶は `AREKA_PROFILE_DIR` に置く（要件 5.1）。
- **Rationale**: `AREKA_ROOT`（根の並び）と対になる語で、「利用者の根」を指す。`AREKA_` の名前空間・ドメインの語の決まりに沿う。

### 決定 10: 記憶の引き継ぎの「記憶が在る」（§5.1 の 13）
- **Selected**:
  - 「`<exe_dir>\profile\areka` がフォルダで、`<書ける根>\profile\areka` が無い」ときに、`profile\areka\` を丸ごと（`mcp-bridge\` を含む）写す。
  - 引き継ぎは、areka と中継の両方の `establish` が行う。先に起きた方が写し、同時なら `rename` で先に入れ替えた方が勝つ。
- **Rationale**:
  - 中継が先に起きて `profile\areka\mcp-bridge\` を作ると、書ける根の側に `profile\areka` が在ることになり、areka の記憶を引き継げなくなる。中継も同じ準備を通せば、これが起きない。
  - 中身の鍵（`sylphya.toml`）で見る案は、在るファイルと無いファイルを混ぜ合わせることになり、半端を残さない形にできない。

### 決定 11: 中継の控えの 2 つの形（§5.1 の 14）
- **Selected**:
  - 隣に `areka.exe` が在る中継は、areka と同じ記憶の置き場の下の `mcp-bridge\`。
  - 隣に無い中継は、今どおり `%LOCALAPPDATA%\areka\mcp-bridge\`（書ける根の直下に並ぶ）。そろえ直さない。
- **Rationale**: 要件 6.2 の字面（今どおり）。隣に `areka.exe` が無い中継は、開発の机で `cargo install` した物だけ。

### 決定 12: 並びの記録の根ごとの出どころ（§5.1 の 15）
- **Selected**:
  - `root_listed` の各行に欄 `user_root` を足す（書ける根なら `default`／`env_var`、ほかは空）。
  - `ResolvedRoots` には欄を足さず、`log_roots` が `listed_user_root()` を見て付ける。判断は `log_roots_with` に置き、値で確かめる。
- **Rationale**: `ResolvedRoots` に欄を足すと、約束のファイル `roots/edit.rs` の `apply` が壊れる。
- **2026-10-16 設計の討議で改めた（議題 3・推奨案で承認）**: 「書ける根だ」の印を並びの各項目に持たせる（`areka_ghost::BasewareRoot` に `RootMark { Plain, UserRoot { from_env } }`・既定は `Plain`・付けるのは `resolve_roots_from` だけ）。
  - 印は並びと一緒に `BootContext.roots` へ入り、足す・外す・先頭へ（どれも今の項目を複写する）でも消えない。
  - 記録・メニュー・写す判断・インストールの宛先・更新の対象は並びの値から読む。プロセスの形（`layout()`）を読むのは `resolve_roots`（今も環境変数と記憶を読む薄い口）と `default_app_profile_dir` だけ。`log_roots_with` と `CopyContext` は要らなくなった。
  - 理由: 関数の中でプロセスの形を読むと、テストの中でも本物の `establish` が走り、`AREKA_USER_ROOT` を開けた端末でテストの結果が変わる（要件 10.6）。入口ごとに読む案は World を通る経路（切替・並びの変更）のテストを塞げず、World に置く案は `main.rs` に触る。

### 決定 13: 「置き場を先頭へ ▸」の置き場（討議で引き取った件）
- **Selected**:
  - 判断 `decide_front` と入口 `move_to_front` を `roots/edit.rs`（`decide_add`／`decide_remove` の隣）に置き、今の `apply` を呼ぶ。
  - メニューの項目（`Frame::RootFront`）も、同じ最後のタスクに入れる。
- **Rationale**:
  - 並びを変える手続きを 1 か所にそろえる。記憶へ書く口と、書けなかった結果の受け渡しを 2 つに割らない。
  - `write-failure-notice` の着地の後の `apply` の形に合わせる（要件 3.9）。

## Synthesis（一般化・作るか借りるか・簡素化）
- **一般化**:
  - 「書く前に写す」を、5 つの場面（起こす・入れ直す・シェル・supplement・バルーンの更新）ごとに書かず、1 つの口 `writable_copy(roots, kind, dir)` にした。フックは 4 か所に 1〜数行ずつ。
  - 形の判断と記憶の置き場は、areka と中継で同じ関数を使う。
- **作るか借りるか**:
  - 写す部品は標準ライブラリで作る（`areka-nar` の部品は非公開で、借りると境界が広がる）。
  - 書けるかの確かめは、OS の権限の確かめ（ハンドルを開く）を借りる。
  - winget の見分けは、winget 自身の控えを借りる。
  - 新しい外部依存は 0。
- **簡素化**:
  - `BootContext`・`ResolvedRoots` に欄を足さない（`OnceLock` の口から読む）。
  - 写しの版の記録は持たない（要件 4.9 の裁定）。
  - 作業フォルダの残骸の自動の片付けは足さない。
  - 中継の「隣に `areka.exe` が無い」形は、そろえ直さない。

## Risks & Mitigations（設計の段）
- 写しの経路の漏れが要件 4.1 の「0」を壊す — design の「書く経路の表」で、書く物の行き先を全部挙げた。実測で、実行ファイルのフォルダの下に areka が作った物を数える。
- winget の控えの名前の規則が変わる — Revalidation Triggers に載せた。機械の全員向けは、書けるかの確かめが拾う。
- 切替の写しで UI が固まる — 写すのは一生に 1 回で、`ms` を記録して実測する。長ければ背景へ出す（設計の討議へ）。
- `write-failure-notice` が `apply` の形を変える — 「置き場を先頭へ ▸」のタスクを最後に置き、着地の後の形に合わせる。

## References（設計の段）
- [winget-cli PortableInstaller.cpp](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/PortableInstaller.cpp) — 控えのファイル `<製品コード>.db` と ARP の記録。
- [winget-cli PortableFlow.cpp](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCLICore/Workflows/PortableFlow.cpp) — 製品コードの組み方と、控えを作る条件（書庫のとき）。
- [winget-cli Runtime.cpp](https://github.com/microsoft/winget-cli/blob/master/src/AppInstallerCommonCore/Runtime.cpp) — portable の入れ先の根の既定と付け替え。
- [winget の設定（Settings.md）](https://github.com/microsoft/winget-cli/blob/master/doc/Settings.md) — `portablePackageUserRoot`／`portablePackageMachineRoot`。
- [CreateFileW（learn.microsoft.com）](https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-createfilew) — フォルダを開く `FILE_FLAG_BACKUP_SEMANTICS`。

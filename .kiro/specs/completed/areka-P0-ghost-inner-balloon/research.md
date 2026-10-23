# ギャップ分析: areka-P0-ghost-inner-balloon

- 日付: 2026-10-22（ワークツリー `claude/areka-p0-ghost-inner-balloon-dd21e8`・main `6e0ea8d8` の上）
- 入力: `requirements.md`（2026-10-22 に開発者と決めたもの。brief.md と違うところは requirements.md を正とする）・`brief.md`・steering（product・tech・structure）
- やり方: 既存のコードを Grep／Read で読み、要件ごとに「在る・足りない・未確認」を付けた。新しい依存は要らないので、外の調べ物はしていない。

## 1. まとめ

- **選ぶ側は 1 か所で済む形がもうできている**。起動とゴースト切替のバルーンの決め方は、全部 `crates/areka/src/boot_config.rs` の `resolve_balloon_for_ghost` を通り、その中の `list_balloons(roots)` の 1 行が「候補の一覧」を作っている。決め方の本体（`boot_resolve.rs` の `resolve_balloon`）は、渡された一覧の項目の場所（`entry.dir`）をそのまま使う純粋な関数で、根から場所を組み直さない。一覧に「今のゴーストの中のバルーン」を混ぜれば、記憶・descript・同梱・唯一・既定・無作為の段はコードを変えずに当たる。
- **実行中に一覧を引く呼び手は 5 か所**（メニュー・`\![change,balloon,…]`・`updateother`・`\![open,explorer,…]`・入れた後の表示名）。どの呼び手も、今のゴーストのフォルダを手元に持っている。
- **いちばん手のかかる所はインストール**。`crates/areka-nar` は同梱のバルーンを「本体と別の配置」として組み、配置ごとに「宛先が在るか」を**組み上げの時点**で決めてから、全部を組み終えた後に順に入れ替える。同梱の宛先をゴーストの中（本体の宛先の下）へ移すと、本体の入れ替えで同梱の宛先の在る・無いが変わり、`refresh,1` のゴーストでは確定の途中で失敗しうる（下の 4.3）。設計の段で形を決める必要がある。
- **覚え書き (b) の答え**: §8 の中で書き方が食い違っている。コードと後から書かれた行は「フォルダ名は大文字小文字を区別しない」で揃っており、区別すると書いた行は古い（下の 3.2）。要件 3.1 の「根どうしの同名と同じ決まり」は、区別しない決まりになる。
- **規模の見立て**: 16〜18 タスク（20 を超えない）。大きさは M〜L、危うさは中（インストールの入れ替えの順だけが新しい形）。

## 2. 今の形（関係するコード）

### 2.1 一覧を作る所（`crates/areka-ghost`）

| 場所 | 今していること | この spec との関係 |
|---|---|---|
| `catalog.rs` の `scan_balloons(root)`（`pub(crate)`） | `<根>\balloon\` の直下で、descript.txt が読め、`type` が無いか `balloon` のフォルダを `BalloonEntry` にする。`type` が他の値なら `warn!(catalog_type_not_balloon)`、descript が無ければ黙って除く。一時の置き場（`RootMark::Temporary`）は探さない | ゴーストの中の `balloon\` にも同じ決まりを当てる（要件 1.1・1.3）。中身の判定は同じで、探す場所だけが違う |
| `catalog.rs` の `BalloonEntry { dir, root, identity }` | `root` は「走査した根」 | ゴーストの中の項目に何を入れるかが決まっていない（下の設計事項 3） |
| `catalog.rs` の `RootMark`（`Plain`・`UserRoot`・`Temporary`） | 根の印。先例が 2 つ | ゴーストの中は「根」ではないので、印に種類を足すより、項目の側に出どころを持たせる方が素直に見える（設計事項 3） |
| `catalog_roots.rs` の `union`・`list_balloons`・`find_balloon`・`survey`・`install_root` | 根の並びを上から走査し、同じフォルダ名（`fold_key`＝大文字小文字を区別しない）は上を残して下を `Hidden` に控える。並びはフォルダ名のバイト順 | 「ゴーストの中を先に、根の和を後に」で同じ `union` の考え方がそのまま使える。`install_root` と `find_balloon(roots, …)` はインストールの宛先の判断に使われるので、根だけの一覧のまま残す必要がある（要件 6・7.3） |
| `catalog.rs` の `companion_balloon(ghost_dir)` | `<ゴースト>\install.txt` の同梱の最初の 1 個の `*.directory` | 変えない。同梱の段は名前を一覧と突き合わせるだけなので、一覧が変われば要件 4.4 は満たされる |

### 2.2 起動とゴースト切替（`crates/areka/src`）

| 場所 | 今していること | この spec との関係 |
|---|---|---|
| `boot_config.rs` の `resolve_balloon_for_ghost(roots, ghost_dir, pick)`（454〜478 行） | 記憶・descript の 2 鍵・同梱・`list_balloons(roots)` を読んで鎖へ渡す。説明に「ゴーストのフォルダの中の `balloon/` は読まない」 | **ここ 1 行の一覧の作り方を替えるのが中心**。呼び手は起動（375 行）とゴースト切替（`emo2_boot/ghost_switch.rs` 811 行）の 2 つだけで、どちらもゴーストのフォルダを渡している。`--ghost`・位置の引数のゴースト（要件 1.4）もゴーストのフォルダを渡すので同じ道を通る |
| `boot_resolve.rs` の `resolve_balloon` | 段の並びと「当たらなかった」の記録（`last_balloon_not_found`・`descript_balloon_not_found`・`companion_balloon_not_found` の `balloon_store` 欄）。`find_balloon`（フォルダ名・区別しない）と `find_balloon_by_name`（`name`・区別する・**一覧の並びで最初**） | 段の並びは変えない（要件 4.1）。ただし `name` が同じものがゴーストの中と根の両方に在るとき、一覧の並び（フォルダ名順）で最初のものを取るので、根の方が先に並ぶとゴーストの中が負ける＝要件 3.3 に合わない（設計事項 2）。`balloon_store` 欄にゴーストの中の置き場を足すかも決める（設計事項 6） |
| `boot_resolve.rs` の `NoBalloon` | 一覧が空のときの失敗 | 一覧にゴーストの中が入れば、根が空でも失敗にならない（要件 4.5）。コードの追加は要らない |
| `boot_resolve.rs` の `LastUsed::record`・`record_last_balloon` | 記憶はフォルダ名 1 つ（`areka.last.balloon`） | 変えない（要件 5.3）。ゴーストの中のバルーンでもフォルダ名を書くだけ |
| `boot_config.rs` 297 行の `report::log_hidden(survey(...).hidden)` | 根どうしの隠れを起動時に記録 | ゴーストの中が根を隠したことの記録（要件 3.2）を、どこで何回残すかは決まっていない（設計事項 5） |

### 2.3 実行中に一覧を引く呼び手

| 場所 | 今の引き方 | 手元にあるゴーストのフォルダ |
|---|---|---|
| `menu/balloon_frame.rs` 31 行 | `list_balloons(&ctx.roots)` の素性を並べ、`ctx.current.balloon.folder` に印 | `BootContext.current.ghost.dir` |
| `emo2_boot/shell_balloon_resolve.rs` の `balloon_candidates(roots)`（110 行）→ `shell_balloon_switch.rs` 261 行 | `\![change,balloon,…]`（名前・フォルダ名・`random`・`lastinstalled`）とメニューの選択 | `SwitchContext.ghost_dir` |
| `readme/opener.rs` の `named_folder`（308 行） | `balloon_candidates(root)` を `name` → フォルダ名で引く | `OpenContext.ghost_dir` |
| `update/desk.rs` の `resolve`（635 行） | `updateother` の `--balloon=名前` を `list_balloons(&here.roots)` の `name` で引く。今のバルーンの更新は `here.balloon_dir`（場所）をそのまま使う | `Here.ghost_dir` |
| `install/procedure.rs` の `installed_items`（432 行） | 同梱のバルーンの表示名を `find_balloon(&facts.roots, folder)` で引く | 入れた場所 `element.path` が手元にある |

- `balloon_candidates` の呼び手は 2 つ（切替と `\![open,explorer,…]`）なので、引数にゴーストのフォルダを足せば両方が揃う。
- メニュー・`updateother` は `list_balloons` を直接呼んでいるので、同じ新しい口へ替える。

### 2.4 読み直し（要件 5.7）

- `emo2_boot/reload.rs` の `current_skin`（590 行）は、バルーンの読み直しで一覧を引かず、`ctx.balloon_dir`（今のバルーンの場所）で切替先を組む。ゴーストの中のバルーンでも同じ場所で読み直せる＝**コードの変更は要らない見込み**。確かめるテストを足す。
- ゴーストの読み直し（`\![reload,ghost]`・MCP の `reload`）は自分自身への切替（`start_switch`）で、バルーンは `resolve_balloon_for_ghost` の記憶の段で決まる。切替で書いた記憶（フォルダ名）がゴーストの中のものに当たれば戻る＝2.2 の変更で揃う。確かめるテストを足す。

### 2.5 インストール（`crates/areka-nar`・`crates/areka/src/install`）

| 場所 | 今していること | この spec との関係 |
|---|---|---|
| `areka-nar/src/manifest.rs` の `collect_companions`（388 行） | **ゴーストとシェルの書庫**の両方で同梱のバルーンを読む（`handles_companions = Ghost | Shell`）。同梱ごとに `balloon.refresh` などの扱い（`existing`）を持つ | 鍵の意味は変えない（要件 7.2）。シェルの書庫の同梱をどこへ入れるかは要件に書いていない（設計事項 7） |
| `areka-nar/src/plan.rs` の `body_placement` | 本体の配置。同梱の取り出し元（`balloon.source.directory`）に当たるエントリだけを本体から除き、それ以外の `balloon\<名>\` はゴーストの中へそのまま入る | 要件 7.5 は**今のままで満たされる**（brief 棚卸㉓の確認どおり） |
| `areka-nar/src/plan.rs` の `companion_placement` | 同梱 1 つを `request.roots.balloon_root(dir)` の `<根>\balloon\<dir>` へ置く**別の配置**にする | 宛先を「本体の宛先 `\balloon\<dir>`」へ替える（要件 7.1）。根を尋ねなくなる |
| `areka-nar/src/install.rs` の `stage_placement`・`commit_all`・`commit_one` | 全配置を**先に全部組み上げ**（このとき宛先が在れば `Overlay`＝丸ごと下敷き・`Replace{keep}`＝残す名前だけ下敷き、`ExistingState` を決める）、その後に配置の順で `rename` で入れ替える。途中で失敗したら逆順に戻す | 同梱の宛先が本体の宛先の中に入るので、本体の入れ替えの後では、同梱の宛先の在る・無いが組み上げのときと違いうる（下の 4.3） |
| `areka/src/install/destination.rs` の `CatalogDestinations::balloon_root` | 見えている同じフォルダ名のバルーンの根へ入れる。実行ファイルの隣の根の物なら書ける根へ写してから（`copied_root`） | 同梱が根を尋ねなくなれば、根の同じ名前のバルーンを写す・上書きする道が無くなる（要件 7.3 の助けになる） |
| `areka/src/install/procedure.rs` の `installed_items` | 同梱の表示名を根の一覧から引く | 入れた場所（`element.path`）の descript から引けば足りる（要件 7.6） |
| `areka/src/install/desk.rs` 373 行 | バルーンだけの書庫を入れた後、根の一覧で引いて `lastinstalled` の控えと記憶（`remember_balloon`）に書く | 変えない（要件 7.4）。ただし今のゴーストの中に同じフォルダ名が在ると、その記憶は次からゴーストの中のものに当たる（覚え書き (a) と同じ形・設計事項 1） |

### 2.6 ネットワーク更新（要件 8）

- `areka-update` は `updates2.dau`／`updates.txt` のファイルの一覧で対象のフォルダを書き換える。パスの検査（`manifest.rs` の `invalid_path`）は `balloon/x/descript.txt` のような下の段のパスを拒まない。ゴーストの更新では、ゴーストの中の `balloon\` も他のファイルと同じ決まりで替わる（要件 8.1）＝**変更は要らない見込み**。
- 更新には「同梱」という考えが無い。要件 8.2 の「ネットワーク更新で届いた同梱のバルーン」は、ゴーストの更新の一覧に `balloon/<名>/…` が載っていればゴーストの中へ入る、という意味にしかならない（根の置き場へ入る道はもともと無い）。
- 今のバルーンの更新（要件 8.3）は場所（`Here.balloon_dir`）を対象にする。`update/worker.rs` の `prepare_targets` は書ける根への写し（`copy_for_write`）を通すが、ゴーストの中のバルーンは親が `<実行ファイルの隣>\balloon` でないので写さずにそのまま書く。ゴーストそのものは起こす前に書ける根へ写されているので、ふつうは写しの中を書く。

### 2.7 記録の決まりとテストの置き場

- 失敗は `warn!`＋除外で、列挙は `Result` を返さない（`catalog.rs` の頭の説明）。ゴーストの中も同じ形にできる（要件 10.3）。
- テストの一時フォルダは `temp-path-kit` の `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。要件 10.2 は既存の道具で満たせる。
- テストは実装の兄弟ファイル（`*_tests.rs`）に置く形。

## 3. 討議の覚え書きへの答え

### 3.1 覚え書き (a): 同じ名前の決まりと「利用者の選んだものを作者の指定で上書きしない」

- 記憶はフォルダ名 1 つ（要件 5.3 で変えないと決めた）。だから記憶からは「根の X を選んだ」と「ゴーストの中の X を選んだ」を区別できない。要件 3 の優先（ゴーストの中が勝つ）と組むと、次の 2 つの道で、利用者が根の X を選んだ後にゴーストの中の X が出る。
  1. 利用者が根の X を選んで記憶した後、ゴーストの更新・上書きインストールでゴーストの中に同じフォルダ名の X が入る。
  2. バルーンだけの書庫で根に X を入れた（`install/desk.rs` が記憶に X を書く）が、今のゴーストの中にすでに X が在る。入れた直後の `\![change,balloon,lastinstalled]` もゴーストの中の X に当たる。
- 引数で渡したバルーン（要件 6.3）は場所で持つので、この食い違いは起きない。
- 取れる形（決めるのは要件の討議）:
  - **案 1（今の要件のまま）**: 「そのゴーストでは X はゴーストの中の X を指す」と読み、上書きとは数えない。記録（要件 3.2 の隠れの記録）で分かるようにし、§8 と README に書く。コードの追加は 0。
  - **案 2**: バルーンだけの書庫を入れたときに今のゴーストの中に同じフォルダ名が在れば、記録を 1 件足す（入れたものがこのゴーストでは隠れる、と分かるように）。小さい追加。
  - **案 3**: 記憶に置き場を区別する欄を足す。要件 5.3・5.6 と食い違うので、取るなら要件を直す。

### 3.2 覚え書き (b): フォルダ名の大文字小文字（§8 の書き方の確認）

§8 は**同じ表の中で食い違っている**。

- `doc/COMPAT_ARCHITECTURE.md` 370 行（`areka-P0-ghost-standard-balloon` の行）: 「`default.balloon.path` の起点は … 値はその**直下のフォルダ名 1 段**として、フォルダ名と大文字と小文字を区別する完全一致で突き合わせる」。同じ行の `balloon` の説明も「どちらも大文字と小文字を区別する完全一致」。根拠に挙げたテスト名 `descript_match_is_case_sensitive` は、今のコードに無い。
- 同じ §8 の 376 行（`areka-P0-baseware-root-list` の行・後から着地）: 「フォルダ名の同じ・違うは大文字小文字を区別しない `same_folder_name` の 1 つの規則」。
- コード: `catalog_roots.rs` の `find_balloon`（137 行）も、`boot_resolve.rs` の `find_balloon`（144 行・`default.balloon.path`・記憶・同梱・既定の段）も `same_folder_name`（`fold_key`＝区別しない）で突き合わせる。`name` の突き合わせ（`find_balloon_by_name` の前半）だけは区別する。テストは `boot_resolve_balloon_tests.rs` の `descript_folder_match_ignores_case_but_name_match_does_not`（443 行）で、「フォルダ名は区別しない・`name` は区別する」を固めている。
- `emo2_boot/shell_balloon_resolve.rs` の頭の説明（8 行）も「フォルダ名（大文字小文字を区別）」と書くが、本体（168 行）は `same_folder_name` で区別しない。説明だけが古い。

結論: 動きは「フォルダ名は区別しない・`name` は区別する」で揃っており、370 行と `shell_balloon_resolve.rs` の説明が baseware-root-list の着地の後に直されずに残っている。要件 3.1 の「根どうしの同名と同じ決まり」は `same_folder_name`（区別しない）になる。どちらに揃えるかを選ぶ議題ではなく、**古い 2 か所の書き直し**を本 spec の文書の仕事（要件 9.1 で §8 を直すついで）に入れるかどうかの話になる。

## 4. 要件と資産の対応

印: **在る**＝今のままで満たす・**足す**＝コードが要る・**未確認**＝設計で調べる・**制約**＝既存の形が縛る。

| 要件 | 対応する資産 | 印と中身 |
|---|---|---|
| 1.1 ゴーストの `balloon\` の直下を数える | `scan_balloons` の判定 | **足す**: ゴーストのフォルダを受ける走査を足す（判定の本体は共有する） |
| 1.2 直下のフォルダ・`balloon\` そのもの・さらに下は数えない | `subdirs`（直下 1 段だけ） | **在る**: 探す場所を `<ゴースト>\balloon\` に限れば自然に満たす。テストは足す |
| 1.3 決まりを満たさないフォルダの記録 | `catalog_type_not_balloon` | **在る**（同じ判定を通せば同じ記録）。descript の無いフォルダは根でも黙って除くので同じ |
| 1.4 引数で渡したゴースト | `resolve_balloon_for_ghost` は `ghost.dir` を受ける | **在る**（一覧の作り方を替えれば揃う）。テストは足す |
| 1.5 ゴーストの中に無いときは今どおり | — | **在る**（足すものが 0 件なら一覧は今と同じ） |
| 2.1 すべての候補に入れる | 2.3 の 5 か所 | **足す**: 5 か所を新しい口へ替える |
| 2.2・2.3 他のゴーストから見えない・切替で入れ替わる | 一覧を毎回ゴーストのフォルダから作る | **在る**（一覧を保存していないので、引くたびに今のゴーストで作れば満たす）。テストは足す |
| 2.4 同じ一覧に並べ、今のものに印 | `balloon_frame.rs`・`skin_frame` | **足す**（一覧の口の差し替え）。並びの順は設計事項 2 |
| 3.1 同じフォルダ名はゴーストの中が勝つ | `union` の上から順の考え方 | **足す**: ゴーストの中を先に走査する合成 |
| 3.2 隠したことの記録 | `Hidden`・`log_hidden` | **足す**: 種類（ゴーストの中が隠した）と、記録する時と回数（設計事項 5） |
| 3.3 同じ `name` はゴーストの中が勝つ | `find_balloon_by_name`・`resolve_skin_target`・`skin_by_name`・`updateother` は「一覧の並びで最初」 | **足す／制約**: 一覧の並びかつ `name` の引き方の順を決める（設計事項 2） |
| 3.4 別のゴーストでは根のものが見える | — | **在る**（一覧をゴーストごとに作るので） |
| 4.1〜4.4・4.6 段の並びと各段の突き合わせ先 | `resolve_balloon`（純粋） | **在る**（一覧を替えれば各段が当たる）。テストは足す |
| 4.2 `default.balloon.path` がゴーストの中に当たる | `boot_config_balloon_tests.rs` の `default_balloon_path_does_not_see_ghost_inner_balloon` | **足す**: このテストは逆の答えを固めているので、書き換える |
| 4.3 区切り入りの値は当たらない | `find_balloon` はフォルダ名の完全一致 | **在る**（`balloon/claudia` は `claudia` に一致しない）。ゴーストの中の `balloon\claudia` が在っても当たらないことをテストに足す |
| 4.5 根が空でもゴーストの中から決める | `NoBalloon` は一覧が空のときだけ | **在る**（一覧が空でなくなる）。テストは足す |
| 4.7 当たらなかったときの記録の形 | `balloon_store` 欄 | **未確認**: 欄にゴーストの中の置き場を足すか（設計事項 6） |
| 5.1・5.2 記憶する | `record_last_balloon`・`LastUsed::record` | **在る**（フォルダ名を書くだけ）。テストは足す |
| 5.3・5.6 記憶の形を変えない | `areka-sylphya` の `persist` | **在る**（触らない） |
| 5.4・5.5 記憶から戻る・無ければ次の段 | `resolve_balloon` の段 2 | **在る**（一覧を替えれば揃う） |
| 5.7 読み直しで同じゴーストの中のまま | `reload.rs` の `current_skin`（場所で持つ） | **在る**見込み。テストで確かめる |
| 6.1〜6.4 根の置き場は変えない | `list_balloons(roots)`・`find_balloon(roots,…)`・`install_root` | **制約**: 根だけの口はインストールの宛先と根の隠れの記録に使われているので残す。新しい口を足し、選ぶ側だけ差し替える |
| 7.1 同梱をゴーストの中へ | `plan.rs` の `companion_placement` | **足す**＋**未確認**（4.3 の入れ替えの順） |
| 7.2 鍵の意味を変えない | `manifest.rs` | **在る**（触らない） |
| 7.3 根の同じ名前に触らない | `destination.rs` の `balloon_root`→`copied_root` | **足す**（同梱が根を尋ねなくなれば満たす）。テストは足す |
| 7.4 バルーンだけの書庫は根へ | `body_placement` の `InstallKind::Balloon` | **在る**（触らない） |
| 7.5 名指しされない `balloon\<名>\` はそのまま入る | `body_placement` の除き方 | **在る**。テストは足す |
| 7.6 入れた後の表示名 | `installed_items` | **足す**（入れた場所の descript から引く） |
| 7.7 入れられないときは今と同じ扱い | 確定の失敗は全部を戻す | **在る**（配置の失敗の扱いは同じ）。ただし 4.3 で新しい失敗の道を作らないこと |
| 8.1 更新・上書きで一緒に替わる | `areka-update`・`body_placement` | **在る**見込み（2.6）。本体の `refresh,1` と同梱の `balloon.refresh` の関係は設計事項 8 |
| 8.2 以前に根へ入れた同梱は動かさない | 同梱が根を尋ねない | **足す**（7.1 と同じ変更で満たす）。テストは足す |
| 8.3 今のゴーストの中のバルーンの更新 | `Here.balloon_dir`・`prepare_targets` | **在る**見込み。テストで確かめる |
| 8.4 ゴーストのフォルダが消えたら残らない | 一覧を毎回作る | **在る**（areka に削除の操作は無い・範囲外）。フォルダを消した後に候補に出ないテストを足す |
| 8.5 更新で無くなったら次の段 | 段 2 の `last_balloon_not_found` | **在る** |
| 9.1〜9.4 文書 | §8 の 370・376・308 行・`dist/README.txt` 83・109〜114 行 | **足す**（4.4） |
| 10.1〜10.3 テストと記録 | `TempPath::under_target`・`warn!` の形 | **足す**（テスト）・**在る**（道具） |

### 4.1 足りないもの（まとめ）

1. ゴーストの中の `balloon\` を走査し、根の和と「ゴーストの中が先」で合わせた一覧を返す口（`areka-ghost`）。
2. 項目がどこのものか（根・ゴーストの中）の区別。
3. `name` の同着でゴーストの中を先にする並べ方か引き方。
4. ゴーストの中が根を隠したことの記録。
5. 選ぶ側の 6 か所（起動・切替の 1 か所＋実行中の 5 か所）の差し替え。
6. 同梱の宛先をゴーストの中へ替えることと、それに伴う入れ替えの順の手当て（`areka-nar`）。
7. 入れた後の表示名の引き方。
8. 文書（§8 の行の書き直しと新しい行・README）。

### 4.2 今のままで満たすもの（テストだけ足す）

- 段の並び・記憶の形・読み直し・ネットワーク更新・ゴーストの書庫の中の名指しされない `balloon\<名>\`・バルーンだけの書庫の入れ先・根どうしの隠れ。

### 4.3 未確認: 同梱をゴーストの中へ入れるときの入れ替えの順（調べ物）

今の `areka-nar` の流れは「全配置を組み上げる（宛先の在る・無いと下敷きをここで決める）→ 配置 0（本体）から順に `rename` で入れ替える」。同梱の宛先を `<ゴースト>\balloon\<dir>` にすると、次のずれが出る。

- 本体が `Overlay`（`refresh` 無し）: 本体の組み上げが古い `balloon\<dir>` も下敷きに写す。本体を入れ替えた後も `balloon\<dir>` が在るので、同梱の入れ替え（在る→退避→置く）は通る。中身は同梱の組み上げどおりになる。同じ物を 2 回写すだけで、答えは合う。
- 本体が `Replace{keep}`（`refresh,1`）: 本体の組み上げは残す名前のファイルだけを写すので、古い `balloon\<dir>` はほぼ消える。同梱の組み上げは古い宛先を見て「在る」と決めているので、確定のときに在るはずの宛先が無く、`rename(宛先, 退避)` が失敗して**インストール全体が巻き戻る**。残す名前に当たるファイルが `balloon\<dir>` の下に在ると、中途半端な木が在る形になる。
- 初めて入れるゴースト（宛先が無い）: 本体も同梱も「無い」で、本体を置いた後に同梱を置く。問題なし。

取れる形（設計で決める）:

- **形 A**: 本体の組み上げで、同梱の宛先（`balloon\<dir>`）を下敷きから外す。同梱の組み上げは今どおり古い宛先を見て自分の `balloon.refresh` で下敷きを作る。確定では、同梱の宛先は本体の入れ替えの後には必ず無いので、「無い所へ置く」1 手にする（組み上げの時点の `ExistingState` と、確定の手の選び方を分ける）。戻しは「同梱を消す → 本体を戻す（古い木に古い同梱が入っている）」で、今の逆順の戻しのまま。
- **形 B**: 同梱を本体の配置に混ぜ、本体の木の `balloon\<dir>\` として組む（配置は 1 つ）。入れ替えは 1 回で済むが、同梱の `balloon.refresh` を本体の中の一部へ当てる組み上げが要り、`InstallOutcome.installed` の要素が 1 つ減る（`OnInstallComplete` の参照・入れた物の表示・`procedure.rs` の `installed_items` の数え方が変わる）。
- **形 C**: 確定の直前に宛先の在る・無いを見直す。組み上げと確定の間に誰かが作った木を消さないための今の約束（`commit_one` の説明）とぶつかるので、勧めにくい。

あわせて確かめること: `plan.rs` の兄弟テスト（`plan_roots_tests.rs` など）は「宛先は必ず `root.join("ghost"|"balloon").join(名)` の形」「外へ出ない」を全ての固定入力で歩いて数えている。同梱の宛先の形が変わるので、この約束の書き方（`plan.rs` の頭の説明）とテストを直す。

## 5. 実装の案

### 案 A: 今の部品を広げる

- `catalog_roots.rs` に `list_balloons_for(roots, ghost_dir)`・`find_balloon_for(...)` を足す。中で `<ゴースト>\balloon\` を `scan_balloons` と同じ判定で走査し、`union` と同じ「先に見つけた方を残す」でゴーストの中 → 根の和の順に合わせる。隠れは `Hidden` に控える。
- `BalloonEntry` に出どころの欄を足す（例: 根かゴーストの中か）。`root` には、ゴーストの中の項目ならゴーストのフォルダを入れる、などで済ませる。
- 選ぶ側 6 か所を新しい口へ替える。根だけの口（`list_balloons`・`find_balloon`・`install_root`・`survey`）は残す。
- `plan.rs` の `companion_placement` の宛先を本体の宛先の下へ替え、4.3 の形 A で入れ替えの順を手当てする。
- 良いところ: 触るファイルは brief の見立てどおり。新しいファイルは要らない。根の側の答えは 1 行も変わらない（要件 6）。
- 気になるところ: `catalog_roots.rs`（270 行）と `catalog.rs`（475 行）が少し太る。`BalloonEntry` の欄の追加は、根の項目を作る所（`scan_balloons`）とテストの組み立てに波及する。

### 案 B: ゴーストの中のバルーンを別の部品にする

- `areka-ghost` に `ghost_balloons.rs`（仮）を新しく置き、ゴーストの中の走査と、根の和との合成だけを持たせる。`catalog_roots.rs` は触らない。
- 良いところ: 「根」と「ゴーストの中」の考えが分かれ、根の部品の約束（`union` 1 つの上で決める＝baseware-root-list 要件 2.6）を崩さない。
- 気になるところ: 合成の決まり（同じフォルダ名の比べ方・並べ方・隠れの控え）を 2 か所に書くと、片方だけ直す事故が起きうる。`union` と `fold_key` を呼べば避けられるが、`union` は今は `fn(&BasewareRoot)` の走査しか受けない。

### 案 C: 組み合わせ

- 走査（`scan_ghost_balloons(ghost_dir)`）は `catalog.rs` の中で `scan_balloons` と本体を共有し、合成は `catalog_roots.rs` に 1 つの口として足す（案 A）。インストールの変更は `areka-nar` の中に閉じる（4.3 の形 A）。
- 実質は案 A と同じで、「走査と合成のどちらに何を置くか」をはっきりさせた形。

### 比べ

| | 案 A | 案 B | 案 C |
|---|---|---|---|
| 新しいファイル | 0 | 1 | 0 |
| 根の側の決まりの書き場所 | 1 か所 | 2 か所になりうる | 1 か所 |
| 根の答えが変わる恐れ | 低（根の口は残す） | 低 | 低 |
| 先例との揃い | `RootMark`・`survey` の先例に近い | シェル（`list_shells(ghost_dir)`）の先例に近い | 両方 |

## 6. 大きさと危うさ

- **大きさ: M〜L**（3〜7 日の上の方）。選ぶ側は一覧の口の差し替えが中心で、決め方の本体は触らない。インストールの入れ替えの順だけが新しい形になる。
- **危うさ: 中**。選ぶ側は既存の形を広げるだけで低い。インストールは、配置の入れ子という今まで無かった形で、`refresh,1` のときの巻き戻しと戻しの順を固めるテストが要る。

### タスク数の見立て: 16〜18（20 を超えない）

1. ゴーストの中の走査（`catalog.rs`・判定の共有・1.1〜1.3 のテスト）
2. 根の和との合成と隠れの控え（`catalog_roots.rs`・3.1・3.4・6.2 のテスト）
3. 出どころの欄と `name` の同着の決まり（3.3）
4. 隠れの記録（3.2・記録する時と回数）
5. 起動とゴースト切替の一覧の差し替え（`boot_config.rs`・逆の答えを固めたテストの書き換え・4.1〜4.7）
6. 起動の分かれ目のテスト（根が空・引数のゴースト・記憶から戻る・区切り入りの値）
7. ゴースト切替で入れ替わる・他のゴーストから見えない（2.2・2.3・4.6）
8. `\![change,balloon,…]` と `\![open,explorer,…]` の候補（`balloon_candidates` に引数を足す・`random`・`lastinstalled`）
9. メニュー（2.4・5.1）
10. `updateother` と今のバルーンの更新（2.1・8.3）
11. 読み直しの確かめ（5.7）
12. `areka-nar` の同梱の宛先（`plan.rs`・兄弟テストの約束の書き直し）
13. `areka-nar` の入れ替えの順（`install.rs`・`refresh,1`・巻き戻し・7.7）
14. `areka` のインストールの側（`destination.rs` が同梱で根を尋ねないこと・`installed_items` の表示名・7.3・7.6・8.2 のテスト）
15. ゴーストの書庫の中の名指しされない `balloon\<名>\`（7.5）とバルーンだけの書庫（7.4）のテスト
16. `doc/COMPAT_ARCHITECTURE.md` §8（370・376・308 行の書き直し・新しい行）
17. `dist/README.txt`
18. （入れるなら）最後の通しの確かめ

設計事項 7（シェルの書庫の同梱）で「目当てのゴーストの中へ」を取ると、12〜14 にテストが 1〜2 本増える。それでも 20 には届かない見込み。

## 7. 設計の段へ持ち込む事項

### 7.1 決めること（要件の討議・設計の討議の議題になりうるもの）

1. **同じ名前の優先と、利用者の選んだ記憶のぶつかり**（覚え書き (a)・上の 3.1）。案 1（今の要件のまま・書いて知らせる）・案 2（バルーンだけの書庫を入れたときに記録を 1 件足す）・案 3（記憶に欄を足す＝要件 5.3 を直す）。
2. **`name` の同着の決め方**（要件 3.3）。今の `name` の引き方（起動の `find_balloon_by_name`・切替の `resolve_skin_target`・`\![open,explorer,…]` の `skin_by_name`・`updateother`）はどれも「一覧の並びで最初」。取れる形は、(i) 一覧をゴーストの中を先・根を後に並べる（メニューの並びも変わる。要件 2.4 は「同じ一覧」とだけ書く）、(ii) 一覧はフォルダ名順のまま、`name` を引くときだけ出どころでゴーストの中を先に見る（引く所が 4 か所）、(iii) 合成の時点で、ゴーストの中と同じ `name` の根の項目も候補から外す（要件 3.1 は「同じフォルダ名」だけを外すと書くので、要件より広くなる）。
3. **項目の出どころの持ち方**。`BalloonEntry` に欄を足す・`root` の値で見分ける・ゴーストの中用の別の型にする、のどれか。`RootMark` に種類を足す先例はあるが、ゴーストの中は根ではないので、項目の側に持つ方が筋が通る。
4. **案 A／B／C のどれで組むか**（上の 5）。
5. **隠れの記録（要件 3.2）を残す時と回数**。一覧はメニューを出すたびにも作るので、作るたびに記録すると同じ行が何度も出る。起動とゴースト切替の 1 回だけにするか、作るたびにするか。記録の名前を根どうしの `root_hidden_item` と分けるか。
6. **「当たらなかった」の記録の `balloon_store` 欄**（要件 4.7「今と同じ形」）。ゴーストの中の置き場を欄に足すか、今のまま根の置き場だけを書くか。0 の失敗（`NoBalloon`）の告知の文面に載せる場所も同じ問い。
7. **シェルの書庫の同梱のバルーンの入れ先**。`manifest.rs` はシェルの書庫でも同梱を読む。要件 7.1 は「ゴーストの書庫」とだけ書き、7.4 は「バルーンだけの書庫は根」と書く。シェルの書庫の同梱を、(i) 今どおり根へ、(ii) 宛先のゴーストの中の `balloon\` へ、のどちらにするか。
8. **本体の `refresh,1` と同梱の `balloon.refresh` の関係**（要件 8.1）。名指しされない `balloon\<名>\` は本体の決まりで替わる。同梱は自分の `balloon.refresh` を持つ。上の 4.3 の形 A なら、同梱は本体の `refresh` に関わらず自分の鍵で替わる。この読み方でよいか。
9. **入れ替えの順の形**（上の 4.3 の形 A／B／C）。
10. **§8 の古い 2 か所の書き直しを本 spec でするか**（覚え書き (b)・上の 3.2）。370 行の「区別する」とテスト名、`shell_balloon_resolve.rs` の頭の説明。要件 9.1 で §8 のこの行を書き直すので、ついでに直すのが手数は少ない。

### 7.2 調べ物（設計の段で確かめる）

- `areka-nar` の兄弟テストのうち、宛先の形と「外へ出ない」を数えるテストの一覧と、書き換えの範囲（`plan_tests.rs`・`plan_roots_tests.rs`・`plan_source_path_tests.rs`・`install_commit_tests.rs`）。
- 同梱をゴーストの中へ入れたときの `InstalledElement.path` と `OnInstallComplete` の参照・入れた後の警告の文面（`procedure.rs` の `skipped_notice`）の見え方。
- 書ける根の写し（`roots/user_root.rs` の `plan_copy`）が、実行ファイルの隣のゴーストの中のバルーンの更新でどう動くか（ゴーストを写す前に、そのゴーストの中のバルーンだけが更新の対象になる道が在るか）。
- `--ghost` で渡した一時の置き場のゴーストでも、ゴーストの中のバルーンが数えられること（根の側は一時の置き場の `balloon\` を探さないが、ゴーストの中は別の道なので数えられる見込み）。

### 7.3 他の spec との重なり（ファイル）

- 同じファイルを触る未着地の spec は brief の棚卸㉘のとおり（`default-balloon-selfmade`・`readonly-mode`・`startup-install-option`・`dev-folder-alias`・`shell-companion-balloon`・`config-parse-diagnostics`・`install-live-target-hazards`）。働きの上で待つものは無い。
- `areka-nar` の `plan.rs`・`install.rs` を触るので、`install-live-target-hazards`（使用中の宛先への上書き）とファイルが重なる。

## 8. 次の段

- この分析は要件の討議（`/kiro-requirements-discussion areka-P0-ghost-inner-balloon`）の材料になる。7.1 の 1・2・5・6・7・8 は要件の書き方に響くので、要件の討議で答えを出すのがよい。3・4・9・10 は設計の段でも決められる。
- 要件が決まったら `/kiro-design areka-P0-ghost-inner-balloon` へ進む。

---

# 設計の段の調べと決め（2026-10-22・`/kiro-spec-design -y`）

## Summary

- **Feature**: `areka-P0-ghost-inner-balloon`
- **Discovery Scope**: Extension（既存の目録・決め方・インストールを広げる）。軽い調べ（`design-discovery-light`）。新しい依存は 0 なので外の調べ物はしていない。
- **Key Findings**:
  - 項目の出どころ（根かゴーストの中か）を読む呼び手は、合わせる口の外に 1 つも無い。並び・隠れの控えは合わせる口の中で済み、呼び手は場所 `dir` を使うだけ＝欄は足さなくてよい。
  - `name` を引く 4 か所はどれも「一覧の並びで最初」なので、並びを「ゴーストの中が先」にするだけで要件 3.3 を満たす。引き方は変えない。
  - 同梱の宛先をゴーストの中へ移すと、`refresh,1` のゴーストの上書きで巻き戻る危うさ（gap 分析 4.3）は**確かにある**。もう 1 つ、書庫の本体に同梱と同じ名前の `balloon/<dir>/` が在ると、本体の確定の後に宛先が在って同梱の `rename` が失敗する危うさが見つかった。
  - 今の同梱は `InstallRoots::balloon_root` を尋ね、`areka` の `CatalogDestinations::copied_root` が実行ファイルの隣の根の同じフォルダ名のバルーンを書ける根へ**写す**。同梱が根を尋ねなくなれば、この写しの道が消える（要件 7.3 の助け）。
  - 入れた後の表示名は根の目録から引いているので、移した後は根の同じフォルダ名（別の版）の `name` を取りうる。ゴーストの中から引く形へ替える。

## Research Log

### `areka-nar` の組み上げと確定（gap 分析 4.3・7.2 の調べ）
- **調べた所**: `crates/areka-nar/src/plan.rs`（`body_placement` 123 行・`companion_placement` 191 行・`existing_target_ghost` 229 行・`Placement` 47〜71 行）、`install.rs`（`stage_placement` 286〜299 行・`copy_existing` 326〜354 行・`commit_one` 503〜532 行・`unwind` 580〜607 行・`area.retired` 162 行）。
- **分かったこと**:
  - `ExistingState` は組み上げのときに `destination.is_dir()` だけで決まる。`Overlay` は古い木を丸ごと（空のフォルダも）写し、`Replace{keep}` は残す名前のファイルだけを写す（ASCII の大文字小文字を区別しない）。
  - 確定は「在れば宛先 → 退避、組み上げ → 宛先」。退避は `<根>\.nar-work\<pid>-<連番>\old-<k>` で、宛先の隣ではない＝ゴーストの木に漏れない。戻しは逆順で、退避と組み上げは全部の確定の後に 1 回だけ消す。
  - 本体が `Overlay` なら、本体の新しい木に古い `balloon\<dir>` の写しが入り、同梱の入れ替えはそれを退避して置く＝答えは合う（写しが 2 回になるだけ）。
  - 本体が `Replace{keep}` なら、本体の確定の後に `balloon\<dir>` がふつうは無く、同梱の `rename(宛先, 退避)` が NotFound で失敗し、全体が巻き戻る（確認）。
  - 同梱が組み上げで「新しい」でも、書庫の本体に `balloon/<dir>/` の項目が在ると本体の確定の後に宛先が在り、`rename(組み上げ, 宛先)` が失敗して全体が巻き戻る（新しく見つけた）。
  - シェルの書庫は、同梱の宛先 `ghost\<対象>\balloon\<dir>` が本体 `ghost\<対象>\shell\<名>` の下に無いので入れ子にならない。
- **影響**: 形 A（本体の下敷きと書庫の本体の項目から置く同梱の宛先を外し、入れ子の同梱は「無い所へ置く」1 手）で両方の危うさが消える。

### 同梱の宛先の問い方と写し
- **調べた所**: `crates/areka/src/install/destination.rs`（`balloon_root` 94〜107 行・`copied_root` 131〜133 行・`install_into` 180〜196 行）、`areka-nar/src/install.rs` の `InstallRoots`（46〜51 行）。
- **分かったこと**: `balloon_root` の呼び手は `plan.rs` の 2 か所（バルーンだけの書庫の本体・同梱）だけ。同梱をやめてもバルーンだけの書庫のために残す。`install_placed` の記録は `starts_with` で根を引くので、ゴーストの中の場所でも動く。
- **影響**: 同梱の `root` は本体の根にする（作業の場所が同じ根になり、`rename` が同じボリュームで済む）。

### 入れた後の表示名とイベント
- **調べた所**: `install/procedure.rs`（`installed_items` 422〜441 行・`skipped_notice` 444〜478 行・`installed_record` 481 行〜）、`install/judge.rs`（163〜184 行）。
- **分かったこと**: `InstalledElement.path` は配置の宛先そのもの。`OnInstallCompleteEx` の Reference2 がゴーストの中の場所になる（意図どおり）。`OnInstallComplete`・`skipped_notice`・`installed_record` は場所の変化の影響を受けない。
- **影響**: 表示名だけを `balloons_for` から引く形へ替える。

### バルーンだけの書庫の後（要件 3.5）
- **調べた所**: `install/desk.rs` 372〜379 行、`boot_config.rs` の `BootContext`、`emo2_boot/ghost_switch.rs` 844 行（切替で `ctx.current` を替える）。
- **分かったこと**: その場に `BootContext.roots` と `ctx.current.ghost.dir` が在る。`balloons_for(..).hidden` に入れたフォルダ名が在れば、「入れた場所」と「隠すゴーストの中の場所」がそのまま取れる。

### ネットワーク更新（要件 8・gap 分析 7.2）
- **調べた所**: `update/worker.rs` `prepare_targets` 257〜268 行、`roots/user_root.rs` `plan_copy` 57〜84 行、`emo2_boot/switch_copy.rs` 45 行、`crates/areka-update/src/manifest.rs` `invalid_path` 121 行。
- **分かったこと**: 書ける根への写しは、場所の親が `<実行ファイルの隣>\balloon` のときだけ。`<実行ファイルの隣>\ghost\<g>\balloon\<b>` は写さずにそのまま書く。ただし起こすゴーストは前もって書ける根へ写されるので、ふつうは写しの中を書く。写されずに動くゴーストは、ゴーストそのものもそのまま書く決まりなので、扱いは揃っている。
- **影響**: コードの変更は 0。確かめるテストを足す。（起動のときのゴーストも `switch_copy` を通るかは読み切っていない＝下の Risks）

### `--ghost` の一時の置き場（gap 分析 7.2）
- 一時の置き場の印は根の走査（`scan_balloons`）だけが見る。ゴーストの中の走査はゴーストのフォルダを直接受けるので、`--ghost` で渡したゴーストでも数える（要件 1.4）。

### 項目の欄の読み手
- `BalloonEntry` を作る所は `catalog.rs` の `scan_balloons` とテスト 4 ファイル（`catalog_tests.rs`・`boot_resolve_tests.rs`・`boot_resolve_balloon_tests.rs` ほか）だけ。`areka` の中で `BalloonEntry.root` を読む所は 0 か所（`install_root` の中だけが根の一覧の `root` を読む）。

## Architecture Pattern Evaluation

| 案 | 中身 | 良いところ | 気になるところ | 結論 |
|---|---|---|---|---|
| A 今の部品を広げる | `catalog_roots.rs` に合わせる口・`BalloonEntry` に出どころの欄 | 新しいファイル 0 | 欄の追加が根の項目とテストに波及 | 欄は足さない形で C に吸収 |
| B 別の部品 | `ghost_balloons.rs` を新しく置く | 根の部品に触らない | 合わせ方の決まりが 2 か所に | 取らない |
| C 組み合わせ | 走査は `catalog.rs` で本体を共有・合わせる口は `catalog_roots.rs` | 決まりの書き場所が 1 か所・根の口は不変 | 2 ファイルが少し太る | **取る** |

インストールの形:

| 形 | 中身 | 結論 |
|---|---|---|
| A | 本体から同梱の宛先を外し、同梱は「無い所へ置く」 | **取る**（欄 2 つ・分かれ 2 つ） |
| B | 同梱を本体の木に混ぜる | 取らない（木の一部ごとの `refresh` と在る・無い・入れた物の数え方の変更・シェルは別の道が残る） |
| C | 確定の直前に在る・無いを見直す | 取らない（組み上げと確定の間に作られた木を消さない約束とぶつかる） |

## Design Decisions

### Decision: 項目の出どころの持ち方（gap 分析 7.1 の 3）
- **Context**: 項目が根かゴーストの中かを、どこに持つか。
- **Alternatives Considered**:
  1. `BalloonEntry` に出どころの欄を足す
  2. `RootMark` に種類を足す
  3. 欄を足さず、`root` 欄にゴーストのフォルダを入れる
- **Selected Approach**: 3。`root` の説明を「走査した基のフォルダ」に直す。
- **Rationale**: 出どころを読む呼び手が無い（並び・隠れの控えは合わせる口の中で済む）。読まない欄は足さない。ゴーストの中は根ではないので `RootMark` も合わない。
- **Trade-offs**: 将来 `root` を「根」として読む呼び手を足すときは確かめ直しが要る（design の Revalidation Triggers）。

### Decision: 組み方（gap 分析 7.1 の 4）
- **Selected Approach**: 案 C。`catalog.rs` の `scan_balloon_store(store, base)` を根とゴーストの中で共有し、`catalog_roots.rs` の `balloons_for(roots, ghost_dir) -> GhostBalloons { listed, hidden }` を 1 つの口にする。根の口は変えない。
- **Rationale**: 判定と記録が 1 つの関数を通るので、片方だけ直す事故が起きない。根の答え（要件 6）が 1 行も変わらない。

### Decision: 名前の同着（要件 3.3）
- **Selected Approach**: 並びを「ゴーストの中が先」にするだけ。`find_balloon_by_name`・`resolve_skin_target`・`skin_by_name`・`updateother` は変えない。
- **Rationale**: 4 か所はどれも「並びで最初」。要件の討議の決め 2 と同じ答えを、変更 0 行で 4 か所に揃えられる。

### Decision: 「ゴーストの中が根を隠した」記録の時と回数（gap 分析 7.1 の 5）
- **Alternatives Considered**: 一覧を作るたび／起動と切替だけ／起動時の `survey` に混ぜる。
- **Selected Approach**: 起動とゴースト切替（読み直しの自分への切替を含む）で `resolve_balloon_for_ghost` が 1 件ごとに info `ghost_balloon_hides_root` を残す。合わせる口は記録を残さず `hidden` を返すだけ。根どうしの `root_hidden_item` とは名前を分ける（`roots_hidden_count` の件数に混ぜない）。
- **Rationale**: メニューを出すたびに同じ行が増えない。根どうしの隠れの記録（起動時・並びの変更時）と同じ「場面で 1 回」の先例に揃う。`survey` はゴーストを知らないので混ぜられない。

### Decision: `balloon_store` 欄と `NoBalloon` の告知（gap 分析 7.1 の 6）
- **Selected Approach**: 今どおり根の置き場の列だけを載せる。記録の文の「根に見つからない」は「根にもゴーストの中にも見つからない」へ直す（事象名と欄は変えない）。
- **Rationale**: 要件 4.7「今と同じ形」を欄の綴りまで守る。ゴーストの中の置き場はゴーストのフォルダから一意に決まり、起動と切替の記録に出る。告知は利用者向けで、利用者がバルーンを置く先は根の置き場（バルーンだけの書庫も根へ入る）。

### Decision: 本体の `refresh` と同梱の `balloon.refresh`（gap 分析 7.1 の 8）
- **Selected Approach**: 名指しされない `<ゴースト>\balloon\<名>\` は本体の `refresh` で替わる。名指しされた同梱は、本体の `refresh` に関わらず自分の `balloon.refresh`・`balloon.refreshundeletemask` で替わる。
- **Rationale**: 同梱の替わり方を決める鍵は ukadoc の `balloon.refresh` で、要件 7.2 はその意味を変えないと決めている。要件 8.1 の「ゴーストのフォルダの他のファイルと同じ決まり」は「ゴーストの書庫の入れ方（`install.txt` の鍵）で替わり、別の道を作らない」と読む。
- **Trade-offs**: 本体だけ `refresh,1` で同梱に `balloon.refresh` が無いと、同梱の古いファイルが残る。SSP で根に入れていたときと同じ振る舞いなので、作者の書き方で選べる。設計の討議で確かめてもらう点として残す。

### Decision: インストールの入れ替えの順（gap 分析 7.1 の 9）
- **Selected Approach**: 形 A。`Placement` に `carve_out: Vec<String>`（本体の下敷きから外す相対のフォルダ）と `within_body: bool`（確定で「無い所へ置く」1 手）を足す。ゴーストの書庫の本体の項目のうち、置く同梱の `balloon/<dir>/` の下も除き、除いたら warn `companion_overrides_archive_folder` を 1 件。
- **Rationale**: 2 つの危うさ（`refresh,1` の巻き戻り・書庫の同名のフォルダ）を、今の戻しの順を変えずに消せる最小の形。
- **Follow-up**: `lib_tests.rs` の「2 つ目の根で失敗したら 1 つ目を戻す」（TwoRoots）は、同梱が別の根へ行かなくなるので入れ子の戻しのテストへ置き換える。

### Decision: シェルの書庫の同梱（要件 7.8）
- **Selected Approach**: 宛先は `existing_target_ghost` が確かめた対象のゴーストの `balloon\<dir>`。入れ子にならないので、確定は今の手のまま。

### Decision: 古い記述の直し（gap 分析 7.1 の 10）
- **Selected Approach**: 本 spec で直す（要件 9.2）。§8 の 370 行の「区別する完全一致」と無いテスト名 `descript_match_is_case_sensitive`、`shell_balloon_resolve.rs` の頭の説明の「フォルダ名（大文字小文字を区別）」。あわせて §8 の 343 行の飛ばした同梱の置き場の綴りも直す。

## Synthesis（一般化・作るか使うか・削る）

- **一般化**: 「今のゴーストの候補の一覧」は 6 か所の呼び手の同じ問いなので、1 つの口（`balloons_for`）にした。`resolve_balloon` の段や `name` の引き方は一般化せず、一覧の並びだけで答えを揃えた。
- **作るか使うか**: 走査は既存の判定（`scan_balloons` の本体）・比べ方（`fold_key`）・隠れの型（`Hidden`）・テストの一時フォルダ（`TempPath::under_target`）をそのまま使う。インストールは既存の `rename` と逆順の戻しを使う。新しい依存は 0。
- **削ったもの**: 出どころの欄・`RootMark` の種類・`find_balloon_for`・ゴーストの中用の別の型・`balloon_store` 欄の追加・`NoBalloon` の文面の変更・記録の場面の設定。

## Risks & Mitigations

- `areka-nar` の比べ方は ASCII の大文字小文字だけを区別しないので、古いフォルダ名と `balloon.directory` が ASCII の外の字の大文字小文字だけ違うと下敷きから外せない — そのときは同梱の `rename` が失敗し、全体が今どおり巻き戻って記録が残る（壊れた木は残らない）。
- `install-live-target-hazards` と `plan.rs`・`install.rs` が重なる — 同じウェーブに置かない。後の方が `within_body` の確定の手を前提に組む。
- 起動のときのゴーストが `switch_copy` を通るかは読み切っていない — 通らなくても、ゴーストそのものと同じ「そのまま書く」決まりなので、ゴーストの中のバルーンだけが違う扱いにはならない。実装の段でネットワーク更新のテストを書くときに確かめる。
- 名前の同着がメニューの並びに依る — 並びを変える後の変更は要件 3.3 を壊す。`catalog_roots_ghost_balloon_tests.rs` で並びを固める。

## タスク数の見立て

13〜15（20 を超えない）: 走査と合わせる口／起動と切替の差し替えと記録／切替・読み直しのテスト／実行中の 4 か所／インストールの後の記録と表示名／`plan.rs` の宛先と除外／`install.rs` の入れ子の確定／`areka-nar` のテストの書き直し／`areka` のインストールのテスト（根に触らない・表示名・バルーンだけの書庫）／ネットワーク更新の確かめ／§8／README／通しの確かめ。

## 使った決まり・手引き

- `.claude/skills/kiro-spec-design/rules/design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・`design-principles.md`、`.kiro/settings/templates/specs/design.md`・`research.md`。
- steering の `logging.md`（事象名の形）・`structure.md`（兄弟テスト）。
- 調べの一部（`areka-nar` の組み上げと確定・インストールの側・ネットワーク更新）はサブエージェントに読ませ、要点だけを上に写した。

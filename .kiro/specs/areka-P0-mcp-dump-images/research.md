# ギャップ分析: areka-P0-mcp-dump-images

> 2026-10-04・本ブランチ（main `e2a373b5` の後）のコードを読んで書いた。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> この文書は**材料と選択肢**を並べるもので、どれを採るかは要件ディスカッションと設計で決める。

## 1. 要約

- **今の見た目（`surface` 省略）は、表示の層を 1 行も変えずに作れる。** 絵のバイト列・大きさ・今の surface ID・「表示していない」の判定は、どれも `EmoPresenter` の既存の公開の読み口で取れる。`error!` を出さずに事前に確かめる手もある。足すのは結線状態 `Emo2Wiring` の読み口の `#[cfg(test)]` を外すことだけ。
- **指定した surface 単体は、`areka-emo-compose` は触らずに済むが、`areka-emo-present` に公開の口を 1 本足さないと作れない。** 合成器 `Composer::compose` は純粋な CPU の関数で呼ぶだけでよい。しかし入力（そのスコープの `EmoWorld` とアトラス）は装着のときに表示の層へ所有ごと渡され、外から借りる口が無い。surface ID が在るかどうか（要件 4.2）を `error!` なしで確かめる口も同じ場所にしか置けない。**これは同じウェーブの約束（触るファイルの一覧）の外**なので、止めて報告する対象。
- **バルーンは、閉じた後も背景・文字とも読める。** ただし画面の拡大率が 1 でないと、背景（原寸）と文字（拡大後の大きさ）が合わない。文字を縮めて重ねるか、原寸で描き直すかで、手間と触るファイルが大きく変わる。
- **シェルの絵の中の箱の文字は、キャラクターの読み戻しに写らない。**（別の面として窓の子に付いている。）
- **PNG は WIC の符号化器（新しい依存なし・圧縮あり）か、標準ライブラリだけの無圧縮の書き出し（テストに前例あり）のどちらか。** base64 は標準ライブラリだけで数十行。応答の大きさの上限は areka 側には無い。
- 規模は M のまま。リスクは中（約束の外のファイルを触る要が出たこと・拡大率が 1 でないときのバルーン）。

## 2. 今あるもの

### 2.1 MCP の入口

- 2 本のダミーは `crates/areka/src/mcp/dump_surface.rs`・`dump_balloon.rs` の `handle(world, ghost, args, reply)`。`reply.send(outcome::ng("not implemented yet"))` の 1 文だけ。テストは各 `_tests.rs` に 1 本ずつ（`NG:not implemented yet` を固定）。
- 呼ばれるのは汲む系 `drain`（`crates/areka/src/mcp/mod.rs`）から。`drain` は `Input` の段の排他の系で、UI スレッドで走る。
- 結果を作る関数は `crates/areka-mcp/src/tools/outcome.rs` の `value`・`ok`・`ng`・`with_image(outcome, png_base64: String)`。`ok(note)` は `OK:<note>` を作るので、成功の本文 3 種はそのまま渡せる。
- 返事の口 `ReplyTo`（`crates/areka-mcp/src/tools/bridge.rs`）は送り手と合図と文字列だけでできていて、スレッドをまたいで運べる（MCP のスレッドから UI スレッドへ運ばれてくる）。その場で答えられない処理は `later`（`mcp/mod.rs`）へ預ける口もある。
- 待ちの上限 `REPLY_WAIT`（`crates/areka-mcp/src/tools/mod.rs`）は 10 秒。
- 要求の本文の上限は 4 MiB（`crates/areka-mcp/src/dispatch.rs` の `MAX_BODY_BYTES`）。**応答の側の上限は areka のコードに無い。**

### 2.2 キャラクターの絵（表示の層）

`crates/areka-emo-present/src/presenter/read.rs` の `impl EmoPresenter` にある公開の読み口:

| 読み口 | 返すもの | 本 spec での使い道 |
|---|---|---|
| `read_back(target)` | 合成メモが持つ原寸の絵（乗算済み BGRA・1 行＝幅×4）。大きさは返さない | 今の見た目・バルーンの背景 |
| `text_slot_view(target)` の `surface_size()` | 表示中の絵の原寸（拡大の前） | PNG の幅と高さ |
| `current_surface_id(target)` | 最後に確立した surface ID。隠した・一度も出していない・未登録は `None` | 本文の surface ID と、要件 4.3 の判定 |
| `target_visible(target)` | 未登録は `None`、登録済みは見えているか | その target が在るかの判定 |
| `applied_ratio(target)` | 実際に掛かっている拡大率（有理数）。一度も表示が成立していなければ `None` | 「一度も表示していない」の事前の判定・バルーンの拡大率 |

確かめた性質:

- `read_back` は、未登録・一度も表示していない・メモから消えた、の 3 つで `error!` を出して失敗を返す。前の 2 つは上の読み口で**事前に見分けられる**（`target_visible` が `None`／`applied_ratio` が `None`。拡大率と「最後の表示の入力」は同じ 1 か所＝`apply_show` の表示成立点（`presenter/show.rs`）で書かれる）。3 つ目（メモから消えた）は事前に見分ける口が無いが、起きるのはメモの全破棄（`InvalidateCache`）の後に再表示が来ていない間だけ（最後に表示した絵は常に一番新しく使われた席なので、容量 3 の入れ替えでは消えない）。これは要件 4.7 の「想定外の失敗」に当たり、`error!` が出てよい場面。
- `read_back` は読むだけ（`cache.get` は使用順を動かさない）。要件 1.5・3.8 を満たす。
- 絵は画面の拡大率に依らない原寸（合成は常に原寸で、拡大は描画の変換行列が掛ける）。要件 1.3 を満たす。
- 隠す処理 `apply_hide`（`presenter/hub.rs`）はメモも「最後の表示の入力」も消さない。隠した後の `read_back` は前の絵を返す＝キャラクターでは要件 4.3 のために**先に `current_surface_id` を見る**必要がある。
- 本番で `read_back` を呼んでいる所は 0（呼び手はテストと examples だけ）。結線状態の包み `Emo2Wiring::read_back_target`（`crates/areka/src/emo2_boot/frame/wiring.rs`）は `#[cfg(test)]`。同じファイルの `presenter()`・`runtime()` は本番で使える読み口。

### 2.3 指定した surface を画面の外で合成する材料

- 合成器 `Composer::compose(world, atlas, surface_id, binds, pattern)`（`crates/areka-emo-compose/src/lib.rs`）は窓にも GPU にも触れない関数で、結果を保持しない。着せ替えは渡した `BindSet` だけで決まり、アニメーションのコマは渡した `PatternState` だけで決まる（空なら「始まる前」）。**`areka-emo-compose` は呼ぶだけで足りる。**
- surface が無いとき、計画の段 `build_plan`（`areka-emo-compose/src/plan.rs`）が `error!` を出す。要件 4.6 のために、呼ぶ前に `EmoWorld::surface(id)`（`areka-emo-compose/src/world.rs`）で在るかを確かめる必要がある。
- **入力に外から届かない。** `EmoWorld` とアトラスは `EmoPresenter::attach_target`（`presenter/hub.rs`）へ所有ごと渡され、`PresentTarget`（`presenter/target.rs`）の `pub(super)` の欄になる。`EmoPresenter` に、これらを貸す口も、画面を変えずに合成する口も、surface が在るかを答える口も無い。起動の資産 `BootAssets` は装着の相 `run_attach_phase`（`emo2_boot/frame/attach.rs`）が使い切る。
- **今の着せ替えの置き場**は 2 つ。
  - 表示の層: `PresentTarget.last_show` の `BindSet`（最後に画面へ出た入力・隠しても残る・非公開）。
  - seriko: 起動時にオンの着せ替え（`static_binds`）と、その後の `\![bind]` の積み上げ。seriko は別スレッドの実行系で、UI スレッドから今の集合を尋ねる口は見つからなかった。
- シェルは最初の `\s` が来るまで表示されない（`run_attach_phase` はシェルの `ShowSurface` を出さない）。その間は表示の層に着せ替えの記録が無い。

### 2.4 バルーン

- **背景**は表示の層の target（`balloon_target(scope)`＝`emo2_boot/target_map.rs`）。装着の相が、見えないまま面 0 を確立する（`set_visibility_ownership(External)` の後に `ShowSurface`）。**だから背景だけ見ても「まだ描いていない」は分からない。**
- **文字**は別の面 `TextSurface`（`crates/areka-emo-text/src/surface.rs`）。`read_back()` は GPU の面を読み出し用の面へ写して読む本物の読み戻しで、大きさは**拡大後**（文字を描く領域の寸×拡大率を切り上げ）。覆うのは文字を描く領域だけで、窓の中の位置は領域の原点×拡大率。
- 文字の面に届く道: `Emo2Wiring::runtime()` → `TextLayerRuntime::surface(actor)`（`areka-emo-text/src/actor.rs`・公開）→ `TextSurface::read_back()`・`size()`。**emo-text の受け口（`TextMsg`）を広げる必要は無い**（brief の見立てどおり）。
- 文字の面が生まれるのは、そのスコープに文字が来た最初のフレーム（`present_frame`＝`areka-emo-text/src/actor_present.rs`）。一度も話していないスコープには面が無い（`surface(actor)` が `None`）。
- 文字の中身を消すのは `\c`（`Clear`）と台詞の頭の全消し（`ClearAll`）だけ（`TextLayerRuntime::apply_cue`）。バルーンを閉じる・時間切れで消す処理（`emo2_boot/balloon_visibility_phase.rs`）は表示の層へ `Hide` を出すだけで、文字の層へは何も送らない。
- 文字の状態（何が書かれているか）は `TextLayerRuntime::state()` → `actor_state(actor)` → `items()` で読める（公開）。「描いた内容が残っているか」はここで判定できる。
- 拡大率が変わると、文字の面は捨てられて次のフレームで新しい大きさで作り直され、残っている状態から全部描き直される（`refresh_actor_binding`＝`areka-emo-text/src/actor_attach.rs`）。
- 背景と文字の位置合わせに要る値: 背景の原寸（`text_slot_view(balloon_target).surface_size()`）、拡大率（`applied_ratio`）、文字の面の窓の中の位置（差し込み口の entity の `Arrangement` の offset＝拡大後の px。`text_slot_view(..).slot()` で entity が取れる）。
- 選択肢にポインタが乗っているときの強調は文字の面に焼かれている。撮ればそのまま写る。

### 2.5 シェルの絵の中の箱

- 箱の文字は、場所の鍵 `PlaceKey`（`areka-emo-text/src/place.rs`）の `TextPlace::Box(名前)` ごとに別の `TextSurface` を持ち、シェルの窓の直接の子として付く（`TextSurface::attach_window_child`）。
- キャラクターの `read_back` が返すのは `Composer::compose_into` の出力だけで、箱の文字の面は入っていない。

### 2.6 スコープと窓

- 窓は descript が宣言したスコープの分だけ作られる（`detect_scopes`＝`crates/areka/src/placement/config.rs`）。窓の一覧は World の `GhostWindows`（`crates/areka/src/placement/spawn.rs` の `scopes()`・`char_window(scope)`・`balloon_window(scope)`）。
- 絵の資産はスコープ 0 と 1 だけ（`derive_scopes`＝`crates/areka/src/emo2_boot/mod.rs` が `[0, 1]` を返す）。**窓はあるが絵の無いスコープ（2 以降）**は、装着の相が警告を出して飛ばす＝表示の層に target が無い。
- 装着の相は GPU の準備ができたフレームで 1 回だけ走る。起動の直後は、窓も結線状態もあるのに target がまだ無い時間がある。

### 2.7 窓の無いゴースト

- 窓への結線が成立しなかった起動では `Emo2Wiring` が World に入らない（`emo2_frame_system`＝`emo2_boot/frame.rs` の冒頭の注記）。起動の単位 `GhostSession`（`crates/areka/src/ghost_session.rs`）の `logsink_fallback()` も真になる。
- `handle` に渡る `ActiveGhost`（`mcp/resolve.rs`）はこの旗を持たないが、`handle` は World を受け取るので `GhostSlot` と `Emo2Wiring` を自分で読める。
- **毎フレームの系が結線状態を一時的に外す間に MCP の処理が走ることは無い。** `emo2_frame_system` は `Update` の段の排他の系で、外して戻すまでが 1 回の呼び出しの中で閉じる。MCP の `drain` は `Input` の段の別の排他の系。本番で `Emo2Wiring` を外すのはここだけ。

### 2.8 PNG と base64

- 本番のコードに PNG の符号化も base64 も無い。`areka-mcp` は rmcp の base64 の機能を切っている。`image` クレートは `wintf` のテスト用の依存で、本番には入っていない。
- WIC は `windows` クレートの宣言済みの機能（根の `Cargo.toml` の `Win32_Graphics_Imaging_D2D`・`Win32_System_Com`・`Win32_UI_Shell`）で使える。今は復号だけに使っている（`crates/areka-emo-atlas/src/decode/wic_arm.rs`。COM の初期化は呼ぶ側の責任と書いてある）。
- 標準ライブラリだけで PNG を書く前例はテストにある（`crates/areka-emo-text/src/viewbox_draw_png_dump_tests.rs`＝無圧縮）。CRC-32 は本番のコードに在る（`crates/areka-nar/src/crc32.rs`）が、`areka` から `areka-nar` への依存の向きは設計で確かめる。

### 2.9 テストの土台

- GPU を通すクレート内の検査の土台は `crates/areka/src/emo2_boot/spine*.rs`（`read_back_target` で画素を確かめている・x64 だけで接続）。
- 文字の面の読み戻しの検査は `crates/areka-emo-text/tests/` と `src/actor_*_tests.rs` に多数。
- SSP との差の一覧は `doc/ssp-mcp/transport-diff-areka.md`。

## 3. 要件の作者が未確認として残した 3 点

### ⑴ バルーンを閉じた後も、背景と文字は読めるか（要件 3.4）

**読める。**

- 背景: 閉じる処理は表示の層の `Hide`。`apply_hide` はメモと「最後の表示の入力」を残すので、`read_back(balloon_target)` は成功して最後の背景を返す（`current_surface_id` は `None` になるが、バルーンではこれを判定に使わなければよい）。
- 文字: 閉じても文字の面も文字の状態もそのまま。消えるのは `\c` と次の台詞の頭の全消しのときだけ。

「描いた内容が残っていない」（要件 4.4）の判定に使えるもの:

- 文字の面が無い（`TextLayerRuntime::surface(actor)` が `None`）＝一度も文字が来ていない。
- 文字の状態が空（`actor_state(actor)` の `items()` が空）＝全消しの後、または一度も話していない。

注意が 2 つ:

- 台詞の頭の全消しは、ゴーストが何か話し始めるたびに全スコープに効く。スコープ 0 が話した後にスコープ 1 だけが話すと、スコープ 0 のバルーンは「残っていない」になる。これは areka の文字の層の今の振る舞いで、SSP が同じかは未実測。
- 背景の surface（`\b[2]` などで替えた面）は「最後の表示の入力」に残るので、閉じた後も替えた後の背景が返る。

### ⑵ 指定した surface に今の着せ替えを掛けられるか——`areka-emo-compose` を触らずに（要件 2.2）

**`areka-emo-compose` は触らずに済む。ただし `areka-emo-present` に公開の口が 1 本要る。**

- 合成そのものは `Composer::compose` に `BindSet` を渡すだけ。
- 入力（`EmoWorld`・アトラス）と今の着せ替え（`last_show` の `BindSet`）は表示の層の非公開の欄にしか無い。アプリの側から届く道は無い。
- 足す口の形の例: 「この target の資産で、この surface を、最後に表示したときの着せ替えで、アニメーション無しで合成して返す。surface が無ければ `error!` を出さずにそう答える」。読むだけ（`&self`）で書け、メモも画面も変えない。新しいファイル 1 つと、親モジュールへの 1 行で足せる。
- **まだ一度も表示していないスコープ**では表示の層に着せ替えの記録が無い。起動時にオンの着せ替えは seriko が持っていて、装着の相は自分の分を捨てている。このときに何を掛けるかは決めが要る（§7 の論点 2）。
- 隠している間に `\![bind]` が来たとき、表示の層の記録が古くなるかは未確認（§6）。

### ⑶ シェルの絵の中の箱の文字は、キャラクターの読み戻しに写るか

**写らない。** 箱の文字は窓の子に付いた別の面で、合成メモの絵には入っていない。`dump_surface`（省略）は、画面では箱に文字が見えていても、文字の無いキャラクターの絵を返す。写すなら、バルーンと同じ「文字の面を読んで重ねる」処理が箱ごとに要る（拡大率が 1 でないときの問題も同じ）。要件 10 の暫定の裁定は「どちらでも合否を変えない・差の一覧に書く」。

## 4. 要件と今あるものの対応

印: **在る**＝そのまま使える／**無い**＝作る／**不明**＝設計で調べる／**制約**＝今の作りが縛る。

| 要件 | 要るもの | 今あるもの | 印 |
|---|---|---|---|
| 1.1 今の絵 | 合成済みの絵の読み戻し | `EmoPresenter::read_back`・`Emo2Wiring::presenter()` | 在る（`read_back_target` の `#[cfg(test)]` を外すか、`presenter()` 経由で呼ぶ） |
| 1.2・3.2 スコープ省略＝0 | 引数の読み替え | `Args.scope: Option<i64>` | 無い（数行） |
| 1.3・2.5 原寸 | 拡大の前の画素 | `read_back`・`Composer` とも原寸 | 在る |
| 1.4 本文と surface ID | 今の surface ID | `current_surface_id` | 在る |
| 1.5・2.3・3.8 画面を変えない | 読むだけの口 | `read_back`・`TextSurface::read_back` は読むだけ | 在る（指定 surface は新しい口の作り方で守る） |
| 2.1 指定 surface 単体 | 画面の外の合成 | `Composer::compose`（入力に届かない） | **無い・制約**（表示の層に口が要る） |
| 2.2 今の着せ替え | そのスコープの `BindSet` | `last_show`（非公開）・seriko | **無い・制約**／一度も表示していないときは**不明** |
| 3.1 背景＋文字 | 2 枚を重ねる | 背景 `read_back`・文字 `TextSurface::read_back`・位置は差し込み口の `Arrangement` | 無い（重ねる処理） |
| 3.3 原寸・位置がずれない | 拡大率が 1 でないときの文字 | 文字の面は拡大後の大きさ | **制約**（縮めるか描き直すか） |
| 3.4 隠れていても返す | 閉じた後の読み戻し | §3 ⑴ | 在る |
| 3.5 話している途中 | その時点の面 | 文字の面は毎フレーム更新 | 在る |
| 3.6 箱を含めない | バルーンの場所だけ読む | `TextLayerRuntime::surface(actor)` は普通のバルーンだけ | 在る |
| 4.1 無いスコープ | キャラクターの窓の有無 | `GhostWindows::char_window(scope)` | 在る（窓はあるが絵の無いスコープの扱いは**不明**） |
| 4.2 無い surface ID | そのスコープのシェルの面の表 | `EmoWorld::surface(id)`（届かない） | **無い・制約**（2.1 と同じ口で解ける） |
| 4.3 何も表示していない | 表示の有無 | `current_surface_id` | 在る |
| 4.4 バルーン未描画 | 文字の有無 | `TextLayerRuntime::state()`・`surface(actor)` | 在る |
| 4.5 窓の無いゴースト | 結線の有無 | `Emo2Wiring` の有無・`GhostSession::logsink_fallback()` | 在る |
| 4.6 ERROR を出さない | 事前の判定 | §2.2 の読み口で足りる（指定 surface は新しい口の中で） | 在る／無い |
| 4.7 想定外の失敗 | `NG:`＋`error!` 1 件 | `outcome::ng` | 無い（数行） |
| 4.9 判定の順 | 窓や GPU の要らない判断の関数 | — | 無い（純粋な関数として新規） |
| 5.1 base64 | 標準の文字集合・詰めあり | — | 無い |
| 5.2 乗算を戻した RGBA の PNG | 乗算を戻す処理＋PNG の符号化 | WIC（復号だけ使用中）・テストの無圧縮の書き出し | 無い |
| 5.5 上限なし | 応答の大きさ | areka 側の応答の上限は無い | 在る（受け取る側の上限は**不明**） |
| 6.1 UI を止めない | 1 回の呼び出しで終わる | 読み戻しは同期・数 ms の見込み。符号化の時間は方式による | **不明**（測る） |
| 6.2 イベント 0・台本 0・ファイル 0 | — | 読み口はどれも副作用なし | 在る |
| 7.1〜7.3 決定論テスト | 符号化・判断・本文 | — | 無い |
| 7.4 描画を通るテスト | GPU の検査の土台 | `emo2_boot/spine*.rs`・emo-text の読み戻しの検査 | 在る（土台）／無い（本 spec の検査） |
| 7.5 ダミーのテストの書き換え | — | `dump_surface_tests.rs`・`dump_balloon_tests.rs` | 無い |
| 7.6 差の一覧 | — | `doc/ssp-mcp/transport-diff-areka.md` | 無い（追記） |
| 7.7 実機 | — | — | 無い |

## 5. 実装の案

全体は「自分のツールのファイルとその子モジュールに新しく書く」が土台で、既存を広げるのは最小限。分かれ目は 4 つあり、それぞれ独立に選べる。

### 5.1 指定した surface 単体をどう作るか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| **A. 表示の層に読むだけの口を 1 本足す** | `areka-emo-present` の `presenter/` に新しいファイル 1 つ（＋親への 1 行）。その target の資産で合成して返す。surface が無ければ `error!` なしでそう答える | 資産の二重持ちなし。要件 2.1・2.2・4.2・4.6 を 1 本で満たす。画面もメモも変えない | **同じウェーブの約束の外**（本 spec の触るファイルに `areka-emo-present` は無い）。同じウェーブで `surface-element-nesting` が `areka-emo-present/src/cache.rs`（要れば `shell_target.rs`）を触るが、新しいファイルなら重ならない見込み |
| B. アプリの側に資産の写しを持つ | 起動と切替のたびに `EmoWorld` をもう 1 つ組み、アトラスも別に持つ | 表示の層を触らない | メモリが倍。起動・シェル切替・ゴースト切替の 3 か所（`emo2_boot` の結線の列＝他の spec と重なる）を触る。今の着せ替えは結局届かない |
| C. 一時的に画面へ出して読み、戻す | 既存の `ShowSurface` → `read_back` → 元へ戻す | 新しい口が要らない | 要件 2.3（画面を変えない）に反する。採れない |

A のときの小さな選択: 口が返すのを「合成済みの絵」にするか、「資産を借りる口」にしてアプリの側で `Composer` を呼ぶか。前者のほうが表示の層の中身が外へ漏れない。

### 5.2 バルーンの背景と文字をどう重ねるか（拡大率が 1 でないとき）

拡大率が 1 のときは、どの案でも「背景の上に、文字の面を領域の原点へ重ねる」で画素が正確に合う。

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| **⒜ 文字を縮めて重ねる** | 文字の面を 1/拡大率 に縮めて（CPU の面積平均など）背景へ重ねる | 触るのは自分のファイルだけ。スクロール位置も強調もそのまま写る | 拡大率が 1 でないとき字がぼやける（200% はほぼ劣化なし、125%・150% は少しにじむ）。縮める処理を書く。端の 1 px の丸めを決める |
| ⒝ 原寸で描き直す | 文字の層に「拡大率 1 で画面の外へ描く」口を足す | 字が鮮明 | `areka-emo-text` を触る（同じウェーブの `shell-balloon-frame-align` が `actor.rs`・`actor_present.rs`・`actor_box.rs` を触る）。描く流れ（配置→装飾→描画）とスクロールの状態が `present_actor` と描画実行部の中に閉じていて、同じ絵を再現する口が大きい。規模が 1 段上がる |
| ⒞ 拡大後の大きさで返す | — | — | 暫定の裁定 1 で取らないと決めている |

### 5.3 PNG の符号化

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| **A. WIC の符号化器** | メモリ上のストリームへ PNG を書く。乗算を戻すのは自前か WIC の変換 | 圧縮される（応答が小さい）。新しい依存なし。OS の実装 | COM の呼び出しが数十行。呼ぶスレッドの COM の初期化の状態と、宣言済みの機能だけでメモリのストリームが作れるかを設計で確かめる |
| B. 標準ライブラリだけの無圧縮の PNG | テストの前例を本番へ | COM なし・どのスレッドでも動く・決定論テストが楽 | 圧縮されない。キャラクター 1 枚で数 MB の base64 になりうる（幅×高さ×4×約 1.37）。受け取る側が扱えるかが不明 |
| C. 自前の圧縮つき | 固定ハフマンなどを自作 | 依存なしで小さくなる | 自作の圧縮を持つことになる。WIC があるのに割に合わない |
| D. `png` クレートを足す | — | — | 範囲の外（`Cargo.toml` を変えない・このウェーブで依存を足す席は別の spec が使う） |

テスト（要件 7.1）で「PNG にして読み戻す」には復号が要る。WIC の復号（別の実装で読む）が使えると検査が強い。B だけだと自分で書いたものを自分で読む検査になる。

### 5.4 符号化をどこで走らせるか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| **A. UI スレッドでその場で** | 読み戻し → 符号化 → `reply.send` | 単純。`later` も要らない | 符号化の時間だけ 1 フレームが延びる（大きさと方式による。測って決める） |
| B. 別のスレッドへ渡す | 読み戻しだけ UI スレッドで行い、バイト列と `ReplyTo` を渡して符号化と返事を任せる | UI を止めない | スレッドの後始末・終了の途中の扱いが増える。WIC ならそのスレッドの COM の初期化が要る |

### 5.5 まとめた組み合わせ

- **小さく収める組み合わせ**: 5.1-A ＋ 5.2-⒜ ＋ 5.3-A ＋ 5.4-A。触るのは自分の 2 ファイルと子モジュール・`wiring.rs`・`areka-emo-present` の新しいファイル 1 つ。
- **表示の層を一切触らない組み合わせ**: 5.1-B。メモリと触る場所が増え、着せ替えの要件が満たせない。
- **字の鮮明さを取る組み合わせ**: 5.2-⒝。`areka-emo-text` を触るので、同じウェーブの並びを組み直す要が出る。

## 6. 規模とリスク

- **規模: M**（brief の見立てのまま・10〜14 タスク）。新しい型は少なく、既存の読み口をつなぐ仕事が中心。5.2-⒝ を採ると M〜L。
- **リスク: 中。**
  - 約束の外のファイル（`areka-emo-present`）を触る要が出た。
  - 拡大率が 1 でないときのバルーンの絵の質と、端の丸め。
  - WIC の符号化を UI スレッドで呼ぶときの COM の前提（未確認）。
  - 大きな画像の応答を受け取る側が扱えるか（未確認）。

### 設計へ持っていく調べもの

1. UI スレッドの COM の初期化の状態で WIC の工場が作れるか。宣言済みの機能だけでメモリのストリーム（例: `SHCreateMemStream`、または WIC のストリーム）が使えるか。
2. 隠している間に `\![bind]` が来たとき、seriko は表示の指令を出すか（表示の層の `last_show` の着せ替えが古くならないか）。
3. 符号化の所要時間（キャラクター 1 枚・バルーン 1 枚）と、base64 にした応答の大きさ。受け取る側（Claude Code など）の画像の応答の上限。
4. rmcp の応答の経路に大きさの上限が無いか（areka のコードには無い）。
5. 窓はあるが絵の無いスコープ（2 以降）と、装着の相が走る前の起動直後に、どの文言を返すか。
6. 背景の原寸と文字の面の位置を重ねるときの、領域の原点の取り方（差し込み口の `Arrangement` を拡大率で割るか、覚えているバルーンの定義から解き直すか）。
7. 描画を通るテストをどの土台に載せるか（`emo2_boot/spine*.rs` に足すか、ツールの子モジュールに置くか）と、拡大率が 1 でない状態をテストでどう作るか。

## 7. 要件ディスカッションへ出す論点

答えで作業が変わるものだけ。

1. **`areka-emo-present` に読むだけの口を 1 本足してよいか。** 指定した surface 単体（要件 2.1・2.2）と、無い surface ID の判定（要件 4.2・4.6）は、これが無いと作れない。同じウェーブの約束では本 spec の触るファイルに入っていない。新しいファイル 1 つ＋親モジュールへの 1 行で、同じウェーブで `areka-emo-present` を触る `surface-element-nesting`（`cache.rs`）とは重ならない見込み。
2. **まだ一度も表示していないスコープで `surface` を指定されたとき、着せ替えは何を掛けるか。** 表示の層には記録が無い。⒜ 着せ替えなしで返す ⒝ 起動時にオンの着せ替えの写しを結線状態に持たせて掛ける（装着の相を触る）⒞ `NG:` で断る（要件 2.3 の「何も表示していなくても返す」を改める）。
3. **拡大率が 1 でないときのバルーンの文字は、縮めて重ねる（少しにじむ・自分のファイルだけ）でよいか。** 原寸で描き直すなら `areka-emo-text` を触り、規模が上がり、同じウェーブの `shell-balloon-frame-align` と重なる。
4. **「バルーンに描いた内容が残っている」の判定を、文字の状態が空でないこと、にしてよいか。** こうすると、次の台詞の頭の全消しで、話していないスコープのバルーンも「残っていない」になる。背景だけ（文字なし）のバルーンは返さない。
5. **PNG は WIC（圧縮あり）でよいか。** 無圧縮の自前の書き出しは単純だが、応答が数 MB になりうる。
6. **`dump_surface`（省略）に箱の文字を写さない、でよいか。** 今の作りでは写らない。写すなら論点 3 と同じ重ねる処理が箱ごとに要る。写さないなら差の一覧に「画面では箱に文字が見えていても、返る絵には無い」と書く。
7. **窓はあるが絵の無いスコープ（今はスコープ 2 以降）の答え。** 要件 4.1 の字面では「キャラクターの窓がある」ので存在するスコープになり、省略なら「何も表示していない」、`surface` 指定なら答える文言が決まっていない。窓ではなく「絵の資産があるスコープ」を存在の基準にする手もある。

## 8. SSP の実測の追補（2026-10-04・SSP 2.9.07・ゴースト「えも2DEBUG」・手元の SSP の MCP で測定）

要件の「暫定の裁定」で「SSP は未実測」としていた点を測った。**要件と食い違うものに ★ を付ける。**

### 8.1 測った結果

| 呼び方 | SSP の答え | 要件との関係 |
|---|---|---|
| `dump_surface`（省略・スコープ 0／1） | `OK:scope 0, surface 1000 as currently shown (with running animations and dressups, before scaling and transparency)`＋画像 | 要件 1.4 と一致 |
| `dump_surface`（`surface` 指定・在る ID） | `OK:scope 0, surface 2105 rendered alone in its initial state (not what is on the screen now)`＋画像 | ★ 要件 2.4 の文言（`…(before scaling and transparency)`）と末尾が違う |
| `dump_surface`（`\s[-1]` で隠した後・省略。プロパティ `currentghost.scope(0).surface.num` は `-1`） | `OK:scope 0, surface 1000 as currently shown (…)`＋**隠す直前の絵**。失敗にしない | ★ 要件 4.3・裁定 4（`NG:No surface is currently shown in this scope`）と逆 |
| `dump_surface`（スコープ 2＝このゴーストに無い） | `NG:No such scope in this ghost` | 要件 4.1 と一致 |
| `dump_balloon`（スコープ 2） | `NG:No such scope in this ghost` | 裁定 3 の推定どおり（実測で確定） |
| `scope: -1`（2 本とも） | JSON-RPC のエラー `Invalid params`（`NG:` の結果ではない） | ★ 要件 4.1「負の数を含む」と違う。areka の入口（`mcp-tool-entrances`）が負の数をどう通すかと合わせて決める |
| `scope: 0.5` | スコープ 0 として成功（小数は切り捨て） | 入口の検査の範囲 |
| `surface: 9999`・`surface: -1`・`surface: 5.7` | `NG:No such surface ID. Check get_expression_table tool` | 要件 4.2 と一致 |
| `surface: 5`（`get_expression_table` には `\s[5]` と出る＝別名） | `NG:No such surface ID. …` | ★ **`surface` は別名を解かない生の ID**。別名の先の `1000`・`2105`・`10` は成功 |
| `scope: 0, surface: 2105`（スコープ 1 用の絵）／`scope: 1, surface: 0` | どちらも成功 | ★ **surface の在る・無いはシェル全体で見る**（要件 2.1・4.2 の「そのスコープのシェルに在る」ではない）。スコープは本文と着せ替えにだけ効く |
| 表示中と同じ ID を `surface` で指定 | 成功・本文は `rendered alone…`（動いているアニメーションの無い絵） | 要件 2.1 と一致 |
| `dump_balloon`（話している途中） | 成功・途中までの文字 | 要件 3.5 と一致 |
| `dump_balloon`（台詞の頭で消された後の、話していないスコープ） | 成功・**背景だけの絵** | ★ 要件 4.4・裁定 5（`NG:No balloon has been drawn in this scope yet`）と逆 |
| `dump_balloon`（文字の無い台本 `\0\s[-1]\e` を送った後） | 成功・背景＋送り主の印（`from MCP (local)`）だけ | ★ 同上。文字が 0 でも失敗にしない |
| 存在しない `ghost_name` | `NG:Cannot find active ghost from specified name` | `mcp-tool-entrances` の範囲 |

### 8.2 画像の形（返った 16 枚の PNG のヘッダを読んだ）

- 8 ビット・カラータイプ 6（RGBA）・インターレース無し・チャンクは `IHDR`・`IDAT`・`IEND` だけ（付帯情報なし）。**圧縮あり**（434×687 のキャラクターが 80〜150 KB、400×224 のバルーンが 10〜14 KB）。
- キャラクターはどの surface でも同じ大きさ（434×687＝このシェルの絵の大きさ）。バルーンはスコープごとの背景の大きさ（400×224・288×203）。
- バルーンの画像には、背景・文字のほかに**上へ送る矢印の印**と **SSTP の送り主の印**が写る（バルーンの窓に描かれたものすべて）。

### 8.3 ここから出る「要る機能」の一覧

1. 今の見た目: 最後に表示した絵と surface ID を返す。**隠していても返す**（一度も表示していないときだけが未決）。
2. 指定の surface: 生の ID・シェル全体から探す・別名は解かない・動く前の姿・今の着せ替え・画面を変えない。
3. バルーン: 背景＋窓に描かれたもの全部（文字・矢印・送り主の印）。**文字が無くても背景だけで返す**。話している途中・隠れた後も返す。
4. 失敗は 3 種だけ: スコープが無い（`NG:No such scope in this ghost`）・surface ID が無い（`NG:No such surface ID. Check get_expression_table tool`）・ゴーストが見つからない（入口の文言）。SSP に「表示していない」「描いていない」の失敗は無い。
5. 画像: 圧縮した 8 ビット RGBA の PNG・原寸・1 枚。
6. areka だけの失敗として残るもの: 窓の無いゴースト（裁定 6）・まだ一度も表示していないスコープ・装着の前。

### 8.4 測れなかったもの

- 画面の拡大率が 1 でないときの大きさ（SSP の拡大の設定を変えていない）。
- 起動してから一度も台詞を出していないスコープのバルーン（測った時点でどちらも描いた後）。背景だけが返ると見るのが 8.1 の 2 行と整合する。
- 着せ替えの切り替えが指定の surface に掛かるか（このゴーストに着せ替えが無い）。

## 9. 要件ディスカッションで設計へ送ったもの（2026-10-04）

§7 の論点のうち、作り方の選択なので設計で決めるもの。

- PNG の符号化の方式（§5.3。SSP は圧縮した 8 ビット RGBA・付帯情報なし＝§8.2。WIC が第一候補）と、符号化を走らせる場所（§5.4）。
- `dump_surface`（省略）に箱の文字を写すか（§7 の論点 6。要件の裁定 10 のとおり合否を変えず、差の一覧に書く）。
- バルーンの窓に描かれるもの（文字・上へ送る矢印の印・送り主の印）のうち、areka が今描いているものをそのまま返す（§8.2。足りない印は本 spec で足さない）。
- 背景と文字の位置合わせの原点・テストの置き場（§6 の調べもの 6・7）。

## 10. 設計の段の調べ（2026-10-04・軽い発見＝既存の仕組みへの追加）

§6「設計へ持っていく調べもの」と §9 を、コードを読んで確かめた結果。**確かめられなかったものは「未確認」と書く。**

### 10.1 UI スレッドの COM と WIC の符号化（調べもの 1）

- UI スレッドは MTA で初期化済み（`WinApp` の構築＝`crates/wintf/src/runtime/mod.rs` が `CoInitializeEx(COINIT_MULTITHREADED)` を呼ぶ）。WIC の工場は UI スレッドで作れる（`wintf::com::wic::wic_factory`）。
- メモリのストリームは `SHCreateMemStream`（`Win32_UI_Shell`＋`Win32_System_Com`。どちらも根の `Cargo.toml` で宣言済み）で作れる。
- **しかし WIC の符号化そのものが、宣言済みの機能だけでは呼べない。** `windows` 0.62.2 の `IWICBitmapEncoder::CreateNewFrame` と `IWICBitmapFrameEncode::Initialize` は `#[cfg(feature = "Win32_System_Com_StructuredStorage")]` の下にある（引数の `IPropertyBag2` がその機能の型）。`cargo metadata` の解決結果では、ワークスペースで有効な `windows` の機能 64 個にこれが無い。有効にするには `Cargo.toml` の変更が要る＝要件の境界の外。
- brief と §2.8・§5.3 の「WIC の符号化器は宣言済みの機能で使える」は誤りだった（復号は使えるが、符号化はフレームを作る口が別の機能の下）。

### 10.2 隠している間の着せ替え（調べもの 2）

- seriko の `ScopeStates::apply_bind`（`crates/areka-seriko/src/state.rs`）は、スコープが隠れている（または未表示）とき `BindApplyOutcome::StateOnly` を返し、表示の指令を出さない。新しい集合は次の表示の指令に載る。
- だから、隠している間に `\![bind]` が来ると、表示の層の `last_show` の着せ替えは次に表示するまで古いまま。要件 2.2 は「最後に表示したときのもの」と定めているので要件どおり。差の一覧に書く。

### 10.3 応答の大きさ（調べもの 3・4）

- areka の応答の経路に上限は無い（`MAX_BODY_BYTES`＝`crates/areka-mcp/src/dispatch.rs` は要求の本文だけ）。
- rmcp 3.5.0 の Streamable HTTP のサーバ（`transport/streamable_http_server/`）で大きさに関わる設定は `max_request_body_bytes` だけ＝応答の上限は無い（ソースを検索して確認）。
- 受け取る側（Claude Code など）の画像の応答の上限は**未確認**。符号化の所要時間も**未測定**。どちらも実機確認で記録する。

### 10.4 窓はあるが絵の無いスコープ・装着の前（調べもの 5）

- 絵の無いスコープ（今は 2 以降）は、装着の相が飛ばすので表示の層に target が無い。「シェルの target が登録されているか」（`EmoPresenter::target_visible(shell_target(scope)).is_some()`）がそのまま「絵の資産が組まれているスコープ」の判定になる。
- 装着の前は、資産のあるスコープも target が無い。区別には結線状態の `attached` の欄が要る（今は `frame` の中だけに見える）。`wiring.rs` に読み口を 1 本足す。

### 10.5 文字の位置合わせの原点（調べもの 6）

- 文字の面は、差し込み口の entity に「物理寸・窓の原点からの物理 px の offset（領域の原点×拡大率）」の `Arrangement` で付く（`TextSurface::attach`・`physical_arrangement`＝`crates/areka-emo-text/src/surface.rs`）。
- 背景の原寸と物理寸は `TextSlotView` の `surface_size()`・`physical_size()` で取れる。
- `ScaleRatio` は分子と分母を公開していない。`as_f32` は寸法の計算に使わない約束（`presenter/read.rs` の注記）。だから縮める比は「物理寸÷原寸」の整数比を使う。物理寸は丸めた値なので、厳密な拡大率との差は端で最大 0.5 物理 px。

### 10.6 描画を通るテストの置き場と、拡大率が 1 でない状態の作り方（調べもの 7）

- `emo2_boot/spine.rs` の `SpineHarness` は欄も作り口も非公開で、`mcp` のテストからは使えない。
- `emo2_boot` の外に、GPU つきの統合テストの前例がある: `crates/areka/src/shell_balloon_switch_session_lap_tests.rs` が、公開（クレート内）の土台 `SwitchRig`（`ghost_switch_test_support.rs`）に `GraphicsCore::new()`＋`WucGraphicsResource` と窓の一式（`spawn_ghost_windows`＋偽の HWND＋`DPI` の component）を足し、本番の `Input`・`Update` の段を回している。同じ組み方をツールの子のテストの土台に書ける。
- 拡大率は窓の `DPI` の component で決まる（`DPI::from_dpi`）。96 以外を入れれば 1 でない状態になる。
- GPU を通るテストの接続条件は `#[cfg(all(test, target_pointer_width = "64"))]`（`emo2_boot/frame.rs` の接続と同じ）。

### 10.7 そのほか確かめたこと

- 各スコープのシェルの target は、シェル全体から組んだ `EmoWorld` を 1 つずつ持つ（`ScopeAssets`＝`emo2_boot/assets.rs`。読み込みは 1 回で、World だけスコープごとに組む）。「シェル全体から探す」はそのスコープの target の `EmoWorld::surface(id)` で足りる。別名は `AliasMap` で、`surface(id)` は見ない。
- 隠す処理 `apply_hide` は `current_surface_id` を `None` にする。隠した後の surface ID は非公開の `last_show` にしか残らない＝要件 1.6 のために、表示の層の新しいファイルに「最後に表示した ID と絵」を返す口が要る（要件ディスカッションの議題 2 で挙げた 2 本に加えて 3 本目。同じ 1 ファイルの中で、読むだけ）。
- 合成メモの全破棄（`PresentCommand::InvalidateCache`）を本番で送る所は 0 か所（テストの語彙だけ）。「最後に表示した絵がメモに無い」は今の本番では起きない。
- 合成の層は、surface が無いとき（`SurfaceNotFound`）と定義層が皆無のとき（`EmptyComposition`）に自分で `error!` を出す（`crates/areka-emo-compose/src/plan.rs`）。前者は事前に `EmoWorld::surface` で避けられる。後者は避ける口が無い。
- CRC-32 の本番の実装は `areka_nar::crc32`（`crates/areka-nar/src/crc32.rs`・公開）。`areka` は既に `areka-nar` に依存している。PNG の CRC と同じ多項式。
- 箱の文字がキャラクターの絵に写らないこと（§3 ⑶）は読み直して同じ結論。

## 11. 設計の決定と、捨てた案（2026-10-04）

### 11.1 まとめる・借りる・削る

- **まとめる**: 「今の見た目」「指定した surface」「バルーンの背景」は、どれも「原寸・乗算済み BGRA の絵 1 枚を PNG にする」。符号化は 1 関数（`png_base64`）、判断は 1 つの事実の口（`ShellFacts`）に寄せ、2 本のツールで共用する。
- **借りる**: 合成は `Composer::compose`、CRC は `areka_nar::crc32`、後から答える仕組みは `mcp::later`、テストの土台は `SwitchRig` と配置の準備。新しく書くのは、乗算を戻す・PNG の器・base64・縮めて重ねる、の 4 つだけ。
- **削る**: 「バルーンに文字が残っているか」の判定（消えた面は透明なので要らない）。`wiring.rs` の `read_back_target` を本番へ開けること（`presenter()` 経由で足りる）。符号化の別スレッド。資産の写しを結線状態に持つこと。

### 11.2 PNG の作り方

| 案 | 判定 | 理由 |
|---|---|---|
| WIC の符号化器 | 取れない | §10.1。`Cargo.toml` の変更が要る |
| **標準ライブラリだけの無圧縮の PNG** | **採用** | 要件 5.1〜5.5 を満たす。COM も `unsafe` も要らない。チャンクは SSP と同じ 3 つ。応答が SSP の約 10 倍（434×687 で base64 約 1.6 MB） |
| 自前の圧縮（固定ハフマン＋連長） | 取らない | 自作の圧縮器を持つことになる。圧縮が要ると分かったら、`crates/areka/Cargo.toml` の `windows` の機能に `Win32_System_Com_StructuredStorage` の 1 語を足して WIC を使うほうが小さい |
| `miniz_oxide`・`png` クレート | 取れない | `areka` の `Cargo.toml` の変更が要る |

- **設計ディスカッションへ出す点**: 無圧縮のままでよいか、`crates/areka/Cargo.toml` に機能 1 語を足して WIC の圧縮 PNG にするか。後者は要件の境界（`Cargo.toml` を変えない）を緩める判断。同じウェーブで `Cargo.toml` を触るのは `animated-image-decode`（`areka-emo-atlas` の側）で、`crates/areka/Cargo.toml` は重ならない見込み。どちらでも替わるのは `png_base64` の中身だけ。

### 11.3 装着の前の呼び出し

| 案 | 判定 | 理由 |
|---|---|---|
| そのまま判断する | 取らない | 在るはずのスコープ 0 に `NG:No such scope in this ghost` を返す |
| 新しい文言で断る | 取らない | 要件に無い文言が増える |
| **`mcp::later` に預けて装着の後に答える** | **採用** | 既存の口。新しい文言なし。済まなければ入口の 10 秒が答える |

### 11.4 表示の層の新しいファイルのテスト

- 合意は「新しいファイル 1 つ＋親への 1 行」。`areka-emo-present` にテストのファイルを足すと 2 つ目のファイルになるので、足さない。3 本の口は、本番の唯一の呼び手であるツールの側の、実際の描画を通るテストで固定する。
- **設計ディスカッションへ出す点**: `areka-emo-present` の側に兄弟のテストファイル（`presenter/snapshot_tests.rs`）を足してよいなら、GPU 無しで `has_surface`・`compose_alone` を固定できる（他の spec の触るファイルとは重ならない）。

### 11.5 文字の縮め方

- 面積で重み付けした平均（箱フィルタ）1 本。拡大率 1 では写しと同じ値、1 より小さいときも同じ式で通る。別の分岐を持たない。
- 厳密な拡大率（`ScaleRatio`）を使わず「物理寸÷原寸」を使う。差は端で最大 0.5 物理 px で、縮めた後の絵では 1 画素に満たない。

### 11.6 残るリスク

- 受け取る側が 1〜数 MB の画像を扱えるか（未確認・実機確認で確かめる。だめなら §11.2 の設計ディスカッションの点へ戻る）。
- 壊れたシェル（定義層の無い surface）を指定すると ERROR の記録が 2 件になる（合成の層の 1 件＋ツールの 1 件）。
- 拡大率が変わったフレームは文字の面が作り直しの途中で、その 1 フレームだけ背景だけの絵が返りうる。

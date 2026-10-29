# ギャップ分析: shell-mouse-cursor

- 実施日: 2026-10-29（ブランチ `claude/kiro-start-shell-mouse-cursor-82e1d7`・基点 `fdb14576`）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）
- 方法: コードを Grep／Read で読んだ。外の実物（SSP・areka）は動かしていない。正典は ukadoc MCP で `descript_ghost` の `mousecursor.*` とプロパティ `currentghost.mousecursor` を引き直した。
- この文書は**判断の材料**であって、決めではない。決めは要件討議・設計の段で行う。

## 1. 要約

- **在るもの**: cursorブレスを読まずに捨てる読み手（`decode.rs`）と、同じ文面を読む 2 つ目の転記の前例 2 つ（`shell/boxes.rs`・`shell/undrawn.rs`）。窓ごとの「今どの当たり判定の上か」（`input_events/hover_region.rs` の `HoverState`）と、押下の控え（`MouseWiring.presses`）・ホイールの溜め（`MouseWiring.wheel`）。wintf の `WM_SETCURSOR` の口（今は診断の十字だけ）。OS の「マウスを止めたと見なす時間」を読む関数（`wintf::ecs::tooltip::os::read_hover_time`）。
- **無いもの**: cursorブレスの転記・検証・記録、ゴーストの descript の `mousecursor.*` の読み取り、カーソルの絵（`HCURSOR`）を作る・持つ・捨てる所、`WM_SETCURSOR` に形を答える口、止まった・ホイール・押下のときに形を出し直す口、`system:hand`／`system:grip` の絵、実機でカーソルの形を読む道具。
- **一番の壁は時の合わせ方**: `HoverState` は毎 tick の系（`char_watch`）が更新するので、`WM_SETCURSOR`（移動の直後に同期で届く）の時点では 1 tick 古い。`HoverState` だけで形を選ぶと要件 2.3（描画の 1 回分も残さない）を破る。`WM_SETCURSOR` の中でその場で当たり判定を引く（プル）形にし、カーソルの来ない場面（止まった・ホイール・捕捉中の解放・面替わり）だけ tick や窓の手続きから出し直す（プッシュ）組み合わせが要る。
- **推しの方向（案 C）**: 転記は `shell/cursor.rs`（新）、検証と記録は emo-present の新しいファイル、形の選び方は areka の純関数（`hover_region.rs` の子・`mod.rs` に触らない）、Win32 は wintf の新しいカーソルの口のファイルに閉じ込める。
- **調べが要る点**: 捕捉中・解放時・合成の移動での `WM_SETCURSOR` の届き方、`.ani` を含む `HCURSOR` を作るスレッドと捨てる順、DPI とカーソルの大きさ、同梱する手の形の絵の持ち方。

## 2. 今のコード（関係する所）

### 2.1 surfaces.txt の読み手（areka-parsers）

- `crates/areka-parsers/src/shell/decode.rs`（598 行）: 見出しで振り分けるのは descript・別名・追記・サーフェスの 4 つだけ。cursorブレスは中身ごと黙って消える。この crate には記録の呼び出しが無い（網羅台帳の注記どおり）。
- 2 つ目の転記の前例:
  - `shell/boxes.rs`（137 行）: `lexer::lex` で同じ文面を読み、閉じたブレスだけを見て、欄を文字列のまま持つ。検証も記録もしない（失敗しない）。厳しく読むのは下流（`areka-emo-compose` の `fold_boxes`）、記録は `load_shell_target`。
  - `shell/undrawn.rs`（62 行）: 同じ型。記録は入口が出す。
- `shell/mod.rs`（72 行）: 新しい転記を足すときは `mod cursor;`・`#[cfg(test)] mod cursor_tests;`・`pub use` の 3 か所が要る（`seriko-pattern-calls` が同じ所に行を足すときだけ重なる）。

### 2.2 シェルの読み込みと運び（areka-emo-present・areka）

- `crates/areka-emo-present/src/shell_target.rs`（855 行）: fs を触る唯一の入口 `load_shell_target`。`surfaces.txt` を 1 回読み、`parse`・`parse_boxes`・`parse_undrawn_elements` に同じ文面を渡し、記録は読み込み 1 回につき 1 度だけここで出す。`ShellTarget` は箱の表 `boxes()` を持つ。1,000 行の目安まで残り 145 行。
- `crates/areka/src/emo2_boot/assets.rs` の `build_shell_assets`: `load_shell_target` を呼び、箱は `ShellBoxAssets` として `ShellAssets` に載せて運ぶ。シェルのフォルダ（`model.shell.dir`）とゴーストの `ghost/master`（`model.shiori.dir`）の両方がここで手に入る。
- シェルの切り替え: `emo2_boot/switch_assets.rs` の `build_shell` が**背景のスレッド**で `build_shell_assets` を呼び（「UI スレッドで読まないためここで読む」）、`shell_balloon_switch.rs` の `SwapFinish::Shell { …, boxes }` で UI スレッドへ渡して入れ替える。カーソルの表もこの道に乗せるなら、`SwapFinish::Shell` に 1 欄足す形になる。

### 2.3 ゴーストの descript

- `MountModel`（`areka-parsers/src/package/model.rs`）は `names`・`shiori`・`shell`・`bindgroups`・`readme` だけを持ち、`cursor`／`mousecursor.*` は持たない。
- `placement/source.rs` がゴーストの descript を `kv::parse_kv`（`BTreeMap<String, String>`）で読み、`DescriptSource.ghost_kv` として持つ。切り替えの `build_shell` も `source.ghost_kv` を持っている。読み口の候補はこれ。

### 2.4 入力の層（areka）

- `input_events/hover_region.rs`（102 行）: `HoverState { scope, client, surface, region }` と純関数 `plan_hover`。型と関数は `pub(super)`。読む口は無い（あちらの design「読み手が居ないので作らない」）。
- `input_events/char_watch.rs`（234 行）: `Input` の段で `dispatch_pointer_events` の前に走る排他の系。`PointerState` の持ち主からキャラ窓を引き、**動いた・初めて見た・ホイールが回ったときだけ**当たり判定を解く。絵の透けた所（持ち主が窓そのもの）は当たり判定「無し」。完了 `mouse-click-wheel-events` の要件 7.9 で「止まったまま表情が替わっても、次に動くまで出入りを送らない」と決めてある。
- `input_events/mod.rs`（611 行）: `MouseWiring`（`presses`・`hover`・`wheel` は非公開の欄）と `resolve_hit_owned(&World, scope, x, y)`（非公開）。Rust の可視性で、`input_events` の子孫のモジュール（例: `hover_region.rs` の `#[path]` の子）からはこれらを読める＝**`mod.rs` に行を足さずに読む口を作れる**。
- 押下の控え `click::note_press` は `OnPointerPressed`（tick の中）で書く。`WM_SETCURSOR` の時点では書かれていない。
- 当たり判定の解決 `emo2_boot/hit_region.rs` の `resolve_hit_region` は `&EmoPresenter` だけで答える純粋な読み（窓 client 物理 px を受け、÷k は presenter が吸収）。`&World` から呼べる。

### 2.5 窓の手続き（wintf）

- `crates/wintf/src/ecs/window_proc/mouse_move.rs`（632 行）の `WM_SETCURSOR`: 環境変数 `WINTF_CROSSHAIR` のときだけ `HTCLIENT` 上で `IDC_CROSS` を出し `LRESULT(1)`。ほかは `None`（`DefWindowProcW` へ）。
- 窓のクラスは外の crate `wintf-winmsg-executor 0.0.5` が登録し、`hCursor = LoadCursorW(None, IDC_ARROW)`。`DefWindowProcW` はクラスのカーソル（矢印）を出す＝今の見え方。
- `WM_NCHITTEST`（`pointer/nchittest_cache.rs`）: α の当たり（`hit_test_in_window`）で `HTCLIENT`／`HTTRANSPARENT` を返す。左を押している間は `HTCLIENT` を強制。結果は `thread_local` の窓ごとの控えに置き、tick の終わりに消す＝**世界の借用の外に窓ごとの状態を置く前例**。
- クリック透過は `WS_EX_TRANSPARENT` の切り替え（`ecs/clickthrough/`・カーソル監視のワーカ → UI スレッドで適用）。透過中の窓には `WM_SETCURSOR` が来ない＝要件 2.5 は今の仕組みのままで満たせる。
- ドラッグは `ecs/drag/capture_guard.rs` で `SetCapture`／`ReleaseCapture`。捕捉中は `WM_SETCURSOR` が来ない。
- `WM_MOUSEWHEEL` は量を溜めるだけ（`add_wheel_vertical`）。`TrackMouseEvent` は `TME_LEAVE` だけで、`TME_HOVER`（`WM_MOUSEHOVER`）は使っていない。
- `tick_wake`: `WM_SETCURSOR` は `POINTER` の旗。期限の預け `arm_deadline` がある（ツールチップと `char_watch` のホイールが使う）。
- ツールチップ `ecs/tooltip/`: 「止まったら出す」の状態機械と `read_hover_time`（`SPI_GETMOUSEHOVERTIME`・読めないときの代わりの値つき）。`unsafe` は `os.rs` だけ、という配置の前例。

### 2.6 台帳・文書・道具

- `doc/ukadoc-coverage/ledger/assets.toml`（8,338 行）: 当たる 21 行。cursorブレスの 5 場面（7150・7167・7184・7201・7219 行）、`system:*` の 10 行（7651〜7796 行）、ゴーストの `cursor／mousecursor`（2775）・`mousecursor.arrow／grip／hand／text／wait`（3109〜3185）。今 `owner = "shell-mouse-cursor"` は `mouseup*` と `system:hand` の 2 行だけで、残りは持ち主なし・全部 `absent`。cursorブレスの見出しそのものの行は台帳に無い。
- `doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）は 124 行目から。行を足す場所は在る。
- `doc/pasta-real-run.md` の段 3: 「当たり判定ごとにマウスの形が替わる」が見る所に書いてある。ただし**カーソルの形を読む道具が無い**（`tools/mouse-input.ps1` は `move`・`click`・`drag`・`shot` だけ・`GetCursorInfo` はリポジトリのどこにも無い）。SSP の測り（`HAND`・`custom`・`ARROW`）の読み方を再現する手段が要る。

## 3. 要件と在るものの対応

| 要件 | 使える物 | ギャップ |
| --- | --- | --- |
| 1.1〜1.4 cursorブレスの読み取り | `lexer::lex`・`boxes.rs` の型・`decode::parse_targets` は不要（見出しは `sakura`／`kero`／`char*` ＋ `.cursor`） | **Missing**: 転記 `shell/cursor.rs`・検証（場面名・番号・`system:` 名・拡張子）・重なりの後勝ち・記録 |
| 1.5 ほかの読み取りを変えない | 2 つ目の転記の型（`decode.rs` に触らない） | — |
| 1.6 当たり判定名の綴りの突き合わせ | `HitRegion.region`（不透明文字列） | 突き合わせは文字列の一致だけで足りる。`bust` の大小（議題 8）は **Constraint** |
| 2.1〜2.4 乗る・移る・外へ出る | `resolve_hit_owned`・`HoverState` | **Missing**: `WM_SETCURSOR` で形を答える口。**Constraint**: `HoverState` は 1 tick 古い（2.3 を破る） |
| 2.5 透けた所 | `WS_EX_TRANSPARENT` と `HTTRANSPARENT` | 今の仕組みで満たせる。透過を外した直後は次の移動まで形が来ない（**Unknown**・5.6） |
| 2.6 止まったまま面が替わる | `mouse-click-wheel-events` 要件 7.9（次に動くまで出入りを送らない） | 読み方次第で何も足さずに済む／tick で出し直す口が要る（**Unknown**・議題 6） |
| 2.7 窓ごと・スコープごと | `MouseWiring` の鍵が窓の entity | — |
| 2.8 バルーンは今のまま | 口を部品で付けるならキャラ窓だけに付く | — |
| 3.1〜3.2 押したとき | `WM_SETCURSOR` の lParam 上位語（どのマウスのメッセージか） | **Missing**: 押した瞬間の形の選び直し。`presses` は tick で書くので使えない |
| 3.3〜3.5 押している間・放したとき | `DragState`（`read_drag_state` は同期で読める）・`GetKeyState` | **Missing**: 押したときの形の窓ごとの控え。**Constraint**: 捕捉中は `WM_SETCURSOR` が来ない＝放したときは自分で `SetCursor` が要る |
| 3.6 今の動きを変えない | — | 形を出すだけで `LRESULT` 以外の流れは触らない |
| 4.1〜4.2 止まったとき | `read_hover_time`・`tick_wake::arm_deadline`・`TrackMouseEvent` | **Missing**: 静止の判定と期限での出し直し。`mouse-hover-event` と時間を共有する見込み |
| 4.3〜4.5 ホイール・戻り | `WM_MOUSEWHEEL` の口・`MouseWiring.wheel` | **Missing**: ホイールの形への切り替えと、動いたら戻す判定（合成の移動を「動いた」と見ない工夫が要る見込み・**Unknown**） |
| 5.1〜5.8 ゴーストの descript | `placement/source.rs` の `ghost_kv`・`parse_kv` | **Missing**: 6 つの鍵の読み取り・検証・記録（`mousecursor` と `cursor` の両方・`text`／`wait` を読んで使わない） |
| 6.1 `system:*` の OS の形 | `LoadCursorW(None, IDC_*)` | 対応表が要る（`finger`→`IDC_HAND`・`move`→`IDC_SIZEALL`・`text`→`IDC_IBEAM` など） |
| 6.2 `system:hand`／`grip` | 無い（Windows の標準に「手を開いた形」「握った形」は無い。`IDC_HAND` は指差し） | **Missing**: 同梱の絵とその持ち方（議題 6・12） |
| 6.3 `.cur`・`.ani` | `LoadImageW(LR_LOADFROMFILE)`／`LoadCursorFromFileW` | **Missing**: 作る・持つ・捨てる所。**Unknown**: 背景のスレッドで作ってよいか・捨てる順 |
| 6.4〜6.6 探し方・フォルダの外 | `model.shell.dir`・`model.shiori.dir`（`ghost/master`） | **Missing**: 探索と、絶対パス・`..` の拒否 |
| 6.7・6.9 記録 1 回・読み込み 1 回 | `load_shell_target` の「1 回だけ記録」の型 | 同じ型に乗る |
| 6.8 替えた後は新しい定義だけ | `SwapFinish::Shell` | **Missing**: 表の入れ替えと、古い絵を捨てる前に形を出し直す順 |
| 7.1 詳しい記録 | `tracing::debug!`／`trace!` | — |
| 7.2 決定論のテスト | `hover_region_tests.rs`・`boxes_tests.rs`・`MouseWiring::with_clock`・`RegionSource::Mock` | 純関数に寄せれば檻に入る。Win32 の口は薄く |
| 7.3 実機の確かめ | `tools/mouse-input.ps1`・`doc/pasta-real-run.md` 段 3 | **Missing**: カーソルの形を読む手段（`GetCursorInfo` の動詞） |
| 7.4 網羅台帳 21 行 | `assets.toml` | 19 行の `owner` を足し、21 行の `status`・`note` を書き直す |
| 7.5 §8 の登記 | `doc/COMPAT_ARCHITECTURE.md` §8 | 行を足すだけ |

## 4. 実装の方向の案

### 案 A: 今の部品を広げる（`HoverState` を読んで tick で出す）

- `char_watch` が当たり判定の変わった tick で形を選び、wintf の窓に「出すカーソル」を持たせ、`WM_SETCURSOR` はそれを返すだけ。
- 触る所: `char_watch.rs`・`hover_region.rs`・wintf の `mouse_move.rs`（数行）・`shell_target.rs`・`shell/mod.rs`。
- ✅ 新しい仕組みが一番少ない。wintf は「窓ごとの HCURSOR を返す」だけで当たり判定を知らない。
- ❌ **要件 2.3 を破る**: `WM_SETCURSOR` は移動の直後・`WM_MOUSEMOVE` の配送より前に届くので、境目をまたいだ瞬間は前の形が 1 tick 残る。記憶「1 フレーム遅らせる解は取らない」にも当たる。押下（3.1）・放したとき（3.4）も同じく遅れる。
- 結論: 単独では採れない。

### 案 B: wintf にカーソルの仕組みを丸ごと作る

- wintf に「範囲（矩形・形）→ カーソル」の登録表を作り（ツールチップの `TooltipRange` と同じ型）、`WM_SETCURSOR` で wintf が自分で範囲を引く。
- ✅ `WM_SETCURSOR` の中で同期に答えられる（0 フレーム）。wintf の汎用機能になる。
- ❌ 当たり判定の形（矩形・楕円・多角形・element の子の持ち込み・DPI の ÷k）は areka-emo-present が持つ。wintf に写すと解決が 2 か所になり、`OnMouseMove` の当たり判定と食い違う恐れ（要件 Adjacent「同じ名前を返すこと」）。面が替わるたびに登録し直す手間も出る。

### 案 C: 組み合わせ（その場で引くプル＋静止系だけプッシュ）— 推し

- **wintf（新しいカーソルの口のファイル）**: 
  - カーソルの値の型（`system` の種類か、ファイルから作った物を包む型。捨てるのは `Drop`）と作る関数（`unsafe` はこのファイルだけ・ツールチップの `os.rs` の前例）。
  - 窓に付ける問い合わせの部品（例: `fn(&World, Entity, 問い合わせ) -> Option<カーソル>`）。`WM_SETCURSOR` で `world.try_borrow()` が通れば呼ぶ。問い合わせには client 物理 px（`GetCursorPos`→`ScreenToClient`）と、引き金のメッセージ（lParam の上位語＝移動・左押下・右押下など）を載せる。
  - 借用が取れないときのために、窓ごとの「最後に出した形」を `thread_local` に置く（`nchittest_cache` と同じ型）。
  - 静止系のための「今この窓の形を出し直す」関数（カーソルがその窓の上に居る〔`WindowFromPoint`〕か、捕捉の持ち主のときだけ `SetCursor`）。
  - `mouse_move.rs` の `WM_SETCURSOR` は、十字の診断の後に新しい口へ渡す数行だけ。
- **areka（`hover_region.rs` の `#[path]` の子・新しいファイル）**: 形の選び方の純関数（スコープ・当たり判定名・場面・押下の控え → 形と出どころ）。`resolve_hit_owned` と `MouseWiring` の欄を子孫の可視性で読む＝`mod.rs` は無変更。問い合わせの部品の装着は `register_char_watch`（`char_watch.rs`）か `char_watch` の系の中で、部品の無いキャラ窓に付ける。
- **areka-parsers（`shell/cursor.rs`・新）**: `boxes.rs` と同じ転記。
- **areka-emo-present（新しいファイル）**: 検証・後勝ち・ファイルの探索・カーソルの作成・記録。`ShellTarget` に 1 欄と読み口を足す（`shell_target.rs` は数行）。
- ✅ 境目をまたいだ瞬間・押した瞬間に 0 フレームで正しい形。当たり判定の解決は今の 1 か所（presenter）のまま。Win32 は wintf に閉じる。`mod.rs` に触らない（`balloon-scroll-arrows` との約束）。
- ❌ 同期の問い合わせは wintf では新しい型（`WM_NCHITTEST` は wintf の部品だけで答えている）。`WM_SETCURSOR` は移動のたびに来るので、当たり判定の解決がその回数ぶん走る（同じ位置の答えを控える `nchittest_cache` の型で抑えられる見込み）。`HoverState`（tick）と `WM_SETCURSOR`（その場）の 2 か所で当たり判定を引くので、同じ点で同じ答えになることをテストで固定する要がある。

### 規模と危うさ

- **規模: L（1〜2 週）**。4 crate（parsers・emo-present・wintf・areka）と同梱の絵・台帳・実機の道具にまたがる。brief の見立て 6〜9 タスクの上の方。
- **危うさ: 中**。型は前例（`boxes.rs`・`nchittest_cache`・ツールチップ）で揃うが、Win32 のカーソルの振る舞い（捕捉・合成の移動・`.ani`・スレッド・DPI）に未確認の点が残る（6 節）。

## 5. 要件討議・設計に回す判断の項目

要件討議の議題（`requirements.md` 末尾の 1〜9）と重なるものはその番号を添えた。

1. **形を選ぶ時点（2.3・3.1・3.4）**: `WM_SETCURSOR` の中でその場で当たり判定を引く（案 C）か、tick で選んで `WM_SETCURSOR` は控えを返すだけ（案 A・要件 2.3 と衝突）か。brief の「`WM_SETCURSOR` の中で当たり判定をその場で引くか、名前が変わった tick で `SetCursor` を直に呼ぶか」の答え。後者は tick の中で `SetCursor` しても、それまでの間に届いた `WM_SETCURSOR` に古い形を答えるので 0 フレームにならない。
2. **wintf の口の形**: 窓ごとの部品（バルーンの窓に付かないので 2.8 が構造で守られる）か、スレッドに 1 つの登録（窓の種類を中で見分ける）か。借用が取れないときの答え（窓ごとの最後の形）を持つか。
3. **押下の扱い（3.1〜3.5・議題 5）**: 押した瞬間は `WM_SETCURSOR` の引き金のメッセージで分かる。押している間の形を窓ごとに控える置き場（wintf の `thread_local` か areka の `MouseWiring` か）。捕捉中に放したときは `WM_SETCURSOR` が来ないので、解放のメッセージの中（wintf の `mouse_click.rs`）で同期に出し直すか、tick で出し直すか（後者は 1 tick 遅れる）。「押している」の一次情報源をどれにするか（`DragState`・`GetKeyState`・押下の控え）。
4. **止まったと見なす判定（4.1・4.2・議題 2）**: `TrackMouseEvent(TME_HOVER)` で OS の `WM_MOUSEHOVER` を受ける（時間も許容の幅も OS の設定・窓の手続きの中で同期に出せる・毎回掛け直しが要る）か、ツールチップと同じく `read_hover_time` と `tick_wake::arm_deadline` で期限に tick を起こすか。前者は「動かない」を OS の許容の幅（`SPI_GETMOUSEHOVERWIDTH`／`HEIGHT`）で読む。後の `mouse-hover-event`（`OnMouseHover`）が同じ判定を使う見込みなので、置き場は共有できる形に。
5. **ホイールの形と戻り（4.3・4.4・議題 4）**: `WM_MOUSEWHEEL` の中で同期に出すか、`char_watch` の tick で出すか。「動いたら戻す」の「動いた」を、引き金のメッセージが移動であることだけで見るか、位置の違いで見るか（窓の移動・大きさの変更で OS が合成の移動を送るので、位置で見ないと止まったときの形・ホイールの形が勝手に戻る恐れ）。
6. **止まったまま面が替わったとき（2.6）**: 要件の「出入りを決めるのと同じ時点」は、完了 `mouse-click-wheel-events` 要件 7.9 では「次にカーソルが動いた時点」。そう読めば足す物は無い（次の移動の `WM_SETCURSOR` がその場で新しい当たり判定を引く）。面替わりの tick で出し直すと読むなら、出し直しの口と「当たり判定が変わったか」を tick で引く処理が要り、出入りの決まり（7.9）と食い違う。読み方を確定したい。
7. **表の置き場と運び方（6.8）**: `ShellTarget` に載せて `ShellAssets` → `SwapFinish::Shell` → UI スレッドの資源、という箱の束と同じ道か。入れ替えのとき、新しい表で形を出し直してから古いファイルのカーソルを捨てる順を守る。
8. **カーソルを作るスレッドと時点（6.3・6.9）**: シェルの切り替えの資産は背景のスレッドで作る。`LoadImageW` を背景で呼んで UI スレッドへ渡すか、バイト列だけ背景で読んで UI スレッドで作るか（`.ani` はメモリから作る素直な API が無い見込み）。
9. **検証と記録の層（1.3・1.4・6.5〜6.7）**: parsers は転記だけ（記録を持たない・`boxes.rs` の型）にし、検証と記録を emo-present（`load_shell_target` の「1 回だけ記録」の型）に置くか、検証の純関数を parsers に置いて報告だけ返すか。
10. **ゴーストの descript の読み口（5.x）**: `MountModel` を広げる（パースの層に鍵を足す）か、`placement` の `DescriptSource.ghost_kv` を使う（起動と切り替えの両方で既に手元に在る）か、`build_shell_assets` で読み直すか。
11. **ゴーストの descript のファイルの探し場所（6.4・議題 9）**: 要件はゴーストのフォルダだけ。ukadoc のプロパティ `currentghost.mousecursor` は「相対パス指定はシェル→ゴーストフォルダ内の順で探索する」と書き、「カーソル指定はsurfaces.txtのcursorセクションと同じ指定が可能」とも書く（ https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.mousecursor ）。descript の値も同じ探し方かは正典が黙る。SSP で測るなら議題 9 の測りに足せる。
12. **`system:hand`／`system:grip` の絵の持ち方（6.2・議題 6）**: `.cur` を同梱して `crates/areka/build.rs` の資源（`icons.res`・`tools/app-icon.ps1` が作る）に埋めるか、PNG から DPI ごとの大きさで作る（`CreateIconIndirect`・作った物は自分で捨てる）か、`include_bytes!` から作るか。資源に埋める形は `areka.exe` にしか入らないので、テストの実行ファイルでは別の道（作れないときの既定）になる。
13. **`system:*` の対応表（6.1）**: `finger`→`IDC_HAND`（Windows の `IDC_HAND` は指差しの形）・`move`→`IDC_SIZEALL`・`text`→`IDC_IBEAM`・`wait`→`IDC_WAIT`・`help`→`IDC_HELP`・`no`→`IDC_NO`・`cross`→`IDC_CROSS`・`arrow`→`IDC_ARROW`。SSP の測り（× とボールが `HAND`）と合う。表を §8 に載せるか。
14. **ファイルの置き方の約束（Boundary）**: `input_events/mod.rs` に触らない（新しいコードは `hover_region.rs`／`char_watch.rs` の `#[path]` の子）。問い合わせの部品の装着を `attach_char_pointer_handlers`（`mod.rs`）でなく `char_watch.rs` の側で行う形にするか。
15. **十字の診断との順**: `WINTF_CROSSHAIR=1` のときは十字を先にする（診断の意味を保つ）か。
16. **クリック透過を外した直後（2.5 の裏側）**: 透けた所から絵の上へ入ったとき、`WS_EX_TRANSPARENT` を外す前の移動は下の窓へ行き、外した後は次の移動まで `WM_SETCURSOR` が来ない。マウスがそこで止まると下の窓の形が残る。外した時点（wintf の透過の適用）で形を出し直すか、今の性質として受け入れるか。
17. **実機の確かめの道具（7.3）**: `tools/mouse-input.ps1` に `GetCursorInfo` でカーソルの形を読む動詞を足すか、手順書の段 3 に一度きりの読み方を書くか。SSP の測りと同じ読み方（標準の形の名前か `custom`）にそろえる。

## 6. 調べが要る点（設計の段へ持ち越す）

- **Research Needed**: 捕捉中（`SetCapture`）に放したとき、`ReleaseCapture` の後に `WM_SETCURSOR` が来るか。来ないなら 5 節 3 の同期の出し直しが要る。
- **Research Needed**: 右・中の押下で `WM_SETCURSOR` の引き金が何になるか（lParam の上位語）。右を放した後のメニュー（`TrackPopupMenu`）の間と閉じた後の形。
- **Research Needed**: 窓の位置・大きさの変更（面替わりの再配置・ドラッグ）で OS が送る合成の移動が `WM_SETCURSOR` を起こすか。5 節 5・6 の答えが変わる。
- **Research Needed**: `WM_SETCURSOR` が tick の中（世界を借りている間）に届くことがあるか（メニューの中のループ・`SetWindowPos` など）。借用が取れないときの答えの必要性。
- **Research Needed**: 背景のスレッドで作った `HCURSOR`（`.cur`・`.ani`）を UI スレッドで `SetCursor` してよいか。`DestroyCursor` を呼ぶスレッドの縛り。今出している形を捨てたときの振る舞い。
- **Research Needed**: カーソルの大きさと DPI（モニターごとの DPI・利用者のカーソルの大きさの設定）。`LR_DEFAULTSIZE` で作った物がモニターをまたいだときにどうなるか。`system:*`（共有のカーソル）は OS の設定どおりになる。
- **Research Needed**: `TME_HOVER` と `TME_LEAVE` を同じ窓で掛け直す手順と、ツールチップの判定（`tooltip/turn.rs`）との干渉。
- **SSP で測る（議題 2・3・4・9）**: 止まってから形が替わるまでの時間・行の無い場面の止まったとき・ホイールのときの形・ホイールの形から戻る時・`ghost\master` のカーソルのファイルが効くか。測りは記憶どおり開発者の承認を得てから（本分析では測っていない）。

## 7. 同じウェーブ・隣の spec との重なり

- `balloon-scroll-arrows`（後のウェーブ）: `input_events/mod.rs` に mod の行を足す。本 spec は触らない形（案 C）で 0。
- `seriko-pattern-calls`: `areka-parsers/src/shell/mod.rs` に新しいファイルの行を足すときだけ重なる。
- `placement-measure-bake-once`: `shell_target.rs` の読み込みの口を変える案を選んだときに重なる。本 spec の `shell_target.rs` への足しは数行にとどめ、本体は新しいファイルへ。
- `mouse-hover-event`（版は未定）・`shell-tooltip`・`shell-hover-animation`: 「止まった」の判定と当たり判定の読み口を後から使う。置き場を共有できる形にしておく。

## 8. SSP の測り（2026-10-29・要件討議の承認の後）

- 開発者の許し: 要件討議の「承認」。排他テストの机を取り、測り終えてすぐ返した。
- 相手: SSP（pid 41648）の「窓際のぱすたさん」。面 1000、`scope(0)`・`scope(1)` の rect はどちらも `-2555,844,-1724,1870`。OS の「マウスを止めたと見なす時間」（`SPI_GETMOUSEHOVERTIME`）は 400 ミリ秒。
- 道具: Python の ctypes で `SendInput`（`MOVE|ABSOLUTE|VIRTUALDESK`・per-monitor DPI）を送り、`GetCursorInfo` の `hCursor` を `LoadCursorW(NULL, IDC_*)` と比べて読んだ。押す前に `WindowFromPoint` で SSP の窓かを確かめた。点は 10-23 の測りの点を、今の rect の左上からの差で写した。クリックはしていない。
- 結果（止めて 3 秒のあいだ 5 ミリ秒ごとに読んだ）:

| 点 | 窓のクラス | 止めている間の形 | ホイール 1 回（+120）の後・その後 2px 動かした後 |
| --- | --- | --- | --- |
| ×（-1857,901・行なし） | `Tmainform` | `HAND`（指差し）のまま替わらない | `HAND` のまま・`HAND` のまま |
| ボール（-2388,1448・行なし） | `Tmainform` | `HAND` のまま | — |
| 身体（-2183,1093・`system:hand`） | `Tmainform` | `custom`（SSP の手の形）のまま | `custom` のまま・`custom` のまま |
| 身体（-2073,1283・`system:hand`） | `Tmainform` | `custom` のまま | — |
| 10-23 に矢印だった白地（-2473,1643） | `Tkeroform` | `HAND` のまま | — |

- 読み方:
  - 議題 3: `mousehover*`・`mousewheel*` の行の無い当たり判定の上で、止めても（400 ミリ秒の 7 倍以上）ホイールを回しても、SSP は ukadoc の既定の `system:arrow` を出さず、乗ったときの形のままだった。要件 4.1・4.3・5.4 を「行が無ければ乗ったときの形のまま」に改めた。
  - 議題 2・4: 形が替わらなかったので、止まったと見なす時間とホイールの形から戻る時は測れなかった（ぱすたに `mousehover*`・`mousewheel*` の行が無い）。測るには行のあるシェルが要る。推奨のまま。
  - 議題 9: 測るには SSP に入っているぱすたへ `.cur` を置き、`surfaces.txt` を書き換えて読み直させる必要がある。入っているゴーストを書き換えないので測らなかった。推奨のまま。
  - 白地の点は、今回は窓の枠（`\1`・`Tkeroform`）の上で指差しになった。10-23 は矢印だった。窓の枠の面の当たり判定の上に当たったと見られるが、どの当たり判定かは引いていない（本 spec の決めには効かない）。
- 根拠: スクリプトはセッションの一時フォルダの `cursor_measure.py`（リポジトリの外・残さない）。出力は上の表がすべて。

## 9. 次の段

- 要件討議（`kiro-requirements-discussion`）で 5 節の項目を議題 1〜9 と合わせて決める。
- その後 `/kiro-design shell-mouse-cursor` で設計へ。

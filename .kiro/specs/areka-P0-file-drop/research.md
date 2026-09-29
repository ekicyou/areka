# ギャップ分析: areka-P0-file-drop

> 実測は **2026-09-29・本ブランチ**（main `c3876110`＝`ghost-install` の完了 PR#198 のコミット・ワークツリー `claude/areka-p0-file-drop-9fdc80`）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指す。正典の語は ukadoc MCP（`ukadoc:list_shiori_event:OnFileDrop2:1`・`OnDirectoryDrop:1`・`OnFileDropping:1`）で引き直した。
> 本書は答えを決めない。要件ディスカッションで決める議題は末尾「設計へ持ち越す判断」に番号で並べる。

## 1. 要約

- **受け口は 0・送り道と依頼の口は在る。** 本番に `WM_DROPFILES`／`WS_EX_ACCEPTFILES`／`DragQueryFileW` は 0 件（`crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の 2 ファイルだけ）。一方、窓に関数を差す部品（`OnCloseRequest`・`OnSessionEnd`）、ゴースト窓へ差す系（`app_exit::attach_os_close_request`）、汎用の通知の入口（`KanadeMsg::RaiseEvent`）、依頼の口（`install::submit`）は全部そろっており、本仕様の仕事は**この 4 つを 1 本の線でつなぐこと**に尽きる。
- **新しい型は 2 つで足りる見込み。** wintf に「落とし物を受ける関数を差す部品」1 つ（`OnSessionEnd` と同型）、areka に `InstallOrigin` の変種 1 つ。それ以外は既存の口へ渡すだけ。
- **裁量が残るのは 5 点**: ⑴ 落とし物の振り分けと送出を「窓手続きの中で同期に済ませる」か「一度ためて Input の段で捌く」か ⑵ 定常でないときの `warn!` を誰が出すか（kanade は送り口が在るときしか出せない） ⑶ 終了が指示された後の見分け方（`FirstExit` か `WorkGate` か） ⑷ MIME 表の広さ ⑸ wintf の受け口の決定論テストで `HDROP` を偽造するか読み手を差し替えるか。
- **規模 S（brief どおり 5〜7 タスク）・リスク低。** 先進坑が go で OS 境界の不確かさは消えている。残るのは配線と決定論テストの組み方だけ。

## 2. 今あるもの（要件 → 資産の対応表）

| 要件 | 既存の資産（定義名＋ファイル） | 状態 |
|---|---|---|
| 1.1 受け入れの宣言 | `window_style()`（`crates/areka/src/placement/spawn.rs`）が `WS_EX_LAYERED \| WS_EX_TOOLWINDOW` を返し、バルーン窓・キャラ窓の両方の spawn が同じ関数を使う。wintf の `compute_ex_style`（`crates/wintf/src/runtime/window_factory.rs`）は `WS_EX_LAYERED` を落とし `WS_EX_NOREDIRECTIONBITMAP` を足す以外の bit を通す（同ファイルのテスト `ex_style_dcomp_preserves_other_bits` が固定） | **Missing**: `WS_EX_ACCEPTFILES` の 1 bit。既存テスト `t_i2_no_window_has_ws_ex_topmost`（`placement/spawn_assembly_tests.rs`）が `ex_style == WS_EX_LAYERED \| WS_EX_TOOLWINDOW` を直書きで判定しているので **書き換えが要る**（要件 9.9「置き換え無しに消さない」の対象） |
| 1.2 建て直した窓 | `reopen_ghost_windows` → `prepare_ghost_windows` → `spawn_ghost_windows`（`crates/areka/src/ghost_session.rs`・`placement/spawn.rs`）＝初回と同じ `window_style()` を通る。受け手を差す系は `Added<WindowHandle>` 起点でプロセスに 1 回登録（`ghost_session::register_systems` が `FrameFinalize` へ `attach_os_close_request` を登録） | **在る**: 宣言も装着も起こし直しの回数によらず同じ道を通る。テストは `attach_os_close_request_puts_both_receivers_on_ghost_windows`（`app_exit_tests.rs`）が偽の `WindowHandle` と `run_system_once` で判定する型を持つ |
| 1.3 他の窓に付けない | `GhostWindowMarker` を持つ窓だけが標的（`attach_os_close_request` の `Query<Entity, (With<GhostWindowMarker>, Added<WindowHandle>)>`）。wintf の example の窓は `WindowStyle::default()`（`window/components.rs`）で `WS_EX_LAYERED` だけ | **在る**: 同じ絞り込みを使えば 0 枚は構造で守れる |
| 1.4 パスの一覧とスコープ番号 | 一覧: 先進坑 `dropfiles.rs` の `query_path`（長さを問うてから取り出す）・`handle_drop`（`DragQueryFileW(hdrop, u32::MAX, None)` で件数）。スコープ: `CharWindowMarker { scope }`・`BalloonWindowMarker { scope }`（`placement/spawn.rs`・バルーンは「対応するキャラ窓と同じ番号」）を `app_exit::on_ghost_os_close` が両方読む型 | **Missing**: 本番の受け口。読み方の型は在る。先進坑のコードは「本坑へ写さない」規律（README 冒頭）なので、同じ手順を wintf に書き起こす |
| 1.5 絵の外は何もしない | 先進坑の検証結果: 落とし先は OS が `WS_EX_TRANSPARENT` の bit で決め、透けている所は背後の窓へ渡る（ⓑ・到着の行 0 件）。透過の付け外し `apply_click_through`（`crates/wintf/src/win_style.rs`）は `WS_EX_TRANSPARENT` の bit しか触らない | **在る**: 本仕様のコード 0 行。テストは「宣言の bit が透過の付け外しで消えない」を `apply_click_through` の読み書きで固定できる（要件 1.10） |
| 1.6〜1.8 記録・失敗・0 件 | `WM_ENDSESSION`（`crates/wintf/src/ecs/window_proc/lifecycle.rs`）が「`try_borrow_mut` 失敗→`warn!`／entity 破棄済み→`debug!`／部品なし→`debug!`／在れば呼ぶ」の 4 腕を持つ | **Missing**: `WM_DROPFILES` の腕。4 腕の型をそのまま使える。`DragFinish` は先進坑の `FinishOnDrop`（`Drop` で必ず返す）の型 |
| 1.9 描画・台詞を止めない | `WM_DROPFILES` は待ち行列経由で届く（先進坑: `opaque="unknown"` 0 件＝tick の途中の同期配送は無かった）。振り分けは拡張子とフォルダ判定だけ | **在る**: 同期送信（`SendMessageW`）を足す理由が無い。`ALLOWED_SYNC_SENDS`（`crates/areka/src/session_end_sync_send_tests.rs`）は不変で通る |
| 2.x 振り分け | 該当する既存関数なし。拡張子の大小無視は `eq_ignore_ascii_case`（`install/desk.rs` の `record_installed` がフォルダ名で使う型） | **Missing**: 純粋関数 1 つ（一覧＋「フォルダか」の判定器 → 3 種の一覧）。判定器は `std::fs::metadata(..).is_dir()` で、テストは閉包で差し替える |
| 3.x 書庫を依頼 1 つで | `install::submit(world, InstallOrder { archives, origin }) -> SubmitVerdict`（`crates/areka/src/install/mod.rs`）。`InstallOrigin { Menu, Script }`（`Copy`・`Debug` が記録の語）。待ち行列は `InstallDesk.queue`、1 本ずつは `hand_next_order`（`desk.rs`）、`OnInstallCompleteAll` は `run_order`（`procedure.rs`）が「2 本以上が全部 `Installed`」で 1 回。`origin` は `run_archive`／`archive_steps` へ渡って `info!(install_begin, origin = ?origin)` に載るだけ（分岐 0） | **在る**: `InstallOrigin` に変種 1 つ（例 `WindowDrop`）を足すだけ。完了 `ghost-install` の設計「再検証の引き金」に「`file-drop` は `InstallOrigin` に自分の出どころを 1 つ足す」と明記済み |
| 3.5 終了後は口が断る | `submit` は `desk.gate.is_closing()` で `Closing`。ただし門が閉じるのは `exit_wait::begin_close`（呼び手は `main.rs` の `run()` の後と `session_end.rs`） | **注意（Constraint）**: 終了が指示されてから `run()` が返るまでの間は門が開いたままで、`submit` は `Queued` を返し、後で `discard_for_exit` が捨てる。「終了が指示された」を run の中で見分ける印は `FirstExit`（`app_exit.rs`・`quit_app` が挿す）。§5 ⑶ |
| 4.x `OnFileDrop2`・5.x `OnDirectoryDrop` | `KanadeMsg::RaiseEvent { id, references, method, reply: Option<..> }`（`crates/areka-kanade/src/msg.rs`）→ `on_raise_event`（`schedule/change.rs`）: `allowed_static` で許可表照合→`Phase::Steady` でなければ `warn!(raise_event_not_steady)`→`events::raise` で渡された列のまま GET。送り口の取り方は `desk::send_held`（`GhostSlot` → `GhostSession::kanade()`）。応答の置き換えは `value_replaces_active_talk`（`schedule/events.rs`） | **Missing**: 許可表の 2 語と Reference の組み立て。区切り byte 値 1 は `install/judge.rs` の私有 `const SEPARATOR: &str = "\u{1}"` が同じ値を持つ（`pub(crate)` へ上げれば再利用可・複製すると 2 か所） |
| 4.4〜4.5 MIME | `mime`／`application/` は `crates/` に 0 件 | **Missing**: 表 1 つ（広さは設計） |
| 6.3 定常でないとき | kanade 側は `raise_event_not_steady` の `warn!` で捨てる（`raise_event_tests.rs` の `request_outside_steady_is_dropped_with_one_warn_and_not_queued` が固定済み）。ただし切替の途中は `ghost_switch::take_down` が `GhostSlot` の中身を `take()` するので**送り口そのものが無い** | **Constraint**: 「送り口が無い」腕の `warn!` は areka 側で出すしかない。§5 ⑵ |
| 7.3 許可表 21→23 | `ALLOWED_EVENT_IDS`（`schedule/events.rs`・各語の直上に `// ukadoc:` の行）。`events_change_tests.rs` の `allowed_static_returns_the_table_spelling_for_the_two_change_events` が `assert_eq!(ALLOWED_EVENT_IDS.len(), 21)` | **Missing**: 2 語＋数の書き換え。型は完全に前例どおり |
| 8.3〜8.4 台帳・報告書・下書き | `doc/ukadoc-coverage/ledger/shiori.toml` の `OnFileDrop2`（`introduced = "2.7.98"`）・`OnDirectoryDrop` は `status = "absent"`・`owner = ""`・`priority = "B4"`。実装済みの行の形は `OnInstallBegin` の `status = "implemented"`・`owner = "areka-P0-ghost-install"`。報告書は `cargo run -p ukadoc-survey -- report`／`report-summary`。`roadmap-draft.md` は `[briefs] count = 37`（2026-09-29・`ghost-install` の行が最後）で、束の表の「4 投げ込み」の行は候補名を **`areka-P0-file-drop-events`（0 本）** と書いている | **Missing**: 2 行の書き換え＋報告書の作り直し。**注意**: 下書きの候補名が実名 `areka-P0-file-drop` と違う。`[[spec]]` の行を足すなら `count` を 38 へ（`cargo test -p ukadoc-survey` が見張る） |
| 8.5 依存 0 | ワークスペースの `Cargo.toml` の `windows` の features に `Win32_UI_Shell`（`DragQueryFileW`・`DragFinish`・`HDROP`・`DROPFILES`）と `Win32_UI_WindowsAndMessaging`（`WM_DROPFILES`・`WS_EX_ACCEPTFILES`）が既に在る（先進坑がこの 2 つだけで組めている） | **在る**: 外部クレート 0・機能フラグの追加も 0。`Win32_System_Memory`（`GlobalAlloc`）は**無い**＝§5 ⑸ の偽造案に効く |
| 8.7 1,000 行 | `placement/spawn.rs` 769・`input_events/mod.rs` 545・`schedule/events.rs` 561・`window_proc/lifecycle.rs` 471（テストが同居する歴史的形式）・`window_proc/mod.rs` 332 | **Constraint**: 既存ファイルへ足すのは宣言 1 bit・振り分け表 1 腕・許可表 2 語・変種 1 つに留め、受け口の本体と振り分けは新規ファイルへ |
| 9.9 `log-capture-kit` | `lifecycle.rs` の `wm_close_skips_stale_entity_as_normal_teardown` が wintf 側の `capture_under_filter`、`raise_event_tests.rs` が kanade 側の捕捉を使う | **在る** |

## 3. 足りないもの（一覧）

1. wintf: `WM_DROPFILES` の腕（`dispatch_window_message` の `match` に 1 行）と、その受け手（新規ファイル・`lifecycle.rs` の `WM_ENDSESSION` と同じ 4 腕）。
2. wintf: 窓に差す部品 1 つ（`window/components.rs`・`OnSessionEnd` と同型で、関数の署名にパスの一覧が増える）。
3. areka: `window_style()` の `ex_style` に `WS_EX_ACCEPTFILES`（1 bit）＋ `spawn_assembly_tests.rs` の直書きの更新。
4. areka: ゴースト窓へ部品を差す（`attach_os_close_request` の `insert` に 1 つ足すか、同型の系を隣に置く）。
5. areka: 落とし物の受け手（新規ファイル）＝スコープ番号の読み取り→振り分け→`OnFileDrop2`→`OnDirectoryDrop`×n→`submit`。
6. areka: 振り分けの純粋関数・Reference の組み立て・MIME 表（同じ新規ファイルか兄弟ファイル）。
7. areka: `InstallOrigin` に変種 1 つ。
8. kanade: 許可表 2 語＋テストの数。
9. 台帳 2 行・報告書・`roadmap-draft.md`・`signoff.md`。

## 4. 設計の選択肢

### 4.1 wintf の受け口の形（部品の署名と OS 境界の切り方）

**A. `OnSessionEnd` と同型の部品＋OS 読み取りは受け手の中**
`pub struct OnFilesDropped(pub fn(&mut World, Entity, Vec<PathBuf>))`（名前は仮）を `window/components.rs` に置き、`WM_DROPFILES` の受け手（新規 `window_proc/drop.rs`）が `HDROP` から一覧を読み（先進坑の `query_path` の手順）、`DragFinish` を `Drop` で保証し、部品が在れば呼ぶ。
- ✅ `WM_ENDSESSION` の 4 腕をそのまま写せる。areka は OS を知らない。
- ❌ 受け手の「成功の腕」を決定論で踏むには本物の `HDROP` が要る（§4.5）。

**B. OS 読み取りを関数 1 つに切り出し、受け手はそれを呼ぶだけ**
`fn read_dropped_paths(hdrop: HDROP) -> Option<Vec<PathBuf>>`（OS 境界・`unsafe` はここだけ）と `fn deliver_dropped(world, entity, paths) -> HandlerResult`（World 側・純粋）に分け、`WM_DROPFILES` の腕は 2 つを続けて呼ぶ。
- ✅ `deliver_dropped` は偽の一覧で 4 腕を全部踏める。`read_dropped_paths` は「null の `HDROP` → 0 件で `None`（`warn!`）」の失敗の腕だけ実物で踏める。
- ❌ 振り分け表 → 受け手の「成功の腕」の到達は、`read_dropped_paths` を経由する 1 本の線を実物で踏めない（要件 9.1 の「受け口が 1 回呼ばれる」を配送表からの線で固定するには §4.5 の偽造が要る）。

**C. A＋読み手の差し替え口（trait／閉包）**
- ❌ 差し替えのための抽象が 1 つ増える（実装は 1 つ）。B で足りる。

### 4.2 宣言と装着の置き場

**A. `window_style()` に 1 bit＋`attach_os_close_request` の `insert` に部品 1 つ**（最小差分）
- ✅ 変更 2 行。起こし直しにも他の窓の除外にも既存の構造が効く。
- ❌ `app_exit.rs` の系が「終了」以外の部品を差すことになり、名前と中身がずれる（名前を変えるか、doc で「ゴースト窓に OS の受け手を差す系」と読み替える）。

**B. 装着だけ別の系（新規ファイル側に `attach_file_drop` を置き `register_systems` で隣に登録）**
- ✅ 責務が名前どおり。落とし物の受け手と装着が同じファイルに並ぶ。
- ❌ `Added<WindowHandle>` 起点の系が 3 本になる（`register_ghost_windows_click_through`・`attach_os_close_request`・新設）。同じ query の系が増えるだけで害は無い。

どちらでも要件 1.1〜1.3・9.2 は満たせる。

### 4.3 振り分けと送出をどこで済ませるか

**A. 窓手続きの中で同期に全部済ませる**（受け手の関数が `&mut World` を持つので `submit` も `Sender::send` も呼べる）
- ✅ 要件 6.5「受け取ったその巡の中で終える」を文字どおり満たす。状態を持たない。
- ❌ 窓手続きの中で World を借りている時間が伸びる（振り分けは拡張子と `metadata` だけなので短い）。

**B. 受け取った一覧を World の資源にためて Input の段で捌く**（`InstallDesk::drain` と同じ型）
- ✅ 窓手続きは軽い。
- ❌ 資源と系が 1 組増え、要件 6.5 の「次の投げ込みを待たない・順を入れ替えない」を別途固定する必要が出る。ためる理由（重い処理・スレッド跨ぎ）が無い。

### 4.4 送り口と「定常でない」「終了後」の見分け

- 送り口: `desk::send_held` と同じく `GhostSlot` → `GhostSession::kanade()` を `cloned()` で取り、`KanadeMsg::RaiseEvent { .., method: ShioriMethod::Get, reply: None }` を送る。
- 定常でない: 送り口が在れば kanade が `raise_event_not_steady` の `warn!` を出す（areka のテストは「送った内容」を偽の `mpsc::Sender` で固定し、捨てる判定は kanade の既存テストに任せる）。送り口が無い（切替中・LogSink の起動で実行系なし）ときは areka が `warn!` を 1 件出す。
- 終了後: `FirstExit` の有無（run の中で唯一の印）。門（`WorkGate`）は run が返るまで閉じないので、要件 6.4「依頼は手続きの口が断り」を字義どおりにするなら `submit` の判定を足すか、要件の語を「捨てられる」へ緩める（§5 ⑶）。

### 4.5 決定論テストの組み方（要件 9.1）

**A. 本物の `HDROP` を偽造する**: `DROPFILES`（`Win32_UI_Shell` に在る）を `GlobalAlloc` で作り、`DragQueryFileW` に読ませる。窓もメッセージループも要らない。
- ✅ 配送表 → 受け手 → 部品の 1 本の線を実物で踏める。
- ❌ `GlobalAlloc`／`GlobalLock` は `Win32_System_Memory` で、ワークスペースの features に**無い**（要件 8.5 は「`windows` クレートの機能を足すことは数えない」としているので可）。テスト専用の追加なら `[dev-dependencies]` 側で足せるか確かめる要あり（Research）。

**B. §4.1-B の分割で World 側だけを踏み、OS 側は失敗の腕だけ**
- ✅ 機能フラグ 0。
- ❌ 成功の線は「実機」でしか踏めない（`signoff.md` へ回る）。

### 4.6 MIME の表

**A. 定数の表（拡張子 → MIME）を areka に置く**: 広さの案 ⑴ 画像・音・動画・文書・書庫・テキストの 20〜30 語 ⑵ IANA の主要 60〜80 語。
**B. OS のレジストリ（`AssocQueryStringW(ASSOCSTR_CONTENTTYPE)`）**: 機械ごとに答えが違い決定論テストに乗らない＝要件 4.5 と衝突。
**C. A を主、B を表に無いときの補い**: 決定論は A の部分だけ。

要件 4.5 が「表を持つ・全項目を判定する」と定めているので A。広さだけが設計の判断（§5 ⑷）。

## 5. 設計へ持ち越す判断（番号つき）

1. **wintf の受け口の切り方**（§4.1 A か B）と部品の名前・署名（一覧の型は `Vec<PathBuf>` か `Vec<String>`）。
2. **定常でないときの `warn!` の出し手**: kanade（送り口あり）と areka（送り口なし）で 2 か所になるのを承知するか、areka が常に自分で出すか（後者は kanade の判定と二重）。
3. **終了が指示された後の見分け**: `FirstExit` を見て `submit` を呼ばない（依頼 0）か、`submit` に任せる（`Queued` → `discard_for_exit`）か。要件ディスカッションで要件 3.5・6.4 を「入れない・記録 1 件」の結果の語へ改め、どちらの道で止めるかをここへ預けた。
4. **MIME 表の広さ**（§4.6 A-⑴ か ⑵）と、表を areka の新規ファイルに置くか。
5. **決定論テストで `HDROP` を偽造するか**（`Win32_System_Memory` を test でだけ足せるか＝Research）。
6. **装着の置き場**（§4.2 A: `attach_os_close_request` に足す／B: 新設の系）。
7. **`InstallOrigin` の変種の綴り**（`WindowDrop`／`Drop`／`FileDrop`・記録の検索語になる）。
8. **区切り byte 値 1 の定数の置き場**: `install/judge.rs` の `SEPARATOR` を `pub(crate)` へ上げて共用するか、新規ファイルに同じ値を持つか。
9. ~~`roadmap-draft.md`~~ → 要件ディスカッションで要件 8.4 に確定（行を足して 38・束の表の「投げ込み」の行を台帳に合わせる）。設計の判断は残らない。
11. **書庫の目次だけを読む口**（要件ディスカッション議題 1 で追加・要件 2.1・2.2・2.4）: 振り分けで `.nar`／`.zip` の最上位に `install.txt` が在るかを見る。今の `NarArchive::open`（`crates/areka-nar/src/lib.rs`）は全エントリを伸長してから `install.txt` を解釈するので、窓手続きの中で呼ぶと大きな書庫で描画が止まる（要件 1.9）。案: `areka-nar` に中央ディレクトリ（目次）だけを読む公開関数を足し、`install.txt` の探し方は私有の `manifest::locate_install_txt` を共用する（振り分けと手続きの判定を 1 か所にする＝要件 2.2）。背景へ回す案は要件 6.5「その巡で終える」と衝突する。見立て +1〜2 タスク（S のまま 6〜9）。目次の読めない書庫の `warn!` の語もここで決める。
10. **パスの扱い**: `DragQueryFileW` が返す綴りをそのまま渡す（正規化しない）。エクスプローラは絶対パスを返すが、要件 1.4 の「絶対パス」を areka が検査するか（`Path::is_absolute()` で `warn!` だけ）。

## 6. 規模とリスク

- **規模: S**（1〜3 日・brief の 5〜7 タスクどおり）。理由: 触るのは配線 4 か所＋新規ファイル 2〜3 本＋表 2 行。新しい概念は無い。
- **リスク: 低**。理由: OS 境界（届くか・宣言の bit・絵の外）は先進坑が go で決着済み。残るのは §5 の 10 点で、どれも答えで作業が変わる幅は小さい。
- 注意点: ⑴ `spawn_assembly_tests.rs` の直書きの更新を忘れると赤 ⑵ `events_change_tests.rs` の 21 を 23 に ⑶ `lifecycle.rs` は 471 行でテスト同居なので腕を足さず新規ファイルへ ⑷ `Win32_System_Memory` を本番の features へ足すと「本番が使わない機能」を抱えるので、足すなら test 限定で。

## 7. Research Needed（設計で確かめる）

- `[dev-dependencies]` の `windows = { features = ["Win32_System_Memory"] }` がテストビルドにだけ効くか（cargo の feature 統合は package 単位なので**本番ビルドにも効く**可能性が高い。効くなら偽造案を捨てるか、要件 8.5 の読みで許すか）。
- `DragQueryFileW` が返すパスに `\\?\` 接頭辞が付く場面があるか（無ければ正規化の議論は不要）。
- `OnDirectoryDrop` を 1 つずつ送ったとき、kanade の `RaiseEvent` 2 件目が 1 件目の応答より先に処理される順序（`Action::ShioriRequest` は直列なので順は保たれる見込み。`raise_event_tests.rs` に「連続 2 件」の檻は無い）。

## 8. 次の段階

`/kiro-design areka-P0-file-drop`（要件ディスカッションで §5 を消化した後）。設計の骨は brief の Approach 表の 5 段のままで足りる。

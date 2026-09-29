# 設計レビュー: areka-P0-file-drop

> 2026-09-29・本ブランチ（`claude/areka-p0-file-drop-9fdc80`・main `c3876110` 相当）。設計書が「既存」と書いた主張は、下の「確かめた実物」の一覧のとおり全部ソースを開いて突き合わせた。正典は ukadoc MCP（`OnFileDrop2:1`・`OnDirectoryDrop:1`）で引き直した。本レビューは対話なしで書いた（利用者への問いは出していない）。

## レビュー要約

設計は、既に在る 4 つの部品（窓に関数を差す部品の型・ゴースト窓へ部品を差す場所・汎用の通知の入口・依頼を渡す口）を 1 本の線でつなぐ最小の形で、主張した既存コードの姿はすべてソースと一致した。振り分けの順（フォルダ → インストール対象 → それ以外のファイル）と Reference の形（`OnFileDrop2` は 3 つ・`OnDirectoryDrop` は 2 つ）は要件と正典のとおりで、失敗の腕にはどれも記録が付いている。「終了が指示された後は何も足さない」も、終了の道筋を追った限り安全と判断した。残るのは数の誤記 1 件と、テストが踏まない腕 2 つ、境界事例の言い切り 1 件で、いずれも設計の骨を動かさずに直せる。

## 確かめた実物（設計の主張とソースの突き合わせ）

| 設計の主張 | 実物 | 一致 |
|---|---|---|
| `dispatch_window_message(world: &Rc<RefCell<EcsWorld>>, entity, msg) -> Option<LRESULT>`・30 腕・`WM_DROPFILES` なし・`_ => None` | `crates/wintf/src/ecs/window_proc/mod.rs` | ○ |
| `OnCloseRequest`／`OnSessionEnd(pub fn(&mut World, Entity))`・`SparseSet`・`Clone, Copy` | `crates/wintf/src/ecs/window/components.rs` | ○ |
| `WM_ENDSESSION` の 4 腕（借用中 → `warn!`／破棄済み → `debug!`＋`DESPAWNED_SKIP_TAG`／部品なし → `debug!`／在れば `info!` して呼ぶ）・全腕 `Some(LRESULT(0))` | `crates/wintf/src/ecs/window_proc/lifecycle.rs` | ○ |
| `window_style()` は私有・`WS_POPUP \| WS_VISIBLE`／`WS_EX_LAYERED \| WS_EX_TOOLWINDOW`・キャラ窓とバルーン窓の 2 か所だけが呼ぶ | `crates/areka/src/placement/spawn.rs` | ○ |
| `t_i2_no_window_has_ws_ex_topmost` が `ex_style == WS_EX_LAYERED \| WS_EX_TOOLWINDOW` を直書き | `crates/areka/src/placement/spawn_assembly_tests.rs` | ○ |
| `compute_ex_style` は `(ex_style & !WS_EX_LAYERED) \| WS_EX_NOREDIRECTIONBITMAP`（他のビットは通す） | `crates/wintf/src/runtime/window_factory.rs` | ○ |
| `apply_click_through` は `WS_EX_TRANSPARENT` のビットだけ触る | `crates/wintf/src/win_style.rs` | ○ |
| `prepare_ghost_windows` の閉包で `spawn_ghost_windows` → `attach_char_pointer_handlers` → `attach_release_handlers` → `attach_balloon_pointer_handlers` を同期に呼ぶ・`reopen_ghost_windows` は同じ関数へ委譲 | `crates/areka/src/ghost_session.rs` | ○ |
| `InstallOrder { archives, origin }`・`InstallOrigin { Menu, Script }`・`submit -> SubmitVerdict { Queued, Empty, NoDesk, Closing }`・`install_order_queued`／`install_order_refused` | `crates/areka/src/install/mod.rs` | ○ |
| `judge.rs` の私有 `const SEPARATOR: &str = "\u{1}"`・`judge` は `pub(crate) mod` | `crates/areka/src/install/judge.rs`・`mod.rs` | ○ |
| `desk::send_held` が `GhostSlot → slot.0 → session.kanade().cloned()` で送出端を取る・`GhostSession::kanade() -> Option<&Sender<KanadeMsg>>`・`GhostSession::for_test(Some(tx), dir)` | `crates/areka/src/install/desk.rs`・`ghost_session.rs` | ○ |
| `discard_for_exit` は終了時に待ち行列を捨て `warn!(install_pending_discarded)` | `crates/areka/src/install/desk.rs` | ○ |
| `KanadeMsg::RaiseEvent { id, references, method, reply: Option<..> }` | `crates/areka-kanade/src/msg.rs` | ○ |
| `on_raise_event`: 許可表に無ければ `raise_event_not_allowed`・定常でなければ `raise_event_not_steady` で捨てる（積まない） | `crates/areka-kanade/src/schedule/change.rs` | ○ |
| `ALLOWED_EVENT_IDS` は 21 語・各語の直上に `// ukadoc:`・`events_change_tests.rs` が `len() == 21` | `crates/areka-kanade/src/schedule/events.rs`・`events_change_tests.rs` | ○ |
| `NarArchive::read` = `fs::read` → `read_central_directory` → `validate_entry_names` → 全エントリ `inflate_entry` → `locate_install_txt` → `parse_manifest`・`locate_install_txt(&[EntryName])` は `MissingInstallTxt` を返す | `crates/areka-nar/src/lib.rs`・`manifest.rs`・`container.rs`・`names.rs` | ○ |
| `quit_app` は `despawn_app_windows` を先に呼んでから `FirstExit` を挿し `request_exit`・areka 本番で `request_exit` を呼ぶのはここだけ・OS のセッションの終了も `quit_app` を通る | `crates/areka/src/app_exit.rs`・`session_end.rs`（`quit_app(world, ExitOrigin::SessionEnd)`）・`main.rs`（`ExitPolicy::Explicit`） | ○ |
| `ghost_switch::take_down` が `slot.0.take()` する（切替中は送出端が無い） | `crates/areka/src/emo2_boot/ghost_switch.rs` | ○ |
| 根の `Cargo.toml` に `Win32_UI_Shell` あり・`Win32_System_Memory` なし・`resolver = "2"` | `Cargo.toml` | ○ |
| 先進坑の手順（`DragQueryFileW(hdrop, u32::MAX, None)` で本数・長さを問うてから取り出す・`FinishOnDrop` の `Drop` で `DragFinish`） | `crates/pilot/examples/pilot-dropfiles-on-wuc-window/dropfiles.rs` | ○ |
| `sample-ghost-kit` の `NarBuilder`・`log-capture-kit` の `capture`／`capture_under_filter` が areka の dev-dependencies から使える | `crates/sample-ghost-kit/src/nar_writer.rs`・`crates/log-capture-kit/src/`・`crates/areka/Cargo.toml` | ○ |
| `areka-nar` の字面の見張り（`inflate_entry(` は container 1・lib.rs 1／`tracing::` は lib.rs だけ・`tracing::error!` は 1 つ） | `crates/areka-nar/src/lib_tests.rs` | ○（`peek_install_txt` は伸長も記録もしないので通る） |
| 正典 `OnFileDrop2`: Ref0 パス（複数は byte 値 1 区切り）・Ref1 スコープ番号・Ref2 MIME（同じ区切り）／`OnDirectoryDrop`: Ref0 パス 1 つ・Ref1 スコープ番号 | ukadoc MCP | ○ |

### 「終了が指示された後は何も足さない」（設計で決めたこと 3）の検証

安全と判断した。根拠は次のとおり。

1. areka 本番で終了を指示する（`request_exit`）のは `quit_app` の 1 か所で、その関数は指示の**前**に同じ呼び出しの中で全ゴースト窓の entity を `despawn` する。窓の entity が消えるのと終了の指示の間に、別のメッセージが割り込む余地は無い（同じスレッド・同じ関数の中）。
2. entity が消えてから OS の窓が実際に壊れるまでの短い間に届いた `WM_DROPFILES` は、wintf の受け手の「entity 破棄済み」の腕が `debug!` で打ち切り、`DragFinish` だけ呼ぶ。この腕は決定論テスト 4 で踏む。
3. 終了の指示より前（別れの台詞の再生中）に落とされた物は「終了が指示された後」ではない。イベントは kanade が定常でないとして `warn!` で捨て、インストール対象は完了 `ghost-install` の待ち行列に積まれ、終了時に `discard_for_exit` が `warn!(install_pending_discarded)` で捨てる——メニュー・台本からの依頼と同じ扱いで、本仕様が新しく作る経路は無い。

補足として 1 点。wintf の橋（`runtime/wndproc_bridge.rs`）は、World が借用中なら振り分け表を呼ばずに `None`（既定処理）へ流す。この道では `DragFinish` も記録も無い。ただし `WM_DROPFILES` は投函で届き、投函されたメッセージが配られるのは tick の外（メニューの表示も World を借りずに行う）なので、本番でこの道に入る場面は見つからなかった。`WM_ENDSESSION` 以外の全メッセージが同じ扱いであり、本仕様の境界の外として据え置きでよい。設計書の記録の表の「World が借用中 → `warn!`」は受け手を直に呼ぶテスト（5）でだけ踏める腕であることを、実装者が承知していればよい。

## 重大な指摘（3 件・どれも設計の骨は動かさない）

### 🔴 指摘 1: MIME の表の要素数が本文と食い違う（41 と書くが、表を数えると 38）

- **問題**: 「設計で決めたこと 4」「MIME の表」の末尾・テスト 12 は「拡張子 41・MIME 33 種」「表の要素数（41）を直書きで判定」と書く。表の行を実際に数えると、拡張子は png・jpg・jpeg・gif・bmp・webp・ico・svg（8）＋mp3・wav・ogg・flac・mid・midi・m4a（7）＋mp4・webm・avi・mkv・mov（5）＋txt・csv・htm・html・css・js・md（7）＋xml・json・pdf（3）＋zip・nar（2）＋7z・gz・tar・rar（4）＋exe・dll（2）＝**38**。MIME の 33 種は合っている。
- **影響**: 実装者が設計どおり 41 を直書きすると赤になり、赤を消すために表へ拡張子を 3 つ足すか数を書き換えるかの判断が実装の中で起きる。「両方の枝が同じ誤った手書きの数を書くと黙って通る」型の誤りで、設計の段で潰しておくべきもの。
- **提案**: 本文とテスト 12 の数を 38 に直す。表を 41 にしたいなら足す 3 語（例: `tif`／`tiff`・`aac`・`doc` など）を表に書き加える。どちらでも、テストは「表の要素数」と「表に無い拡張子・拡張子なし・大文字」を判定する形のまま。
- **対応する要件**: 4.5・9.8
- **設計書の場所**: 「設計で決めたこと」表の 4／「MIME の表」の末尾の段落／Testing Strategy 12

### 🔴 指摘 2: 記録つきの腕のうち、決定論テストが踏まない分岐が 2 つ残る

- **問題**: ⑴ `send_event` の「kanade が止まっている」腕（`Sender::send` が `Err` → `warn!(file_drop_send_failed)`）は、配線テスト 13〜20 のどれにも検体が無い（17 は「送出端そのものが無い」場合で、別の腕）。⑵ `DropReadError::Query { index, total }` は `DragQueryFileW` が長さ 0 か中身 0 を返したときだけ作られるが、`HDROP` を偽造しない方針（設計で決めたこと 5）では決定論で到達せず、実機の ⑶ は成功の線なのでこちらも踏まない。テスト 6 は `Display` の文字列を見るだけで腕には入らない。
- **影響**: 要件 8.1「記録の無い失敗の経路 0」と 9.9 の決定論網羅に対し、記録の語が正しく出ることを誰も確かめない腕が残る。⑴ は 1 検体で埋まる（安い）。⑵ は構造上踏めないので「踏まない腕」として設計に明記しないと、実装レビューで毎回同じ問いが立つ。
- **提案**: ⑴ 配線テストに「`GhostSession::for_test(Some(tx), dir)` で置き場を作り、受信端 `rx` を先に落としてから落とす → `WARN` の `file_drop_send_failed` がイベント 1 件につき 1 件・依頼は積まれる」を 1 本足す。⑵ Testing Strategy 6 に「`Query` の腕は決定論でも実機でも到達しない（`GlobalAlloc` を足さない代償）。`Display` の検査で語だけを固定する」と 1 行書き、記録の表の当該行にも同じ注記を添える。
- **対応する要件**: 8.1・9.6・9.9
- **設計書の場所**: `input_events::file_drop` の Service Interface（`send_event` の説明）／Error Handling「記録の表」／Testing Strategy 6・17

### 🔴 指摘 3: 名前の検証で撥ねられる書庫の行き先を、要件の語で言い切っておく

- **問題**: `peek_install_txt` は `read_central_directory` → `validate_entry_names` → `locate_install_txt` の順で、名前の検証に落ちた書庫（たとえば `..` を含むエントリ名がある一方で最上位に `install.txt` は在る）は `Err(Refused)` → 振り分けでは「インストール対象でないファイル」→ `OnFileDrop2`＋`warn!(file_drop_archive_unreadable)` になる。同じ書庫をメニューから入れると手続きは `OnInstallRefuse` を送る。裁定 7 の字面「`.nar`／`.zip` で目次の最上位に `install.txt` が在る物＝インストール対象」とは食い違い、要件 2.4「目次の読めない書庫」に名前の拒否を含める読みは設計書の Postconditions（「`NarError::Refused`（構造・名前の拒否）」）にしか書かれていない。加えてテスト 21 の「`Ok(true)` ⇔ `open` が `MissingInstallTxt` 以外」という同値は、この検体では成り立たない（`peek` は `Err`、`open` は名前の拒否）。列挙した 3 本の検体に限れば通るが、主張の書き方が一般則に見える。
- **影響**: 利用者から見える結果の差は「壊れた名前を持つ書庫を落としたとき、ゴーストが『ファイルを渡された』台詞を返すか、『入れられない』台詞を返すか」。どちらも失敗は記録に残り、展開は 0 件なので危険は無いが、決めずに実装へ進むと実装者が `validate_entry_names` を飛ばして `locate` を先に置く（`EntryName` を作れないので別の探し方になり要件 2.2 の「探し方の同一性」が崩れる）方向へ迷いうる。
- **提案**: 設計の今の順（検証 → 探す）を採ると明記し、要件 2.4 の「目次の読めない書庫」には名前の拒否も含める（`open` も同じ順で撥ねるので探し方は同じ）と 1 行書く。テスト 21 の同値の主張は「同じ 3 本に限る」と限定するか、名前を撥ねる検体を 1 本足して「`peek` が `Err(Refused(名前))` ⇔ `open` も同じ理由で `Refused`」を並べる。
- **対応する要件**: 2.1・2.2・2.4・10.7（裁定 7）
- **設計書の場所**: `areka_nar::peek_install_txt` の Service Interface と Postconditions／`sort_drops` の規則 4／Testing Strategy 21

## 設計の長所

- **既存の型に完全に乗っている。** OS の境界は `WM_ENDSESSION` の 4 腕、装着は spawn 直後の同期装着、依頼は `install::submit`、送出は `KanadeMsg::RaiseEvent`——新しく作る型は wintf の部品 1 つ・振り分けの結果の型 1 つ・`InstallOrigin` の変種 1 つ・`areka-nar` の公開関数 1 つで、設計書が「既存」と書いた主張は上の表のとおり全部ソースと一致した。依存の向き（wintf は areka を知らない・`placement` は `crate::` を持てないので装着は `input_events` 側）も既存の規律どおり。
- **判断の分かれ目が純粋な関数に閉じている。** `sort_drops` は「フォルダか」「目次に `install.txt` が在るか」を閉包で受けるので、要件 2.1〜2.6 の全規則（フォルダ優先・大小無視・`install.txt` の有無・目次が読めない・問い合わせ失敗・順の保持）を fs に触らず決定論で踏める。記録は入口 `on_ghost_files_dropped` と `send_event` に寄せ、失敗の表に「場面・場所・水準・語・続き」を全部書いてあるので、要件 8.1 の照合が表 1 つで済む。

## 最終判断

**GO**

- **理由**: 既存の構造との食い違いは 0、要件 1〜10 の各項目に部品と流れが対応し（覆した裁定 0）、実装の道筋は新規 2 ファイル＋変更 4 か所で明快。上の 3 件は数の訂正・テスト 1 本の追加・境界事例の 1 行の言い切りで、設計の骨も規模の見立て（8〜9 タスク）も動かさない。
- **次の段階**: 設計ディスカッションで指摘 1〜3 を設計書に反映（MIME の数 38・`file_drop_send_failed` の検体・`Query` の腕の注記・名前の拒否の行き先とテスト 21 の限定）してから `/kiro-spec-tasks areka-P0-file-drop` へ。

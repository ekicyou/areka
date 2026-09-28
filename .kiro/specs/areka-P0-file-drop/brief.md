# Brief: areka-P0-file-drop

> 2026-09-28 `/kiro-discovery`（`ghost-install` の要件ディスカッションの議題 1 から）で起票。`ghost-install` の見立てが 1 spec の上限 20 タスクを超えたため（ギャップ分析 20〜26）、**ゴーストの窓へのドラッグ＆ドロップを丸ごと本仕様へ切り出した**（開発者確定「分けてよい。分けるspecを今作ってください」）。`ghost-install` はメニュー「インストール…」と台本 `\![execute,install,path,…]` の 2 つの入口と、インストールの手続きそのものを持つ。
> 本文の file:line・行数は**起票時の実測**（2026-09-28・ソースは main `10a8d724` から不変）。出どころは `ghost-install` の `brief.md`（09-26 先進坑の節・09-27／09-28 の再測定の節）と `research.md`（ギャップ分析）で、どちらも実物で引き直してある。**着手時に必ず引き直すこと**（とくに `ghost-install` が着地した後の依頼の型の名前）。

## Problem

**誰の何が困っているか**:

- **`.nar` を手に入れた第三者**: 正典（`ukadoc:manual_install`）は「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる」と書く。伺かの利用者が最初に試すのは、エクスプローラから `.nar` をゴーストの上へ落とすこと。`ghost-install` が着地してもメニューからしか入れられず、落としても何も起きない。
- **ゴーストの作者**: 辞書に書いた `OnFileDrop2`（ファイルを渡されたときの台詞や遊び）・`OnDirectoryDrop` が areka では 1 度も動かない。

## Current State

- **受け口が 0。** `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget`／`WS_EX_ACCEPTFILES` は `crates/**/*.rs` のうち `crates/pilot/` の 2 ファイルにしか無く、本番は 0 件（較正: 同じ範囲で `WM_ENDSESSION` は 8 ファイル 29 件）。
- **wintf の振り分け表に腕が無い。** `crates/wintf/src/ecs/window_proc/mod.rs`（332 行）の `dispatch_window_message` の `match msg.msg` に `WM_DROPFILES` は 0 行（受け手の形は `fn(world, entity, hwnd, wparam, lparam) -> Option<LRESULT>`・表に無いものは `_ => None` で既定処理へ）。
- **窓に関数を差す部品の前例**: wintf `ecs/window/components.rs`（348 行）の `OnSessionEnd(pub fn(&mut World, Entity))` と、`ecs/window_proc/lifecycle.rs` の `WM_ENDSESSION` の受け手（完了 `ghost-shell-balloon-switch` が足した）。窓への装着の前例は areka `app_exit::attach_os_close_request`（`GhostWindowMarker` を持つ窓に差す）。
- **受け入れの宣言が無い。** ゴースト窓の様式 `crates/areka/src/placement/spawn.rs`（769 行）の `window_style`＝`WindowStyle { style: WS_POPUP|WS_VISIBLE, ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW }`。キャラクター窓とバルーン窓で共通。wintf の `window_factory.rs` は `WS_EX_LAYERED` を外し `WS_EX_NOREDIRECTIONBITMAP` を足す以外の bit を通す。
- **窓のスコープ番号は読める。** `CharWindowMarker { scope }`・`BalloonWindowMarker { scope }`（`placement/spawn.rs`）。`app_exit::on_ghost_os_close` が同じ読み方をしている。
- **イベントの送り道は在る。** 汎用の通知の入口 `KanadeMsg::RaiseEvent { id, references, method }`（kanade `msg.rs`）→ `schedule/change.rs` の `on_raise_event`。許可表 `ALLOWED_EVENT_IDS`（kanade `schedule/events.rs`・535 行）に無い名前と定常以外の依頼は `warn!` の上で捨てる（積まない）。`OnFileDrop2`・`OnDirectoryDrop` は許可表に 0 語。`ghost-install` が 13 語 → 21 語にした後（予定）、本仕様が 2 語足して 23 語にする。数は `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), …)` が直書きで判定している。
- **網羅台帳**: `doc/ukadoc-coverage/ledger/shiori.toml` の `OnFileDrop2`・`OnDirectoryDrop` は `status = "absent"`・`owner = ""`。

### 先進坑 `pilot-dropfiles-on-wuc-window` の結果＝go（2026-09-26 開発者判定）

一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md` の「検証結果」（完了 spec は `.kiro/specs/completed/pilot-dropfiles-on-wuc-window/`）。

1. **`WM_DROPFILES` で行ける**（`IDropTarget` への切り替えは不要）。本番と同じ様式＋`WS_EX_ACCEPTFILES` の窓で、絵の上への落とし 5 回がすべて窓手続きまで届き `DragQueryFileW` でパスが取れた。先進坑は `SetWindowSubclass` で重ねた手続きで受けた。本番は wintf の振り分け表に 1 腕足す形で足りる見込み。
2. **宣言は `WS_EX_ACCEPTFILES` のビットだけで足りる**。`DragAcceptFiles`・生成後の付け直しは要らなかった（生成直後の読み戻しで `accept_files=true`・`raw=0x200090`）。
3. **絵の外（クリック透過中）に落とすと窓には届かず背後の窓へ抜ける**。落とし先は OS が `WS_EX_TRANSPARENT` のビットで決める＝**「絵の外なら捨てる」処理は要らない**。透過の付け外しを 36 回繰り返した後も `WS_EX_ACCEPTFILES` は残った。
4. **既知の制限の候補**: 絵の縁ぎりぎりに素早く落とすと、12ms の監視が追い付く前のビットで落とし先が決まる可能性は残る（実測では縁の 7 物理 px 内側でも正しく届いた）。
5. 配送は待ち行列経由（tick の途中の同期配送は観測されず）。ふつうの権限で測った（管理者として起動したときの手当て `ChangeWindowMessageFilterEx` は対象外）。

## Desired Outcome

完了時に次が真になっている。

1. **キャラクター窓・バルーン窓の絵の上へ `.nar`／`.zip`（拡張子は大文字小文字を区別しない）を落とすと、`ghost-install` のインストールの手続きへ渡る。** メニュー・台本から入れたときと同じ手順・同じイベントになる（本仕様は依頼を作って渡すだけ）。1 回に複数の書庫を落としたら 1 つの依頼にまとめて渡す（全部入ったときの `OnInstallCompleteAll` は `ghost-install` の手続きが送る）。
2. **書庫でないファイルを落とすと `OnFileDrop2`** を GET で 1 回送る: Reference0＝落とされたファイルの絶対パス（複数は byte 値 1 区切り）・Reference1＝落とされた窓のスコープ番号（本体 0・相方 1…）・Reference2＝各ファイルの MIME タイプ（Reference0 と同じ並び・拡張子から決め、決められなければ空）。
3. **フォルダを落とすと、フォルダ 1 つにつき `OnDirectoryDrop`** を GET で 1 回送る: Reference0＝フォルダの絶対パス・Reference1＝スコープ番号。`install.txt` を持つフォルダもフォルダとして扱う（`.nar` を作らない）。
4. **混ざった投げ込みは、書庫でないファイル（`OnFileDrop2` 1 回）→ フォルダ（`OnDirectoryDrop`）→ 書庫（手続き）の順に扱う。** 書庫の手続きは時間がかかり、終わるとゴーストが替わりうるので、落とされた物の知らせは今のゴーストへ先に届ける（`ghost-install` 要件 12 の暫定の確定 12 から引き継いだ）。
5. **絵の外（透けている所）へ落としたら何もしない**（OS が背後の窓へ渡す・先進坑 3）。
6. **ゴーストが定常でないときに落とされた書庫でないファイル・フォルダは、イベントを送らずに `warn!` を 1 件残す**（汎用の通知の入口が定常以外を捨てるのと揃える）。書庫は `ghost-install` の手続きの待ち行列が定常まで待たせる。
7. **受け取った 1 件ごとに記録を残す**（件数・窓のスコープ番号・種類の内訳）。記録の無い失敗の経路を作らない（`DragQueryFileW` の失敗も `warn!`／`error!` に残す）。
8. **正典の語だけを送る**: 新しいイベント名 0 個。`OnFileDropping`（ドラッグ中＝`WM_DROPFILES` では取れない）・旧仕様の `OnFileDrop`／`OnFileDropEx`／`OnFileDropped` は送らない（0 件）。
9. **網羅台帳と文書が実物と揃う**: `shiori.toml` の `OnFileDrop2`・`OnDirectoryDrop` を実装済みへ動かし、生成物と `roadmap-draft.md` の数は生成器で作り直す（手で数を直さない）。

## Approach

`ghost-install` の brief の Approach 段 ④（`WM_DROPFILES`＋wintf の窓手続きに 1 分岐 → 拡張子で振り分け）を、先進坑の結果（宣言は 1 ビット・絵の外の処理は要らない）で縮めた形。

| 段 | 中身 | 検証 |
|---|---|---|
| ① wintf の受け口 | `window_proc/drop.rs`（新規）: `WM_DROPFILES` → `DragQueryFileW`（件数とパス）→ `DragFinish`。`window_proc/mod.rs` の振り分け表に 1 腕。窓に差す部品 `OnFileDrop`（`OnSessionEnd` と同型・パスの一覧を渡す）を `window/components.rs` に | 決定論: 偽の `WM_DROPFILES` を振り分けへ流し、部品の関数がパスの一覧で呼ばれる／部品の無い窓では既定処理へ |
| ② 宣言と装着 | `placement/spawn.rs` の `window_style` の `ex_style` に `WS_EX_ACCEPTFILES`。ゴースト窓（キャラクター・バルーン）に部品を差す（`attach_os_close_request` と同じ場所・同じ形） | 決定論: 様式のビット・装着の有無 |
| ③ 振り分け | `input_events/drop.rs`（新規）: パスの一覧を「書庫・書庫でないファイル・フォルダ」に分ける純粋な関数と、`OnFileDrop2`／`OnDirectoryDrop` の Reference の組み立て・MIME の表。書庫は `ghost-install` の依頼の型へ | 決定論: 分類・順序・Reference・MIME・定常でないときの `warn!` |
| ④ イベント | kanade 許可表に 2 語（21 → 23）・`events_change_tests.rs` の数 | 決定論: 許可表の数 |
| ⑤ 台帳と実機 | 台帳 2 行・生成物・実機確認 | 実機（下記） |

**実機確認**（`signoff.md` に記録・`ghost-install` の要件 11.11 から ⑴⑷⑸ を引き継いだ）: ⑴ ゴーストの `.nar` を絵の上へ落とす → 入る → 切り替わる ⑵ バルーン窓へ落としても同じ ⑶ 書庫でないファイルを落とす → `OnFileDrop2`（記録で Reference を確かめる）⑷ フォルダを落とす → `OnDirectoryDrop` ⑸ 絵の外へ落とす → areka の記録に受け取りが出ない ⑹ 複数の `.nar` を一度に落とす → 最後に `OnInstallCompleteAll`。**手で落とす走行の前に、何が起きないのが正しいか（絵の外・透けた余白の位置）を先に決めて記録し**、記録の水準をイベントの送出と振り分けの分かれ目が見える所まで開ける（完了 `pilot-dropfiles-on-wuc-window` の教訓）。

## Scope

- **In**:
  - wintf の `WM_DROPFILES` の受け口（腕 1 本・`DragQueryFileW`／`DragFinish`・窓に差す部品）
  - ゴースト窓の受け入れの宣言（`WS_EX_ACCEPTFILES` の 1 ビット）と部品の装着
  - 振り分け（書庫・書庫でないファイル・フォルダ）と、書庫を `ghost-install` の手続きへ渡すこと
  - `OnFileDrop2`・`OnDirectoryDrop` の送出（許可表 +2）と MIME の表
  - 網羅台帳 2 行・生成物・`roadmap-draft.md`（`[[spec]]` の行を足すなら `[briefs].count` も生成器で）
  - 決定論テストと実機確認
- **Out**:
  - インストールの手続きそのもの（`accept` の照合・利用条件・展開・`OnInstall*` の送出・`OnInstallCompleteAll` の判断・`lastinstalled`・入れた後の切替）＝`ghost-install`
  - `OnFileDropping`（ドラッグ中）・旧仕様の `OnFileDrop`／`OnFileDropEx`／`OnFileDropped`
  - `OnTextDrop`／`OnURLDropping`／`OnURLDropped`／`OnURLDropFailure`／`OnOtherObjectDropping`／`OnOtherObjectDropped`（`IDropTarget` が要る・OLE の STA は WUC の MTA と衝突しうる＝α 後。URL の投げ込みは `network-update` の Out にも明記）
  - 管理者として起動した areka への、ふつうの権限のエクスプローラからの投げ込みの手当て（`ChangeWindowMessageFilterEx`）
  - フォルダから `.nar` を作ること（`OnNarCreating`／`OnNarCreated`）
  - `dist/README.txt` の「■ .nar の入れ方」の本文（`alpha-release-signoff`）。「できないこと」の 2 行から「インストール」を消すのは `ghost-install`（メニューで入れられるようになるため）

## Boundary Candidates

- **wintf の受け口**（OS のメッセージ → パスの一覧。areka を知らない）
- **areka の宣言と装着**（どの窓が受けるか）
- **振り分け**（パスの一覧 → 書庫の依頼／`OnFileDrop2`／`OnDirectoryDrop`）＝純粋な関数に寄せる
- **`ghost-install` との界面**: 書庫のパスの一覧を 1 つの依頼で渡す口 1 つだけ

## Out of Boundary

- インストールの意味論と手続き（`ghost-install`）
- 展開の意味論と安全性（`nar-install`／`nar-install-hardening`）
- 透過の付け外しとカーソル監視（wintf 既存・先進坑で投げ込みと両立することを確認済み）

## Upstream / Downstream

- **Upstream**: `areka-P0-ghost-install`（依頼の型と手続きの入口・**実装は同 spec の着地後**。依頼が書庫を 1 本以上並べられる形は同 spec の要件 9 が約束する）／完了 `pilot-dropfiles-on-wuc-window`（go）／完了 `ghost-shell-balloon-switch`（汎用の通知の入口）／完了 `popup-menu-minimal` 等は無関係。
- **Downstream**: `alpha-release-signoff`（第三者の手順「`.nar` を窓へ落とす」）。`shell-balloon-switch` は許可表の数を本仕様の 23 から動かす（+3 で 26）。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。`ghost-install` から切り出した部分で、`ghost-install` の要件 9 が界面を定める。
- **Adjacent**:
  - `ghost-install`（直前・共有: kanade `schedule/events.rs`＋`events_change_tests.rs` の数・`ghost_session.rs`（装着を `open_ghost_windows` 側で行う場合）・台帳 `shiori.toml`・生成物・`roadmap-draft.md`）＝**直列**
  - `shell-balloon-switch`（直後・共有: 許可表の数・`ghost_session.rs`・台帳）＝**直列**
  - `network-update`（URL の投げ込みは同 spec の Out）

## Constraints

- 新規の外部依存 0（`DragQueryFileW`／`DragFinish` は `Win32_UI_Shell`＝根の `Cargo.toml` で有効済み。`WS_EX_ACCEPTFILES` は `Win32_UI_WindowsAndMessaging`）。
- 窓は UI スレッド固定。振り分けは速い（ファイルの種類の判定だけ）ので UI スレッドで行ってよい。書庫の手続きは `ghost-install` が背景で走らせる。
- 本番コードに `SendMessageW(`／`SendMessageTimeoutW(` を足さない（`session_end_sync_send_tests.rs` の `ALLOWED_SYNC_SENDS` が赤にする）。
- 決定論テスト網羅は必達。テストでログを捕捉するときは `log-capture-kit` を通す。
- 1 ファイル 1,000 行（上限が近い: `spine.rs` 993・`ghost_switch_tests.rs` 987・`actor_tests.rs` 970＝足さない）。
- 規模 **S（5〜7 タスク）**: wintf の腕と部品 1〜2・宣言と装着 1・振り分けと MIME と Reference 1〜2・許可表 0.5・台帳と生成物 0.5・実機 1。
- 要件定義は **Opus で足りる**（議題の候補は MIME の表の広さだけ＝設計で決める how・答えで作業が変わる議題は見込み 0）。

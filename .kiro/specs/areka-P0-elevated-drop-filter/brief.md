# Brief: areka-P0-elevated-drop-filter

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「管理者として動かした areka へは投げ込みが届かない」から起票した。完了 `file-drop` の要件 8.9 が既知の制限として残した件。**手当てをするかどうかは開発者の判断**（議題 1）。「手当てをしないと決めて閉じる」も、この spec の正しい終わり方に含める。

## Problem

areka を管理者として起動すると、ふつうの権限で動いているエクスプローラから `.nar` やファイルをキャラクターへ落としても、何も起きない。Windows が、権限の低いプログラムから高いプログラムの窓へ届くメッセージを止めているため（利用者の画面の保護の仕組み）。areka には何も届かないので、記録も 1 行も出ない。利用者から見ると「落としたのに無視された」になる。

## Current State

- キャラクターとバルーンの窓は、投げ込みを受ける宣言（拡張スタイル `WS_EX_ACCEPTFILES`）を持つ。付けているのは全ての窓に共通のスタイルを作る関数 `window_style`（`crates/areka/src/placement/spawn.rs`・775 行）。OS の受け入れ関数は呼ばず、宣言のビットだけ（完了 `file-drop` の先進坑の結論）。
- 落とされた物は wintf の窓の手続きが `WM_DROPFILES` で受ける（`crates/wintf/src/ecs/window_proc/drop_files.rs`）。
- 完了 `file-drop` の要件 8.9 は「管理者として起動した areka へふつうの権限から落とせないことを、既知の制限として申し送る（手当て 0）」と決めた。
- **配布物の説明には既に書いてある**: `dist/README.txt` の「既知の制限」に「管理者として起動した areka へは、ふつうの権限のエクスプローラから .nar を落としても届きません（ふつうに起動した areka へは届きます）」の 1 行がある。
- 窓の番号（HWND）が手に入る場所は同じ `spawn.rs` にある: 窓ができた瞬間（`Added<WindowHandle>`）にクリックの透過の見張りへ登録する処理 `register_ghost_windows_click_through`。手当てを入れるなら、ここと同じ形で 1 回だけ呼べる。
- 手当ての中身（覚え書き）: 窓ごとに `ChangeWindowMessageFilterEx` で `WM_DROPFILES`・`WM_COPYDATA`・`0x0049` の 3 つのメッセージを通す。3 つとも要るのか、`WM_DROPFILES` と `0x0049` だけで足りるのかは、areka の窓では確かめていない。
- `WM_COPYDATA` は areka の中で別の用途に使っている: 32 ビットの SHIORI を動かす補助プログラムとのやり取り（受けるのは `crates/shiori-host32-host/src/parent_window.rs` の専用の窓）。キャラクターの窓は `WM_COPYDATA` を処理していない。
- areka が自分の権限を調べるコードは製品には無い（先進坑の `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` に `IsUserAnAdmin` の例があるだけ）。

## Desired Outcome

次のどちらかで閉じる。

- **手当てをする場合**: 管理者として起動した areka でも、ふつうの権限のエクスプローラからの投げ込みが、ふつうに起動したときと同じに届く。通すメッセージは投げ込みに要る最小の数だけで、通す窓はキャラクターとバルーンの窓だけ。`dist/README.txt` の既知の制限の行を外す。
- **手当てをしない場合**: コードは変えない。`dist/README.txt` の今の行をそのまま残し、覚え書きから外して閉じる（タスク 0）。

## Approach

まず議題 1 を開発者に聞く。手当てをするなら、窓ができた瞬間の処理に 1 つ足し、通すメッセージの組を定数 1 か所に置く。実機で「管理者の areka へ、ふつうの権限のエクスプローラから落とす」を確かめ、要らないメッセージは外す。

## Scope

- **In**（手当てをする場合）: キャラクターとバルーンの窓へのメッセージの通し・通す組の最小化・失敗したときの記録・決定論テスト（通す組が定数どおりであること・呼ぶのは窓 1 つにつき 1 回）・実機の確かめ・`dist/README.txt` の行。
- **Out**: areka を管理者として起動させること／させないことの仕組み・OLE の投げ込み（`IDropTarget`）への乗り換え・補助プログラムとのやり取りの窓・SSTP など外からのメッセージの受け口。

## Boundary Candidates

- 窓ができた瞬間の処理（`crates/areka/src/placement/spawn.rs`）。
- 配布物の説明（`dist/README.txt`）。

## Out of Boundary

- wintf の投げ込みの受け手（`drop_files.rs`。届いた後の処理は変えない）。
- 投げ込みの仕分け（`crates/areka/src/input_events/file_drop.rs`）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（`file-drop` は完了済みで消化できない＝新しい spec）。
- **Adjacent**: `dpi-realign-remembered-chain`・`extra-character-windows`（どちらも `spawn.rs` に触る）・`dist/README.txt` の同じ節に行を足す spec（`extra-character-windows`・`update-check-options` ほか。本 spec は自分の 1 行だけ）。

## Constraints

- Windows が利用者を守るために置いている境を、areka の側から緩める変更になる。緩めるのは投げ込みに要るメッセージだけ・キャラクターとバルーンの窓だけに限る。補助プログラムとのやり取りの窓には絶対に当てない。
- 通したメッセージで届いたファイルの扱いは、ふつうの投げ込みと同じ道（インストールの確かめ・`OnFileDrop2`）を通す。管理者の権限で `.nar` を入れることになる点を要件に書く。
- 失敗（古い Windows・関数が偽を返す）は記録して続ける（記録なしの失敗にしない）。
- 実機の確かめは管理者の権限が要る＝自動の走行では回せない。開発者の机で 1 回。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: `crates/areka/src/placement/spawn.rs`（775）と新しい兄弟のテスト・`dist/README.txt` の 1 行。手当てをしない場合は 0。
- **規模**: S（2〜4 タスク）。手当てをしない場合は 0。
- **先に要るもの**: なし。同じウェーブに置けない相手: `dpi-realign-remembered-chain`・`extra-character-windows`（`spawn.rs`）。
- **優先度の区分**: C（完了した spec が既知の制限として決めた持ち越し。配布物の説明にも書いてあり、ふつうの起動では起きない）。
- **要件定義のモデル**: Fable（安全の境を緩めるかどうかは開発者の判断）。
- **議題**:
  1. **そもそも手当てをするか（開発者の判断）**。選べる形は 3 つ。⑴ しない（今の `dist/README.txt` の行のまま閉じる）。⑵ 投げ込みに要るメッセージだけを通す。⑶ 通さずに、管理者として動いていることを起動のときに記録へ 1 行出すだけにする（利用者が原因に気付ける）。areka を管理者として動かす理由は見当たらない（インストール先は areka のフォルダの中）ので、⑴ か ⑶ でも困る利用者は少ない見込み。
  2. ⑵ のとき、通す組を 3 つ（`WM_DROPFILES`・`WM_COPYDATA`・`0x0049`）にするか、`WM_COPYDATA` を外した 2 つで足りるか。実機で 2 つから試し、届かなければ 3 つにする。
  3. ⑵ のとき、いつも通すか、管理者として動いているときだけ通すか。ふつうの起動では通しても通さなくても同じなので、いつも通す方がコードは短い。
- **ukadoc の照合**: 不要（Windows の仕組みの話で、正典の主張を含まない）。

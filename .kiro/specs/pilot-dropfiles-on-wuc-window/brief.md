# Brief: pilot-dropfiles-on-wuc-window（先進坑・使い捨て）

> 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で起票。`areka-P0-ghost-install`の再測定が「`WM_DROPFILES` が WUC 合成の窓に届くかは未測定で、届かなければ設計が `IDropTarget`（OLE・STA）へ変わる」と挙げたものを、`two-tunnel.md` の規約（`pilot-` 接頭辞・1 仕様＝1 フォルダ・go 判定は開発者・go 前の本坑着手は規律違反）に沿って独立の spec にした。成果物はコードではなく**知見**（go／違う／直す＋学び）。一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md`（3 幕）。
> 対応する本坑: `areka-P0-ghost-install`（roadmap に `_Depends(confirmed): pilot-dropfiles-on-wuc-window`）。
> 本文の file:line は**起票時の実測値**（2026-09-26・main `13b72893`）。

## Problem（何が怪しくて掘るのか）

α の一周は「`.nar` を窓へ落とす」から始まる（`roadmap.md` の M2 ゴール）。棚卸⑭の仮裁定 7 は「投げ込みは `WM_DROPFILES`（`IDropTarget` は OLE の STA を要求し WUC の MTA と衝突しうる）」と決めたが、**areka のゴースト窓で `WM_DROPFILES` が実際に届くかは誰も測っていない**。怪しい点は 3 つある。

1. ゴースト窓は WUC／DComp の GPU 合成で描かれ、wintf は拡張スタイルに `WS_EX_NOREDIRECTIONBITMAP` を足す（`crates/wintf/src/runtime/window_factory.rs`）。リダイレクト面を持たない窓の上でシェルのドラッグ＆ドロップの当たり判定が効くか。
2. クリック透過は `WS_EX_TRANSPARENT` の動的トグルで成り立つ（`roadmap.md` の制約）。**透過にしている瞬間の窓には落とし物も届かない**はずで、絵の上で落としたときに透過が外れているか（α マスクの当たり判定と投げ込みの当たり判定が一致するか）。
3. 受け入れは `DragAcceptFiles` を呼ばずに拡張スタイル `WS_EX_ACCEPTFILES` を足すだけで済む見込み（`crates/areka/src/placement/spawn.rs` の `WindowStyle { style: WS_POPUP|WS_VISIBLE, ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW }` の 1 行。wintf は `WS_EX_LAYERED` を外し `WS_EX_NOREDIRECTIONBITMAP` を足す以外の bit を通す）だが、wintf の窓手続き（`crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message`）に `WM_DROPFILES` の腕が無く、既定の処理へ流れたときに何が起きるかも未確認。

届かなければ `ghost-install` の設計は `IDropTarget`（OLE の登録・STA のスレッド）へ倒れ、WUC が MTA で動く前提（記憶 areka-wuc-runs-on-mta-thread）との衝突を本坑の中で解くことになる。**本坑の要件の前に答えが要る。**

## Desired Outcome（確かめたい 1 点）

**WUC 合成・クリック透過のトグルありのゴースト窓（本番と同じ拡張スタイル）へエクスプローラから `.nar` を落としたとき、`WM_DROPFILES` が届き、`DragQueryFileW` でパスが取れるか。絵の外（透過している所）へ落としたときは背後の窓へ抜けるか。**

合否基準（go／違う／直す）:
- **go**: 絵の上で落とすと `WM_DROPFILES` が届いてパスが取れる。絵の外では届かない（背後の窓へ抜ける）。
- **直す**: 届くが条件付き（例: 透過のトグルの時機・`WS_EX_ACCEPTFILES` の付け方・`ChangeWindowMessageFilterEx` の要否）で、本坑の中で直せる。直し方を README に書く。
- **違う**: どう組んでも届かない → `IDropTarget` へ倒す（その場合の STA の置き場所の見立ても学びに書く）。

## Approach

- `crates/pilot/examples/_template/` を `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` へ写して着手。
- wintf で窓を 1 枚建て（本番と同じ `WS_POPUP`＋`WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_ACCEPTFILES`・α を持つ絵 1 枚・クリック透過の機構あり）、窓手続きの前段で `WM_DROPFILES` を拾って `DragQueryFileW` の結果をログへ出す（先進坑なので wintf 本体は改変しない。フックの形は設計で決める）。
- 観測は開発者の手（エクスプローラから落とす）とログの grep。有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS` と同じ作法）。開発機は画面の拡大率 200%＝実走中に 100% を頼むときは質問で（記憶: 先進坑 `pilot-balloon-asset-swap` の罠）。

## Scope

- **In**: 上の 1 点の実験・README 3 幕（動機・概要・検証結果）・`crates/pilot/Cargo.toml` の dev-dependencies（wintf は既にある）
- **Out**: `.nar` の展開（`areka-nar`・本坑）／`OnFileDrop2`／`OnDirectoryDrop` の送出（本坑）／ファイル選択の箱（本坑）／テキスト・URL の投げ込み（α 後）／先進坑コードの production 流用（本坑はクリーンに掘り直す）

## Boundary Candidates

- 受け入れの宣言（`WS_EX_ACCEPTFILES`）と受け取り（`WM_DROPFILES`・`DragQueryFileW`・`DragFinish`）
- 透過のトグルとの噛み合い（観測）

## Out of Boundary

- wintf 本体・areka 本体の改変（先進坑は `crates/pilot/` だけ）

## Upstream / Downstream

- **Upstream**: なし（wintf の既存の窓とクリック透過の機構の上で試す）
- **Downstream**: `areka-P0-ghost-install`（`_Depends(confirmed)`）

## Existing Spec Touchpoints

- **Extends**: なし
- **Adjacent**: `ghost-shell-balloon-switch`（同じウェーブで並走・共有 0＝触るのは `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` と、要れば `crates/pilot/Cargo.toml` の dev-dependencies だけ）

## Constraints

- 葉ノードの隔離を守る（`crates/pilot` は空の lib＋examples だけ・他クレートから依存させない）。
- 規模 **XS〜S（1〜3 タスク）**。要件は Opus で足りる（答えは実験が出す）。go 判定は開発者。

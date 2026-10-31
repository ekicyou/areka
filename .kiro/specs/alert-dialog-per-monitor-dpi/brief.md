# Brief: alert-dialog-per-monitor-dpi

> **起票 2026-10-31**（`install-before-boot` の完了時の棚卸・`/kiro-discovery`）。出所: 同 spec のタスク 7.2（起動の前の利用条件の画面の DPI を実機で 1 回確かめた）で気づいたが、確かめの範囲の外で見分けていない 2 件。証跡: 同 spec の `tasks.md` の Implementation Notes 7.2（`completed/install-before-boot/`）と、そのときの記録（worktree `emo2-specimen-freshness-check-b831f3` の `target\dpi72\stdout.log`・作業木と一緒に消える見込み）。
>
> - **優先度の区分**: B（バグの疑い・確かめていない）

## Problem

拡大率の違うモニタを 2 枚つないだ利用者に、次の 2 つが起きている疑いがある。

- **件 1**: 利用条件・警告・失敗の画面がにじむ。areka の画面（`crates/areka/src/alert.rs` の `raise`・`ask_yes_no`・`warn_ok`＝`MessageBoxW`）は、プロセスがモニタごとの DPI（`PER_MONITOR_AWARE_V2`）でも、窓としては system aware を返した（2026-10-31 実測 `GetWindowDpiAwarenessContext` の awareness=1）。主のモニタ（200%＝system DPI と同じ）ではにじまなかったが、拡大率の違う副のモニタ（150%）に出ると OS が縮めて描き、文字がにじむ見込み。
- **件 2**: ゴーストの窓を 200% のモニタから 150% のモニタへ動かした後、バルーンの窓は枠が見えず矢印（▲）だけが見えた。`WM_DPICHANGED`（dpi 144）は届き、窓は 652×338 → 489×254 に縮んだ。会話が終わって中身が空になっただけなのか、拡大率の切替でバルーンの描き直しが欠けたのかは見分けていない。

## Current State

- `alert.rs` は `MessageBoxW` を直に呼ぶ（`unsafe` はこの 3 か所に閉じる）。画面は起動の前（`install-before-boot` の `BootPorts`）・窓口（ドロップしたときの利用条件）・起動の失敗の告知で使う。
- DPI の扱いは `wintf::ensure_process_dpi_awareness()`（プロセスで 1 回・`PER_MONITOR_AWARE_V2`）。2026-10-31 の実機で設定は 1 回・失敗 0 件。
- 件 2 の窓の追従そのもの（大きさ 0.75 倍・`WM_DPICHANGED` の受け取り）は今どおり動いている。描かれた中身だけが疑わしい。
- 近い spec: `dpi-transition-two-tick-bounce`（拡大率の切替で位置と寸法が 2 ティックに分かれる跳ね）＝件 2 とは症状が違う（跳ねでなく中身）。

## Desired Outcome

1. 件 1: どのモニタに出ても、areka の画面がそのモニタの拡大率で描かれる（にじまない）。
2. 件 2: まず再現の手順と見分けを決める（会話の途中で移したときと、会話が終わってから移したときを分けて撮る）。描き直しの欠けなら直し、空のバルーンが矢印だけを見せるのが今の作りどおりなら、それが SSP と同じかを ukadoc と SSP の MCP で確かめて決める。

## Approach

件 1 の直し方は複数あり、要件の段で裁定する。

- 案 A: 画面を出す間だけ `SetThreadDpiAwarenessContext` でスレッドの DPI の扱いを `PER_MONITOR_AWARE_V2` に囲んでから `MessageBoxW` を呼ぶ（差分が小さい。`MessageBoxW` がそれで per-monitor になるかを先に実機で確かめる）。
- 案 B: `TaskDialogIndirect`（comctl32 v6）へ替える（per-monitor の扱いは OS 任せ・マニフェストの依存が増える）。
- 案 C: wintf の窓で自前の画面を作る（見た目も DPI も自分で持つ・規模が大きい）。

件 2 は先に見分ける（要件の段の前に、実機で 2 通りを撮る）。

## Scope
- **In**: `alert.rs` の 3 つの画面の DPI の扱い（件 1）。モニタを移した後のバルーンの描き直しの見分けと、欠けなら直し（件 2）。それぞれの実機の確かめ（2 枚のモニタ・排他テストの机）。
- **Out**: 画面の文言・出す時の決まり（`install-before-boot`・`startup-install-option` の持ち物）。拡大率の切替の跳ね（`dpi-transition-two-tick-bounce`）。

## Boundary Candidates
- 告知の画面（`alert.rs`）の DPI の扱い
- バルーンの拡大率の切替での描き直し（`emo2_boot/frame/dpi.rs` の族・emo の再表示）

## Out of Boundary
- 画面を出す時と文言、抑止（`AREKA_NO_ALERT`・`AREKA_APP_SMOKE_EXIT_MS`）の決まり
- 窓の位置と寸法の追従（今どおり動いている）

## Upstream / Downstream
- **Upstream**: `wintf::ensure_process_dpi_awareness`（✅ `install-before-boot`）・`alert.rs`
- **Downstream**: 告知の画面を使う spec（`install-before-boot`・窓口のインストール・起動の失敗の告知）

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `dpi-transition-two-tick-bounce`（件 2 の触る場所と近い＝着手の前に重なりを測る）

## Constraints
- 起票の条件（完了時の棚卸の例外）: 件 1 は直し方が複数で裁定が要り、確かめに 2 枚のモニタの実機の走行が要る。件 2 は原因が分かっておらず見分けに実機の走行が要る。
- GPU・実機の確かめは実の GPU・実のモニタで（WARP などへ替えない）。画面の操作は排他テストの机で順を取る。
- 規模の見立て: S〜M（件 1 は案 A なら 3〜5・件 2 は見分け次第）。要件定義のモデルは Opus。

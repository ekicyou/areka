# Brief: balloon-dpi-switch-redraw

> 起票: 2026-11-01 `alert-dialog-per-monitor-dpi` の完了の棚卸（`/kiro-discovery`・同 spec の要件 5.2＝件 2 を持ち主の決まった起票として渡す）。
> 起票の条件（完了の棚卸の手順 3）: 確かめに 2 枚のモニタの実機の走行を何度も回す必要があり、描き直しの欠けか作りどおりかで直し方と裁定が分かれる。

## Problem

拡大率の違うモニタを 2 枚つないだ利用者。ゴーストの窓を 200% のモニタから 150% のモニタへ動かした後、バルーンの窓は枠が見えず矢印（▲）だけが見えた（2026-10-31 `install-before-boot` タスク 7.2 の実機）。バルーンが描き直されていないなら、移した後のトークが読めない見込み。

## Current State

- 2026-10-31 の実機: `WM_DPICHANGED`（dpi 144）は届き、バルーンの窓は 652×338 → 489×254（0.75 倍）に縮んだ。窓の追従そのものは今どおり動いている。描かれた中身だけが疑わしい。
- 会話が終わって中身が空になっただけなのか、拡大率の切替でバルーンの描き直しが欠けたのかは見分けていない（撮り分けていない）。
- 2026-11-01 `alert-dialog-per-monitor-dpi` 3.1 の実機では逆向き（150% → 200%）に本体とバルーンを移し、窓の大きさは 489×254 → 652×338 に追った。中身は撮っていない。
- 近い spec: `dpi-transition-two-tick-bounce`（拡大率の切替で位置と寸法が 2 ティックに分かれる跳ね）＝症状が違う（跳ねでなく中身）。

## Desired Outcome

1. 再現の手順と見分けを決める: 会話の途中（バルーンに字が出ている間）で移したときと、会話が終わってから移したときを分けて撮る。
2. 拡大率の切替でバルーンの描き直しが欠けているなら直し、移した後もバルーンの枠と字がそのモニタの拡大率で描かれる。
3. 空のバルーンが矢印だけを見せるのが今の作りどおりなら、それが SSP と同じかを ukadoc と SSP の MCP で確かめ、同じなら記録して閉じ、違えば直す。

## Approach

先に見分ける（要件の段の前に、2 枚のモニタの実機で 2 通りを撮る）。見分けの結果で直し方が分かれるので、直し方は要件の段で決める。

## Scope
- **In**: モニタを移した後のバルーンの描き直しの見分けと、欠けなら直し。実機の確かめ（2 枚のモニタ・排他テストの机）。
- **Out**: 告知の画面の DPI（`alert-dialog-per-monitor-dpi` で済み）。拡大率の切替の跳ね（`dpi-transition-two-tick-bounce`）。シェルとバルーンの倍率の設定（`shell-balloon-scale-setting`）。

## Boundary Candidates
- バルーンの拡大率の切替での描き直し（`emo2_boot/frame/dpi.rs` の族・emo の再表示）
- 空のバルーンの見せ方（矢印だけを見せるか）

## Out of Boundary
- バルーンの窓の位置と寸法の追従（今どおり動いている）
- 告知の画面

## Upstream / Downstream
- **Upstream**: DPI 追従の基本設計（`WM_DPICHANGED` の受け取り・`emo2_boot/frame/dpi.rs`）
- **Downstream**: なし

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `dpi-transition-two-tick-bounce`・`shell-balloon-scale-setting`

## Constraints
- 実機の確かめは実の GPU と拡大率の違う実のモニタ 2 枚で、排他テストの机を取ってから行う。
- 優先度の区分: B（描き直しの欠けの疑い・未見分け）。

# Brief: areka-P0-wintf-tooltip

> 2026-10-05 `/kiro-discovery` で起票。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。
> 開発者指示（同日）「wintf 向けの話があれば spec 分割してベース実装 → 本議題の実装、って感じにしてください。tooltip は色々汎用的にできた方が良いと思いますので」。

## Problem

- **利用者**: マウスを止めた所に短い説明を出したい areka の各機能（バルーンのリンク＝`balloon-link-hover`、キャラクター窓の当たり判定＝`shell-tooltip`、今後のメニューや設定画面）。
- 今の wintf には、マウスが**止まったこと**を見分ける仕組みも、ツールチップを出す仕組みも無い。各機能が自前で作ると、待ち時間・消えるきっかけ・DPI・重なり順がばらばらになる。

## Current State

- wintf にあるのは、マウスが窓から出たことの追跡だけ（`crates/wintf/src/ecs/window_proc/mouse_move.rs` の `TrackMouseEvent(TME_LEAVE)`・`WM_MOUSELEAVE`、状態は `crates/wintf/src/ecs/pointer/types/mod.rs` の追跡の欄）。`dwHoverTime` は 0 で、`WM_MOUSEHOVER` は使っていない。
- `tooltip`・`TOOLTIPS_CLASS` は wintf にも areka にも無い（areka-sylphya の語の表に SHIORI の `tooltip`・`balloon_tooltip` の名前があるだけ）。
- areka のバルーンは選択肢の行の光らせ用に自前でホバーを追っている（`crates/areka/src/input_events/balloon.rs` の `hover`）。これは「どの行の上か」で、「止まったか」ではない。

## Desired Outcome

wintf の窓ならどれでも使える土台。

- **止まったことの検出**: 窓（の中の範囲）の上でマウスが OS の待ち時間（`SPI_GETMOUSEHOVERTIME`）だけ止まったら、知らせが出る。動いた・窓から出た・押した・窓が隠れたら取り消し。
- **ツールチップを出す・消す**: 文字（複数行可）を、マウスの近くに、画面の端からはみ出さず、透過の窓やいつも手前の窓より手前に出す。DPI に追従。
- **使い方は 2 通り**:
  - 静的: 「この範囲にマウスが止まったら、この文字」を登録しておく（キャラクター窓の tooltipブレスのような使い方）。
  - 動的: 止まった知らせを受けてから文字を決めて出す。文字が後から決まる場合（SHIORI に尋ねて答えを待つ `balloon_tooltip`）にも使えるよう、「止まった → 文字が届いたら出す・届く前に動いたら出さない」を土台が面倒みる。
- 記録: 出した・消した・取り消したを `debug` で残す（判定の分岐は `trace` まで開けられる）。

## Approach

- 窓は Windows 標準のツールチップ（comctl32 の `TOOLTIPS_CLASS`・`TTF_TRACK`）を使うか、wintf の自前の窓で描くかを設計で決める（areka の窓は透過の layered 窓なので、標準のツールチップが重なり順と DPI で素直に動くかを試す）。開発者の方針「OS の受け口を最大限活用」に沿い、まず標準を当たる。
- 止まったことの検出は `TrackMouseEvent(TME_HOVER)` を使うか、wintf のポインタの状態から時計で判定するかを設計で決める。決定論テストのため、時計は差し替えられる形にする。

## Scope

- **In**: 止まったことの検出・ツールチップの表示と消去・静的な登録と動的な表示の 2 通りの口・DPI・画面の端・決定論テスト・wintf の公開の口の文書。
- **Out**:
  - バルーンのリンクの説明（`balloon-link-hover`）。
  - キャラクター窓の tooltipブレス（`shell-tooltip`）。
  - ツールチップの見た目の着せ替え（バルーンの絵で描くなど）。要望が出たら起票。

## Boundary Candidates

- 止まったことの判定（時計と位置の純粋な状態機械）。
- ツールチップの窓（OS の境界）。

## Out of Boundary

- areka 側の配線（どの範囲に何を出すか）。

## Upstream / Downstream

- **Upstream**: wintf のポインタの追跡（既存）。
- **Downstream**: `balloon-link-hover`・`shell-tooltip`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: なし（wintf だけを触り、どの直列の列にも入らない）。

## Constraints

- wintf は crates.io に公開している（`crates-io-publish`）＝公開の口は版の上げ方に気を配る。
- wintf の中だけで閉じ、areka の型に依存しない。
- 段: 優先（バルーン関係の土台）。規模の見込み M（8〜12）。

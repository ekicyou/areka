# Brief: areka-P0-link-context-copy

> 2026-10-05 `/kiro-discovery` で起票（開発者「リンクを右クリックしたらリンクに登録されている文字列（URL とか）をコピペできたりしますか？」）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。
> **areka 独自の拡張**: ukadoc に、選択肢やアンカーの右クリック・クリップボードを扱うタグは無い（2026-10-05 に ukadoc 最新版の `list_sakura_script.html`・`list_shiori_event.html` の本文を grep して 0 件・選択肢やアンカーのイベントにボタンの種類の Reference も無い）。
> 開発者裁定（同日）:
> - **議題 1（何をコピーするか）＝3 段すべて**: ⑴ `script:` の中に開く系のタグがあればその行き先（`\j[X]`・`\![open,○○,X]` の X）→ ⑵ ID や引数に `http://`・`https://`・`file:///`・`mailto:` で始まる値があればその値 → ⑶ どちらも無ければ表示されている文字。`OnXxx` などの内部の ID はコピーしない。
> - **議題 2（どう見せるか）＝案 A**: 右クリックした場所に小さなメニューを出し、選ぶとコピー。メニューの外を押せば何も起きない。

## Problem

- **利用者**: バルーンのリンクの行き先（URL・パス）を、開かずに控えたい・人に送りたい人。ブラウザの「リンクのアドレスをコピー」と同じことをしたい。
- 今の areka ではバルーンの右クリックは何も起きない。

## Current State

- バルーンの押下は `crates/areka/src/input_events/balloon_pressed.rs` の `on_balloon_pointer_pressed` が**左だけ**受ける（`if !state.left_down { return false; }`）。シェルの中の箱は `shell_box_handler.rs` も左だけ（`judge_box_click`）。
- 選択肢の当たりは `input_events/balloon.rs` の `hit_choice_row`。
- メニューの仕組みは `crates/areka/src/menu/win32.rs`（計画から `HMENU` を組み、`TrackPopupMenuEx` で出して選ばれた識別子を返す）。今はキャラクター窓の右ボタンの離しからだけ開く（`menu/trigger.rs`）。
- クリップボードを扱うコードは 0 件（`OpenClipboard`・`SetClipboardData` が `crates/` に無い）。

## Desired Outcome

右クリックした選択肢・アンカー・範囲の選択肢から、上の 3 段でコピーする文字列を決め、メニューを出す。

- 例: `\q[公式サイト,script:\j[https://example.com/]]` → `コピー: https://example.com/`
- 例: `\_a[OnURL,https://example.com/]example.com\_a` → `コピー: https://example.com/`（2 段目）
- 例: `\q[今日の天気,OnWeather,tokyo]` → `コピー: 今日の天気`（3 段目）
- 例: `\q[資料,script:\j[https://a.example/]\![open,file,manual.pdf]]` → 行き先 1 つにつき 1 項目＝`コピー: https://a.example/` と `コピー: manual.pdf`
- 長い行き先はメニューの上で 40 字ほどで切って「…」。コピーするのは全文。

あわせて次のようにする。

- 右クリックは選択肢を**選んだことにしない**。選択肢のタイムアウトの残りも変えない。
- コピーする文字列は右クリックした瞬間に決める。メニューを開いている間にバルーンが閉じても、選べばコピーできる。
- クリップボードへは Unicode の文字（`CF_UNICODETEXT`）で書く。書けなかったら `error!` で残す。
- 普通のバルーンとシェルの中の箱の両方。

## Approach

- 3 段の決め方は純粋な関数にする。1 段目は `open-external-tags` の「行き先を取り出す関数」を使う。2 段目の頭の文字の判定もここ。`balloon-link-hover` が 1・2 段目を使い回す。
- メニューは `menu/win32.rs` の仕組みを使い回し、クリップボードへの書き込みは数行なのでその隣に置く（wintf には出さない）。
- 右ボタンの押下と離しを、バルーンと箱の押下の受け口へ足す。

## Scope

- **In**: 3 段の決め方・右クリックの受け口・メニュー・クリップボード・選択肢・アンカー・`\__q` の 3 種・箱の中・決定論テスト（メニューとクリップボードは偽の境界）・areka 独自の拡張としての文書（`doc/COMPAT_ARCHITECTURE.md` など置き場は設計で決める）。
- **Out**:
  - キャラクター窓の右クリックメニュー（既存）。
  - 文字の選択・範囲のコピー（バルーンの本文を選んでコピーする機能）。
  - ゴーストへ右クリックを知らせる新しいイベント（作らない）。

## Boundary Candidates

- コピーする文字列の決め方（純粋）。
- 右クリックの受け口とメニュー・クリップボード（UI と OS の境界）。

## Out of Boundary

- 行き先を取り出す関数そのもの（`open-external-tags`）。
- ホバーの説明（`balloon-link-hover`）。

## Upstream / Downstream

- **Upstream**: `open-external-tags`（行き先を取り出す関数）・`anchor-tag-canon`（アンカー）・`range-choice-tag`（範囲の選択肢）。
- **Downstream**: `balloon-link-hover`（1・2 段目を使い回す）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `popup-menu-residue`（`menu/` を触る次の spec へ相乗りが安い＝本 spec が取ってよいかを要件の段で決める）・`choice-interact`（完了済み・左の押下の配線）。

## Constraints

- 文字とバルーンの列（`input_events/` のバルーンと箱）。
- `crates/areka/src/readme/destination.rs` の `link_destinations` に付けた `#[cfg_attr(not(test), allow(dead_code))]` は、本 spec が本番の呼び手になったら外す（`open-external-tags` の完了時の申し送り・2026-10-06）。
- 段: 優先（バルーン関係）。規模の見込み S〜M（6〜10）。

# Brief: balloon-communicatebox

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。版は **v0.1.0**（ゴールの 2 つ目「ぱすたが SSP と同じに動く」）。

## Problem

- **ゴーストの利用者とバルーンの作者（ぱすた）**: ぱすたの縦書きのバルーン `pasta_tategaki` は、話しかける入力欄（CommunicateBox）の位置と大きさ `communicatebox.x,8`・`communicatebox.y,20`・`communicatebox.width,304`・`communicatebox.height,24` を持つ。areka には CommunicateBox そのものが無く、鍵も読まない。利用者がゴーストに文字で話しかけられない。

## Current State

- **証拠**: `verification.md` 10 節の拾った一覧（縦書きのバルーンの descript の `communicatebox.x/y/width/height`）と突き合わせの表の「バルーン `communicatebox.*`」の欄（absent・持ち主なし・本 spec の範囲の外）。書庫の中身は `target\pasta-verify\root\ghost\pasta-in-windows\pasta_tategaki\descript.txt` の 74〜77 行目。ぱすたの辞書に `OnCommunicate` の受け口は無い（10 節のイベントの一覧に無い）＝ぱすたが答えなくても、入力欄が正しい所に出て、送った・閉じたイベントが届くことが SSP と同じに動く条件。
- **SSP の基準**: ぱすたの測り（`ssp-measurement.md`）では CommunicateBox を開いていない＝見えた答えは無い。基準は ukadoc と、要件の段で SSP の MCP で測る値。
  - バルーンの descript: `communicatebox.x`「入力可能域始点のX座標。」・`.y`・`.width`「入力可能域の横幅。」・`.height`。ほかに `communicatebox.font.name`・`.font.color.r/g/b`（2.5.36）・`.background.color.r/g/b`（2.6.39）・`.placeholder.color.r/g/b`（2.9.10）。
  - `\![open,communicatebox]`「CommunicateBoxを開く。入力決定後はOnCommunicateが発生する。閉じるとOnCommunicateInputCancelが発生する。」（SSP は既定の文字 `\![open,communicatebox,テキスト]`・`--text=`・`--placeholder=` などのオプション）。`\![close,communicatebox]`「CommunicateBoxを閉じる。OnCommunicateInputCancelを発生させないので注意。」
  - `OnCommunicate`: Reference0＝箱からは `user`・Reference1＝入力の中身。`OnCommunicateInputCancel`: Reference1＝閉じられた場合 `cancel`。
  - 利用者が箱を開く操作（右クリックメニューの項目か）は ukadoc で確かめられなかった＝SSP で測る。
- **areka**: 文字を入力する欄が 1 つも無い（`inputbox`・`teachbox` も無い）。kanade の状態の文字列に `opening(communicate/…)` の語の表だけがある（`crates/areka-kanade/src/status.rs`）。網羅台帳: `assets.toml` の `communicatebox.*` 11 行・`sakura-script.toml` の `\![open,communicatebox]`・`\![close,communicatebox]`・`shiori.toml` の `OnCommunicate`・`OnCommunicateInputCancel` は absent・持ち主なし（`shiori.toml` の注記に候補の名前 `areka-P0-communicate-events` があるが起票していない）。
- **隣の spec**: `inputbox-user-input`（v0.3.0）が `\![open,inputbox]` と `OnUserInput` を持ち、`communicatebox`・`teachbox` を「別の系統」として Out にしている。`balloon-font-file` が `communicatebox.font.name` のフォントファイルの読み手を持つ（表示は各担当）。

## Desired Outcome

- 利用者の操作（SSP と同じ入口）と `\![open,communicatebox]` で、今のバルーンの上の `communicatebox.x/y/width/height` の所に入力欄が開く。文字を入れて決めると `OnCommunicate`（Reference0 `user`・Reference1 入力の中身）、閉じると `OnCommunicateInputCancel`（Reference1 `cancel`）が届く。`\![close,communicatebox]` はイベントを送らずに閉じる。
- 縦書きのバルーン（ぱすた）でも、SSP と同じ所に同じ向きで出る。
- 入力欄は日本語の入力（IME）を受ける。

## Approach

- 入力欄は areka で初めての文字の入力の部品＝作りを要件の段で決める（Windows の編集の窓を子として置くか・wintf で描いて IME を受けるか）。`inputbox-user-input` が後から同じ部品を使えるようにする。
- 開く・閉じるの命令は受け手の表に 1 行（`open`・`close` の第 1 引数 `communicatebox`）。イベントは kanade の許可の表に 2 語。

## Scope

- **In**: `communicatebox.*` の鍵の読み取り（位置・大きさ・字の色・地の色・案内の字の色・字の名前）・入力欄の表示と入力（IME）・`\![open,communicatebox]`（既定の文字と SSP の主なオプションは要件の段で範囲を決める）・`\![close,communicatebox]`・`OnCommunicate`・`OnCommunicateInputCancel`・利用者が開く入口（SSP で測ってから）・決定論のテスト・ぱすたの実機の確かめ（`ghost-inner-balloon` の着地の後）・網羅台帳の行。
- **Out**: `\![open,inputbox]` などほかの入力欄と `OnUserInput`（`inputbox-user-input`）・`teachbox`・ほかのゴーストからの `OnCommunicate`（SSTP の Communicate・複数ゴースト）・`sstp.allowcommunicate`・フォントファイルの読み手（`balloon-font-file`）。

## Boundary Candidates

- 入力欄の部品（新しい窓か wintf の部品・IME）
- 命令の受け手とイベント（`emo2_boot` の受け手の表・kanade の許可の表）
- バルーンの鍵の読み手（areka-parsers）

## Out of Boundary

- バルーンの面と寿命（完了 `balloon-lifecycle-events`）
- 右クリックメニューの組み立てそのもの（入口に項目を足すなら `popup-menu-shell-look`・`popup-menu-residue` と調整）

## Upstream / Downstream

- **Upstream**: なし（働きの上で）。実機でぱすたの縦書きのバルーンを見るには `ghost-inner-balloon`（v0.1.0・C10）の着地が要る。
- **Downstream**: `inputbox-user-input`（v0.3.0・同じ入力欄の部品を使う）・`mcp-user-response`（利用者の入力をエージェントへ返す）・複数ゴーストの `OnCommunicate`（`multi-ghost-runtime` の後）。

## Existing Spec Touchpoints

- **Extends**: `inputbox-user-input`（`communicatebox` を本 spec が持つ・入力欄の部品は本 spec が先に作る＝同 brief に 1 行）。
- **Adjacent**: `balloon-font-file`（`communicatebox.font.name`）・`popup-menu-shell-look`（入口をメニューに足すなら `menu/mod.rs`）・`balloon-scroll-arrows`（同じバルーンの鍵の読み手の新しいファイル）。

## Constraints

- 入力の部品は areka で初めて＝IME とフォーカス（ゴーストの窓は普段フォーカスを取らない）の扱いを先に確かめる。
- 受け手の表 `consumer_ledger.rs` に行を足す spec は 1 本ずつ。
- 決定論のテストは必達・実機の確かめは排他テストの机を持ってから。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-parsers/src/balloon/`（鍵の新しいファイル）・入力欄の部品（新しいファイル・`crates/areka/src/` か wintf）・`crates/areka/src/emo2_boot/consumer_ledger.rs`＋受け手の新しいファイル・kanade の `schedule/events.rs`（許可の表に 2 語）・入口をメニューに足すなら `crates/areka/src/menu/`・網羅台帳 `assets.toml`・`sakura-script.toml`・`shiori.toml`。
- 規模: M〜L（12〜18）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。段: 優先（バルーン）。
- 要件定義のモデル: Fable（areka で初めての文字の入力の部品＝先例の無い境界・IME とフォーカス）。
- 議題（答えで作業が変わるものだけ）: ⑴ 入力欄の作り（Windows の編集の窓か wintf の部品か）。⑵ SSP の MCP で測る: 利用者が開く入口・縦書きのバルーンでの入力欄の向き・既定の大きさ（鍵の既定は ukadoc で「不明」）。⑶ SSP のオプション（`--text=`・`--placeholder=` ほか）をどこまで入れるか。

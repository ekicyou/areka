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

- **Upstream**: なし（働きの上で）。実機でぱすたの縦書きのバルーンを見るには: `ghost-inner-balloon` は完了（main #364）。ぱすたの縦書きのバルーンは、ぱすたの書庫の直し（ghost_dev の `c32229e`・未リリース）で入る。それまでは手順書 `doc/pasta-real-run.md` の入れ方で入れる。
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

## 2026-10-23 棚卸㉙の追記
- **台帳の注記を直すのは本 spec のタスク**: 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `Status [SSP拡張]` の項目（16142 行目）の注記（16163〜16165 行目）は、状態 `opening` の宛先に、一度も起票していない候補 `areka-P0-communicate-events` を挙げる。本 spec が `shiori.toml` の行（`OnCommunicate`・`OnCommunicateInputCancel`）を直すタスクで、この注記も一緒に直す（communicate の `opening` の宛先は本 spec の名前へ。並んで挙がる候補 `areka-P0-inputbox-dialog` は、今の `inputbox-user-input` へ）。
- 状態の語 `opening(communicate)` を本 spec では立てないと決めたときも、注記の宛先は本 spec のままにして、決めた理由を注記に書く。
- 直すのは注記の文だけで、`Status` の項目の持ち主の欄は変えない。
- 上の再測定の「見つけた穴」の 3 つ目（16165 行の注記）を、作業の約束にした。台帳はこの棚卸では直していない。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: ✅ `mouse-click-wheel-events`（10-24）はマウスのイベント（押す・離す・クリック・ホイール・出入り）だけを足し、キーボードの文字の入力・日本語の入力（IME）・フォーカスには触れていない。wintf に在るキーの扱いは `crates/wintf/src/ecs/window_proc/keyboard.rs` の `WM_KEYDOWN`（窓に差した `OnKeyDown` の関数へ渡す近道と、ドラッグの取り消し）だけで、文字（`WM_CHAR`）・IME（`WM_IME_*`）・入力欄の部品は今も無い。kanade の送ってよいイベントの表 `crates/areka-kanade/src/schedule/events.rs` は 913 行（上限まで 87・マウスの判定は兄弟の `mouse_events.rs` へ出た）。受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` は 504 行・登記 38 件（✅ `descript-window-keys` が 2 行足した）。`crates/areka/src/menu/mod.rs`（386）は ✅ `readonly-mode` が触った。
- **触るファイル**（行数）: バルーンの読み手 `crates/areka-parsers/src/balloon/{mod.rs 26, model.rs 774, parse.rs 252}`＋鍵の新しいファイル・入力欄の部品（新しいファイル。wintf に置くなら wintf の窓の作りにも）・`consumer_ledger.rs`（504・`(open,communicatebox)`・`(close,communicatebox)` の 2 行と担当の種類・手書きの数 38→40・説明の一覧）と見本 `consumer_ledger_agreement_tests.rs`（406）＋受け手の新しいファイル（`emo2_boot/mod.rs`・`runtime_boot.rs` に 1 行ずつ）・`schedule/events.rs`（913・2 語）・開いている間を状態として出すなら `crates/areka-kanade/src/status.rs`（539）・入口をメニューに足すなら `menu/mod.rs`・台帳 `assets.toml` の 11 行・`sakura-script.toml` の 2 行・`shiori.toml` の 2 行と注記・`doc/ukadoc-coverage/roadmap-draft.md` の数の行。IME を Windows の API で直に扱うなら、ワークスペースの `Cargo.toml` の `windows` の機能の一覧（85 行目の近く・今は `Win32_UI_Input_KeyboardAndMouse` だけで `Win32_UI_Input_Ime` が無い）に 1 行。
- **規模**: 14〜19 タスク（変わらず）。上限 20 の内。
- **先に要るもの**: 働きの上ではなし。受け手の表に行を足す `stay-on-top-directives`・`script-small-tags`・`shell-scale-linkage`・`mcp-strict-errors`（案 2）と 1 本ずつ（行は避けられない＝`open`・`close` は第 2 引数ごとの登記で、`communicatebox` の組が無い）。`schedule/events.rs` は `shell-scale-linkage`（ほかに `balloon-break-position`・`balloon-link-hover`・`ukadoc-catalog-refresh`）。バルーンの読み手は `balloon-scroll-arrows`（と印を描くときの `script-small-tags`）。メニューに入口を足すなら `popup-menu-shell-look`・`popup-menu-residue`。`Cargo.lock` は動かない見込み（機能を 1 つ足しても版は変わらない・着手のときに確かめる）。emo-text の席は使わない。人の待ち: SSP の MCP で開く入口・縦書きでの向き・既定の大きさを測る（開発者の許し）・日本語を打つ確かめは人の手の実機。静かな机は要らない。
- **優先度の区分**: A（開発者「0.1.0までに解決しない場合、独立して起票せよ」・v0.1.0 のゴール「ぱすたが SSP と同じに動く」）。
- **要件定義のモデル**: Fable のまま。当たる決まり＝「先例の無い境目・約束」: 普段フォーカスを取らないゴーストの窓の上で、キーボードのフォーカスと日本語の入力を受ける部品は、`mouse-click-wheel-events` の着地の後も areka と wintf に先例が無い（`OnKeyDown` はキーを押した知らせだけで、文字も IME もフォーカスも扱わない）。置き場（Windows の編集の窓か wintf の部品か）で wintf と areka の間に新しい約束ができる。
- **分割の案**: 切らない。要件の段で 20 を超えたら、「入力欄の部品と開く・閉じる・2 つのイベント（位置と大きさの 4 つの鍵）」と「見た目の鍵の残り（字の名前・字の色・地の色・案内の字の色）と SSP の開く命令のオプション」に切る。前が先。
- **見つけた穴・古くなった記述**:
  - 本文 Scope の「ぱすたの実機の確かめ（`ghost-inner-balloon` の着地の後）」は古い（着地済み）。
  - 状態の語 `opening(communicate)`（`status.rs` の `OpeningKind::Communicate`）は誰も立てていない＝入力欄を開いている間に状態として出すかを要件の段で決める（MCP の `get_status` の答えが変わる）。
  - 棚卸㉙の追記の台帳の注記は、今 `shiori.toml` 16211 行目（候補 `areka-P0-communicate-events`）に動いた（追記の 16142・16163〜16165 行目は古い）。ukadoc の `communicatebox.placeholder.color.r/g/b`（2.9.10）は台帳に行が無い＝範囲に入れるなら台帳の写しの更新（`ukadoc-catalog-refresh`）が先か、行の無いまま作るかを決める。

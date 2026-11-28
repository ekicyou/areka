# Brief: play-open-directive

> 決まりの正本: `doc/ideas/play-layer-folders-2026-11-28.md` の 2.8（劇を始める入口 ④）。

## Problem

- **ゴーストの作者**: デスクトップのゴーストの台詞から、劇を開きたい（例: ゴーストが「新しいお話があるよ」と言って劇を開く）。`play-package` の入口（メニュー・インストールの直後・起動の引数）は、どれも利用者の操作から始まり、台本からは開けない。

## Current State

- 劇の入れ物と入口 3 つは `play-package` が作る。
- ghost 層は play 層を引かない向きに置く（`doc/ideas/play-layer-folders-2026-11-28.md` の 1 章）。

## Desired Outcome

- ゴーストの台本の areka だけの命令（仮 `\![open,play,識別子]`。名前と形は要件の段で決める）で、識別子の劇を開ける。劇は `ghost-material-metadata` の探す順で解く。
- 命令は本体の側で受ける。ghost 層は play 層を引かない。
- どのゴーストがどの劇を開いてよいか（許諾）を決め、許されない時は開かずに記録に残す。
- 開く劇がもう開いていれば、その窓を前に出す（`play-package` の決まり）。見つからなければ開かずに記録に残す。

## Approach

台本の受け口で命令を拾い、本体の側の劇を開く口（`play-package` の入口と同じ道）へ渡す。

## Scope

- **In**: 命令の読み・受け口・本体の側へ渡す道・許諾の決まりと記録・決定論テスト。
- **Out**: 劇の入れ物と入口 3 つ（`play-package`）・全画面の許可（`fullscreen-consent`）。

## Boundary Candidates

- 命令の受け口（ghost 層）と、劇を開く口（本体の側）。
- 許諾の判断。

## Out of Boundary

- 劇の起動そのもの。

## Upstream / Downstream

- **Upstream**: `play-package`。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `ghost-call`（ゴーストを呼ぶ命令）・`consumer_ledger.rs`（命令の受け口の表）。

## Constraints

- 正典のタグの意味を変えない（areka だけの命令として足す）。
- 区分 A・ノベルゲーム基盤の段・入口 ①〜③ の後。

# Brief: areka-P0-impl-watch-skills

> 起票: 2026-10-10（`areka-P0-impl-watch` の完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-impl-watch/brief.md` の「範囲の外」と Downstream（「スキルの書き替えは、アプリが動いてからの続きの spec にする」）、および同 `tasks.md` の Implementation Notes（5.1 の裁定待ち 9・6.2・6.3・7.4 の「知っていて開けてある残り」）。区分 C（開発の道具・バグでない持ち越し）。

## Problem

- **開発者と、並走する Claude のセッション**: 机の貸し出し（マージの机・負荷テストの机・停止要請と再開）を行うコンソールアプリ `areka-impl-watch` は出来たが、スキルはまだそれを呼ばない。`kiro-impl` と `kiro-complete` の机の段は、調停役のセッション（スキル `kiro-watch`）へメッセージを送る形のままで、2026-10-10 から「一時停止中」の注記で丸ごと飛ばしている。つまり今は、机の調停が誰にも行われていない。
- 調停役のセッションに頼る形の弱点（送信の上限・調停役が落ちたときの取りこぼし・`target/` の掃除で状態が消える）は、スキルがアプリを呼ぶ形に替わるまで解けない。

## Current State

2026-10-10 時点（`areka-P0-impl-watch` の完了時）。

- アプリ: `crates/areka-impl-watch`（bin・`publish = false`）。コマンド 14 個＋`--help`。手順書は `doc/impl-watch.md`（7 節）。置き場所は環境変数 `AREKA_IMPL_WATCH_HOME`（絶対パス。開発者の機械には `C:\home\maz\store\.areka-impl-watch` が設定済み・中身は空）。
- スキル: `.claude/skills/kiro-watch-clear/SKILL.md`（全部消す）だけがアプリを呼ぶ。exe は `$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe` に在る前提で、無ければ `stop: no-exe` で止まる。**exe をそこへ置く手順（ビルドして写す）は、まだどこにも無い。**
- `.claude/skills/kiro-impl/SKILL.md` と `.claude/skills/kiro-complete/SKILL.md`: 机の段は「一時停止中（2026-10-10 開発者）」の注記つきで、`【kiro-watch】…` のメッセージを送る古い形の文が残っている。
- `.claude/skills/kiro-watch/`（`SKILL.md`・`kiro-watch.ps1`・`messages.json`）: 調停役のスキルとスクリプト。`areka-P0-impl-watch` は触っていない。
- アプリの側で決まっていること（手順書に書いてある）: 識別（`--id`）は ASCII の小文字に寄る／待つコマンドは同じ識別・同じ種類で 2 本走らない（2 本目は `already running` で 1・害は無い）／番を受けたら負荷テストの前に `watch` を立て直す（1 回でよい）／`watch` が 1 で終わったら直ちに立て直す／3 の文は標準エラーに出る／落ちたセッションは `leave` で外す。

## Desired Outcome

- `kiro-impl`（負荷テストの机・停止要請の見張り・再開）と `kiro-complete`（マージの机）が、メッセージを送らずに `areka-impl-watch.exe` を Bash から呼ぶ。待ちは「番が来たら終わるコマンド」の 1 回の起床だけ。
- 「一時停止中」の注記が消え、机の調停が戻る。
- 調停役のスキル `kiro-watch` とスクリプトをどうするか（退役・残す・`status` を読むだけの薄い形にする）が決まっている。
- exe を置き場所へ置く手順が 1 本在り、スキルと手順書が同じ手順を指す。

## Approach

要件で決める。議題の候補:

1. **`--id` に何を渡すか**: ワークツリー名（例 `areka-p0-impl-watch-da42f1`）・ブランチ名・spec 名・セッションの id。形の検査（英数字と `._-`）に通り、セッションをまたいで同じ値を作れ、落ちたセッションを開発者が `status` で見分けられること。
2. **`watch` を立てる時機**: スキルの最初（`run_in_background`）に立て、停止要請で終わったら `stopped --wait` を呼び、再開したら立て直す。立て直しの回数と、`already running` の 1 を害なしと扱う文。
3. **exe の置き方**: `cargo build -p areka-impl-watch --release` の成果物を置き場所へ写す道具（`tools/` の 1 本か、手順書の 1 節か）。走っている exe は上書きできない（Windows）ので、入れ替えの手順も要る。
4. **環境変数が無い・exe が無いとき**: スキルは止まるのか、机の段を飛ばして進むのか。
5. **`kiro-watch` の扱い**: 退役するなら、スキルのフォルダと `messages.json` を消し、`CLAUDE.md`・steering・記憶に残る呼び名を直す。
6. **`kiro-next` など、机を読むだけのスキル**が `status` を使うか。

## Scope

- **In**: `.claude/skills/kiro-impl/SKILL.md`・`.claude/skills/kiro-complete/SKILL.md` の机の段の書き替え。`.claude/skills/kiro-watch/` の扱い。exe を置き場所へ置く手順。`doc/impl-watch.md` の 3 節・4 節（スキルから呼ぶ形に合わせた追い書き。「`watch` が 1 で終わったら直ちに立て直す」の例外をスキルへ写す）。`.kiro/steering/workflow.md` に机の段を書くなら、その 1 節。
- **Out**: アプリの振る舞いの変更（拾い残しは `areka-P0-impl-watch-residue`）。規則 7 つの変更。ほかのリポジトリのスキル。

## Boundary Candidates

- 負荷テストの机の段（`kiro-impl`）。
- マージの机の段（`kiro-complete`）。
- 調停役 `kiro-watch` の退役と、呼び名の掃除。
- exe の配り方（ビルド・写し・入れ替え）。

## Out of Boundary

- `crates/areka-impl-watch` のソース（呼ぶ側だけを直す。アプリに足りないものが見つかったら `areka-P0-impl-watch-residue` へ申し送るか、別に起票する）。
- 開発者の機械の環境変数（スキルは読むだけで、設定も変更もしない）。

## Upstream / Downstream

- **Upstream**: `areka-P0-impl-watch`（完了）。
- **Downstream**: 並走するすべての spec の `/kiro-impl`・`/kiro-complete`。

## Existing Spec Touchpoints

- **Extends**: なし（スキルの文書の書き替え）。
- **Adjacent**: `areka-P0-impl-watch-residue`（アプリの拾い残し。`--help` や出力の文が変わると、スキルの読み方も合わせる）。`areka-P0-release-cycle`（繰り返しの spec。`kiro-complete` を使わないので机の段は別に要るかを要件で見る）。

## Constraints

- スキルが exe を走らせるときは、受け継いだ `AREKA_IMPL_WATCH_HOME` をそのまま使う。`/c/...` の形へ直さない・`cd` しない・値を書き替えない（アプリは絶対パスでない値を 1 で断る）。
- 試しに走らせるとき（スキルの確かめ・実機の確かめ）は、置き場所をワークツリーの `target\` の下の Windows 形の絶対パスで上書きする。開発者の本物の置き場所へ書かない・`clear` しない。
- トークンの負荷を最優先で抑える（起こすのは待ちの終わりの 1 回だけ・出力は終了コードと ASCII の数行）。
- `.claude/skills/` は保護された場所なので、書き替えはこのワークツリーのブランチの上で行い、PR で入れる。

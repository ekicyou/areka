---
inclusion: manual
updated_at: 2026-01-02
---

# Roadmap — 見本（tools/roadmap-overview.ps1 -SelfTest の入力・本物の roadmap.md の難所を写した断片）

> この断片は道具の自分の検査だけが読む。中身は架空の spec で、本物の roadmap.md とは関係しない。

## M1 ゴール ✅（版の段の節の外に同じ見出し行の表＝読まない）

| 版 | 目標 | spec（並びは依存の順） |
|---|---|---|
| **v0.0.1** | 読まない表 | `pi-reserved`＝**1 本** |

## 版の段（着手の大枠の正本・見本）

> **並べ方**: 若い版から取る。

| 版 | 目標 | spec（並びは依存の順） |
|---|---|---|
| **v0.0.3** | 利用者の物が消えない・致命的なバグ | `alpha-base` → `beta-next`（`alpha-base` の後・鎖＝1 本目→2 本目）・`gamma-fix`（〔入れ子・注〕の中＝外）・`c.rs`・地の文の要素＝**3 本**。着地したら `release-step` で版を上げる（内訳＝**9 本**） |
| **v0.1.0** | よく使われるゴーストが「SSP と同じ」に動く（括弧書き・`zeta-check` は読まない） | 動く＝`epsilon-run` → `zeta-check`／`eta-old` の後続＝`theta-follow`・`iota-solo`＝**4 本** |
| **いつか** | 夢 | `omega-last`＝**1 本** |

- **数**: 3＋4＋1 ＝ **8**。

## 完了サマリ（詳報は各 `completed/` spec）

| Wave | 完了日 | ユニット（PR） | 一行要約 |
|---|---|---|---|
| 耐力壁 | 07-01 | `eta-old` | x64→32bit 駆動 GO |
| W1〜W4 | 〜07-29 | idle-talk／sylphya 他 | 増分ウェーブ全完走（括弧の中。は切らない）。後ろの文は一言に入らない |
| W5 | 08-01 | `mu-done`（PR#97）・`nu-done`（`omicron-done` の前・PR#98） | M-dialogue 完走

`v0.0.3` からは、この表に行を足さない。

### 次のリリースへの申し送り

- 申し送りの項目。

### リリース

| 版 | 日付 | ユニット（PR） | 一行要約 |
|---|---|---|---|
| `xi-release` | 10-06 | `xi-release` | 読まない表 |

## ウェーブ編成（着手順の正本・見本）

> 各ウェーブはフルライフサイクルを完走してから次へ。

| Wave | ユニット（並べた順） | 開始コマンド | 編成根拠 |
|---|---|---|---|
| **C9**（2 本並走・**Opus** 2） | ① `beta-next`（A・`c.rs` を触る） ∥ ② `areka-P0-theta-follow` | `/kiro-start` | 重なり 0 |
| **次のウェーブの候補** | `omega-last` | — | 2 行目は着手中にしない |

## spec 台帳（brief を持つ 8 本・見本）

> 前置きの文。

| spec（`areka-P0-` 省略） | 段 | 何をするか | 規模 | 並び | 前提（先に着地） | Fable |
|---|---|---|---|---|---|---|
| `alpha-base`（**10-01 起票**・`old-name` をまとめた） | バグ・区分 B | 表の欄に `a|b` と \| を書いても割れない。後ろの文 | S（3〜4） | 鎖の 1 本目 | なし（`eta-old` は 07-01 着地） | − |
| `beta-next` | 優先・区分 A | 鎖の 2 本目を張る | M | `alpha-base` の後 | `alpha-base`・`foo.rs` | ○ |
| `gamma-fix` | 優先・区分 A | 台帳の文に `</script>` が在っても HTML が壊れない | S | 単独 | ✅ `eta-old`・`ghost-of-past`（どこにも無い） | − |
| `epsilon-run` | その他・区分 C | ゴーストを動かす（台帳の文） | L | 動くの先頭 | − | − |
| `zeta-check` | その他・区分 C | 動きを確かめる | S | `epsilon-run` の後 | `epsilon-run`・`omicron-done` | − |
| `theta-follow` | その他・区分 C | 後続を拾う | M | `eta-old` の後続 | `areka-P0-iota-solo`〔`zeta-check` は注〕 | − |
| `iota-solo` | 据え置き | テーマの行が無い spec の一言は台帳の何をするかから取り、六十字を越える長い文は六十字で切って末尾に省略の印を足して見せる | M | 単独 | − | − |
| `omega-last` | 保留・区分 D | いつかやる | ? | 単独 | − | − |

## テーマ別の決めごと（起票のときに決めた中身・見本）

> テーマごとの決めごと。

### 土台の鎖（2026-10-01 `/kiro-discovery` で起票）

- **決めたこと**: 鎖の順に着地させる。

#### 触るファイル（読まない小節）

| spec | ファイル |
|---|---|
| `alpha-base` | `a.rs` |

#### Existing Spec Updates

- [x] areka-P0-epsilon-run -- 申し送りの行（完了と数えない）。Dependencies: none
- [x] areka-P0-rho-noted -- 申し送りだけに在る名前。Dependencies: none

#### Specs (dependency order)

- [ ] areka-P0-alpha-base, areka-P0-beta-next -- 土台の鎖を張る（括弧は外す）。後ろの文は一言に入らない。Dependencies: none
- [x] areka-P0-kappa-gone -- 着地済みで完了サマリに無い。Dependencies: areka-P0-alpha-base
- [→] areka-P0-lambda-moved -- （2026-10-11 別の spec に吸収）移した行。Dependencies: none

### **動く**ゴースト（2026-10-02）

#### Specs (dependency order)

- [ ] areka-P0-epsilon-run -- ゴーストを動かす。Dependencies: areka-P0-theta-follow（注の areka-P0-xx-yy は数えない）, areka-P0-gamma-fix
- [x] areka-P0-zeta-check -- 動きを確かめる。Dependencies: areka-P0-epsilon-run
- [ ] areka-P0-theta-follow, areka-P0-gamma-fix -- 後続を拾う。Dependencies: none

### いつかの夢（据え置き）

#### Specs (dependency order)

- [ ] areka-P0-omega-last -- やることが無くなってから。Dependencies: areka-P0-ghost-of-past

## 予約（全て任意・brief なし）

- 予約の項目。

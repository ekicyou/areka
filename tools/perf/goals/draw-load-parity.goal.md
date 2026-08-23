# `/goal` 条件文テンプレート — draw-load-parity

**このファイルは雛形（テンプレート）であり、そのまま `/goal` へ貼るものではない。** 走行ごとの
8 桁トークンが埋まっていないので、下の本文の `<token>` は判定の役に立たない。

## 実際に貼る文の作り方

```
python tools/perf/perf-ledger.py init      --goal draw-load-parity          # 台帳が無いときだけ
python tools/perf/perf-ledger.py goal-check --goal draw-load-parity          # 周 0・トークンを作る
python tools/perf/perf-ledger.py goal-text  --goal draw-load-parity          # 貼る文が出る
```

`goal-check` が 8 桁の走行トークンを作って台帳 `loop-ledger.md` の状態ブロック `run:` へ書き、
`goal-text` がそれを埋めた文を標準出力へ出す。その出力を丸ごとコピーして

```
/goal <ここへ貼る>
```

とする。`goal-check` は目標定義 `tools/perf/goals/draw-load-parity.toml` の必須キー・
`judge-perf.py` の版・アイドル CPU の閾値の一致も同時に確かめ、違えば exit 3 で止まる
（設計 C1・要件 1.1）。

## トークンを埋める理由

`/goal` の判定役は**会話に現れた文字列しか見ず**、テンプレートと実出力を区別できない。そこで
終端行を走行固有のトークン込みでのみ出す。文書・スキル本文・README に載る書式見本は常に
山括弧（`run=<token>`）で書き、実出力とは一字も一致しないようにしてある——この不一致は
`python tools/perf/perf-ledger.py --selftest` が 1 ケースとして固定しており、見本の 2 行
（`GOAL_MET run=<token> …` と `STOPPED run=<token> reason=…`）は判定の正規表現に一致しない。
見本の並びは `python tools/perf/perf-ledger.py --samples` で確かめられる。

## 長さ

`/goal` の条件文は 4,000 字以内（要件 1.6）。下の本文は 1,012 字で、`goal-text` は 4,000 字を
超える文を組んだ時点で exit 3 にするので、長さの確認を人が行う必要はない。

## セッション側の設定（推奨）

- `CLAUDE_CODE_GOAL_CHECKIN_MINUTES=60` — 目標定義 `[goal_runtime].checkin_minutes` と同じ値。
  計測 1 本がこれを超えないよう分けて起動する（要件 1.11）。
- 自動で進む設定（各ターンで確認を求めない）で起動する。ループは開発者へ質問しない。
- メインは Fable、重い作業は `model: opus` のサブエージェント（`.claude/agents/perf-*.md`）。
  Opus 5 をメインにしても手順は同じ（要件 1.12）。

## 本文（`goal-text` の出力そのもの・トークンだけ `<token>` に置換）

以下の区切り線より下は `python tools/perf/perf-ledger.py goal-text --goal draw-load-parity` の
出力と、8 桁トークンを `<token>` に置き換えた点を除いて一字も違わない。文面を直したいときは
このファイルではなく `tools/perf/perf_ledger_goal.py` の `GOAL_TEXT_TEMPLATE` を直す
（字面の所在は 1 箇所・要件 1.6）。

---

目標: draw-load-parity — areka の release アイドル CPU（1 コア換算・定常平均）を 3.0% 未満にし、判定式⑴〜⑷b が 25 分の最終判定で全て合格すること。合否は judge-perf.py 0.4.0 の出力だけで決め、人の目視や主観を使わない。

毎ターンの手順: プロジェクトスキル `perf-loop-iteration` を引数 `draw-load-parity` で 1 回だけ呼び、その最後に出る `PERF-LOOP STATUS …` の行を一字も変えずに返答の最後の行として書く。スキルは背景コマンド（計測・最終判定）を起動するところまで相を進めてターンを終える（相の境界ごとに台帳を更新し状態行を印字する）。背景コマンドが走っている間は待つ——check-in が届いたターンは出力の末尾を読み、進行中なら「待つ」と答える。

達成の判定: 会話に `PERF-LOOP FINAL: GOAL_MET run=<token>` で始まる行が現れたとき。
不可能の判定: 会話に `PERF-LOOP FINAL: STOPPED run=<token> reason=` で始まる行が現れたとき（頭打ち・安全停止・道具の故障・周数上限 30 のいずれか）。
注意: 文書・スキル本文・README に載っている書式見本は山括弧つき（例 run=<token>）で、実出力とは一致しない。判定は上の実トークン <token> を含む字面でのみ行う。

制約:
- 開発者へ質問しない・裁定を仰がない。必要な判断はルール化して行い、根拠を台帳へ残す。
- Cargo.toml を変更しない。採用は 1 周 1 コミット。採用しない変更は元へ戻す。
- 判断の記憶は台帳 `loop-ledger.md` だけに持つ（会話の記憶・要約に頼らない。再開時は台帳の phase から続ける）。
- 連続 3 周で採用に至る改善が無ければ頭打ちとして止まる。周数の上限は 30 周。
- check-in は 60 分（環境変数 CLAUDE_CODE_GOAL_CHECKIN_MINUTES）。計測 1 本がこれを超えないよう分けて起動する。
- 重い作業（計測・順位付け・実装・差し戻し判定）は役割別のサブエージェントへ委ね、結論だけを受け取る。

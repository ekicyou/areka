# 設計バリデーションレポート — areka-P0-cue-playback-duration

> 生成日: 2026-07-13 ／ 言語: ja（spec.json）／ フェーズ: design-generated
> 入力: requirements.md（確定・不変）・design.md（確定・不変）・research.md・.kiro/steering/{product,tech,structure,workflow,roadmap}.md
> 手順: kiro-validate-design（rules/design-review.md の Analysis → Critical Issues → Strengths → GO/NO-GO）。非対話・実コード突合検証。

## Design Review Summary

三権分立（計算＝sakura／保持＝dola／服従＝emo-text）へ「テキスト再生 duration D」という 1 データを貫通させる横断改修設計であり、実装準備度は高い。design.md が主張する既存構造（`Cue`/`CueCommand` 8 variant・compile の offset 累積・`RevealSchedule::extend_chunk`・`to_schedule`・`cue_target_of`・`CueSheet::new` 安定ソート・`TimedSchedule::insert` の同一 at FIFO）は**すべて実コードと一致**を確認した。開発者決裁の 4 不変条件にも適合する。ただし要件字義との調整点 1 件と、reveal「ビット等価」主張の浮動小数点脆弱性 1 件を設計ディスカッションへ送る。

## アーキテクチャ不変条件の適合確認（開発者決裁・research §7/§8）

| 不変条件 | 適合 | 根拠（design.md 位置） |
|---|---|---|
| dola は絶対 start_time を保持し配送時に導出/変換しない・整列は sakura が焼込む（`offset += D`） | ✅ 適合 | §8.4・Architecture Integration（案 2-A）・`to_schedule` が start_time を無変形複写（drive.rs:293-297 で実証） |
| D は第一級 cue データとして各表現者へ搬送・単一真実源＝純関数 `text_playback_duration` 1 本 | ✅ 適合 | Boundary Commitments・§計算層・伝播経路 `compile→Cue.duration→TalkCue.duration→emo-text` |
| dola は「絶対開始＋累積時間」の静的タイムラインのみ・pause/choice は dola 外の Barrier シーム再調停 | ✅ 適合 | Non-Goals・Out of Boundary（`\x`/`\q` を dola へ持ち込まない・8.4） |
| 案 2-B（配送時導出）・案 1-C（Barrier でテキスト占有）を棄却 | ✅ 適合 | §Design Decisions・Architecture Integration で両案を明示棄却 |
| Rust 2024・bevy_ecs・serde のみ・新規 crates.io 依存禁止・tokio 不使用・注入時刻決定論 | ✅ 適合 | Technology Stack「新規依存の追加なし。tokio 不使用」・注入時刻 `talk_time` 駆動維持 |

**結論**: design.md は 4 つの発注者不変条件のいずれにも違反しない。

## Critical Issues（≤3）

🔴 **Critical Issue 1**: 単一純関数が明示 `\_w` を畳まず、R2.4 の字義を再解釈している
**Concern**: R2.1/R2.4 は「暗黙 per-char ＋明示ウェイト（`\_w`）の換算を算出する**単一の純関数**」を規定するが、設計は `text_playback_duration(text: &str) -> f64` を暗黙 per-char のみとし、明示 `\_w` は compile の `offset += Wait` 累積（別経路）へ委譲する（§8.3 項目 3）。純関数は入力が `&str` チャンクのみで構造的に `\_w` を観測できない。
**Impact**: 確定済み requirements.md は変更不可。字義の AC は純関数単体で充足されず、「純関数＋compile 合成」の 2 経路で初めて総再生時間が成立する。設計ディスカッションでこの reconcile が発注者意図（＝重複実装の絶滅・単一真実源）と両立することを明示合意しないと、実装・レビュー時に AC 逸脱と誤判定されうる。
**Suggestion**: 設計は既に §8.3/§8.5 で reconcile を文書化済み——これを追認しつつ、統合テストで「暗黙 D＋明示 `\_w` を合算した総 duration により後続 cue が整列する」ことを end-to-end で固定し、R2 の「単一真実源」意図が暗黙のみならず合成でも検証可能にする（`Text\_w[500]Text` の 2 つ目 start_time = D+0.5 は既に計画済み・§Testing）。
**Traceability**: R2.1, R2.4, R3.4
**Evidence**: design.md §8.3（項目 3 裁定表）・§計算層 text_playback_duration Invariants・§8.5「R2.4 の字義」

🔴 **Critical Issue 2**: reveal の「旧 char_wait=0.05 とビット等価」主張が浮動小数点で成立しない懸念
**Concern**: Implementation Notes は「`cue.duration=N×50ms` かつ interval=`D/N=50ms` のとき reveal 時刻は旧 `char_wait=0.05` と**ビット等価**」と主張するが、`Duration::from_millis(N*50).as_secs_f64() / (N as f64)` は一般に f64 リテラル `0.05` とビット一致しない（例: N=3 で `0.15/3 ≈ 0.049999999999999996` ≠ `0.05 = 0.05000000000000000277`・約 1 ULP 差）。`r_i = max(r_{i-1}+interval, at)` の累積で旧 reveal 時刻と末尾 ULP がずれる。
**Impact**: 機能挙動（注入時刻 tick での可視グリフ数）は不変で実害はないが、設計が回帰リスク最小の根拠として掲げる「ビット等価」を根拠に**旧期待値（リテラル 0.05 由来）へ `assert_eq!` する reveal 回帰テストを書くと FP 差で fail/flaky 化**する。決定論檻を重視する本プロジェクト方針（memory: deterministic-test-coverage-mandate）と衝突する。
**Suggestion**: (a) 期待 reveal 時刻を**実装と同一の `D/N`（または `Duration` 整数 ms）算術で再計算**して比較する（旧 `0.05` リテラルを期待値に使わない）。(b) Implementation Notes の「ビット等価」を「機能等価（全 tick で可視数一致）」へ緩め、必要なら許容誤差比較にする。(c) `interval` 算出を整数 ms 経由（`CHAR_NOMINAL_MS` を割らずチャンク総 ms を N で割る）で決定化する方針を design に明記。
**Traceability**: R5.1, R5.3, R7.1
**Evidence**: design.md §服従層 Implementation Notes（「ビット等価」）・§Testing Strategy（「同一 reveal 時刻を検証」）・§縮退（interval=D/N）

## Design Strengths

1. **主張の実コード接地が完全**: design.md の既存構造記述（`Cue` PartialEq 非導出・`CueCommand` 8 variant externally tagged・compile の Text が offset を進めない・`extend_chunk` の max 追従式・`cue_target_of` の catch-all 無し全域 match）はすべて実ファイルと一致。load-bearing な Clear 前置順序も、`CueSheet::new` の**安定ソート**（sheet.rs:20 `sort_by`）と `TimedSchedule::insert` の**同一 at FIFO**（schedule.rs:87-99・後挿入が先頭 index 0 へ入り末尾 pop で記述順配信）で実証的に成立し、design が自らこれを load-bearing と特定し統合檻を計画している。
2. **不変条件と汎用性の両立**: duration を `Cue` エンベロープの汎用フィールド（テキスト固有でない・D=0 が瞬時点）とし、隣接 `mayuna-compose` の瞬時 bind cue が additive に載る一般化を、実装は現要件どまりに抑えて先取りしている。serde `#[serde(default)]` による後方互換・`CueCommand` ワイヤ形不変も既存 `BalloonSurface` 実績に忠実。

## Final Assessment

**判定: GO**

**Rationale**: design.md は 4 つの発注者不変条件すべてに適合し、全 load-bearing 主張が実コードと一致、単一責務の三権分立・serde 後方互換・決定論・新規依存ゼロを満たす。Critical Issue 2 件はいずれも実装方針の精緻化（要件字義の reconcile 追認・reveal 等価主張の FP 緩和とテスト期待値の算術整合）であって、アーキテクチャの根幹を覆すブロッカーではない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Critical Issue 1（R2.4 reconcile 追認）・Issue 2（ビット等価→機能等価への緩和・期待値算術整合）を解消。
2. 解消後、`/kiro-spec-tasks areka-P0-cue-playback-duration` でタスク生成へ進む。

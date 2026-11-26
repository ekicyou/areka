# 記録に書く事の控え（7.1・12.3 が `migration-ledger.md` へ集める）

進行役が、各タスクの報告に挙がった物をここへ写す。7.1・12.3 が記録を書いた後に、このファイルを消す。

## 2.1

- 外したテスト: `runtime::clock::tests` の `now_is_monotonically_increasing`・`now_returns_positive_finite_value`・`now_has_millisecond_precision`。古い QPC の時計（実数の秒）をファイルごと消したため。同じ事は `time/clock_tests.rs`（戻らない事・OS の値と同じ物差し）が確かめる。
- 外したテストの行: `error.rs` の `display_loop_offset_variants` の `LoopOffsetNegativeMin`・`LoopOffsetNegativeMax` の 2 行。variant を消したため。負のずらしは読み込みの失敗になる。
- 変えた期待: `display_loop_offset_variants` の `LoopOffsetRangeInverted` の字面「min 3 > max 1」→「min 3.0000000 > max 1.0000000」（長さの型の文字の形）。
- 足したテスト: `display_time_variants`（`TimeOutOfDocumentRange`・`LoopDurationOutOfRange` の字面）。
- 取り除いた本番の分かれ道: 古い時計の QPC の周波数の割り算と、Result を捨てていた所。消した欄: `CompiledStoryboard.start_time`。

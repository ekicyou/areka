# 設計の検証: audit-emo-text-layout-draw

- 検証した日: 2026-11-06
- 読んだもの: `spec.json`・`requirements.md`（確定した指摘の一覧を含む）・`design.md`・`research.md`（`.kiro/steering/` の本文は読み直さず、要件と設計が引く決まり＝1,000 行・log-first・1 フレーム遅らせない・`crates/areka` に触らない、だけをコードで確かめた）
- 突き合わせたコード: `crates/areka-emo-text/src/`（範囲の 81 ファイル・範囲の外の `actor_present.rs`・`actor_decoration.rs`・`actor_clear_atomicity_tests.rs`・`state_decoration_tests.rs`）・`crates/areka/src/`（`classify_font_tag`・`ScaleContract::new` の呼び手）・`crates/areka-emo-text/tests/`

## 1. まとめ

設計は、要件の討議で「直す」と決めた指摘の全部（1〜18・21〜35、19 は説明だけ）に、直す場所・直し方・壊れたら落ちるテストとその置き場を挙げており、確かめた行の番号・「呼び手はここだけ」・行の余白・`crates/areka` と `lib.rs` に触らないことは、コードの実物と合っていた。ただし、指摘 1 の守りの範囲が狭く、要件の討議で指摘 20（`\f[height,N]` に上限を置かない）を残した前提（「位置の無限大は指摘 1 の直しで消す」）が設計のままでは成り立たない。また、指摘 6 の打ち消し線の直し方が、既定の見た目の区間を飛ばす今のループに届かない。どちらも設計の作りを変えずに、直す所を 1 か所ずつ足せば片づく。

## 2. 突き合わせの結果（合っていたもの）

| 設計の主張 | 確かめた結果 |
|---|---|
| 範囲は 81 ファイル | `git ls-files` で 81。合う |
| 本番の配置の呼び手は `actor_present.rs:624` の `layout_styled` 1 か所 | ツリー全体の本番コードで 1 か所。合う |
| 保留の改行の和は `layout_scan.rs:204`・下見は `:230-245`・`apply_pending_newline` は 1 つ | 合う（`:350` に関数・`:359` が送り） |
| 指摘 5 の枝は `layout_scan_glyph.rs:111-116` | 合う（「行頭では構造的に不発火」の説明もそこにある） |
| 指摘 7 の面の寸は `actor_present.rs:244-247` | 合う。初めの解決のフレームの枝の中の 2 行 |
| 指摘 21 で赤になる範囲の外の語は `state_decoration_tests.rs:424,438` の `"cursorbrush"` | 合う。`crates/areka`・`tests/`・ゴーストの台本に、ukadoc に無い `\f[cursor…]`・`\f[anchor…]` のキーは無い（`git grep`）。ukadoc のキーは `cursor` 12・`anchor` 19 で、設計の数と合う |
| 指摘 17 の説明は `viewbox_draw.rs:256-257` | 合う。範囲の外の `actor_clear_atomicity_tests.rs` は「消去だけのフレーム」を見ているので、消去と字が同じフレームの描き方を変えても今の檻は崩れない |
| 指摘 4 は `viewbox_diff.rs:426-429` の 4 つの足し引き | 合う |
| 指摘 35 は `draw.rs:411-433` の作り直し | 合う |
| 指摘 30 で消すもの（`layout_with_cursor_warn`・`DrawExecutor::line_layout`・`ChoiceDraw`）を `crates/areka`・`tests/` が使わない | 合う（`tests/viewbox_blit_spike.rs` の `line_layout` は自前の関数）。`ScaleContract::new` は `crates/areka/src/input_events/balloon_pure_core_tests.rs:156` が使うので残す、という設計の段の決めも事実に合う |
| `CursorDegrade` を網羅で `match` する所は 0・`crates/areka` は使わない | 合う |
| `classify_font_tag` の形は変わらず、`crates/areka` の 2 か所は答えの語が変わるだけ | 合う（`strict_judges.rs:34`・`check_script_judge.rs:135`） |
| 書体の台帳は場所ごとに 1 つで、描き手と計測器が `Rc` で共有 | 合う（`actor_decoration.rs:64-66`） |
| テストの置き場の余白（`layout_wrap_tests.rs` 264・`viewbox_axis_tests.rs` 853・`draw_format_metrics_tests.rs` 78・`draw_oracle_tests.rs` 163 ほか表の全部） | 今の行数から出した値と全部合う |
| 新しいファイルを足さない・`lib.rs` に触らない | 消すもの・足すものに `lib.rs` の再公開は無い（`lib.rs` に該当の `pub use` が無い）。合う |
| 1 フレーム遅らせない（指摘 16〜18） | 16 は空→空を `NoChange`、17 は消去と字が同じフレームで全域の `Update`、18 は面と重なる行だけを描く。どれも遅らせる段を作らない |
| 記録を先に（log-first） | 非有限の送り・書式の失敗・字の測りの失敗・`\f` の不明のキーのどれにも記録がある。作り直しを消す 35 も `error!`＋`Err` のまま |

## 3. 大事な問題（3 件まで）

### 問題 1: 折り返しの送りに守りが無く、とても大きな `\f[height,N]` で位置が無限大になる

- **何が問題か**: 設計は「位置が有限でなくなりうるのは保留の改行と `\_l` の 2 か所だけ。折り返しの送り（行送り 1 つ分）は有限の位置へ足しても有限のまま」と書く（「案の選び方」）。これは行送りが小さいときだけ正しい。指摘 20 で `\f[height,N]` に上限を置かないと決めたので、行送り（字の高さ＋2）は 1e38 にもなりうる。`layout_scan_glyph.rs:159` の `self.block_pos += self.block_dir * line_pitch_of(self.metrics, closing)` は守られておらず、`\f[height,1e38]` の字が 4 行に折れると位置が無限大になる（`research.md` の指摘 20 の「困る場面」もこの道を書いている）。
- **なぜ大事か**: 要件の確定した一覧の 20 は「困る場面（位置の無限大・表の際限ない育ち・記録のあふれ）は指摘 1・11・12 の直しで消す」を、残す理由にしている。設計のままでは位置の無限大が残り、残す裁定の前提が崩れる。無限大の位置は見える窓と描き直しの計画へ流れる（指摘 3・4 の飽和で panic は防げても、字は消える）。
- **どう直すか**: 折り返しの送り（`layout_scan_glyph.rs:159`）にも、保留の改行と同じ「送った結果が有限でなければ位置を保つ・`should_warn(actor, NonFinite)` で 1 回 `warn!`」を置く。2 か所が同じ規則になるよう、確かめを小さな私有の関数 1 つにまとめて両方から呼んでもよい（型は要らない）。壊れたら落ちるテストとして「`\f[height,1e38]` で 4 行以上折れる台本で、どの行の位置も有限・`warn!` が 1 件」を `layout_wrap_tests.rs`（余白 264）か `layout_styled_tests.rs` に足す。あわせて、字の送り（`:169` の `inline_pos += advance`）が、とても大きな em の字幅（縮退の字幅を含む）で有限のままかを、同じテストで確かめる。
- **つながる要件**: 要件 3.1（確定した一覧の 20 の扱い）・5.2・6.2・8.1
- **設計の箇所**: 「案の選び方」の 1 つ目の箇条・「配置の位置の守り」の事後条件・「System Flows／保留の改行の適用」・Testing Strategy の指摘 1 の行

### 問題 2: 既定の打ち消し線の直し方が、既定の区間を飛ばすループに届かない

- **何が問題か**: 指摘 6 の直しは「打ち消し線は `look.strike` が真なら焼く（`:212`）」と「既定の見た目だけの行でも線が要れば線を焼く道（`line_layout_with_strokes`）を通す」の組み合わせになっている。ところが線を焼く道の `apply_font_ranges`（`viewbox_draw_decoration.rs:174-176`）は `run.style == StyleId::DEFAULT` の区間を `continue` で飛ばす。既定の見た目が打ち消し線つきでも、既定の区間には `SetStrikethrough` が一度も呼ばれず、設計どおりに直しても既定の打ち消し線は描かれない。逆に `!= default.strike` を `look.strike` に替えるだけだと、`\f[strike,0]` で外した区間の扱いが決まらない。
- **なぜ大事か**: 指摘 6 は「必ず直す」で、完了 spec `areka-P0-balloon-font-descript-keys` の要件 5.5 に戻す直し。テストの表には読み戻しの檻（`viewbox_draw_decoration_tests.rs`）があるので実装で気付けるが、気付いた時に直し方を決め直すことになり、範囲の呼び出しの数の檻（`note_range_call`・要件 14.2 の「区間の呼び出し 0 件」）との兼ね合いもその場で決めることになる。
- **どう直すか**: 設計で直し方を決めておく。たとえば「既定の見た目が打ち消し線つきなら、ループの前に行の全体へ `SetStrikethrough(true)` を 1 回焼き、ループの中は今の `look.strike != default.strike` の比べのまま（外した区間は `false` を焼く）」とする。下線は今どおり areka が塗るので、区間の選び方（`StyleId::DEFAULT` も対象・`!= default.underline` を外す）の直しだけでよいことも明記する。範囲の呼び出しの数の檻が、既定の線のある行でどう数えるかも書く。
- **つながる要件**: 要件 3.1（確定した一覧の 6）・4.2・8.1
- **設計の箇所**: 「線（指摘 6・14・15）」の指摘 6 の箇条・File Structure Plan の `viewbox_draw_decoration.rs` の行

## 4. 良いところ

- **確かめが行の番号まで実物に合っている**: 呼び手の数・行の余白・範囲の外に触る 2 か所（`actor_present.rs:244-247`・`state_decoration_tests.rs:424,438`）・`crates/areka` が使う口が、どれもコードと一致した。設計の段で `ScaleContract::new` の範囲の外の呼び手を自分で見つけ、要件 3.4 の議題にして裁定を取ったのも、消す前の確かめ（要件 4.3）がきちんと回っている証拠になっている。
- **作りを増やさずに直している**: 新しい型は足さず、下見の写しを片づけて改行の守りを 1 か所へ寄せる（案 A）、計画者の状態を `Option` 1 つで「分からない／分かっている」に分ける、表の寿命を今ある消去の口（`request_clear`）に揃える、のように、今ある形の上で最小の直しを選んでいる。1 フレーム遅らせない直し（16〜18）も状態の持ち方で解いており、記憶の裁定に沿う。

## 5. 判定

- **判定: GO**
- **理由**: 作りの食い違いや大きな抜けは無く、直す対象の全部に直し方とテストと置き場がある。上の 2 件は、どちらも直す所を 1 か所ずつ足す話で、設計の作り（案 A・ファイルの割り振り・タスクの見込み）は変わらない。
- **次にすること**: 設計の討議（`/kiro-design-discussion audit-emo-text-layout-draw`）で問題 1・2 を議題にし、設計の該当の箇条とテストの表に直しを足す。その後 `/kiro-spec-tasks audit-emo-text-layout-draw` でタスクを作る。小さな補い（議題にしなくてよい）: 指摘 10 で `TextLook` に `Hash` を足すには、欄の `Script`（`look.rs:53`）にも `Hash` が要る。

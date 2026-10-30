# Brief: animated-surface-log-flood

> 2026-10-23 `areka-P0-pasta-in-windows-verification` のタスク 5.3 で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。開発者の決め（同日のチャット）「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。版に縛らない列の `real-run-log-noise` へはまとめない（同日の開発者の決め）。版は **v0.1.0**。

## Problem

- **実機の走行の判定をする人と道具**: ぱすたの実機の走行の記録の 9 割が、同じ INFO の行 `apply(ShowSurface): 表示・マスクを更新` で埋まる。判定に要る行（起動の決め方・イベント・WARN）が埋もれ、読むのにも数えるのにも手間がかかる。動く絵を持つゴーストなら、どれでも同じことが起きる。

## Current State

- **証拠**: `verification.md` 5 節の崩れた所の表の **21**。本走行の記録 `target\pasta-verify\logs\run1.stdout.log`（6,546 行）のうち 6,073 行がこの行で、そのうち面 1002 の分が 5,989 行・毎秒十数行・どれも `cache_hit=false`。補いの走り `target\pasta-verify-alt\logs\alt2.stdout.log` でも 3,650 行。
- **原因**: 再描画の回数そのものは決まりどおり。面 1002 のボールの 3 枚（`window/ball/eback.webp`・`base_rings.webp`・`efront.webp`）が動く WebP（どれも `ANIM` の塊を持つ）で、コマが替わるたびに合成し直す（完了 `areka-P0-animated-image-playback`）。記録の量は、適用 1 回ごとに INFO を 1 行出すこと（`crates/areka-emo-present/src/presenter/show.rs:402`・426 行のファイル）。
- 同じ適用の対として、すぐ下で perf の行（`timing.emit`）も 1 行出る（`present-emit-tail-latency` が段を足す行）。
- この行は観測点としても使われている: 同じ行の `size_changed` は窓寸の状態照合の観測点（同じファイルの注記）。`crates/areka/examples/`（`collision-probe.rs`・`emo-present*.rs`・`window-placement.rs`）・`crates/areka-emo-present/src/{presenter/hub.rs, presenter/timing.rs, presenter_film_tests.rs, presenter_perf_log_tests.rs}` が行の文言に触れ、完了 spec の実機の記録（`collision-dpi-hittest`・`default-balloon-bundle`・`element-base-method`・`mcp-tool-entrances` など）がこの行を grep している。
- **SSP の基準**: 記録の量は SSP に対応する物が無い（SSP は areka の記録を出さない）。基準は「判定に要る行が埋もれない」ことと、areka の決まり（記録の無い失敗の道を作らない・状態が変わったときは記録する）。

## Desired Outcome

- 動く絵のコマ送りだけで起きる適用は、既定の記録の水準（`RUST_LOG` の手順書の値）で毎回は出ない。面の替わり・窓寸の替わり（`size_changed`）・初めての表示・失敗は、今と同じに出る。
- 動く絵の適用の回数そのもの（決まりどおりの再描画）は変えない。
- 観測点として使う人（例・テスト・手順書）が、変えた後も同じ事実を読める。
- （件 25）動く絵のコマは読み込みのときにアトラスへ展開しておき、コマ替えは面をいちから合成し直さずに済む。何もしていないあいだの CPU が SSP と同じくらいになる（開発者の指示 2026-10-23・下の件 25 の節）。

## Approach

- 案は要件の段で決める: ⒜ コマ送りの適用（面も窓寸も変わらない適用）だけ `debug!`／`trace!` へ下げる ⒝ 同じ面のコマ送りを一定の間まとめて 1 行にする ⒞ 状態が変わったときだけ INFO。areka の決まり「ログ無しの失敗の道を作らない」は失敗の道の話で、成功のコマ送りを下げても触れない。

## Scope

- **In**: `show.rs` の INFO 行の出し方・それに触れるテストと例の追随・手順書 `doc/pasta-real-run.md` の `RUST_LOG` と数え方の見直し・ぱすたの走行の記録の量の確かめ（面 1002 の分が既定の水準で消えること）・（件 25）動く絵のコマのアトラスへの先の展開と、コマ替えで面を作り直さないこと・何もしていないあいだの CPU の測り直し。
- **Out**: 動く絵の適用の回数を減らすこと（決まりどおり・`present-emit-tail-latency` も範囲の外にしている）・perf の行（`present-emit-tail-latency`）・文字の層の毎フレームの `debug!` と同梱の emo2 の WARN（`real-run-log-noise`）。

## Boundary Candidates

- emo-present の表示の段の記録（`presenter/show.rs`）

## Out of Boundary

- 適用の中身と時間（`present-emit-tail-latency`）
- 記録の仕組みそのもの（`tracing` の設定）

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: `pasta-in-windows-verification` の回し直し・SSP 並みの自動試験（記録を grep で判定する道具）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `real-run-log-noise`（同じ「記録の量」の話だが場所が別・まとめない）・`present-emit-tail-latency`・`mcp-strict-errors`（どちらも `show.rs` に触る＝同じウェーブに置かない）。

## Constraints

- 完了 spec の正典（`animated-image-playback` の再描画の決まり）は変えない。
- 行の文言を変えるなら、grep する側（テスト・例・手順書）を同じ変更で追随させる。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-emo-present/src/presenter/show.rs` と兄弟のテスト（`presenter_perf_log_tests.rs` ほか）・必要なら `crates/areka/examples/` の 4 本・`doc/pasta-real-run.md`。
- 規模: S（2〜4）。
- 優先度の区分: A（開発者の指示）。段: その他。
- 要件定義のモデル: Opus。
- 議題: 上の 3 案のどれにするか。

## 2026-10-23 `pasta-in-windows-verification` からの申し送り（件 25・開発者の承認「承認。animated-surface-log-flood に件として足し、記録も進めて」）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表の **25**・6 節の「何もしないあいだの CPU の測り」。件の番号は表の番号）。記録は git の外の `<w>\target\pasta-verify-quiet\debug\logs\`・`<w>\target\pasta-verify-quiet\release\logs\`（`quiet-<版>.health.csv`・`quiet-<版>.stdout.log`・`quiet-<版>.exitcode.txt`）と `<w>\target\pasta-verify-rerun3\logs\rerun3.health.csv` に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。
- **件 25（動く絵を毎フレームいちから作り直す〔`cache_hit=false`〕ため、何もしていない間も CPU を使い続ける〔release で 1 コアの 20%・debug で 69%・毎秒およそ 14 回〕・記録のあふれ〔件 21〕と同じ場所）**:
  - 測り: コミット `a13e19b6` の debug 版と release 版で、ぱすたを手順書 `doc/pasta-real-run.md` 4 節の起動の塊（`AREKA_APP_SMOKE_EXIT_MS=360000` で 6 分で自分を終える）で起こし、何も操作せずに置いた。最初の 60 秒を除いた 5 秒ごとの 50 行で、areka の CPU は 1 コアに対して debug 版が中央値 69%（最大 86%）・release 版が中央値 20%（最大 29%）。WorkingSet は debug 版 197 MB・release 版 176 MB で増えず、答えなくなることは 0 回、panic と ERROR は 0、終了コードはどちらも 0。操作のあった rerun3（debug 版）は中央値 84%（最大 108%）。
  - 根拠: 面 0 の面 1002 の `apply(ShowSurface)` の INFO 行が、debug 版 5,060 行（`07:29:26Z`〜`07:35:20Z`）・release 版 5,063 行（`07:35:39Z`〜`07:41:33Z`）。どちらも約 354 秒に毎秒およそ 14 行で、どの行も `cache_hit=false`。
  - 原因: 件 21 と同じ面 1002 のボールの 3 枚の動く WebP のコマが替わるたびに、面をいちから合成し直す（窓の大きさの絵を作る `crates/areka-emo-present/src/display.rs:174` の道）。何もしていないあいだも、動く絵がある限りこの合成が毎秒およそ 14 回走る。release 版は 1 回の合成が軽いだけで、回数は debug 版と同じ。記録の行の場所は件 21 と同じ（`presenter/show.rs:402`）。
  - 測りの条件: 進行役は排他テストの机だけを持ち、ひとりの机は持っていなかった。測りのあいだの機械ぜんたいの CPU は中央値で debug 版 16%・release 版 15%。
  - SSP との比べ: 同じ測り方で、SSP 2.9.23（32 ビット）に入れてあるぱすたを SSP 自身の保存した状態のまま起こし、何もせずに置くと、SSP の CPU は 1 コアに対して中央値 10%（最大 13%）・WorkingSet 111〜112 MB（`<w>\target\pasta-verify-quiet\ssp\logs\quiet-ssp.health.csv`・最初の 60 秒を除いた 62 行）。areka の release 版はおよそ 2 倍、debug 版はおよそ 7 倍。ただし SSP がそのあいだにどの面を出していたか（動くボールが動いていたか）は確かめておらず、同じ動く絵の比べとまでは言えない。要件の段で、同じ面を出した SSP の基準を測るかを決める。
  - 範囲: この brief の Scope の Out「動く絵の適用の回数を減らすこと」は、決まりどおりの再描画の回数の話。件 25 は 1 回ごとの合成の重さ（変わらない層までいちから作り直すこと）の話で、要件の段で Scope の In に入れる。完了 `areka-P0-animated-image-playback` の再描画の決まりは変えない。
  - 版: **v0.1.0** の中で解く（開発者の決め「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」）。
  - **直しの方向（開発者の指示・2026-10-23 チャット、逐語）**: 「うーん、てっきり動画のコマを事前にアトラス展開していると思ってたのですが。。。そういう感じでブリーフ出しておいてください。」
    - 動く絵のコマは、読み込みのときに一度だけ展開してアトラスへ置く。コマ替えでは、アトラスの中のどのコマを使うかを切り替えるだけにし、面をいちから合成し直さない（emo の自前の合成とアトラスの形に乗せる）。
    - 動く層の下と上にある変わらない層は、コマ替えのたびに作り直さない。
    - 確かめ: 何もしていないあいだの areka の CPU を、件 25 と同じ測り方で測り直す（release 版で SSP の 10% 前後を目安）。コマ替えの適用が `cache_hit=true` になる（または合成の段を通らない）ことを記録で確かめる。
    - コマを先に展開すると記憶が増える。その上限と、大きな動く絵のときの扱い（全部を先に展開するか、近いコマだけにするか）は、要件・設計の段で決める。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: 触るファイルの本体は動いていない（`crates/areka-emo-present/src/presenter/show.rs` 426 行・INFO 行の文は 402 行目・合成の控えの引き当ては 91 行目・`display.rs` 231 行の 174 行目の `CreateBitmap`・emo-compose の `plan.rs` 706・`lib.rs` 218・emo-atlas の `lib.rs` 683）。性能の道具 `tools/perf/judge-followup.py` 107 行目の注記は 402 行目に直った（棚卸㉙の穴 ⑷ は済み）。手順書 `doc/pasta-real-run.md` の「`apply(ShowSurface)` を外して読む」の注記は 274 行目。
- **触るファイル**: 棚卸㉙のとおり＝記録の `show.rs` と兄弟のテスト（`presenter_perf_log_tests.rs` 883・`presenter_film_tests.rs` 726 ほか）・`presenter/hub.rs`（281）・`presenter/timing.rs`（329）。重ね直し＝`display.rs`・`cache.rs`（342）・`presenter/target.rs`（184）・`presenter/budget.rs`（567）・層を分けるなら emo-compose の `lib.rs`（`compose_into`）と `plan.rs`。文を読む側＝`crates/areka/examples/` の 5 本・性能の道具 4 本（`judge-perf.py` 4156 行〔507 行目〕・`judge-followup.py` 931・`invoke-followup-checks.ps1` 865〔137 行目〕・`perf-rank.py` 957）と `tools/perf/fixtures/`・`doc/pasta-real-run.md`。
- **規模**: 10〜16 のまま（記録の量だけなら 2〜4）。
- **先に要るもの**: 働きの上で無し。条件つきだった重なりの決着: ⑴ `real-run-log-noise` とは emo-atlas の `lib.rs` で重ならない——動く絵のコマは読み込みのときにもうアトラスへ全部展開してあり（同じファイル 104〜110 行目）、件 25 の直しは「重ね直し」と「作り直し」＝emo-present（層を分けるなら emo-compose）の側。約束「本 spec は emo-atlas の `lib.rs` に触らない」で 0。⑵ `seriko-pattern-calls` は `move` で `plan.rs` に触る（ほぼ確か）＝本 spec が emo-compose（`plan.rs`・`lib.rs`・`pattern.rs`）に触るなら重なる。直しを emo-present の中（D2D の絵を作り直さず中身だけ書き替える・控えの形を変える）に留めると約束すれば 0。⑶ `mcp-strict-errors` の「再生まで印を運ぶ」案（`show.rs`）・`placement-measure-bake-once`（`shell_target.rs` を変える案なら控えの形を照らす）は棚卸㉙のまま。版の外: `present-emit-tail-latency`（`show.rs`・測ることが本体）・`animated-image-import`（`plan.rs`）。後の版の `audit-emo-present`・`audit-emo-compose`・`audit-tools-perf-judge`、v0.2.0 の `wintf-visual-tree`・`wintf-d2d-composite-rules`（合成を D2D へ移す＝ここで CPU の重ね方を作り込むと作り直しになりうる）。**何もしないあいだの CPU を前後で測るので静かな机が要る**。同じ面を出した SSP の基準を測るなら SSP の机。1 本だけの席・依存の追加は無い。
- **優先度の区分**: A のまま（開発者の指示＝ぱすたの申し送りの件 21・25）。
- **要件定義のモデル**: Opus のまま（常設の席に合成して表示へ渡す今の口の中身を変えるだけ・新しい境目・並行・時刻の約束は無い）。
- **分割の案**: なし（20 に届かない）。記録の量だけ先に着地させたいなら、件 21（記録）と件 25（重ね直し）に分けられる（共通は `show.rs` だけ）。
- **見つけた穴・古くなった記述**（棚卸㉙のうち今も正しいもの）: ⑴ 本文 Desired Outcome と件 25 の節の「読み込みのときにアトラスへ展開しておく」は、もう済んでいる。残る直しの形（変わらない層の控え・控えの数を増やす・D2D の絵の中身だけ書き替える など）は要件の段で開発者に聞く。⑵ 表示の行を毎回 INFO で出さなくすると、`judge-perf.py` の数え方と `invoke-followup-checks.ps1` の「表示が済んだ印」が変わる。⑶ 例のファイルは 4 本でなく 5 本（`emo-present/` の下の 2 本を含む）。⑷ `doc/pasta-real-run.md` は本 spec が書き換える（ほかの v0.1.0 の spec は入れ方を読むだけ）＝同じウェーブのほかの spec が手順書を直すなら別々の行に留める。

## 2026-10-30 件 26 を足す（まばたきの閉じた絵が残る・開発者の決め）

- 出どころ: 「初回起動トークの挙動」のセッションの調べ（2026-10-30）。開発者の決め（同日・本セッション・推奨の案を選んだ）「本 spec に件として足す」。件の番号は件 25 の続き（出どころの表の番号ではない）。
- 区分: B（バグ）。版は **v0.1.0**。門「窓際のぱすたさんが SSP と同じに動く」に直接かかる。
- **件 26（ぱすたさんの目が、まばたきの閉じた絵のまま戻らない）**:
  - 症状（確かめた事実）: areka（main `484263230` のままの release 版）で、ぱすたさん（面 1000）がまばたきの閉じた絵のまま戻らない。`dump_surface` で確かめた。ボールの動く WebP は動き続けている。
  - 目の着せ替えは正しく効いている（確かめた事実）: debug 版の記録の「`seriko: bind 適用`」は 目＝ノーマル・まばたき＝ふつう で終わる。`\![bind,まばたき,ふつう,0]\![bind,まばたき,閉じるだけ,0]` を送ると、すぐ目が開く。続けて `\![bind,まばたき,ふつう,1]` を送ると、約 20 秒後に閉じた絵になり、そのまま戻らない。
  - まばたきの定義（確かめた事実）: ぱすたのシェル `shell/master/surfaces.txt` 221〜227 行の animation1400 は `interval,bind+random,4`。pattern0 が overlay 1410（閉じた目と閉じた眉）0ms、pattern1 が overlay 1411（半目）120ms、pattern2 が `overlay,-1` 50ms。1401 は pattern0 が 1410、pattern1 が `overlay,-1,150`。SSP では `-1` で重ねた絵が消え、目が開く。
  - seriko の判定は対になっている（確かめた事実）: debug 版の記録で「`seriko: loop 抽選発火`」と、その約 170ms 後の「`seriko: loop 停止（負 surface でベース復帰・要件 4.3）`」が 70 回ずつ出ている。停止の道（`crates/areka-seriko/src/looper.rs` 293〜321 行・2026-10-30 に引き直して同じ行）は、コマを外し（310 行の `new_pattern.remove`）、再生の記録を外している（311〜313 行）。
  - 見立て（未確定）: 停止で外したまばたきの重ね絵が、ボールのコマ替えの載せ直しで古い状態のまま使われ、画面に残る。候補は、seriko の載せ直し（`looper.rs` 757 行の `refresh`。呼び手は `actor.rs` 494・688・755・806 行。688 行は着せ替えが替わった直後の道）か、表示の側の合成（控えの引き当てや、本 spec の重ね直し）。本 spec のコマ替えの重ね直し（タスク 2.x・3.3）と同じ場所にかかる。
  - 同時に見えたこと（事実だけ・関係は確かめていない・本件の範囲ではない）: 同じ記録に「`seriko: mustselect カテゴリの脱衣指示を無視`」（`actor.rs` 676 行の `warn!`）が大量に出ていた。ぱすたのシェルは 口・目・眉・うで が mustselect。
  - 確かめ: 失敗する決定論テストを先に立ててから直す。実機では起動から 6 分、まばたきが 閉じる → 半目 → 開く と進み、閉じた絵が残らないことを、`dump_surface` で何度か撮って確かめる。debug 版と release 版の両方。
  - 範囲: 原因を 1 か所で直す。原因が本 spec の境界の外にあると分かったら、直さずに止まって報告する。

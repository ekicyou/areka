# audit-seriko 6.1 audit のかけ直し（4 回目）

- HEAD: `f80f6437`・枝 `claude/kiro-start-audit-seriko-8ceffb`
- 道具: `ponytail:ponytail-audit`。範囲は `crates/areka-seriko/` だけ。要件 1.2・1.3・5.1 に従い、指摘の数に上限を置かずに全部を挙げた。「見ていない」ファイルは無い
- 範囲: 88 ファイル・28,781 行
  - 本番 24 ファイル 8,532 行
  - テストとテストの道具 61 ファイル 20,055 行
  - テスト用の素材 2 ファイル 161 行
  - `Cargo.toml` 33 行
- 読み方
  - 本番の 24 ファイルは、取りまとめ役が 1 行目から最後の行まで読んだ（`looper.rs`・`looper_trigger.rs`・`looper_calls.rs`・`looper_refresh.rs`・`parts.rs`・`parts_trigger.rs`・`parts_calls.rs`・`parts_refresh.rs`・`trigger.rs`・`calls.rs`・`timeline.rs`・`talk.rs`・`actor.rs`・`actor_bind.rs`・`settle.rs`・`output.rs`・`resolve.rs`・`lib.rs`・`table.rs`（中のテストを含む））。`bind.rs`・`state.rs`・`table_calls.rs`・`shell_fallback.rs`・`skin_facts.rs` は読み手 D が全部の行を読み、取りまとめ役は指摘に関わる行を開いて確かめた
  - テストの 61 ファイルと素材 2 ファイルは、4 人の読み手で分けて全部の行を読んだ。読み手の挙げた件は、取りまとめ役が該当の行を開いて確かめた
  - 「壊しても緑のまま」は読みで確かめたもので、摂動は回していない
- 動かしたもの（コードは 1 バイトも変えていない）
  - `cargo clippy -p areka-seriko --all-targets --no-deps -- -D warnings`: 警告 0
  - `cargo doc -p areka-seriko --no-deps`: 警告 0
  - `cargo test -p areka-seriko`: 573 件緑（546＋5＋9＋9＋4＋0）
  - 全体テスト・`--ignored`・areka-impl-watch.exe は回していない
- 扱わないもの: research.md §3 の 1〜20（18・20 は「残す」の裁定どおり）、reaudit-1.md の R1〜R26、reaudit-2.md の S1〜S24、reaudit-3.md の T1〜T15。どれも直し済み

## このクレートがすること・想定した負荷

SERIKO（シェルの面のアニメーション）を、ゴーストごとの役者のスレッドで 16ms ごとに進め、表示の指令を出す。台本の合図（`\s`・`\b`・`\![bind]`）を受けて、面と着せ替えを替える。想定した負荷は、利用者 1 人・ゴースト数体・役者 1 体あたり毎秒約 60 刻み。

## round 3 の直しを確かめた結果

- 本番のバグは見つからなかった。確かめた所は次のとおり。
  - `on_tick` の 3 つの輪（1 つ目は引き金と now までの呼び出し、2 つ目は抽選、3 つ目は `always` と now に始まった再生の頭のコマと進行と部品）
  - 部品の刻みの順（構えてあった部品の引き金 → `run_part_calls(now)` → 抽選・`always` → `run_part_calls(now)` → コマを置く）
  - `refresh`（一番上・部品）が 3 語を判定し、`advance_talk` で数えを進めること
  - `Armed::commit` が、再生中でも `runonce` に印を付けること
  - `talk` の開始の時刻を `.max(since)` で見え始めより前にしないこと
- 出来事の時刻が刻みより前に来る場合も確かめた（窓の知らせは UI の時計で読むので、すでに処理した刻みより前の時刻になりうる）。どれも問題は無かった。
  - 周の数え（`last_lap`）は進むだけ
  - 当て終えた印（`acts_through`）は戻らない
  - `advance_talk` は `max` で進める
- round 3 で足した・期待を変えたテストは、どれも直しを戻すと赤になる（T1・T2・T3・T4・T5・T6・T7 の一番上と部品）。名前と doc も中身と合う。
- `trigger_tests.rs:41-50` の `poll` は、本番の `next_start` → `commit` の組と今も同じ。
- 1,000 行の上限: どのファイルも越えていない（最大は `looper_tests.rs` の 981 行）。

## 指摘

### 必ず直す（0 件）

無い。

### 直したほうがよい（13 件）

**U1. 合図の作り直し（`refresh`）で鳴り直した一番上の再生が先頭の待ちの前のとき、古い残りのコマを消さない。刻みの道は消すので、同じ時刻で答えが食い違う**（`src/looper_refresh.rs:150-163`）
- 何が悪いか: 頭のコマを置く輪は、鳴らした・呼ばれた再生（`heads`）が `Active`・`FinishedResidual` のときだけコマを置く。`Pending`（先頭の待ちの前）と `Stopped` では何もしない。そのため、前の再生が残した最後のコマ（`IdleResidual`）が欄に残る。刻みの道（`put_top_play`）は `Pending` で欄を外す（`looper.rs:279-281`・モジュールの doc の「再発火の瞬間、残留コマは即時クリアされ」）。部品の `refresh` も、鳴った頭を `unwrite` してから置く（`parts_refresh.rs:121`）。同じ決まりを、一番上の `refresh` だけが違う形で当てている。
- 具体例: `periodic,1` の P のコマが「301（待ち 50）→ 302（待ち 50）」で、前の周の再生は 1100 に終わり、302 が欄に残っている。2010 に同じ面の着せ替えの合図が来る。round 3 の直しで、`refresh` が P を境目 2000 で鳴らすようになった。2010 の経過は 10 で `Pending` なので、`refresh` が出す `Show` には古い 302 が載る。同じ 2010 を刻みが処理していれば、欄は空になる。次の刻み（2016 ごろ）で消えるが、それまでの 1 刻みのあいだ古い絵が出る。round 3 より前は、`refresh` は新しい面の `runonce` しか鳴らさなかったので、この道にはほとんど来なかった。
- いちばん小さい完全な直し: `else if heads.contains(&anim.id)` の枝を `match frame_at(..) { Active(i) | FinishedResidual(i) => put_visible(..), _ => pattern.remove(anim.id) }` にする。上の具体例をテストにする（2010 の `refresh` の答えで欄 0 が `None`）。
- 種類 3.1: (1) 要件の文の衝突には当たらない。(2) `shortcut:` の注も無い。(3) 正典との衝突にも当たらない。推奨は「直す」。

**U2. 「2 回目は鳴らない」の輪が、鳴らないことを確かめていない（一番上）**（`src/looper_trigger_tests.rs:70-80` `runonce_plays_once_from_the_event_time_on_first_show`）
- 何が悪いか: 確かめるのは、毎刻みの `started` と欄が `None` であることだけ。
- 具体例: `trigger.rs:160-162` の `self.fired.insert(anim.id)` を消しても緑のまま。1250 からの刻みごとに、`runonce` が見え始めの 1030 で鳴り直す。しかし経過は 220 以上で、200 の `-1` を過ぎているので、同じ刻みのうちに再生が捨てられる。`started` も欄も `None` のまま。
- 直し: 輪を `capture_logs` で包み、`count(&logs, "trigger runonce を鳴らした") == 0` を確かめる。

**U3. 同じ穴が部品の側にもある**（`src/looper_parts_tests.rs:290-293` `part_visible_across_a_surface_switch_keeps_its_runonce_used`）
- 何が悪いか: 「構え直していないのでもう鳴らない」を、欄でしか見ていない。
- 具体例: U2 と同じ摂動で緑のまま。1400・1500 の刻みで、部品の `runonce` が見え始めの 1000 で鳴り直す。しかし 300 の `-1` を過ぎているので、その刻みのうちに時計が消える。
- 直し: 1100・1400・1500 の刻みを `capture_logs` で包み、「part trigger runonce を鳴らした」の行が増えないことを確かめる。

**U4. 部品の `refresh` が `periodic` を先に判定するテストが、「始まって止まった」と「始まらなかった」を見分けない**（`src/parts_calls_tests.rs:432-446` `part_refresh_judges_periodic_before_applying_calls`）
- 何が悪いか: 確かめるのは、1100 で欄が `None` であることだけ。
- 具体例: `parts_refresh.rs:132` の `state.follow(open, at, None)` を `state.show(at, None)` に替えても緑のまま。周の起点が 1010 へ動き、1100 までに `periodic` は鳴らない。欄はそれでも `None` になる。
- 直し: `refresh` を `capture_logs` で包み、`animation_id=1`・`started_at_ms=1000` の「part trigger periodic を鳴らした」が 1 行あることを確かめる。

**U5. テストの時計が逆に進む（T8 と同じ種類）**（`src/actor_clock_tests.rs:165-176` `run_actor`・`:231-236`）
- 何が悪いか: 時計つきの回では、時計が `|| 1030` に固定されている。`run_actor` は刻み 0 → `\s[0]` → 刻み 1100 → `\s[1]` を流すので、2 つ目の合図は、刻み 1100 を見た後に時刻 1030 で処理される。`ClockRig` の約束（観測を追い越さない）を破る。
- 具体例: 今は面 1 に animation が無いので、`(1, None)` が出て緑になる。面 1 に `always` があれば、すでに見た 1100 より前の 1030 に始まる。
- 直し: 時計を `AtomicU64` にして、2 つ目の合図の前に 1100 以上へ進める。または `run_actor` から `\s[1]` を外して、期待の列を縮める。

**U6. 部品のテストの時計も逆に進む**（`src/parts_tests.rs:506-534` `peek_draws_no_rng_and_leaves_clocks_unchanged`）
- 何が悪いか: `for _ in 0..3` の輪の中で、`refresh_at` を 1000 → 1060 → 1100 → 1100 の順に呼び、次の周でまた 1000 から呼ぶ。
- 具体例: 今は HOLD に働きのコマが無いので、`run_part_calls` がすぐ戻り、時計が書き換わらないので緑になる。round 3 の直しで `refresh` が引き金と呼び出しを当てるようになったので、働きのコマがあれば時刻が戻る道を通る。
- 直し: 時刻ごとに 3 回くり返す形にする（`for at in [1000, 1060, 1100] { for _ in 0..3 { refresh_at(.., Some(at), ..) } }`）。

**U7. `arm_part` の「在れば窓の開け閉めを写す」の枝は、もう通らない。doc は偽**（`src/parts_trigger.rs:21-23`・`:36-41`）
- 何が悪いか: `judge_part` の呼び手は 2 つとも、構えてある部品を先に外す（`parts.rs:583`・`parts_refresh.rs:100`）。round 3 の前は、部品の `refresh` の 2 段目だけがこの枝を通っていた。
- 具体例: `Entry::Occupied` の腕を `unreachable!()` にしても、どの入力でも何も変わらない。
- 直し: `armed.entry(part).or_insert_with(|| { debug!(..); Armed::arm(at_ms, open, None) })` にし、doc の「在れば…」を消す。関係する古い doc も直す。
  - `parts_trigger.rs:149` の「2 段目（見えると決まった部品 1 つ）」は、「新しく見えた部品 1 つ」にする。
  - `parts.rs:327-328` の「（`advance` では新しく見えた部品だけ」は、今は `refresh` も同じなので、「`advance` では」を消す。

**U8. 「`refresh` は抽選の時計を書き換えない」は偽**（`src/parts.rs:31-33`・`src/parts_refresh.rs:27-29`・`src/looper_refresh.rs:24-25`）
- 何が悪いか: 部品の `refresh` は `run_part_calls` を回す（`parts_refresh.rs:148-150`・`:168-172`）。`start`・`stop` の働きのコマは、相手が抽選の animation でも、その時計を作る・消す。`looper_refresh.rs:24-25` の「抽選の時計と乱数は触らない」も同じで、さらに選びの乱数（`pick_rng`）は引く。
- 具体例: 部品の `always` 0 の頭に `start,1` があり、1 は `random,2`。`refresh(Some(700))` で、抽選の animation 1 の時計 `Playing(700)` が生まれる。
- 直し: 3 か所に「（働きのコマが始めた・止めた時計を除く）」と足す。`looper_refresh.rs` は「抽選の時計と抽選の乱数は触らない（働きのコマの始め・止めと選びの乱数を除く）」にする。

**U9. 「無変形で受け渡す」を確かめていない**（`src/actor_dispatch_tests.rs:7-46` `emit_forwards_cue_to_inbox`）
- 何が悪いか: doc は、受け口が合図を無変形で受け渡すと言う。しかし確かめるのは `at`・`actor`・`command` だけで、2 件とも `duration` が 0.0 になっている。
- 具体例: `deliver` が `duration` を 0 にしても、落としても緑のまま。`duration` は、担当外の合図でも守る約束の中身そのもの。
- 直し: Text の合図に 0 でない `duration` を与え、受け取った `TalkCue` を丸ごと `assert_eq!` で比べる（`TalkCue` は `PartialEq` を持つ）。

**U10. 「経路を 1 つだけ立てる」と言う檻の着せ替えの例が、2 つの経路を立てる**（`src/table_parts_tests.rs:5-8`・`:55`・`:78`）
- 何が悪いか: 素の `bind` の animation は、今は `CallOnly` として表に載る。そのため、描くコマの経路（`table.rs:458-463` の `framed`）でも 200 が数えられる。
- 具体例: `table.rs:453` の `.chain(p.bind_targets.iter().map(..))` を消しても、全部の例が緑のまま。
- 直し: 例を `animation5.interval,bind+random,0` にする。`k == 0` なので seriko は採らないが、入れ子の表は pattern0 の先を着せ替えの先として持つので、この経路だけが立つ。`table.rs:453` には「表に採らない `bind+random,0` のための経路」と 1 行足す。

**U11. 「`insert` と読めない相手は待ち 0」を確かめていない**（`src/table_calls_tests.rs:22`・`:24`・`:30`）
- 何が悪いか: `insert,3` と `start,abc` は待ちの欄を書いていないので、もともと 0 になる。読み手は待ち・X・Y を語に依らず読む。
- 具体例: `table_calls.rs:74-77` を `FrameAct::Move | FrameAct::Inert => (p.wait, p.x, p.y)` にしても緑のまま。
- 直し: `animation0.pattern7,insert,3,70,5,6` と `animation0.pattern9,start,abc,70,5,6` にする。期待の `waits` の列は変わらない。

**U12. 重なったテスト**（`src/actor_bind_loop_tests.rs:848-890` `bind_name_gate_other_name_is_benign_debug_no_emit`・`src/actor_bind_noevent_tests.rs:234-253` `prefix_alike_name_is_not_bind`）
- 何が悪いか: どちらも「名前が bind・bind-noevent でない正準のキャリアは、debug で読み飛ばし何も出さない」の同じ道を通る。2 本目の確かめは 1 本目を全部含む（全部の行が DEBUG・出ない・着せ替えは変わらない・名前つきの読み飛ばしの行が 1 本）。
- 具体例: 名前の門を壊すと 2 本とも赤になる。1 本目だけが赤になる壊し方は無い。
- 直し: 1 本目を消す。または 2 本目を `["bind-noeventx", "move"]` の輪にする。

**U13. 動く絵の子を持つ世界を組む手書きが 6 か所にある（S15・T14 の漏れ）**
- 同じ組み方が、次の 6 か所にある。どれも、解析 → `SurfaceMaster` を引く → 道で `e.kind = ElementKind::Film(..)` を入れる → `FilmSheet` を手で書いて `from_world_and_films` に渡す、の順に組む。
  - `src/settle_tests.rs:73-94`（doc に「`parts_film_tests.rs` と同じ組み方」とある）
  - `src/table_film_tests.rs:47-63`・`:159-166`
  - `src/parts_film_tests.rs:49-67`
  - `src/looper_film_count_tests.rs:26-46`
  - `src/table.rs:935-941`
- `settle_tests` の絵の列は、`looper_film_count_tests::one_film_table` と同じ（`FilmId(10)`・`a.png`・コマ 10/11/12・各 100ms）。
- 直し: `lib_test_support.rs` に `film_world(text, &[(path, FilmId)]) -> EmoWorld`（と、要るなら `film_sheet(..)`）を 1 つ置き、写しを替える。

### できれば（7 件）

**U14. 刻みの順を書いた 2 つの doc が、round 3 で足した 1 つ目の輪を書いていない**（`src/looper.rs:4-7`・`src/lib.rs:18`）
- 中身: `looper.rs` のモジュールの doc は「(1) 単調性ガード → (2) 抽選 → (3) 進行 → (4) 反映」、`lib.rs` は「`on_tick`（抽選→進行→PatternState 差分）」のまま。今は抽選の前に、引き金と呼び出しを now まで当てる輪がある（`on_tick` の doc には書いてある）。
- 直し: 2 か所に「抽選の前に引き金と働きのコマを今の時刻まで当てる」を足す。

**U15. 番号の spec を取り違えている**
- `src/timeline_calls_tests.rs:109` の「要件 4.3」: モジュールの doc は seriko-pattern-calls を指すが、そこの 4.3 は `alternativestop` の決まり。言いたいのは「`-1` で止めてベースへ戻す」で、`areka-P0-seriko-loop` の要件 4.3。
- `src/looper_calls_tests.rs:677` の「seriko-pattern-calls 要件 3.1・要件 2.3」: seriko-pattern-calls の 2.3 は `[ ]`・`.` の並びの決まり。言いたいのは「再生中は抽選し直さない」で、`areka-P0-seriko-loop` の要件 2.3。
- `src/actor_replace_tests.rs:377` の「（要件 3.3）」: モジュールの doc は areka-P0-shell-balloon-switch を指すが、`log_shell_fallback` の決まりは descript-window-keys 要件 3.3（`actor.rs:427`）。
- 直し: spec の名を添える。

**U16. どの spec のものか分からない番号（T11 の漏れ）**
- `src/actor_dispatch_tests.rs`: モジュールの doc が spec を名指さないまま、R1.1・R11.6・「ケース1」〜「ケース14」・D4 を使う。
- `src/actor_bind_loop_tests.rs`: ケース15〜22・D1・D5・D7・D8①〜⑤・R3.5・R7.1・「要件 14.2」（`:777`）を使い、「bindopt」（areka-P0-bindoption-exclusivity）を一度も書き下さない。
- `src/actor_tick_tests.rs:14`: R1.1・6.1・6.3・7.1・7.5・8.3。
- `src/parts_tests.rs:445`: 「── 7.3: …」。
- 直し: 各ファイルに、spec の名を書いた `//!` の行を 1 つ足す（「bindopt＝areka-P0-bindoption-exclusivity」を含む）。`parts_tests.rs:445` は「areka-P0-surface-element-nesting tasks.md 7.3」にする。

**U17. 前の版と比べる語の残り（T11 の漏れ）**
- テスト
  - `src/actor_bind_loop_tests.rs:181-182`「是正前は…握り潰していた」・`:280`・`:480`「既存流儀」・`:325`「従来どおり」・`:411-412`「既存流儀の維持」「従来の単一発行点」
  - `src/actor_dispatch_tests.rs:333`「既存 `emote_cue` 経路は無改変」・`:542`「既存 per-scope 冪等ガード」
  - `src/actor_tick_tests.rs:20`・`:64`「既存 emit_display 単一発行点」
  - `src/bind_tests.rs:715`「今の振る舞い」・`src/bind_forms_tests.rs:143`「今どおり」・`:191`「今の当て方」・`src/table_calls_tests.rs:102`・`:124`「今のまま」・`src/table_interval_words_tests.rs:63`「今日どおり」・`src/table_film_tests.rs:128`「今の列」
  - テストの名: `settle_tests.rs:198` `wait_at_the_cap_settles_as_before`・`table_always_tests.rs:95` と `table_trigger_tests.rs:105` の `..._keep_the_existing_empty_warn`
- 本番: `src/bind.rs:29`・`:41`「今の振る舞い」・`:557`「今の当て方」・`src/actor_bind.rs:35`「今の `apply_named_bind`」・`src/looper.rs:721`「従来どおり」・`Cargo.toml:17`「旧 `SurfaceSink` 経路は撤去して一本化」
- 直し: 比べる語を消し、今の振る舞いだけを書く（例「排他置換でも変わったときだけ出す」「単一発行点 `emit_display` から」）。テストの名は `wait_at_the_cap_keeps_the_last_frame`・`..._fall_to_the_empty_warn` などにする。

**U18. 移した・寄せた後に古くなった見出しと説明**
- `src/actor_bind_loop_tests.rs:9-17`: 見出しは「④宛名規律 non-canonical」の檻をこのファイルが持つと言う。しかし f80f6437 の後、その檻は `actor_bind_noevent_tests.rs:256-316` にある。
- `src/actor_bind_noevent_tests.rs:1-5`: 見出しは「bind-noevent は bind と同じに振る舞う」だけで、移してきた他人宛の名のテスト（比べではない）を含まない。
- `src/actor_test_support.rs:9`「捕捉の道具は…1 か所の定義」と `src/state_test_support.rs:3`: 再輸出の行が、捕捉の道具でない `cue`・`text`・`clear_all`・`fresh_states` も持ち込む。
- 直し: bind_loop の ④ を「（④ は `actor_bind_noevent_tests.rs`）」にする。noevent の見出しに 1 行足す。2 つの道具のコメントは「テストの道具は…」にする。

**U19. 確かめていない理由を言う文言**
- `src/trigger_tests.rs:101`: `poll(.., 1266, |_| true, None)` が `None` になるのは、100 行で `runonce` の印が付いたからで、再生中だからではない。それなのに文言は「再生中」と言う。`commit` が `playing` を無視しても、この行は緑のまま。直し: 文言を「鳴らし済み」にする。または `|_| false` を渡す。
- `src/looper_trigger_tests.rs:540-541` `talk_elsewhere_in_the_table_arms_without_top_judgement`: doc は「刻みごとの一番上の判定も生まれない」と言うが、どこにも観測が無い。`looper.rs:524` の `.filter(|_| has_trigger)` を消しても緑のまま（面 0 に 3 語が無いので、空の列が渡るだけ）。直し: doc を、確かめている中身（構える・再生が無い・指令が無い）に縮める。

**U20. 小さなテストの道具の写し（T14 の漏れ）**
- `StageNote::Balloon { scope: scope(), open, face: 0, generation: 0 }` を組む手書きが 5 か所にある。
  - `src/looper_trigger_tests.rs:571`・`:650`
  - `src/looper_calls_tests.rs:643`
  - `src/looper_balloon_tests.rs:21`（`fn note`）
  - `src/looper_talk_tests.rs:246`
- `rt.parts.clock_at(&scope(), slot, PartKey::Film(film), 0)` の閉包が `src/looper_always_tests.rs:167`・`:215`・`:264` に 3 つある。`looper_balloon_tests.rs:41` の `fn clock` と同じ。
- `src/actor_replace_tests.rs:25-29` の `arm_resolver` は、`actor_bind_loop_tests.rs:22` の `arm_bind_resolver` の写し（ID だけ違う）。`:32-34` の `no_facts()` は `src/actor_calls_tests.rs:116` にも手書きされている。
- `src/parts_always_tests.rs:8`・`:206-214` と `src/parts_film_tests.rs:14`・`:192-222` は、`capture_logs` を使わずに `log_capture_kit::capture_lines` を直に呼び、`l.contains(a) && l.contains(b)` の手の絞りで `parts_tests::count` と同じことをしている。
- 直し: `note_balloon`・絵の子の時計の道具を `looper_test_support.rs` へ置く。`arm_bind_resolver`・`no_facts` は 1 つを使い回す。`capture_logs` と `count` に替える。

## 参考（指摘にしない）

- `src/state_surface_tests.rs:158-160`・`:177-179`: `fresh_states()` と同じ値を手で書いているが、テストは「渡した値をそのまま返す」を確かめるので、入れた値が見えるほうが読みやすい。残してよい。
- `use crate::test_support::count;` が `{...}` の行と別の行になっている（`looper_calls_tests.rs:13-14`・`looper_trigger_tests.rs:17-18`）。`actor_dispatch_tests.rs:115` の `use` がファイルの途中にある。
- `crate::test_support::capture_logs` を、取り込み済みなのに完全な道で呼んでいる（`looper_calls_tests.rs:463`・`parts_calls_tests.rs:363`）。
- 「着せ替えなし」の書き方が 4 通りある（`from_ids([])`・`default()`・`DefaultBinds::default()`・局所の閉包）。
- `parts_tests.rs:654` の `count(lines, &[&str])` が、`lib_test_support::count(lines, &str)` と同じ名で引数の形が違う。
- `parts_calls.rs:4` の「2 回」: `fire_part_triggers`・`judge_part` も `run_part_calls` を呼ぶ。
- `trigger.rs:108`: 折り返していない長い doc の行。
- `lib_test_support.rs:23`「中身は同じフォルダの README」: README は `animated-playback/` にあり、`shell/` には無い。`lib_test_support.rs:148` の `part_frames` の doc は「コマの番号」と言うが、返すのはコマの `surface_id`。
- `table_parts_tests.rs:5-8`「4 つの経路」: 5 つ目（動く絵の子）は `table_film_tests` が確かめる。
- `actor_bind_noevent_tests.rs:34` の `run` の doc「スコープ 0 に」: `run` はスコープを受け取り、`"x"` でも呼ばれる。
- `looper_always_tests.rs:237-238`: `rng`（部品）と `loop_rng`（表）の名が取り違えやすい。
- 読み手 D の見込み（届くかは確かめていない）: スコープ `"01"`・`"+2"` では、`DefaultBinds::for_scope` は数として読み、`scope_namespace` は `None` を返す。届く道が確かめられないので、指摘にしない。

## まとめ

| 組 | 件数 | 番号 |
| --- | --- | --- |
| 必ず直す | 0 | — |
| 直したほうがよい | 13 | U1〜U13 |
| できれば | 7 | U14〜U20 |

- 新しい指摘は 20 件で、0 件ではない。件数は 26 → 24 → 15 → 20。必ず直すは 4 → 0 になった。本番のコードの振る舞いの指摘は U1 だけで、round 3 の直し（`refresh` が `periodic`・`talk` も鳴らす）で届くようになった小さな食い違い（古い絵が 1 刻み残る）。
- round 3 の直しが生んだもの: U1（上のとおり）、U6（`refresh` が時刻の戻る道を通るようになった）、U7（`arm_part` の枝が死んだ）、U8（`refresh` の doc を直した行に「だけ」が残った）。
- 残す裁定の候補: どれも 3.1 の (1)〜(3) に当たらない。推奨はすべて直すこと。
- 見ていないもの: 無い（88 ファイルを全部読んだ）。「壊しても緑」は読みで確かめたもので、摂動は回していない。

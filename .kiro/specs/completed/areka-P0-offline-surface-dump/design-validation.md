# 設計の検証: areka-P0-offline-surface-dump

- 日付: 2026-10-22
- 対象: `design.md`（913 行）・`requirements.md`・`research.md`・steering（product・tech・structure・logging）
- やり方: kiro-validate-design の手順（分析 → 重い問題 → 良い点 → GO／NO-GO）。設計の決めごとが今のコードに頼る所は、ソースを読んで確かめた。

## まとめ

設計は要件 1〜14 をすべて部品に割り当てており、新しいモジュール（`offline_dump`）を骨にして、シェルの読み込み・合成・SERIKO の進め方は今の部品を呼ぶだけ、という素直な形になっている。コードで確かめた前提（`main.rs` の順番・`handle_message` が同じスレッドの関数であること・乱数を注入できること・本番の刻みが 16ms であること・記録の受け口がスレッドを起こさないこと）はどれも正しい。残る問題は、作りを変えるものではなく、タスクに分けるときに決めておかないと実装の途中で詰まる 3 点である。**判定は GO**（下の 3 点をタスクの生成で手当てすることが条件）。

## コードで確かめたこと

| 設計の前提 | 確かめた場所 | 結果 |
|---|---|---|
| `--help` の直後で戻れば、記憶・MCP・窓・GPU に触れない | `crates/areka/src/main.rs` 160〜193 行（`log_history::init` → `cli::read` → `help` → `thread_roles::install` → `perf_thread_report::start` → `resolve_boot`） | 正しい。分岐はスレッドを起こす前に置ける |
| `main.rs` は 965 行・足すのは 5 行ほど | 実測 965 行 | 約 970 行で 1,000 行の内。ただし余りは 30 行ほど |
| `handle_message` は crate の中の同期の関数 | `crates/areka-seriko/src/actor.rs` 432 行 | 正しい。`actor_test_support.rs` の `ClockRig` が同じ形で回している |
| 乱数は注入でき、random と BindRandom だけが引く | `looper.rs` 474〜530 行・`timeline.rs` の `should_fire`（`rng(k) == 0`） | 正しい。`always`・`runonce`・`periodic`・`talk` は乱数を引く前に飛ばす。`|_| 1` なら抽選は当たらない |
| 本番の刻みは 16ms | `crates/areka-ghost/src/ticker.rs` 262 行（`LoopTickerConfig` の既定） | 正しい |
| シェルの面は常に「見えている」扱い | `state.rs` の `stage_slots`（シェルは `open = true`） | 正しい。窓の知らせ（`Stage`）を送らなくても `always` と runonce が動く |
| `\s[N]` の合図で runonce がその時刻から始まる | `looper.rs` 760〜830 行（合図の時刻で `arm_slot` と `fire_top_triggers`） | 正しい。時刻 0 の合図から始まる |
| 記録の受け口はスレッドを起こさない | `log_history.rs` 409〜416 行（標準出力の層＋履歴の層だけ） | 正しい |
| シェルの読み込みの道にスレッドが無い | areka-emo-present・areka-emo-atlas・areka-emo-compose・areka-parsers・areka-seriko の本番のソースに `thread::spawn`・`rayon` が無い | 正しい。写しの窓に他のスレッドの記録が混ざる道は今は無い |
| 読み込みで面の表を 1 度組んで捨てている | `shell_target.rs` 462 行（`probe`）と 236 行（`build_world`） | 正しい |
| `shell_candidates`・`SkinCandidate`・`resolve_skin_target`・`GhostNames`・`MockSurfaceOutput`・`png_base64` が在る | 各ファイル | どれも在る |
| スコープ 2 以上の名前空間が `None` | `bind.rs` 229 行 `scope_namespace` | 正しい |
| 着せ替えの持ち越しはスコープ 2 で全部落とす | `shell_carry.rs` 127〜150 行・`shell_carry_tests.rs` 175 行 | 要件 14 の後も、名前の表が空ならその試験は同じ結果のまま通る |

## 重い問題（3 つ）

### 問題 1: 要件 7.5「1 つの誤りは 1 行」が、合成のときに出る記録を見ていない

**中身**: 設計は、読み込みで面の表を 2 度組むことによる重なりだけを `take_world`（ShellWorldOnce）で防いでいる。ところが合成（`Composer::compose`）は、呼ぶたびに次の記録を出す。

- `plan.rs` 360 行（入れ子の循環）・433 行（知らない animation-sort）・465 行と 534 行（Overlay でないメソッド）
- `plan_extent.rs` 78 行（外形を数えるときの入れ子の循環。命令の側の 360 行と別の文で、同じ合成の中で 2 行出る）
- `blit.rs` 90 行（まだ無い合成メソッド）

element定義で置かれたサーフェスは、それを置く親の面を合成するたびに辿られるので、子の面の 1 つの問題が、親の面の数だけ同じ本文で CSV に並ぶ。入れ子の循環は、読み込みの `log_nest_issue` でも 1 行出ているので、合わせて 3 行になる。`settle_surface` は面ごとに新しい `LoopRuntime` を作るので、「1 つの統括器につき 1 度だけ」の warn（`warned_negative` など）も、部品として共有されたアニメでは面ごとに出うる。

**影響**: 設計の E2E の試験「同じ本文の行が無い」（Testing Strategy の E2E 2 つ目）は、入れ子のある検体で赤になる。逆に試験の検体がたまたま入れ子を持たないと、この重なりは試験に映らずに残る。

**直し方（どちらかを設計の文として決める）**:
- 案 A（推し）: 要件 7.5 の文どおり「同じ表を組み直したための重なり」だけを防ぐ対象とし、「合成は面ごとの記録で、ほかの面を書き出すときに同じ文が出るのは別の記録」と設計に書く。E2E の「同じ本文の行が無い」は、読み込みの段の記録に絞り、入れ子の検体では親の数だけ出ることを固定する。
- 案 B: SSP 2.9.23 で、入れ子の子の問題を持つシェルを書き出して何行出るかを測り、1 行なら、合成の側の記録を「1 回の走りで同じ (surface_id, 種類) は 1 度」にする口を合成に足す（`log_history` の写しの側で間引くのは要件 7.5 の「間引かない」に反する）。

### 問題 2: 要件 14 の型の付け替えの範囲が、試験のファイルを数えておらず、今の振る舞いを保つ組み立て方が無い

**中身**: `ScopeStates::new`・`spawn_seriko(_clocked)`・`SerikoReplace::Shell`・`static_binds` を使うファイルは 58 本ある。設計の Modified Files に挙がるのは本番のファイル 13 本ほどで、試験のファイルは 1 本も挙がっていない（areka-seriko の `tests/cue_sequence.rs`・`bind_e2e.rs`・`loop_integration.rs`・`regression.rs`・`balloon_face_e2e.rs`、`actor_*_tests.rs`・`looper_*_tests.rs`・`state_*_tests.rs` の約 20 本、areka の `emo2_boot` の試験とその支え 15 本ほど、`crates/areka/tests/emo2_real_run.rs`）。

`DefaultBinds` の組み立て方は `from_groups` と `single` の 2 つだけである。今の試験には「スコープ 1 の Show も同じ既定の組を載せる」ことを固定しているものがある（`crates/areka-seriko/tests/cue_sequence.rs` 70 行・「数値 `"10"` @ scope "1" → `binds:<static>`」）。`single(0, ..)` に書き換えるとこれが赤になり、試験の期待を 1 本ずつ書き直すことになる。

**影響**: タスクの大きさの見積もり（research.md の M〜L・14〜18 タスク）が、この付け替えの分だけ小さく出ている。並走のウェーブを組むときの「触るファイル」も足りない。

**直し方**: `DefaultBinds` に「どのスコープも同じ組」の組み立て方（例 `DefaultBinds::uniform(BindSet)`）を 1 つ足し、今の試験はそれで機械的に移す（期待は変わらない）。本番の `assets.rs` だけが `from_groups` に替わる。設計の Modified Files に試験のファイルの群を足し、タスクでは「型の付け替え（試験はすべて `uniform` で移す・期待は変えない）」と「画面の既定をスコープごとにする（`assets.rs` と新しい試験）」を別のタスクに分ける。

### 問題 3: `BindGroupName::line` をどこから取るかが決まっておらず、`resolve.rs` が 1,000 行に近い

**中身**: 要件 5.2（`__ALL__` を descript.txt に書かれた順で当てる）のために `BindGroupName::line` を足すとしているが、今の `read_bindgroup_defaults`（`crates/areka-parsers/src/package/resolve.rs` 70 行）は `parse_kv` の `BTreeMap`（キーの昇順）を回しており、行の番号は既に失われている。行の番号を得るには、`parse_kv` を変える（ほかの読み手すべてに効く）か、行の番号つきの読みを別に足すかが要るが、設計はどちらとも書いていない。

合わせて `resolve.rs` は今 866 行で、ここに `char{N}.bindgroup{M}` の 2 つの番号のキーの読み（`default`・`name`・`bindoption.group` の 3 種類）と行の番号の扱いを足すと、1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）に近づくか超える。

**直し方**: 設計に「`read_bindgroup_defaults` だけが使う、行の番号つきの読み（`parse_kv` は変えない）」と書き、着せ替えの読みを兄弟のファイル（例 `resolve_bind.rs`）に移してから `char*` を足す、と決める。`parse_kv` と同じく値の前後の空白を取ること・同じキーは後勝ちで行の番号も後の行になることを、試験の分岐に入れる。

## 小さめの気付き

1. 写しの窓はプロセス全体に 1 つである。今は書き出しの間に他のスレッドが無いことを確かめたが、それは「呼ぶ部品がスレッドを起こさない」という今の事実に頼っている。窓を開けたスレッドの番号を覚えて、ほかのスレッドの記録は写さない、と 1 行足せば、将来 WIC やほかの部品がスレッドを使っても CSV が汚れない。
2. `ShellTarget` は `#[derive(Debug)]` で、`EmoWorld` は `Debug` を持たない。`Option<EmoWorld>` の欄を足すには `Debug` を手で書くか欄を飛ばす必要がある。また画面は `take_world` を呼ばないので、読み込みで組んだ面の表が `ShellTarget` の寿命のあいだ使われずに残る（シェル 1 つにつき面の表 1 つ分のメモリ）。画面の側で捨てるか、承知の上と書く。
3. 画面の刻みは絶対の格子（`BoundarySchedule`）に揃っているので、`\s[N]` から最初の刻みまでの位相は画面では 0〜16ms のどれかになる。書き出しは位相 0 に固定している。runonce の最後のコマは同じになるが、periodic の鳴る刻みは画面とずれうる。`settle_tests.rs` の periodic の表は「書き出しの決め方」として固定する、と書いておくと読み手が迷わない。
4. `rng = |_| 1` は、`random,1`（k=1）でも当たらない（値 1 は範囲外だが `== 0` でないので無害）。画面では `random,1` は毎秒必ず鳴るので、その面だけは画面の 1 秒後の絵と違う。SSP の測り（random は出ない）とは合っているので、そのまま試験の表に入れておけばよい。
5. 要件 14 で画面のスコープ 1 の初めの着せ替えが `sakura.*` から `kero.*` に変わる（正典どおり・Revalidation Triggers に既出）。`bind_tests.rs` の `scope_namespace_maps_zero_one_only` は書き直しになる（期待の変更として正しい）。`shell_carry_tests.rs` 175 行のスコープ 2 の試験は、名前の表が空なので同じ結果のまま通る。
6. ファイルの行数: `main.rs` 965 → 約 970（余り 30 行）・`cli/parse.rs` 459 → 約 600・`log_history.rs` 424 → 約 515・`areka-seriko/src/bind.rs` 375 → 約 575・`shell_target.rs` 757 → 少し増える。どれも 1,000 行の内。`main.rs` はこれ以上足すなら分ける前提で。

## 良い点

1. **runonce の時点を、画面と同じ道（`handle_message` → `LoopRuntime::on_tick` → `frame_at`）を同じスレッドで回して求める**。写しを作らないので、部品・`always`・動く絵の子・`-1` の扱いが画面とずれない。注入の時計と乱数の口はもう在り（`ClockRig` が同じ使い方をしている）、新しく作るのは公開の関数 1 つと T の数え方だけで済む。スレッドを起こさないので、記録の写しもそのまま届く。
2. **「書かない」を作りで守っている**。分岐を `--help` の直後（スレッド・記憶・二重起動の確かめ・MCP・窓より前）に置き、`offline_dump` が使ってよい部品と使ってはならない部品を名指しで分けた。あわせて、画面と書き出しの既定の着せ替えと名前の表を 1 つの決め方（`DefaultBinds::from_groups`・`BindResolver::from_groups`・`apply_named_bind`）にまとめたので、要件 4.5（画面と同じ絵）が実装の約束でなく構造で保たれる。

## 判定

**GO**

理由: 作りの骨（新しいモジュール・同じ道での時点 T・プロセスの受け口に足す写しの層・既定の着せ替えの共有）は、確かめたコードの事実と合っており、作り直しは要らない。上の 3 つの問題は、どれも「設計に 1〜2 文足し、タスクの切り方を決める」ことで片付き、要件を変えない。

次にすること:
1. 問題 1 は案 A か案 B かを決め、設計の Error Handling と Testing Strategy（E2E の「同じ本文の行が無い」）に書き足す。
2. 問題 2 は `DefaultBinds::uniform` を設計の Service Interface に足し、Modified Files に試験のファイルの群を足す。
3. 問題 3 は行の番号の取り方と `resolve_bind.rs` への切り出しを設計に足す。
4. そのうえで `/kiro-spec-tasks areka-P0-offline-surface-dump` でタスクを作る。型の付け替え（期待を変えない）と画面の既定の変更（期待が変わる）は別のタスクにする。

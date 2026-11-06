# 設計の検証: audit-seriko

- 検証の日: 2026-11-05
- 対象: `design.md`（コミット `aca021d7`）・`requirements.md`・`research.md`
- 作法: `kiro-validate-design`（分析 → 大事な問題 → よい点 → GO／NO-GO）。対話なしで実施。

## まとめ

要件 8 の「直す」18 件（1〜17・19）のどれにも、最も小さな完全な変更・直したことを示すテスト・触るファイルの 3 つがそろっている。設計が挙げたコードの行の番号と中身は、ディスクで確かめた範囲で全部合っていた。直す前に答えを決めておくべき点が 2 つあるが、どちらも設計の中の 1〜2 行の足しで済み、作りを変える話ではない。

## 確かめたこと

### 1,000 行の上限（要件 6.1）
今の行の数を数え直した。設計の表と全部一致した。

| ファイル | 設計の「今」 | 数え直し |
| --- | --- | --- |
| `looper.rs` | 993 | 993 |
| `parts.rs` | 973 | 973 |
| `looper_tests.rs` | 970 | 970 |
| `table.rs` | 929 | 929 |
| `parts_tests.rs` | 921 | 921 |
| `actor_bind_loop_tests.rs` | 1,375 | 1,375 |

- `actor_bind_loop_tests.rs` は `mod tick_loop_tests`（説明の行を含め 1012 行から）を移すと約 1,011 行で、指摘 3 の約 20 行減りで 1,000 行の下に入る。余裕は 10 行ほどしかないが、越えたときの逃げ道（939〜1010 行も移す）が書いてあるので足りる。
- 範囲の外の 2 ファイル: `file_length_guard_test.rs` の 75〜78 行が `actor_bind_loop_tests.rs` の行、101 行が `OVER_LIMIT_ALLOWED_COUNT = 9`。設計のとおり。

### コードについての主張（危ないものから順に）
- **指摘 11（`BTreeMap` にしても抽選の順は同じ）**: 正しい。`ActorKey` の `Ord` は中の文字列の辞書順（`crates/dola/src/cue/command.rs:21`）で、`looper.rs:476-481` の並べ替えと同じ比べ方。(スコープ, 段) の組は重ならないので、「スコープの昇順 → シェル → バルーン」で組んだ列は今の並べ替えの答えと同じになる。`state.rs:338` の `shown.sort()` も同じ理由で消せる。ただしテストの守りに穴がある（大事な問題 1）。
- **指摘 5（`max` を `run_calls` へ移す）**: 正しい。印を戻さない決まりは今 `looper_calls.rs:108` と `parts_calls.rs:49` の 2 か所にあり、`run_calls`（`calls.rs:116-118`）は `playing(id)` から前の印を読めるので、そこで `max` を取れる。`stop` の `always` の判定も、今の `anims.iter().any(..)` と `animation_of(target)` は同じ答えになる。
- **指摘 3（`groups_of` で組む道）**: 在る。`bind_tests.rs:443` の `groups_of` は `pub(crate)` で、`bind.rs:647` が `pub(crate) mod tests` なので、もう `actor_bind_noevent_tests.rs`・`bind_forms_tests.rs`・`actor_bind_loop_tests.rs:725` から呼ばれている。`shell_carry_tests.rs:216` の `char2_names` も同じ形。
- **指摘 19（上限の道）**: `settle.rs:94-99` の `while now < until` の `until` を `until.min(37_500 × 16)` に置き換えれば、上限の内は道が変わらない（要件 8.5）。`warn!` の `wait_ms` に入れる元の `until` も手元にある。
- **指摘 17（借用の形）**: `parts.rs:560-565` は `contains_key` → `insert` → `get_mut` で、`else` には届かない。`match clocks.get_mut(scope) { Some(c) => c, None => { insert; get_mut } }` の形は、関数から借用を返すわけではないので今の借用の検査で通る。
- **指摘 4・16**: 写しは `looper.rs:219-224`／`parts.rs:183-188`、`looper_trigger.rs:124-147`／`parts_trigger.rs:106-132` に在り、記録の文言の違いは「part 」と欄 `part` だけ。`Armed::arm` の呼び手は 6 か所で、全部 `Some(..)` を渡している。
- **指摘 15**: 7 つの口に `crates/areka-seriko/` の外の使い手は無い（`areka` の `ScopeState`・`R_POST_AND_KOMAINU` は名前が同じ別物）。7 つとも本番の中に使い手があるので、`pub(crate)` にしても使われない口の警告は出ない。`BindResolver::new`・`BindOptionDecls` の外の使い手は `shell_carry_tests.rs` だけ（git grep で確かめた）。
- **そのほかの行の番号**（`actor.rs:663-667`・`actor.rs:204-208`・`bind.rs:367-373`・`table.rs:227-243`・`table.rs:398-405`・`table.rs:742-743`・`calls.rs:167-184`・`looper.rs:433-441`・`looper_trigger.rs:181`）は全部中身が合っていた。

### 範囲・依存・文書
- 範囲の外に触るのは要件 7.6 の 2 ファイルだけで、触る行も決めてある。
- 新しい依存は無い（使うのは今ある `areka_parsers::package::resolve_with_shell`・`sample_ghost_kit::WorkDir`・`log-capture-kit`）。
- `doc/`・`docs/` を名指す行は足さない。

## 大事な問題

### 問題 1: 指摘 11 の「足すテストは無い」では、スコープをまたぐ並びが守られない
- **何が問題か**: 抽選の順のテストは「スコープ 0 → 1（シェルだけ）」（`looper_tests.rs:408`）と「同じスコープでシェル → バルーン」（`looper_tests.rs:456`）しか見ていない。素直に `scopes` を回してから `balloon` を回す形で `shown_slots()` を書くと、並びは「0 シェル → 1 シェル → 0 バルーン」になり、今の「0 シェル → 0 バルーン → 1 シェル」と違ってしまう。それでも今のテストは全部緑のままになる。
- **なぜ大事か**: 乱数を引く順が黙って変わる（同じ種で違う絵が出る）。並べ替えを消すのはこの直しの中心なので、「並びが崩れたら赤」という設計の主張が、肝心のところで成り立っていない。
- **直し方**: 指摘 11 の行に、テストを 1 本足すと書く。スコープ 0 と 1 の両方にシェルとバルーンを出し、引く順の列で「0 シェル → 0 バルーン → 1 シェル」を確かめる（たとえば `[1, 0, 1, 1]` で 0 のバルーンだけが鳴る）。シェルを全部回してからバルーンを回す形に変えると赤になることを、摂動で 1 回確かめる。
- **要件**: 4.1・4.2・2.3
- **設計の場所**: 「指摘ごとの直し」の 11 の行、「Testing Strategy」の「振る舞いを変えない直し」

### 問題 2: 指摘 2 の道具の差し替えに、中身が違う 1 組がまじっている
- **何が問題か**: 設計は `shown_shell_states` を `looper_tests.rs` の `shown_shell` と「同じ中身」としている。けれど `shown_shell_states`（`actor_bind_loop_tests.rs:1122`）は、静的な着せ替え `{1100, 1207}` を持つ状態を組む。`shown_shell`（`looper_tests.rs:106`）は空の着せ替えで組む。中身が同じなのは `shown_shell_with_binds(sid, BindSet::from_ids([1100, 1207]))` のほう。
- **なぜ大事か**: 移すテスト（`tick_boundary_cross_emits_show_carrying_pattern` ほか）は `Show` の欄を `..` で読み飛ばしているので、入力が変わっても期待値は変わらず緑のままになりうる。設計の「期待値が変わったら止める」という守りが効かず、テストが確かめる入力が黙って変わる。
- **直し方**: 指摘 2 の行の対応を `shown_shell_states` → `shown_shell_with_binds(sid, BindSet::from_ids([1100, 1207]))` に直す（`pub(crate)` に上げるのもこの関数）。ほかの 6 つ（`pat`・`surface_with`・`shell_table_of`→`shell_table`・`table_single`・`live_cfg`→`cfg`・`always_fire`）は中身が同じことを確かめた。
- **要件**: 2.3・4.2
- **設計の場所**: 「指摘ごとの直し」の 2 の行

## よい点

1. **指摘ごとの表が、直し・触るファイル・壊したら赤になるテストの 3 つを 1 行にまとめている。** 行の番号まで書いてあり、確かめた範囲で全部合っていた。タスクに落とすとき、そのまま 1 行 1 タスクの材料になる。
2. **順の決め方に理由がある。** 1,000 行の見積もりから「指摘 3 → 指摘 2」、道具の置き場から「指摘 6 を先に」と決めている。範囲の外の 2 ファイルも触る行まで決めてあり、例外表の件数は「main を取り込んだ後の行の数」に合わせるとしていて、並走のぶつかりにも備えている。

## 判定

- **判定: GO**
- **理由**: 作りの食い違いや足りない指摘は無く、18 件のどれも実装の道がはっきりしている。上の 2 つは、どちらも設計の表の 1 行を直す（テスト 1 本を足す・道具の対応を 1 つ直す）だけで解ける。
- **次の手**: 設計の討議（`kiro-design-discussion`）で問題 1・2 を設計に入れてから、`/kiro-spec-tasks audit-seriko` に進む。

---
検証で見なかったこと: 指摘 6 の約 40 ファイル分の写しの中身は 1 つずつは比べていない（指摘 2 の 7 組だけ比べた）。指摘 13・14 の消すテストの行は番号だけ見て、残るテストが同じことを見ているかは確かめていない。

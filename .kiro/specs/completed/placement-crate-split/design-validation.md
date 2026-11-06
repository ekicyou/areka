# 設計の検証: placement-crate-split

- 日付: 2026-11-06
- 対象: `.kiro/specs/placement-crate-split/design.md`（要件 `requirements.md`・研究 `research.md`）
- 手順: `kiro-validate-design`（分析 → 重大な問題 → 良い所 → GO／NO-GO）。対話なしで判定した。
- 物差し: 要件の討議の議題 3（開発者「あとで読む人が素直に読めるソースコードの書き方をしないとダメ」）＝テストは自分の crate のファイルを読む・別の crate を読むのは依存と同じ向きで、名前の付いた 1 つの読み口を通す。

## 要旨

設計は、今のモジュールの境目をそのまま crate の境目にする素直な形で、要件 1〜8 の全項目に部品・口・段が対応している。コードで確かめた要の主張（下の表）はどれも事実どおりで、実装に進める。直すべき点は、読み口の約束の線引きの揺れと漏れ（問題 1）と、要件の字面と違う読み 2 件の承認（問題 2）で、どちらも設計の討議で決めれば済む。

## コードで確かめたこと

| 設計の主張 | 確かめた結果 |
|---|---|
| D6: 本体の `emo2_boot/sample_test_support.rs` に同じ働きの `emo2_balloon_root` が既にある | 事実。`pub(super) fn emo2_balloon_root()`（35 行）が `EMO2.balloon("emo2-kakukaku")` を返し、配置の `balloon_root()` と中身が同じ。`emo2_root` も `pub(super)` |
| D3: `reseed::tests` の `booted_world`・`new_shell`・`char_positions` は見本のゴーストを使わない | 事実。使うのは `spawn_ghost_windows`・`MonitorSnapshot`・`GhostTitles::from_scope_titles`・寸法の定数・偽の HWND だけ。`emo2_root`／`balloon_root`／`TempDir` を使うのはこのファイルの中のテスト本体（361 行より後）だけ |
| D10: `synth_declared_dpi_ghost` は道を引数で受ければ `test_support` へ出せる | 事実。中で頼るのは `TempDir`・`emo2_root()`・`balloon_root()` の 3 つで、どれも道を渡せば消える。ほかは `std::fs` だけ |
| D4: 行数の余裕 | 事実。`transition_judge.rs` 999 行・`main.rs` 986 行・`placement/mod.rs` 971 行。`main.rs` は `mod placement;` と説明 3 行が抜けるので、宣言 2 つを足しても 1,000 行に収まる |
| 判定器の一族は 15 ファイル・配置は 160 ファイル | 事実（`transition_judge*`・`transition_signoff*` が 15・`find` で 160） |
| 境をまたぐテストが使う配置の項目 | `reset_balloon_positions`・`BalloonResetTrigger` はもう `pub`。`spawn_assembly_tests` は `spawn::test_support` を使う（設計の表どおり）。`measure_bake_*` は私有の `measure_with_reader`・`measure_baked` を使う（D9 どおり）。署名に出る `DefaultSurfaces` はもう根から出し直されて本体が使っている |
| 1.7: 本番が使う `windows` の名前空間 | 事実。本番のファイルは `Win32::Foundation`・`Win32::UI::WindowsAndMessaging` だけ。`System::Com`・`System::LibraryLoader` はテストのファイルと共有のテストの支えだけ。ワークスペースの表に `Win32_System_Com` がある |
| 番人: 共有のテスト用の crate は開発用の依存なら通る | 事実。`sample_path_guard_test.rs` は `[dev-dependencies]` の `sample-ghost-kit` を違反にしない（`a_dev_dependency_on_the_sample_gateway_is_not_a_violation`） |
| `log-capture-kit` の `capture` は出所の名前で絞らない | 事実。出所が `areka_placement::…` に変わっても本体のテストで捕まる |
| D1: `placement_tests/` はほかの見直しの持ち物と重ならない | 事実。`audit-emo2-boot-*` は `emo2_boot/`、`audit-areka-input-menu` は `input_events*`。`audit-areka-app` は「上の 7 本の持ち物でないもの全部」で、2 本の配置の見直しが名指せば外れる。C18 の兄弟（③〜⑧）の受け持つファイルとも重ならない |
| D11: 根の `[workspace.dependencies]` に areka 系の crate は無い | 事実（path つきで載っているのは `dola` だけ） |

## 重大な問題

### 🔴 問題 1: 読み口の約束の線引きが揺れていて、漏れが 1 つある

- **何が問題か**: 議題 3 の決めは「別の crate を読むのは依存の向きに、1 つの読み口を通して」。ところが設計の中で、どの読みを読み口に通すかの線が 3 か所で揺れている。
  1. **漏れ**: `crates/areka/src/session_end_sync_send_tests.rs:313` は `CARGO_MANIFEST_DIR` から `../..` へ上がり、`crates/*/src` を**ワークスペースの全部**歩いて読む（本体が依存しない crate も含む）。D7 の「本体のほかの 5 ファイル 6 か所」に入っておらず、検証の grep（`'"(\.\./)+(wintf|areka-|…)'`）も `"../.."` を拾わないので、「読み口の外で 0 件」が通っても約束は守られていない。
  2. **自分の crate の読み方が 2 通り**: CrateSourceReader は「自分の crate の兄弟のファイルは相対の `include_str!` のまま」（597 行）と書く一方、分岐の対応表 2 本は本体自身のファイルも `crate_source!("areka", …)` で読む（633 行）。同じ `placement_tests/` の中で、本体の `emo2_boot/assets.rs` を `measure_template_tests` は相対で、対応表は読み口で読むことになる。
  3. **配置の crate の別の読み方**: 配置に残る `zorder_property_deferral_tests.rs:169` は `CARGO_MANIFEST_DIR` から `../areka-sylphya/src` を生の道で読む。依存の向きは正しいが、読み口は無く、設計はこれを「変えない」とだけ書いて理由を残していない。
- **なぜ効くか**: あとで読む人が「別の crate を読む所」を探すとき、読み口を grep しても全部は出てこない。決めが「1 つの読み口」なのに、例外が書かれずに残ると、次の crate 分け（後の 5 本）で同じ揺れが増える。
- **直し方**: 規則を 1 行で書く＝「読み口は別の crate を読むときだけ使い、自分の crate は相対で読む」。対応表は本体の行を相対で、配置と wintf の行を読み口で読む（または、表の行を揃えるために本体の行も読み口で読む理由を 1 行書く）。`session_end_sync_send_tests` は「ワークスペース全体の見張り」として名前を付けた例外にし（置き場の正しい家は `log-capture-kit/tests/` の見張りの置き場）、移すことを `audit-areka-app` へ申し送る。検証の grep に `CARGO_MANIFEST_DIR` からの `"../.."` を足す。配置の `../areka-sylphya` の 1 か所は、依存の向きどおりの唯一の読みとして説明に 1 行書く（配置にも読み口を作るほどではない）。
- **要件**: 5.2・5.4・5.5（議題 3）
- **設計の場所**: 「設計の段で決めたこと」D7、「部品と口」CrateSourceReader（597 行・618 行）、SourceReadTests の表（633 行・635 行）

### 🔴 問題 2: 要件の字面と違う読みが 2 件ある — 討議で承認を取って残す

- **何が問題か**: 設計は、確定した要件の字面と違う形を 2 か所で選んでいる。どちらも中身は正しいが、要件の文はそのまま残る。
  1. D6: 要件 3.2 は「本体の `emo2_boot/sample_test_support.rs` に `balloon_root` を足す」。設計は足さず、同じ働きの `emo2_balloon_root` を `pub(crate)` に広げて使う（3.6 の「写しを作らない」を優先）。
  2. D11: 要件 1.7 は「どれも `workspace = true` で受ける」。設計はワークスペースの中の crate（wintf・areka-*）を `{ path = "../…" }` で受ける（根を変えない 1.5 を守るにはこの形しかない）。
- **なぜ効くか**: 実装の後の見直し（`kiro-validate-impl`・完了の DoD）が要件の字面で照らすと、「`balloon_root` が無い」「`workspace = true` でない」と赤に読める。承認の記録が無いと、そこで手が止まる。
- **直し方**: 設計の討議で 2 件を議題に出して承認を取り、討議の記録（設計の「要件の討議で決めたこと」に当たる節）に「要件 3.2・1.7 はこの読みで満たす」と書く。要件の文は直さない。
- **要件**: 1.5・1.7・3.2・3.6
- **設計の場所**: 「設計の段で決めたこと」D6・D11、PlacementCrate の「責務と約束」

## 良い所

- **移し方が赤で気づける形になっている**: 段 1 だけ一時の別名を置いてコンパイラに公開面を数えさせ、段 3 で消す（D2）。公開面の数え漏れ・文字で読む先の消失・木の走査の狭まり・番人の表の古いパスが、どれもコンパイルかテストの赤になる（「失敗の扱い」）。テストの名前の多重集合の差も、増える 11 名を理由つきで先に表にしてある。
- **テストの支えを頼る相手で分けた**（D8）: 本体のテストを組むときの配置は `test` が立たず機能だけが立つ、という一番の制約を正しく捉え、`test-support` の下の項目は共有のテスト用の crate と `#[cfg(test)]` に頼らない、と線を引いた。番人の決まりを 1 つも変えずに済み、写しも作らない。

## そのほかの気づき（重大ではない）

- 6.6 の grep の `areka::placement` は、新しいモジュール `placement_tests` の出所 `areka::placement_tests::…` にも当たる。今そう書いた文字は無いが、grep は `areka::placement\b`（または `areka::placement::`・`areka::placement=`）にすると誤った当たりが出ない。
- `crates/areka/examples/collision-probe.rs` の説明（90 行あたり）は `../pilot/...` を読むと書いているが、今の中身と合っていない古い説明に見える。この仕様の書き換え（4.2）で触るので、ついでに確かめる程度でよい（直すかは `audit-areka-app` の持ち物）。

## 判定

- **判定: GO**
- **理由**: 依存の向き・公開面・テストの支え・置き場・検証の手順が、要件と今のコードに合っていて、実装の道筋がはっきりしている。2 つの問題は、どちらも決めと記録の足し直しで済み、設計の骨組みは変わらない。
- **次の手順**: 設計の討議（`kiro-design-discussion`）で問題 1・2 を議題にして決め、design.md に反映する。その後 `/kiro-spec-tasks placement-crate-split` でタスクを作る。

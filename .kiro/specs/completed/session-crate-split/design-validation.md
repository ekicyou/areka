# 設計の検証: session-crate-split

- 対象: `.kiro/specs/completed/session-crate-split/design.md`（requirements.md・research.md・steering・roadmap の C25 の行と照らした）
- 日付: 2026-11-22
- 判定: **GO**（議題 3 つは設計の討議で決める）

## 設計の検証のまとめ

設計は、前例（`placement-crate-split`・`runtime-crate-split`・`input-crate-split`）の形にそろえ、今ある `BodySystems` に欄を足して World の資源として据えるだけで向きを逆にしている。新しい仕組みは足しておらず、要件 1.8 の決めに合う。段 0〜6 の並び・段ごとに緑にするもの・要件の対応表・失敗の扱いまで具体的で、実装へ進める。残る危うさは、テストの置き場の決め方と公開面の開け方の 2 つと、呼ぶ順の確かめの 1 本が実際に回せるかに集まっている。

## 事実の突き合わせ（コードを読んで確かめた）

| 設計の主張 | 確かめた結果 |
|---|---|
| `BodySystems` は `ghost_session.rs:36` で 2 欄・`#[derive(Clone, Copy)]`・`main.rs:365` の `BODY_SYSTEMS` | 一致 |
| C1〜C5 の所（`ghost_session.rs:18・298・304・307`・`ghost_session_runtime.rs:16・34〜51`・`ghost_switch.rs:504・513・515・525・675`・`reload.rs:309・312`・`shell_balloon_switch.rs:131`・`runtime_boot.rs:24・26・89・90`・`alert.rs:286`） | 一致。本番で本体を名指すのはこのほかに `boot_config.rs:209`（`#[cfg(test)]` の `resolve_boot_from`・D12）だけ |
| 本体の関数の型（`on_switch_end(&mut World, bool)`・`is_busy(&World) -> bool`・`raw_sender(&World)`・`reseed(&World, &GhostRuntime)`・`attach_*(&mut World)`） | 欄の型（`fn(&mut World, &GhostRuntime)` ほか）に収まる |
| `areka_ghost::sink::DiscardSink` が在り `BootCueSink` に当たる | `sink.rs:86`（`Clone, Copy`）・`BootCueSink` は blanket impl。一致 |
| `SessionHooks::of` の `#[cfg(not(test))]` と `error!`・`NO_BODY` | `session_hooks.rs:46〜60`。一致 |
| `SwitchRig::new(` は本体 22 か所・集めた層 35 か所、使うファイルは本体 17・集めた層 30 | 一致（要件の「32 ファイル」は設計の 30 で読み替え） |
| 1,000 行: `ghost_session.rs` 897・`ghost_switch.rs` 928・`ghost_switch_tests.rs` 982・`ghost_switch_test_support.rs` 721・`main.rs` 623 | 一致。口を通す書き換えは行を減らす向きで、上限を越えない |
| 下の crate の `test-support` 機能（runtime・input・placement） | 3 つとも在る。runtime の機能は `crash-report-kit` を連れ、`install` がプロセスの寿命の MTA も立てる（5.10 に合う） |
| `crate_source!` の `env!("CARGO_MANIFEST_DIR")` は呼び手の側で解ける | `macro_rules` の展開なので正しい |
| `ghost_switch_test_support.rs` は番人の crate を `sample_ghost_kit::SampleRoot` のほかに引かない | 一致（`RigSample` で外せる） |
| 同じウェーブの約束 ⑶ | 設計が触る `crates/areka-runtime/`・`crates/areka-kanade/` はコメントの行だけで、`areka-placement`・`dola`・`areka-ghost`・`areka-seriko`・`placement_tests/` には触らない。roadmap の C25 の行と食い違いは無い |

## 重い指摘（3 つ）

🔴 **重い指摘 1**: 偽物の組（`NO_BODY`）で緑のまま残る集めた層のテストが、守りを黙って失う

- **懸念**: D9 は「偽物の組で走らせて赤になったテストだけを本体へ移す」で決める。要件 5.4 は「どのテストが本物の組を要るかは 1 本ずつ決め、5.1 の記録に入れる」と求めている。赤にならないことは、本物の組を要らないことの証にならない。設計自身が書いているとおり、本物の `UpdateDesk` は定常のたびに `homeurl` の照会を送り、`progress_probe` と `exit_wait` の門（install・update）に効く。偽物の組では、これらに触れる表明（「警告が 0 件」「照会の数」「待ちの打ち切り」ほか）がそのまま空で通りうる。
- **影響**: 集めた層の 30 ファイルの一部が、名前と中身は同じなのに、本体とつないだ切替を守らなくなる。テストの名前の多重集合（8.2）ではこの差は見えない。
- **提案**: 段 2 では、赤かどうかに加えて、表明を読んだ上での決めも記録する。`SwitchRig` を通る 30 ファイルを、本物の組と偽物の組の両方で走らせ、記録の数（偽の SHIORI への要求・`capture` の件数・待ちの回数）が違うテストを洗い出す。違うものは 1 本ずつ「本体へ移す」か「偽物で足りる（理由つき）」に決め、TestPlacement の表に理由と一緒に載せる。
- **要件**: 5.1・5.2・5.4
- **設計の所**: D9・TestPlacement の表の後の注・危うさ 1

🔴 **重い指摘 2**: 本体へ移すテストが引く私有の項目を、どう開けるかが決まっていない

- **懸念**: `reload_tests.rs`（859 行・ファイルごと）・`shell_balloon_switch_tests.rs`・`ghost_session_restart_tests.rs`・`ghost_session_strict_tests.rs`・`ghost_session_reload_tests.rs` は `use super::*` で親の私有の項目を引く（例: `reload.rs` の `drain`・`start`・`ghost_dir`、`ghost_session.rs` の `boot_wired`）。D7 と PublicSurface は「コンパイラのエラーで 1 つずつ開く」とだけ書いていて、開け方が決まっていない。素の `pub` にして本番の公開面に入れるのか、`#[cfg(any(test, feature = "test-support"))]` で囲った `pub use` の出し直しにするのか。数も測っていない。
- **影響**: 本体のテストのためだけに、中の関数が本番の公開面（`areka_session::reload::drain` ほか）に出うる。そうなると要件 4.1 の「外から使われる項目だけ」の境目が段 3 の場当たりで決まり、`audit-areka-app` へ送る 4.5 の記録も大きくなる。
- **提案**: 決まりを設計に書く。⑴ 本体の本番が使う項目は `pub` ⑵ 本体のテストだけが使う私有と `pub(crate)` の項目は、`test-support` のときだけの `pub` にする（PublicSurface の「`#[cfg(test)]` の 8 つ」と同じ扱い）。あわせて、移す 24 ファイルの `use super::*`・`super::` が引く私有の項目を段 2 の前に数え、見込みを表に載せる。
- **要件**: 4.1・4.5・5.8
- **設計の所**: D7・D8 ⑵・PublicSurface の責務と約束

🔴 **重い指摘 3**: 呼ぶ順の確かめ 1 本目が、窓の口（C1）の回数を本当に記録できるか確かめていない

- **懸念**: OrderTests 1 は「窓を開くたびに `after_char_pointer` → `after_balloon_pointer`」を `SwitchRig` の記録で見るとしている。ところが `SwitchRig::boot` は `open_ghost_windows` を通らずに起こす（`ghost_switch_test_support.rs:405〜`）。切替の側も、窓を作る閉包は `WintfTaskPool` へ投函されるだけで、`Input` の段で World に当てられて `spawn_ghost_windows` が通るかは確かめていない。前例の `ghost_switch_balloon_tests.rs` は、閉包を投函する先として作業プールを据えているだけである。
- **影響**: 閉包が当たらなければ、C1 の 2 欄は記録に出てこない。そうなると、要件 2.4（起こし直しのたびに同じ回数）の C1 の分は字面のテスト（OrderTests 4）だけが頼りになり、回数を示せない。実装の段で、表明を変えるか、テストを作り直す羽目になる。
- **提案**: 段 1 の最初に、`SwitchRig` の切替で閉包が当たるかを確かめる。当たらなければ、C1 の回数は「閉包は起こし直しの道の 1 か所だけで組まれる」ことの字面の確かめ（`commit_ghost_windows` の呼び手の数）で示すと設計に書き、OrderTests 1 の表明から C1 を外す。
- **要件**: 2.2・2.4・5.5
- **設計の所**: D14・OrderTests 1・4

## 設計の強み

1. **新しい仕組みを足さずに口を逆にしている**: 束は今の `BodySystems` 1 つに欄を足し、`SessionHooks` と同じ `of`／`NO_BODY`／`#[cfg(not(test))]` の形で据える。どの口も今の呼び出しの所で同期に呼ぶので、1 フレームの遅れが入る余地が無い（2.3）。C1 を 2 欄に分けて入力の差し込みとの交互の順を残す、C4 を 2 欄にして読み直しの断りの理由を残す、といった欄の切り方も、今の分岐の形から導いていて筋が通っている。
2. **段の並びと、段ごとの緑の確かめが具体的**: 本体の中で逆にする（段 1）→ テストを分ける（段 2）→ 移すだけ（段 3）と分けたので、各段の赤の出どころ（効き目が無くなった赤／記録が増えた赤）が段で分かれる。D13 はそこまで書いている。grep の式・`cargo tree`・名前の多重集合の見込み（+8 と内訳）まで決まっていて、検証の手順がそのまま使える。

## 軽い所見（議題にしない・実装の段で拾う）

- `crate_source_test_support.rs` の中の `mod tests` には `#[cfg(test)]` が無い（今は親の `mod` 宣言の `#[cfg(test)]` で囲われている）。`test-support` で開くと、機能つきのビルドで `use super::*` が使われないという警告が出うる（8.4）。移すときに `#[cfg(test)]` を付ける。
- `doc/COMPAT_ARCHITECTURE.md` の書き換える行（353・355・411）が、`test-name-work-tags` の名の行（354・412）の隣にある。約束 ⑷ で許した重なりの中だが、git は隣り合う行の変更を競合として出すので、後から着地する側で手で解くことになる。
- 「同じウェーブとの重なり」の最後の項（C25 の行に無く足したファイル）に、`crates/areka-runtime/src/lib.rs:391`・`strict_judges.rs:5`・網羅台帳・`doc/ideas/`・2 本の audit の brief が入っていない。棚卸のセッションへ伝える一覧には全部載せる。

## 最終の判定

- **判定**: GO
- **理由**: 境目・依存の向き・前例との合致に、根っこから食い違う所は無い。要件の全項目に部品と段が当たっている。重い指摘 3 つは、どれもテストの置き場・公開面・確かめの形の決め方の話で、設計の討議で決めを足せば直せる。
- **次にすること**: 設計の討議（`/kiro-design-discussion session-crate-split`）で、上の 3 つを設計の議題 4・6・5 と一緒に決め、design.md に反映する。その後 `/kiro-spec-tasks session-crate-split`。

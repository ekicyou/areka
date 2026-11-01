# 設計の検証: emo-bake-log-overlap

> 2026-11-01・本ブランチ（`07164005`）の `design.md`・`requirements.md`・`research.md` と、コードの実物を読み比べて書いた。対話なしで判定した。

## 1. まとめ

設計は要件を全部たどれ、コードの実物とも合っている。直し方は前例（`placement-measure-bake-once` の `LoadRecords` と `_quiet` の入口）と同じ形で、`emo2_boot/assets.rs` に触らずに scope ごとの重なりを消せる。直す前に決めておくとよい点が 2 つある（どちらも小さく、テストの書き方の話）。判定は **GO**。

## 2. コードと照らした結果

### 2.1 行数と行番号

設計に書いてある数は、どれも今のコードと合っていた。

| ファイル | 設計の値 | 実物 |
|---|---|---|
| `areka-emo-atlas/src/lib.rs` | 818 行・記録 4 本が 139・171・184・193 行 | 同じ |
| `areka-emo-compose/src/fold.rs` | `warn!` 9 本（40・51・62・71・179・189・244・300・357）・`debug!` 1 本（95） | 同じ |
| `areka-emo-compose/src/base_image.rs` | `warn!` 89 行 | 同じ |
| `areka-emo-present/src/shell_target.rs` | 917 行・`probe` 552 行・`bake` 582 行 | 同じ |
| `areka/src/placement/measure.rs` | `build_world` 401 行・バルーンの面 0 が 462 行 | 同じ |
| `emo2_boot/assets.rs` | `build_world` 355 行・バルーン 473 行 | 同じ |
| `measure_tests.rs`・`fold_tests.rs` | 1,000 行・976 行（足さない） | 同じ |

### 2.2 面の表を組む段の記録はほかに無いか

`EmoWorld::build_with_images` が通る道（`fold.rs`・`base_image.rs`・`hit_import.rs`・`BindSteps::build`）のうち、記録を出すのは `fold.rs` と `base_image.rs` だけだった。`world.rs` 279 行の `warn!` は `resolve_alias`（後から引く問い）で、組む段ではない。`bind_steps.rs` の `debug!` は合成のときの記録である。設計の「黙らせる範囲」に漏れは無い。

### 2.3 必ず出す `error!` は囲まれないか

- `lib.rs`・`fold.rs`・`base_image.rs` には `error!` が 1 本も無い。黙らせるのは `warn!` と `debug!` だけになる。
- 焼く段の前提の崩れの `error!` は `bake.rs`（76・89 行）と `pack.rs`（104・165 行）に在り、設計はこの 2 ファイルに触らない。
- バルーンの「採用面列が空」と「面画像の bake に失敗」の `error!`＋`Err` は、`balloon.rs` の中で焼く・組む前と後の枝に在り、核へ移しても `records` で囲まない形にできる。
- シェルの 3 つの失敗の `error!` は今も `records` に関わらず出ており、設計はそれを変えない。

要件 1.7・3.5 は守られる。

### 2.4 数の流れ

シェルの畳み込みの記録は、①採寸の読み込み（出さない）・②採寸の組み直し（出さない）・③資産の読み込みの核の `probe`（出す）・④資産の scope ごとの `build_world`（出さない）で、起動 1 回に 1 行になる。焼く段は①（出さない）と③（出す）で 1 行。バルーンの面 0 は採寸を `_quiet` に替えて scope ごとに 1 行。動く絵の 3 本の `warn!` は、採寸の読み手 `FirstFrameDecoder` が `probe_animation` を既定（`None`）のまま返すので、採寸の道では今も起きていない。設計の数の表はコードと合う。

### 2.5 ほかの呼び手（要件 4）

`ShellTarget::build_world` を呼ぶのは、資産の組み立て・採寸・examples 3 本・MCP の試験の支え・`take_world` の 2 回目からである。どの `ShellTarget` も記録を出す `load_shell_target` か `load_shell_target_quiet` から作られている。出す入口で作った物は核で 1 行出すので、外の入口でも「少なくとも 1 行・今より増えない」が守られる。着替えの道も `build_shell_assets` を通るので同じである。

### 2.6 今のテストが通り続けるか

- `shell_target_base_image_tests.rs` の `append_to_an_unknown_surface_warns_and_creates_nothing` は、読み込みの中だけを捕まえて 1 行を数えている（`build_world` は捕まえる窓の外）。核が 1 行出すので通る。
- `shell_target_load_tests.rs` 404〜420 行の対照（`build_world` の窓に `areka_emo_compose` の記録が在る）は、装着（`atlas_bind.rs` 44 行の `debug!`）が出すので、畳み込みを黙らせても通る。
- 起動の準備の檻に使う `measure_with_reader`・`build_shell_assets`・`build_balloon_assets`・`MemoryDecoder`（`insert_animated` で「1 枚目は読めるが全コマは読めない」も作れる）は、どれも今の `measure_bake_count_tests.rs` が同じ形で使っている。依存を足す必要は無い。

### 2.7 ウェーブ C15 の約束

- `placement/mod.rs`・`ghost_session.rs`・`emo2_boot/assets.rs` は変えない。設計の変えるファイルの一覧に入っていない。守られている。
- `measure_first_frame.rs` の説明だけの書き直しは、約束の一覧（`placement/measure.rs`）にそのままの名前では無い。ただし、このファイルは `measure.rs` が `#[path]` で読む中の部品（`mod first_frame`）で、`measure.rs` の一部と見てよい。C15 のほかの 5 本はこのファイルに触らない（`shell-scale-linkage` が触るのは `placement/{descript_window_keys,mod}.rs` と場合により `config.rs`）。本 spec が例外 D4 を解くので、「全透明の `warn!` が 1 行増える」という説明を残すと誤りになる。**直してよい**と判断する。
- `areka-emo-atlas` の `lib.rs` は、要件の並走の約束で「足すなら兄弟のファイルへ」と添えてある。しかし黙らせる 4 本は `lib.rs` の繰り返しの中に在るので、`lib.rs` を変えずには直せない。設計は約 30 行足して 850 行ほどにし、テストは兄弟のファイル（`bake_quiet_tests.rs`）に置く。1,000 行の上限には届かず、ウェーブの表の約束（「`lib.rs`（か新しい兄弟）」）の中に収まる。問題にしない。

## 3. 直す前に決めておくこと（2 件）

### 問題 1: 例外 1 の檻を書き直すと、0 行の主張が空振りしうる

**状況**: 決めごと 3 は、`fade_in_animation_warns_all_transparent_only_when_wrapped` を「包んだ読み手＋`bake_quiet` で 0 行・包まない `bake` で 0 行」の主張に置き換え、「包んだ読み手＋ふつうの `bake` で 1 行」は説明の文に残すだけ（主張しない）とする。

**影響**: `bake_quiet` は `lib.rs` の 4 本を絵の中身によらずいつも黙らせる。そのため「包んだ読み手＋`bake_quiet` で 0 行」は、検体が壊れて動く絵が焼かれなくなっても通る。今の檻の「包むと 1 行・包まないと 0 行」という対があることで、検体が本当に「1 枚目だけ全透明」になっていることを確かめていた。それを外すと、この檻は何も確かめなくなる（0 の主張には較正が要る、というこのリポジトリの決まりにも反する）。

**提案**: 「包んだ読み手＋ふつうの `bake` で 1 行」を対照の `assert_eq!` として残す。これは焼く段の事実で、採寸の道の主張ではないので残しても矛盾しない。主張を 3 本（包む＋`bake` で 1・包む＋`bake_quiet` で 0・包まない `bake` で 0）にし、名前は「採寸の焼きでは 0 行」を言うものにする。足すのは 1〜2 行である。

**要件**: 6.2・6.6

### 問題 2: 宛先まるごとの「0 行」は、プロセスで 1 回だけ出る警告を拾いうる

**状況**: 単体の檻 1（`bake_quiet`）・3（`load_shell_target_quiet`）・5（バルーンの出さない入口）は「宛先 `areka_emo_atlas` の記録を 0 行」と主張する。同じ宛先には、動く絵の上限の環境変数（`AREKA_ANIMATED_IMAGE_MAX_*`）が読めない値のときの `warn!`（`limits.rs` 77 行）がある。これは `AnimationLimits::from_env` の `OnceLock` の中で、プロセスの中の最初の焼きのときに 1 回だけ出る。`bake_quiet` も `from_env` を通る。

**影響**: 開発者がその環境変数をおかしな値にしたままテストを回すと、そのプロセスで最初に焼いたテストだけが赤になる。どのテストが最初になるかは走る順で変わるので、赤の出方が揺れる（要件 6.5 の「左右されない」に反する）。ふだんの環境では起きない。

**提案**: 0 行の主張は、黙らせる 4 本の文言（抜き色・0 寸・全透明・着せ替えの「無し」）で数える。起動の準備の檻（Integration Tests 6）はもともと「文言ごとに数える」ので、同じ数え方にそろえる。宛先まるごとで数えたいなら、`limits.rs` の文言を外して数える。テストの書き方だけの話で、本番のコードは変わらない。

**要件**: 6.5・6.6

## 4. 良いところ

1. **触れないファイルに触らずに、本当の原因を直している**: scope ごとの重なりは `assets.rs` の `build_world` の呼び出しから出ているが、設計は `assets.rs` を変えず、`ShellTarget::build_world` を「いつも記録を出さない」にして消す。核の `probe` が読み込み 1 回につき 1 度だけ記録を出すことをコードで確かめたうえでの形で、`ShellTarget` に欄を足さずに済んでいる。回避の形ではなく、約束の書き換えとして筋が通っている。
2. **前例をそのまま下の 2 クレートへ広げている**: 「中身は 1 本・記録の行だけを `if records.emits()` で囲む・`error!` は囲まない・公開の入口の既定は出す」という `LoadRecords` の型を守り、公開の型を増やさず、下のクレートが上のクレートの型を知らない向きも保っている。今のテストの多くが公開の入口の既定に乗っているので、書き直しが少なくて済む。

## 5. 判定

**GO**

- 理由: 要件 1〜6 の全部の項目が部品・入口・檻にたどれる。行数・行番号・記録の場所・必ず出す `error!` の扱い・ほかの呼び手・今のテストとの関係を、どれもコードで確かめて食い違いが無かった。ウェーブ C15 の約束も守られている（`measure_first_frame.rs` の説明の書き直しは `measure.rs` の中の部品なので許してよい）。上の 2 件はどちらもテストの書き方の話で、作りを変えずに直せる。
- 次の手順: 設計の議論で問題 1・2 を決め、決めたとおりに `design.md` の Testing Strategy を直す。そのあと `/kiro-spec-tasks emo-bake-log-overlap` でタスクを作る。

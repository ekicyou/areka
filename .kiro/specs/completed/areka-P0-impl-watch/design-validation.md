# 設計の検証: areka-P0-impl-watch

作成: 2026-10-10（設計 `design.md` の生成の後・設計の討議の前）。対象は `design.md`（要件 1〜14）。この文書は読むだけで書き替えない。確かめたことは下の「確かめたこと」に、直すべきことは「重要な問題」に書く。

## 要約

規則 1〜7 の判断を `plan::apply` の純粋な関数に集め、時刻とプロセスの生死を引数で渡す形は、`kiro-watch.ps1` の `Invoke-Plan` の 4 段の順（停止要請 → 負荷テストの番 → 再開 → マージの番・負荷テストが在る間はマージの段へ来ない）を 1 対 1 で写せていて、決定論テスト（要件 12.1）に向く。排他・生死・置き換え書きを標準ライブラリだけで組む根拠（`File::try_lock`・`rename`・`serde`）は実物で確かめられた。直すべきは、並行の筋書きに残る 2 つの穴（再開の直後の回収・`status.md` の同時書き）で、どちらも設計の形を変えずに数行で閉じる。

## 重要な問題（3 件まで）

### 問題 1: 再開してから見張りを立て直すまでの参加者を、回収が「落ちた」と見なす

**何が起きるか**: `loaddone`（または `unstop`）で「止まった」参加者 B が「作業中」へ戻る瞬間、B には見張りが無い（見張りは停止要請で終わっていて、`stopped --wait` が B を代表している）。回収の規則は「作業中かつ見張りの印が無い者を外す」で、除くのは呼び出した本人（`caller`）だけ。だから、負荷テストの持ち主が `loaddone` の直後に別のコマンド（`leave`・`merge` など）を続けて呼ぶ、または待っている誰かの「周期の一回り」（30 秒に 1 回・待ちの数だけ重なる）が B の次の読み直し（最長 1 秒）より先に来ると、B は回収される。
**どう困るか**: (a) B の `stopped --wait` が 0「再開した」でなく 3「記録が消えた」で終わることがあり、終了コード 3 が「開発者に外された・`clear` された」と「再開したのに自動で消された」の 2 つの意味を持つ。続きの spec（スキルの書き替え）はこれを読み分けられない。(b) ログと `recent` に「回収 B: 見張りなし」が残り、落ちていないセッションを開発者が落ちたと読む（要件 7.3・9.1）。設計は「失うものは無い」と書くが、終了コードの契約と記録の信用は失われる。
**直し方の案**: 参加者に「再開して見張り待ち」の印を 1 つ足す（例: `awaiting_watch_since: Option<u64>`。再計画の再開の段と `unstop` で付け、`Command::Watch` で消す）。回収はこの印の付いた者を外さず、`status` では見張りの無い者として `absent` を付ける（「停止要請中」「止まった」と同じ扱い＝要件 7.7 の考え方の延長。要件 7.7 に半行の追記が要る）。これで B の再開の待ちは必ず 0 で終わり、回収の記録が本物の落下だけになる。
**要件**: 5.8・5.10・5.11・6.5・7.2・7.3・7.7。
**設計の箇所**: 「System Flows」の「流れの決めごと」（回収の規則）・「危うさと手当て」の 4 つ目・`plan.rs` の `reclaim`。

### 問題 2: `status.md` を `status` がロック無しで書くので、状態を変える呼び出しの書きと一時ファイルの名前がぶつかる

**何が起きるか**: 設計は `status.md` を「`status` のたびと状態を変えるたび」に、`state.json` と同じ「`<名前>.tmp` へ書いて `rename`」で書く。`status` はロックを取らない（論点 4）ので、開発者や `kiro-watch-clear` スキルの `status` と、別のプロセスの `with_state`（または待っている者の `Tick`）が同時に `status.md.tmp` を作り、片方の `rename` が「無い」で失敗して終了コード 1（または中身が混ざる）になる。写した前例 `FsPersistIo::commit`（`crates/areka-sylphya/src/persist/io.rs`）は、自身の注記で「単一プロセス前提・競合は想定外」と断っている。
**どう困るか**: 状態は無傷でも、正常な呼び出しが「失敗（1）」で終わる。要件 8.1 の「呼び出しごとに一貫した状態を見せる」と 10.2 の「失敗は本当の失敗だけ」が、読み物の側で崩れる。
**直し方の案**: 一時ファイルの名前にプロセス番号を入れる（`status.md.<pid>.tmp`・`state.json.<pid>.tmp`）。または `status` の `status.md` 書きだけ `state.lock` の中で行う（10 ms の排他・状態は変えないので要件 9.4 のまま）。前者が 1 行で済む。
**要件**: 8.1・8.3・9.4・10.2。
**設計の箇所**: 論点 4（読むだけはロックを取らない）・`store.rs` の「書き: … `status.md` も同じ形で書く」・「流用する前例」。

（3 件目に当たる重大な問題は見つからなかった。）

## 設計の強み

1. **判断が 1 か所・順が元と同じ**: `apply` の「回収 → コマンド → 再計画」と再計画の 4 段は `Invoke-Plan` と同じで、要件 7.5（回収で空いた机を同じ呼び出しで決め直す）が追加の仕組み無しで満たされる。`note`・`idleNotified` を写さない判断も要件の範囲に合う。
2. **外の依存 0 を実物で裏付けた形**: 生死をロックファイルで見るので、プロセス番号の使い回し（要件 7.4）への備えがそもそも要らず、`windows` クレートも `fs2` も入らない。待ちは 1 秒に 1 回の `metadata` だけで、端末へは終わるときの 1〜数行しか出さない（要件 13 の「トークン最優先」に沿う）。

## 判定: GO

**理由**: 置き方・層の分け方・前例の写し方は指針（`structure.md`・`tech.md`・`logging.md`）に合い、流用の根拠は全部実物で確かめられた。問題 1・2 はどちらも並行の端の穴で、状態の型に印を 1 つ、一時ファイルの名前に番号を 1 つ足せば閉じる。設計の形を変える必要は無い。

**次の手**: 設計の討議で問題 1・2 と下の「軽い指摘」を裁き、`design.md`（と要件 7.7 の半行）へ反映してから `/kiro-spec-tasks areka-P0-impl-watch`。

## 軽い指摘（討議でその場で直せるもの）

1. **層の向きと `Presence` の置き場が食い違う**: 「依存の向き」は `plan → presence`（`presence` が `plan` を読む）だが、`plan::apply` が `&dyn Presence` を受けるので `plan` が `presence.rs` を読む。トレイト `Presence` を `plan.rs`（または `state.rs`）に置き、`presence.rs` は実装だけにすれば一致する。構造テスト（`std::fs` 等の綴り）はこの食い違いを拾わない。
2. **`stopped`（`--wait` 無し）と `resume` が 3 で終わったときの行動**を手順書に書く（例: 「参加者の記録が無い → `watch` を立て直して続ける」）。問題 1 を直しても、`leave`・`clear` の後の 3 は残る。
3. **`rename` の失敗に短い再試行**: 読む側がロック無しで `state.json` を開くので、置き換えの `rename` が一瞬ぶつかる可能性がある（rustc 1.99 の `std::fs::rename` は Windows 10 以降で POSIX 流の置き換えを使うので通る見込みだが、設計は実機テスト⑥だけに頼っている）。`hold` と同じ「20 ms × 5 回」を `rename` にも置けば、実機で赤が出ても原因を切り分けられる。
4. **`Tick` で変化が無いときは `status.md` も書かない**ことを明記する（待っている者の数だけ 30 秒ごとに `status.md` が書き直されるのを避ける。`state.json` は `changed` で守られている）。
5. **同じ識別の見張りが残っていると `watch` が 1 で止まる**（Claude のアプリを閉じても見張りが残る場合）。手順書の「困ったとき」に「`status` の `waits` のプロセス番号で見分けて止める」を足す。
6. **構造テストの走査対象**: `structure.md` は `include_str!` の構造テストに兄弟テストファイルも列挙するよう求める。`main_layering_tests.rs` が `plan.rs` だけを見るなら、`plan_*_tests.rs`・`plan_test_support.rs` を見ない理由（テストは時刻を使ってよい）を一言書く。
7. **`Cargo.toml` の `serde`**: 根の `[workspace.dependencies]` に `serde`・`serde_json` は無い（`dola`・`ukadoc-survey` がクレート側で `serde = { version = "1", features = ["derive"] }`・`serde_json = "1"` と書いている）。設計の「`workspace = true`」の書き方は `thiserror`・`tracing`・`tracing-subscriber` だけに当たる。

## 確かめたこと（設計の主張と実物）

| 主張 | 確かめ方 | 結果 |
|---|---|---|
| `std::fs::File::try_lock`／`unlock` が使える（1.89 で安定・道具は 1.99） | `rust-toolchain.toml` は無い。`rustc --version` は 1.99.0。小さなプログラムを `rustc` で組んで走らせた | 同じファイルの別ハンドルで `try_lock` が `TryLockError::WouldBlock`、`unlock` の後は成功。ロック中の別ハンドルでの `truncate` 付き `open` も通る（`hold` の `create` は問題ない）。道具の版はファイルで固定されていないので、1.89 未満の道具では組めない旨を手順書に一言 |
| `FsPersistIo::commit` の置き換え書き | `crates/areka-sylphya/src/persist/io.rs` を読んだ | 一時ファイル → `flush` → `sync_all` → `rename`。一時ファイル名は固定の `.tmp`（単一プロセス前提と注記）→ 問題 2 |
| `TempPath::under_target(label)` | `crates/temp-path-kit/src/lib.rs` | 在る（`new`・`under_target`・`path`・`child`） |
| `serde`（派生）・`serde_json` が `Cargo.lock` に在る | `Cargo.lock` | `serde`・`serde_derive`・`serde_json` が在る。根の `[workspace.dependencies]` には無い → 軽い指摘 7 |
| `CARGO_BIN_EXE_<name>` は統合テストにだけ渡る | Cargo のリファレンス（手元の文書は無し） | 統合テストとベンチにだけ設定される。`tests/real.rs` に置く判断は正しい |
| 配布の zip に入らない | `tools/package.ps1` の `$ALLOWED_EXECUTABLES` | 実行ファイルを名指し（`areka.exe`・`shiori-host32-helper.exe`・`pasta.dll`）。入らない |
| 全体テストが自動で拾う | `tools/test-all.ps1` | `cargo test --workspace --no-fail-fast -j 4`。拾う |
| `publish = false # 理由` の行 | `tools/crates-io.ps1`・`crates/areka-update/Cargo.toml` | 行頭からの `publish = false # 理由` を全体テストの段が要求する。前例の位置（`[package]` の中ほど）と一致 |
| ワークスペース全体の常設検査 | `crates/log-capture-kit/tests/` | 1 ファイル 1,000 行・`std::env::temp_dir` の禁止・`with_default` の禁止が新しいクレートにも自動で掛かる。設計はどれも守る形 |
| `Invoke-Plan` の順と分岐 | `.claude/skills/kiro-watch/kiro-watch.ps1` の `Invoke-Plan`・`switch` | 停止要請 → 負荷の番 → 再開 → マージの番。負荷テストが在れば `return`。`loadtest` は自分を `working` へ、`loadrunning` は待ち行列から外して `running` 付きの持ち主へ、`merged` は参加者を消す。設計の `replan` と同じ |
| 回収の規則が要件の討議の裁定どおり | 要件 7.1・7.2・7.7・設計の「流れの決めごと」 | 印は見張りの 1 種類だけ・回収は「作業中かつ見張り無し」だけ・「停止要請中」「止まった」は回収しない・`status` は `absent` の印。一致（問題 1 はこの規則の端に残る穴） |
| 待ちの振る舞いが要件 13 に合う | 設計の `wait.rs` | 終わるまで何も出さない・1 秒の `metadata`・10 回に 1 回の読み・30 回に 1 回の `Tick`。セッションが起きるのはコマンドの終わり 1 回。合う |
| 終了コードの一貫性 | 設計の「コマンドの表」「終了コード」 | 0／1／2／3 がコマンド間で一貫。`watch` が「すでに止まっている」で 0 を返すのは、呼び出し側の次の行動が同じなので許容。`stopped --wait` の 3 は問題 1 |
| 要件の対応表の正直さ | 要件 1〜14 と表を突き合わせ | 抜けは無い。14.1「版だけ」を「消した記録 1 件を残す」と読む断りが本文にある。7.1 の `pid` は「人が読むため」と明記していて、生死の根拠（ロックファイル）と混ぜていない |
| ファイルの大きさと置き方 | `structure.md` | 兄弟テスト `<stem>_<module>.rs`・`main_<module>.rs`・`tests/real.rs`（既存の `real_pasta_online.rs` と同じ置き方）・1,000 行以下の見込み。合う |

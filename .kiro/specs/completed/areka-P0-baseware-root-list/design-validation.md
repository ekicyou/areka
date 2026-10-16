# 設計の検証: areka-P0-baseware-root-list

> 2026-10-16・ブランチ `claude/areka-P0-baseware-root-list-455a14`（`23711017` の design.md）。対話なしで判定した（`kiro-validate-design`）。
> 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C7 の行）・規則 `rules/design-review.md`。設計の要の主張は、今のコードで確かめた（下の「コードで確かめたこと」）。

## まとめ

設計は、フォルダ名から場所を引く口を `areka-ghost::catalog_roots` の 1 か所に集め、1 根の走査を crate の外へ出さないことで「引く規則は 1 つ」（要件 2.6）を型で守る。`areka-nar` は宛先の根を trait で尋ねるだけにして、依存の向きを今のまま保つ。要件 1〜12 の全項目が部品と口に結び付いており、呼び手の付け替えの表は今のコードの呼び手と一致した。直したほうがよい所は 3 つあるが、どれも設計の骨組みは変えずに直せる。

## 直したほうがよい所（3 つ）

### 1. `areka-nar` の作業フォルダを持っているのは `lib.rs` なのに、変えるファイルの一覧に無い

- **何が問題か**: 設計は「根ごとの作業フォルダ・戻しの列 1 本」を `install.rs`・`plan.rs`・`error.rs` の変更としている。だが今のコードで作業フォルダを 1 つ作り、その場所を記録の欄に渡しているのは `crates/areka-nar/src/lib.rs` の `NarArchive::install`・`place`（`WorkArea::create(request.root)`・`work: &mut Option<PathBuf>`）と `log_failure`（`work` の欄）である。`lib.rs` の冒頭の使い方の例（`rust,no_run`＝文書のテストで組み立てられる）も `InstallRequest { root, target_ghost: None }` と書いている。失敗の値の約束「`survivors` はどれも `work` の下にある」（`lib.rs` の `place` の注記・`error.rs` の `survivors` の注記）も、作業フォルダが複数になると書き直しが要る。
- **困ること**: タスクがこのファイルを拾わないと、根をまたぐインストールの中心（要件 5.9）が抜けるか、作業の途中で範囲が膨らむ。文書のテストは `cargo test` で赤になる。
- **直し方**: 「Modified Files」と AreKaNarRoots の節に `crates/areka-nar/src/lib.rs` を足し、①`place` が配置の根ごとに `WorkArea` を作る、②記録の欄 `work` を作業フォルダの列にする（綴りは `ghost_store` と同じく `; ` でつなぐ等）、③`survivors` の約束を「どれかの作業フォルダの下」に改める、④使い方の例を `SingleRoot` の形に直す、の 4 点を書く。
- **要件**: 5.9・12.4・11.6
- **設計の場所**: 「File Structure Plan」の `crates/areka-nar/src/`・「Modified Files」・「AreKaNarRoots」

### 2. 根を外した後（要件 8.4・8.7 の状態）に、今のゴーストを和から引き直す所が別のゴーストの名前を拾う

- **何が問題か**: `pin_to_current(target, &GhostDecision)` は切替先の場所（`dir`）だけを今のゴーストへ差し替える。`SwitchTarget.name`（`OnGhostChanging` の参照に載る `ChangeTarget.name` の元）は、外した根の下の今のゴーストではなく、新しく見えるようになった別の根のゴーストの descript の `name` のまま残る（`GhostDecision` に名前の欄が無いので、今の形では埋められない）。同じく `running_name(&ctx.roots, …)` は和をフォルダ名で引くので、この状態では起動中の印に別のゴーストの名前を書く（同じフォルダ名が無ければフォルダ名で代える）。
- **困ること**: 要件 8.7 の「今のゴーストを選んだものとして扱う」と、ゴーストに渡る中身（場所は今のゴースト・名前は別のゴースト）が食い違う。12.5 のテストが場所だけを見ると、この食い違いは赤にならない。落ちた後の知らせ（起動中の印＝Ref7）にも別の名前が出る。
- **直し方**: `pin_to_current` は場所と一緒に名前も今のゴーストのものにする（今のゴーストの窓口が持つ名前＝`ghost_switch.rs` の `PrevGhost.name` を作るのと同じ出どころ、または `ghost.dir` の descript を読み直す）。`running_name` は和を引かずに `ghost.dir` の descript を読む形にすると、根の並びと無関係に正しくなる（根が 1 つのときは今と同じ結果＝要件 11.1）。12.5 の檻に「名前も今のゴーストのもの」を足す。
- **要件**: 8.4・8.7・3.5（起動中の印に書く名前）・12.5
- **設計の場所**: 「SameFolderPin」・「呼び手の付け替え」の `boot_resolve.rs::running_name` と `main.rs` の行

### 3. つながっていないネットワークのドライブの根で、UI スレッドが長く止まりうる

- **何が問題か**: 要件 9.3 に従い、和と引き当ては使うたびに各根の在否（`is_dir`）を見て、中を走査する。これはメニューを出すとき・切替・起動のときに UI スレッドで行われる。根が外付けのドライブなら速く失敗するが、切れている割り当て済みのネットワークのドライブ（`Z:\` など）や UNC パスは、OS の問い合わせが数十秒待つことがある。そのあいだゴーストの表示と会話が止まる。今の根は実行ファイルの隣の 1 つだけなので、この危険はこの spec で初めて入る。設計の「Performance & Scalability」は走査の量だけを扱い、この待ちに触れていない。
- **困ること**: 利用者には「メニューを開いたらゴーストが固まった」としか見えず、記録にも残らない（9.1 の記録は起動と変更のときだけ）。
- **直し方**: 今回は直さず、限界として書き残すのが釣り合う。「Performance & Scalability」に `ponytail:` の注記（ネットワークの根は在否の問い合わせで UI スレッドを止めうる・困ったら在否の確かめを別スレッドへ出すか、最後に見た結果を短い時間だけ覚える）を足し、`dist/README.txt` の置き場の節に「ネットワークのドライブを置き場にすると、つながっていないときにゴーストが止まることがある」を 1 行足す。
- **要件**: 9.3・6.3（止めないの趣旨）・8.2・10.1
- **設計の場所**: 「Performance & Scalability」・「CatalogRoots」の実装の注記・「説明書と構成の文書」

## よい所

- **引く規則が型で 1 つに閉じている**: 1 根の走査（`scan_ghosts`・`scan_balloons`）を `pub(crate)` にし、crate の外から一覧と引き当てを得る口を `catalog_roots.rs` だけにした。呼び手が「根＋フォルダ名」で場所を組み直す道が残らないので、要件 2.6・2.8 を後続（`dev-folder-alias` など）が崩しにくい。`areka-nar` は trait `InstallRoots` で根を尋ねるだけで、`areka-ghost` にも bin にも依存しない（今の依存の向きのまま）。
- **今のコードに根を下ろした計画**: 呼び手の付け替えの表は、今のコードで `BasewareRoot`・`ctx.root`・`list_ghosts`／`list_balloons` を使う本番の所（`boot_config`・`boot_resolve`・`main`・`ghost_switch`・`shell_balloon_*`・`menu/*_frame`・`install/*`・`update/desk`・`readme/opener`）と一致した。1,000 行に近いファイル（`main.rs` 956 行・`ghost_switch.rs` 902 行）は数行の置き換えだけにし、足す分を兄弟のファイル（`roots/`・`same_folder_pin.rs`）へ出している。

## 判定

**GO**

- **理由**: 骨組み（和と引き当ての 1 か所・trait での宛先・`BootContext` の並び・メニューの 2 枠）は今の作りと噛み合い、要件の全項目に道筋がある。上の 3 つはどれも設計の文の書き足しと小さな口の直しで済み、作り直しは要らない。
- **次の段取り**: 設計の討議（`/kiro-design-discussion areka-P0-baseware-root-list`）で上の 3 つを議題にし、決めた分を design.md に書き足してから `/kiro-spec-tasks areka-P0-baseware-root-list` へ進む。

## コードで確かめたこと（2026-10-16）

| 設計の主張 | 確かめた結果 |
|---|---|
| 一覧は 1 根を見る `list_ghosts(&BasewareRoot)`・`list_balloons`、項目は `dir` と `identity` | 一致（`crates/areka-ghost/src/catalog.rs`・414 行）。外の crate からの呼び手は `areka` の bin だけ |
| `BootContext.root` が根 1 つ・`resolve_root_from(env, exe)` が純粋な判断 | 一致（`crates/areka/src/boot_config.rs`）。`root_resolved` の記録を読むテストや道具は他に無い |
| 記憶の鍵 `PersistKey` に `LastRunning` は空文字を「無し」と読む・檻 `all_families()`・`canonical_key_round_trips_with_to_canonical_string` | 一致（`crates/areka-sylphya/src/persist/mod.rs`・`format.rs`・`persist_tests.rs`）。`save_scope` は読んで重ねて書く |
| メニューの枠は 7 つ固定（`Frame::ORDER`・`slots`・`FRAME_CAPTIONS`・`group_of`） | 一致（`menu/mod.rs`・`captions.rs`・`plan.rs`）。`slots` と `FRAME_CAPTIONS` は `frame as usize` で引くので、新しい枠は列挙の順と表の順をそろえる必要がある |
| 今のファイル選択は別スレッドで、持ち主の窓なし・`alert::suppressed()` で抑える | 一致（`install/pick.rs` は `GetOpenFileNameW`・`install/desk.rs` の `start_pick`）。フォルダ選択の `IFileOpenDialog` はリポジトリで初めて |
| `windows` の機能 `Win32_UI_Shell`・`Win32_System_Com` は根の `Cargo.toml` で有効 | 一致（`[workspace.dependencies.windows]`） |
| `areka-nar` の `InstallRequest { root, target_ghost }`・`install.rs` 573 行 | 一致。ただし作業フォルダを作るのは `lib.rs`（上の 1 つ目） |
| 行数（1,000 行の目安） | `main.rs` 956・`ghost_switch.rs` 902・`ghost_session.rs` 873・`update/desk.rs` 703・`boot_resolve.rs` 609・`boot_config.rs` 503。設計の足し方なら内側に収まる |

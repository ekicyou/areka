//! inproc_fixture.rs — InProc 実 DLL 駆動のテストゴースト組立と成果物 DLL locate（テスト支援）。
//!
//! 本ファイルの LOCATE 部（task 4.1）は、`cargo test --workspace` がビルドした x64 cdylib
//! `shiori4_testdll.dll` の**単一の正準位置**を決定論的に特定する [`locate_built_test_dll`] を提供する。
//! 組立部（`assemble_test_ghost`・task 4.2）は同ファイルへ後続タスクが追加し、consume は tasks 5.x。
//!
//! # 単一正準位置・フォールバックなし（要件 1.2・5.4／design.md D-1）
//! 正準位置は `target/<profile>/deps/shiori4_testdll.dll`（= テストバイナリ `current_exe()` と同一
//! ディレクトリ）**ただ 1 箇所**である。これは task 1.1 の uplift spike が実測で確定した事実で、
//! `cargo test`/`cargo test --workspace` は cdylib を deps へリンクするが `target/<profile>/`（deps の
//! 親）への uplift は起こさない（design.md D-1 の散文が推定した「deps を pop」は実測で誤り＝spike が
//! 上書きする）。glob／mtime／多段フォールバックは**採らない**——将来 cargo 挙動変化時に古い deps/ の
//! DLL を拾って壊れたビルドを隠蔽する silent green を防ぐため（fail-visible・設計討議#1）。挙動変化は
//! 不在時の明示 panic で即座に顕在化させる。
//!
//! 本モジュールの一部の pub 項目は、consume する後続タスク（4.2 組立・5.x e2e）が結線されるまで
//! この test binary 内で未使用になり得るため、dead-code 警告を抑止する。
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// `cargo test --workspace` がビルドした x64 cdylib `shiori4_testdll.dll` の**単一の正準位置**を
/// 決定論的に特定して返す（要件 1.2・5.4／design.md D-1）。
///
/// 正準位置は `target/<profile>/deps/shiori4_testdll.dll`（= テストバイナリ `current_exe()` と同一
/// ディレクトリ）**ただ 1 箇所**である。導出はテストバイナリの `current_exe()` の親ディレクトリ（deps）へ
/// 契約定数 [`shiori4_testdll::DLL_FILE_NAME`] を join するだけで、`deps` を pop しない
/// （task 1.1 uplift spike の実測に忠実・design.md D-1 散文の「deps を pop」推定を spike が上書き）。
///
/// # 単一正準位置・フォールバックなし（design.md D-1・設計討議#1）
/// glob／mtime／`target/<profile>/`（deps の親）への多段フォールバックは**採らない**。将来 cargo の
/// レイアウトが変わって古い deps/ の DLL が残った場合、フォールバック glob はその陳腐 DLL を拾って
/// 壊れたビルドを silent green で隠蔽し得る——単一位置固定＋不在時の明示 panic なら、レイアウト変化を
/// fail-visible に即座に顕在化できる（`cargo test --workspace` の workspace-artifact 前提と同律）。
///
/// # Panics
/// 正準位置に DLL が不在の場合、次の一手を示す明示的なメッセージで panic する（silent skip はしない・
/// 非在パスも返さない）。この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力
/// するため、単独実行時は先に `cargo build -p shiori4-testdll` を実行すること。
pub fn locate_built_test_dll() -> PathBuf {
    // current_exe = target/<profile>/deps/<name>-<hash>.exe（テストバイナリ）。
    let test_exe = std::env::current_exe().expect("test executable path is available");

    // 親ディレクトリ = target/<profile>/deps（cdylib もここへ出力される・唯一の正準ディレクトリ）。
    // deps を pop しない（task 1.1 spike 実測）。
    let deps_dir = test_exe
        .parent()
        .expect("test executable resides in a deps directory");

    // 正準位置 = target/<profile>/deps/shiori4_testdll.dll（契約定数を testdll crate から参照）。
    let canonical_dll = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);

    assert!(
        canonical_dll.exists(),
        "ビルド済みテスト DLL が正準位置に不在: {}\n\
         この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置へ出力する。\
         単独実行時は先に `cargo build -p shiori4-testdll` を実行すること（フォールバックは設けない・\
         design.md D-1）。",
        canonical_dll.display()
    );

    canonical_dll
}

/// `assemble_test_ghost` が組み立てた一時テストゴーストの所有ハンドル（RAII クリーンアップ付き）。
///
/// `root` は `resolve()`／`boot()` へ渡す ghost_root（`ghost/master/descript.txt` 起点・
/// design.md「fixture 組立と DLL locate」）。[`TempGhost::root`] で取得する。`Drop` で temp root を
/// 再帰削除する（design.md「テスト終了時に削除」——ベストエフォート・失敗は無視）。
pub struct TempGhost {
    root: PathBuf,
}

impl TempGhost {
    /// 組み立てた一時ゴーストの root（`resolve()`／`boot()` の ghost_root として渡す）。
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for TempGhost {
    fn drop(&mut self) {
        // ベストエフォート削除（design.md「テスト終了時に削除」）。既に消えていても・
        // 共有違反等で失敗しても、テスト結果を左右しないため無視する。
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// `src` ディレクトリツリーを `dst` へ再帰的に**実体化**する（外部 crate 非依存・手書き）。
///
/// ディレクトリ構造を再現し、通常ファイルは [`materialize_file`]（ハードリンク優先・フォールバック
/// copy）で張る——emo2 実物 shell の surfaces.txt＋PNG 一式を verbatim 流用するため（要件 4.1）。
/// ハードリンク先は実物とバイト完全同一なので忠実度は copy 以上であり、新規バイト列を書かないため
/// Defender 再走査による FS 飽和（spine starvation）を回避する（`materialize_file` の rationale 参照）。
fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst)
        .unwrap_or_else(|e| panic!("create dir {} に失敗: {e}", dst.display()));

    for entry in std::fs::read_dir(src)
        .unwrap_or_else(|e| panic!("read_dir {} に失敗: {e}", src.display()))
    {
        let entry = entry.unwrap_or_else(|e| panic!("dir entry 取得に失敗（{}）: {e}", src.display()));
        let file_type = entry
            .file_type()
            .unwrap_or_else(|e| panic!("file_type 取得に失敗（{:?}）: {e}", entry.path()));
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&from, &to);
        } else {
            materialize_file(&from, &to);
        }
    }
}

/// `src` の 1 ファイルを `dst` へ**実体化**する。まず**ハードリンク**を試み、失敗時のみ
/// `std::fs::copy` へフォールバックする。
///
/// # なぜハードリンク優先か（FS 飽和＝兄弟 spine テスト starvation の根絶）
/// テストゴースト組立は emo2 実物 shell（65 ファイル・約 2.5MB）＋ビルド済み cdylib を配置する。
/// `std::fs::copy` は**新規バイト列を書き込む**ため、Windows Defender 等のリアルタイム保護が
/// 生成直後のファイル（特に実行可能な DLL）を走査して CPU を占有し、同一 `ghost` テストバイナリ内で
/// 並列実行される spine 側の協調反復ループ（`spine_e2e_test.rs`——壁時計を持たず relay/dispatcher
/// スレッドの被スケジューリングに依存）を starve させ、決定論緑をフレークへ転落させる（レビュー
/// reject の回帰根因）。ハードリンクは**既存（=走査済み）の実体へ新しいディレクトリ項目を張るだけ**で
/// 新規バイト列を書かないため、この再走査ストームが発生せず、かつリンク先は実物とバイト完全同一
/// （`copy` 以上に verbatim 忠実・要件 4.1）。`Drop` の `remove_dir_all` はリンク項目のみを消す
/// （元の emo2 実物・deps DLL は別リンクが残るため無傷）。
///
/// フォールバック（`copy`）は、`dst` が `src` と別ボリューム上でハードリンク不可な環境
/// （temp と repo が別ドライブ等）でも組立が成立するための保険である。同一ボリューム（既定 temp が
/// repo と同一ドライブ）ではハードリンク経路が使われ、飽和回帰は生じない。
fn materialize_file(from: &Path, to: &Path) {
    if std::fs::hard_link(from, to).is_ok() {
        return;
    }
    // 別ボリューム等でハードリンク不可なら実コピーへフォールバック（機能的等価・忠実度同等）。
    std::fs::copy(from, to)
        .unwrap_or_else(|e| panic!("materialize {} -> {} に失敗: {e}", from.display(), to.display()));
}

/// emo2 実物 shell 資産のアンカー（`crates/areka-ghost` = `CARGO_MANIFEST_DIR` 相対・
/// `crates/areka/tests/emo2_real_run.rs` の `../pilot/...` 規約に一致）。
fn emo2_shell_master_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/shell/master")
}

/// 実在ゴーストの外観資産を流用した最小テストゴーストを一意 temp root へ組み立てる
/// （要件 4.1・4.2・4.3／design.md「fixture 組立と DLL locate」Batch/Job Contract）。
///
/// 手順:
/// 1. 一意 temp root（`areka_shiori4_test_ghost_<tag>`）を確定し、冪等性のため事前削除する
///    （spine の `unique_temp_dir`＋事前削除の流儀・Idempotency）。
/// 2. `ghost/master/descript.txt` を UTF-8 生成する。`shiori,` 行は**新設のテスト DLL**
///    （`shiori4_testdll.dll` == [`shiori4_testdll::DLL_FILE_NAME`]）を指す（要件 4.2）。
/// 3. emo2 実物 `shell/master/`（surfaces.txt＋PNG 一式）を `root/shell/master/` へ再帰的に実体化する
///    （[`copy_dir_recursive`]＝ハードリンク優先）——独自の最小 shell は作らず実物をそのまま流用する
///    （要件 4.1）。
/// 4. [`locate_built_test_dll`] が特定した実体 DLL（1.4 完成）を `root/ghost/master/` へ配置する
///    ——`mount.shiori.dir.join(file)` がこの DLL を指す（要件 4.3）。
/// 5. balloon 資産は**非同梱**（起動〜終話経路＝`resolve()` が balloon を消費しない判断・要件 4.2）。
///
/// 返り値 [`TempGhost`] の Drop がテスト終了時に temp root を削除する。
///
/// # 組立の直列化（同時 FS メタデータ churn による spine starvation の除去）
/// 本関数の FS 実体化区間はプロセス全体で 1 本の [`std::sync::Mutex`] で直列化する。個々の組立は
/// ハードリンク優先（[`materialize_file`]）で新規バイト列を書かないが、同一 `ghost` テストバイナリ内で
/// **複数の組立が同時に**多数の NTFS メタデータ操作（`CreateHardLink`／`CreateDirectory`／
/// `remove_dir_all`）を叩くと、瞬間的なメタデータ競合が spine 側の協調反復ループ（壁時計なし）を
/// 稀に starve させ得る。任意時点で組立を高々 1 本に抑えることで、単一組立時の決定論的挙動
/// （実測 0 失敗）へ収束させる。ロックは FS 区間だけを覆い、`resolve`/`boot` 等の consume は覆わない
/// （読み取り専用消費は共有 fixture を並行に読んでよい）。
pub fn assemble_test_ghost(tag: &str) -> TempGhost {
    // 組立の FS 実体化区間をプロセス全体で直列化（同時メタデータ churn を避ける・上記 rationale）。
    // poison は無視（組立途中 panic 後も後続組立は健全にやり直せる——各 root は tag で独立）。
    static ASSEMBLE_FS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _fs_guard = ASSEMBLE_FS_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    // 1. 一意 temp root＋事前削除（冪等・Idempotency）。
    let root = std::env::temp_dir().join(format!("areka_shiori4_test_ghost_{tag}"));
    let _ = std::fs::remove_dir_all(&root);

    // 2. ghost/master/descript.txt（`shiori,` 行はテスト DLL を指す・要件 4.2）。
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master)
        .unwrap_or_else(|e| panic!("create ghost/master に失敗: {e}"));
    std::fs::write(
        ghost_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nname,Shiori4TestGhost\nshiori,{}\nseriko.defaultsurfacedirectoryname,master\n",
            shiori4_testdll::DLL_FILE_NAME
        )
        .as_bytes(),
    )
    .unwrap_or_else(|e| panic!("write ghost descript.txt に失敗: {e}"));

    // 3. emo2 実物 shell/master/ を再帰的に実体化（ハードリンク優先・自作最小 shell を作らない・要件 4.1）。
    let shell_src = emo2_shell_master_src();
    assert!(
        shell_src.is_dir(),
        "emo2 実物 shell/master/ が見つからない（アンカー規約を確認）: {}",
        shell_src.display()
    );
    copy_dir_recursive(&shell_src, &root.join("shell").join("master"));

    // 4. 特定済みビルド済み DLL（4.1）を ghost/master/ へ配置（要件 4.3・`mount.shiori.dir.join(file)`）。
    //    ハードリンク優先（フォールバック copy）——新規 DLL バイト列の書き込みを避け、Defender の
    //    実行可能ファイル再走査による FS 飽和＝spine starvation を根絶する（`materialize_file` 参照）。
    let built_dll = locate_built_test_dll();
    materialize_file(&built_dll, &ghost_master.join(shiori4_testdll::DLL_FILE_NAME));

    // 5. balloon は非同梱（要件 4.2）——何も配置しない。

    TempGhost { root }
}

/// InProc テスト一族が共有する**一度だけ組み立てる**テストゴーストの root を返す
/// （`OnceLock` 直列化・要件 4.1〜4.3／design.md「fixture 組立と DLL locate」）。
///
/// # なぜ共有 fixture か（FS 飽和による兄弟テスト starvation の除去）
/// [`assemble_test_ghost`] は emo2 実物 shell/master/ の PNG ツリー再帰コピー＋ビルド済み cdylib の
/// `fs::copy` という**重い並行ファイル I/O** を伴う。同一 `ghost` テストバイナリ内で cargo 既定の
/// 並列実行の下、読み取り専用の観測 assert がそれぞれ独立に組立を反復すると、FS が飽和して
/// spine 側の協調反復テスト（`spine_e2e_test.rs` S1/S3/S4——壁時計を持たず relay/dispatcher
/// スレッドの被スケジューリングに依存する `for _ in 0..N { Tick; yield_now }` 境界）を starve させ、
/// 決定論緑がフレーク（≈33%）に転落する。これを根絶するため、読み取り専用消費者は**この単一の
/// 共有ゴースト**を使い回し、N 消費者に対し再帰コピーは合計 1 回に抑える。
///
/// # 共有の安全性・意図的リーク・プロセス跨ぎ再利用（プロセス寿命 fixture）
/// - boot/`resolve` はこのゴーストを**読むだけ**（変更しない）なので、多数の消費者が同一 root を
///   並行に読むのは安全である。最初の複数呼出者は [`OnceLock::get_or_init`] 上で無害に競合し、
///   実際に組み立てるのはただ 1 人（他は完成を待つ）。
/// - `static` は**決して Drop されない**。したがってここで組み立てた temp dir は**意図的に
///   クリーンアップされない**——固定パスのプロセス寿命共有 fixture として存置する。これは FS 飽和
///   回帰を除去するための計画的トレードオフである（1 回きりの temp dir 残存を許容して、テスト
///   ごとの反復コピーによる starvation を消す）。破棄あり（Drop 削除）のケースが要る場合は
///   [`assemble_test_ghost`] を直接使うこと。
/// - **プロセス跨ぎ再利用**: 固定パスに前プロセスがリークした共有ゴーストが**健全なまま残って
///   いれば作り直さずそのまま再利用する**（[`shared_ghost_is_fresh`]）。これは `cargo test` を
///   反復起動するワークフロー（各起動が別プロセス）で決定的に効く——再利用時は組立の FS 変更
///   （前回リーク dir の `remove_dir_all` 事前削除＋65 ハードリンク再張り）を**丸ごと省く**ため、
///   spine 協調ループと競合する残余メタデータ churn が消え、read 経路が実質 stat 数回まで軽くなる。
///   健全性は「shell/surfaces.txt＋ghost/master DLL 実在・balloon 非在・かつ**キャッシュ DLL が
///   現行ビルド済み DLL より古くない**」で判定する（DLL 再ビルド＝別 inode で mtime が進むと陳腐化
///   とみなし作り直す。後続タスク 5.x が DLL を実ロードしても常に最新実体を掴む）。
pub fn shared_test_ghost() -> &'static Path {
    static SHARED: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let root = SHARED
        .get_or_init(|| {
            let root = std::env::temp_dir().join("areka_shiori4_test_ghost_shared");

            // 前プロセスがリークした健全な共有ゴーストが残っていれば、FS 変更ゼロで再利用する
            // （反復起動での残余 churn＝spine starvation の除去・上記ドキュメント参照）。
            if shared_ghost_is_fresh(&root) {
                return root;
            }

            // 無いか陳腐化していれば固定タグで**ちょうど一度**組み立てる（get_or_init が初期化を直列化）。
            let temp = assemble_test_ghost("shared");
            let root = temp.root().to_path_buf();
            // TempGhost の Drop（remove_dir_all）を抑止して root をリーク＝プロセス寿命存置。
            // static に格納する PathBuf は Drop されないため temp dir は残る（上記ドキュメント参照）。
            std::mem::forget(temp);
            root
        });

    // 永続状態を**毎呼出し**でリセットする（「shared_test_ghost は永続ファイルを持たない＝常に
    // 初回扱い」不変条件の回復・i1/i2 等が明示依存）。position-persist で永続書込が実際に効くように
    // なったため、共有ゴーストを boot して初回挨拶を完走したテストが
    // `<root>/ghost/master/profile/areka/sylphya.toml`（[boot] count 等）を書き込み、固定パスの
    // リーク再利用で cargo 実行を跨いで生存する。放置すると後続 boot が「2 回目扱い」になり
    // OnFirstBoot をスキップして挨拶列が届かず、初回挨拶を待つテストが hang する。ここで毎回
    // areka profile ディレクトリを消して初回扱いを保証する（ゴースト構造の hardlink 再利用は温存）。
    let persist_dir = root
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    let _ = std::fs::remove_dir_all(&persist_dir);

    root.as_path()
}

/// 固定パスに残る共有ゴーストが**そのまま再利用してよいほど健全か**を判定する
/// （[`shared_test_ghost`] のプロセス跨ぎ再利用ガード）。
///
/// 健全条件（全て満たすとき true）:
/// - `shell/master/surfaces.txt` が実在（emo2 実物 shell 流用の証跡・要件 4.1）。
/// - `ghost/master/<DLL>` が実在（DLL 物理配置の証跡・要件 4.3）。
/// - `balloon/` が非在（balloon 非同梱の証跡・要件 4.2）。
/// - キャッシュ DLL が**現行ビルド済み DLL より古くない**（再ビルド陳腐化の検出）。ビルド済み DLL の
///   再ビルドは新 inode 生成で mtime を進めるため、旧ハードリンクを掴んだキャッシュはここで陳腐と
///   判定され作り直される（後続タスク 5.x の実ロードが常に最新実体を掴む保証）。
///
/// mtime 取得不能などの疑わしいケースは**安全側**（false＝作り直す）へ倒す。
fn shared_ghost_is_fresh(root: &Path) -> bool {
    let surfaces = root.join("shell").join("master").join("surfaces.txt");
    let cached_dll = root.join("ghost").join("master").join(shiori4_testdll::DLL_FILE_NAME);
    let balloon = root.join("balloon");

    if !surfaces.is_file() || !cached_dll.is_file() || balloon.exists() {
        return false;
    }

    // DLL 鮮度: 現行ビルド済み DLL が cached より新しければ（＝再ビルドされた）陳腐＝作り直す。
    let built = locate_built_test_dll();
    let built_mtime = std::fs::metadata(&built).and_then(|m| m.modified());
    let cached_mtime = std::fs::metadata(&cached_dll).and_then(|m| m.modified());
    match (built_mtime, cached_mtime) {
        (Ok(built_t), Ok(cached_t)) => cached_t >= built_t,
        // mtime 取得不能なら安全側で作り直す。
        _ => false,
    }
}

/// このテストゴーストの LOCATE 檻。
mod tests {
    /// `locate_built_test_dll()` は (a) 実在し (b) ファイル名が `shiori4_testdll.dll`
    /// （== [`shiori4_testdll::DLL_FILE_NAME`]）で (c) 親ディレクトリ名が `deps` のパスを返すこと
    /// （要件 1.2・5.4／design.md D-1「単一正準位置・フォールバックなし」）。
    #[test]
    fn locate_returns_existing_canonical_deps_dir_dll() {
        let dll = super::locate_built_test_dll();

        // (a) 実在（不在なら locate 自身が明示 panic するので、成功戻り値は必ず実在）。
        assert!(dll.exists(), "locate は実在するパスを返すこと: {}", dll.display());

        // (b) ファイル名 == 契約定数 DLL_FILE_NAME。
        assert_eq!(
            dll.file_name().and_then(|s| s.to_str()),
            Some(shiori4_testdll::DLL_FILE_NAME),
            "locate 戻り値のファイル名は契約定数と一致すること: {}",
            dll.display()
        );

        // (c) 親ディレクトリ名 == "deps"（単一正準位置＝deps・design.md D-1）。
        assert_eq!(
            dll.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()),
            Some("deps"),
            "locate 戻り値の親ディレクトリは deps であること: {}",
            dll.display()
        );
    }

    /// locate 戻り値は絶対パスで `target` ツリー内を指すこと（正準位置の健全性・design.md D-1）。
    #[test]
    fn locate_returns_absolute_path_inside_target_tree() {
        let dll = super::locate_built_test_dll();
        assert!(dll.is_absolute(), "locate 戻り値は絶対パスであること: {}", dll.display());
        assert!(
            dll.components().any(|c| c.as_os_str() == "target"),
            "locate 戻り値は target ツリー内を指すこと: {}",
            dll.display()
        );
    }

    // ---- assemble_test_ghost 組立檻（task 4.2） ----
    //
    // 読み取り専用の観測 assert（要件 4.1・4.2・4.3）は**単一のテスト**へ畳み込み、**共有 fixture**
    // （[`super::shared_test_ghost`]）に対して全て実行する。これは 2 段の FS 飽和対策である:
    //   (1) 共有 fixture＝`OnceLock` により組立は**1 回**（各 assert が再組立しない）。
    //   (2) 単一テストへ畳む＝read 経路の並行スレッドを 1 本に抑える（`package::resolve` による
    //       shell dir 走査を別テスト群と同時多発させない）。
    // これにより read 経路の同時 FS 負荷が、単一の破棄あり組立（Drop 檻）と同程度まで下がり、
    // 壁時計を持たない兄弟 spine 協調ループを starve させない（レビュー reject の回帰根因）。
    // 破棄あり（Drop 削除）のケースだけが [`super::assemble_test_ghost`] を直接呼んで独自 root を組む。

    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;

    /// 共有テストゴースト（[`super::shared_test_ghost`]）が要件 4.1・4.2・4.3 の観測可能な完了条件を
    /// すべて満たすこと。read 経路の並行スレッドを 1 本へ抑えるため 4 観点を単一テストへ畳み込む
    /// （FS 飽和による spine starvation 回避——モジュール冒頭コメント参照）:
    /// - **4.2 実マウント解決**: `package::resolve` が Ok を返し、`shiori,` 行がテスト DLL 名
    ///   （== [`shiori4_testdll::DLL_FILE_NAME`]）を指す（boot が内部で通すのと同一契約）。
    /// - **4.3 DLL 物理配置**: 特定済みビルド済み DLL が `ghost/master/shiori4_testdll.dll` に在る。
    /// - **4.2 balloon 非同梱**: `balloon` ディレクトリを同梱しない。
    /// - **4.1 emo2 実物 shell 流用**: `shell/master/surfaces.txt` と入れ子 PNG
    ///   （`purple/0/base1.png`）が実在＝自作最小 shell でなく実物の入れ子構造まで流用している。
    #[test]
    fn assembled_shared_ghost_satisfies_mount_dll_shell_and_no_balloon() {
        let root = super::shared_test_ghost();

        // 4.2 実マウント解決（boot が内部で通すのと同一契約）が Ok＝完全な解決可能ゴースト。
        let mount = package::resolve(root, DefaultEncoding::Utf8)
            .expect("組み立てた共有ゴーストは実マウント解決を成功裏に通過すべき（shell dir 込み）");
        // `shiori,` 行がテスト DLL 名（== 契約定数）を指す（要件 4.2）。
        assert_eq!(
            mount.shiori.file,
            Some(shiori4_testdll::DLL_FILE_NAME.to_string()),
            "mount.shiori.file は組立時に書いたテスト DLL 名を指すべき"
        );

        // 4.3 特定済みビルド済み DLL が ghost/master/ へ物理配置されている。
        let dll = root
            .join("ghost")
            .join("master")
            .join(shiori4_testdll::DLL_FILE_NAME);
        assert!(
            dll.is_file(),
            "ビルド済みテスト DLL が ghost/master/ へ物理配置されているべき: {}",
            dll.display()
        );

        // 4.2 balloon 資産は非同梱（起動〜終話経路＝`resolve()` が balloon を消費しない判断）。
        let balloon = root.join("balloon");
        assert!(
            !balloon.exists(),
            "balloon 資産は非同梱であるべき（要件 4.2）: {}",
            balloon.display()
        );

        // 4.1 emo2 実物 shell 流用（surfaces.txt＋入れ子 PNG まで＝自作最小 shell でない）。
        let shell_master = root.join("shell").join("master");
        assert!(
            shell_master.join("surfaces.txt").is_file(),
            "emo2 実物の surfaces.txt が複製されているべき（自作最小 shell でない・要件 4.1）: {}",
            shell_master.display()
        );
        let nested_png = shell_master.join("purple").join("0").join("base1.png");
        assert!(
            nested_png.is_file(),
            "emo2 実物の入れ子 PNG まで再帰的に流用されているべき: {}",
            nested_png.display()
        );
    }

    /// `TempGhost` の Drop（RAII クリーンアップ）が temp root を実際に削除すること
    /// （design.md「テスト終了時に削除」）。
    #[test]
    fn temp_ghost_drop_removes_the_temp_root() {
        let root_path = {
            let temp = super::assemble_test_ghost("drop_cleanup");
            let p = temp.root().to_path_buf();
            assert!(p.is_dir(), "組立直後は temp root が存在するべき: {}", p.display());
            p
            // ここで temp が drop され、Drop が root を remove_dir_all する。
        };
        assert!(
            !root_path.exists(),
            "TempGhost の Drop 後は temp root が削除されているべき: {}",
            root_path.display()
        );
    }
}

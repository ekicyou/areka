//! inproc_fixture.rs — InProc 実 DLL 駆動のテストゴースト組立と成果物 DLL locate（テスト支援）。
//!
//! 本ファイルの LOCATE 部（task 4.1）は、`cargo test --workspace` がビルドした x64 cdylib
//! `shiori4_testdll.dll` の**単一の正準位置**を決定論的に特定する [`locate_built_test_dll`] を提供する。
//! 組立部は錠つきの共有ゴースト [`shared_test_ghost`] を配る（consume は I1・I2）。
//!
//! # 単一正準位置・フォールバックなし（要件 1.2・5.4／design.md D-1）
//! 正準位置は `target/<profile>/deps/shiori4_testdll.dll`（= テストバイナリ `current_exe()` と同一
//! ディレクトリ）**ただ 1 箇所**である。これは task 1.1 の uplift spike が実測で確定した事実で、
//! `cargo test`/`cargo test --workspace` は cdylib を deps へリンクするが `target/<profile>/`（deps の
//! 親）への uplift は起こさない（design.md D-1 の散文が推定した「deps を pop」は実測で誤り＝spike が
//! 上書きする）。glob／mtime／多段フォールバックは**採らない**——将来 cargo 挙動変化時に古い deps/ の
//! DLL を拾って壊れたビルドを隠蔽する silent green を防ぐため（fail-visible・設計討議#1）。挙動変化は
//! 不在時の明示 panic で即座に顕在化させる。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};

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

/// `src` ディレクトリツリーを `dst` へ再帰的に**実体化**する（外部 crate 非依存・手書き）。
///
/// ディレクトリ構造を再現し、通常ファイルは [`materialize_file`]（ハードリンク優先・フォールバック
/// copy）で張る——emo2 実物 shell の surfaces.txt＋PNG 一式を verbatim 流用するため（要件 4.1）。
/// ハードリンク先は実物とバイト完全同一なので忠実度は copy 以上であり、新規バイト列を書かないため
/// Defender 再走査による FS 飽和（spine starvation）を回避する（`materialize_file` の rationale 参照）。
fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst)
        .unwrap_or_else(|e| panic!("create dir {} に失敗: {e}", dst.display()));

    for entry in
        std::fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {} に失敗: {e}", src.display()))
    {
        let entry =
            entry.unwrap_or_else(|e| panic!("dir entry 取得に失敗（{}）: {e}", src.display()));
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
/// （`copy` 以上に verbatim 忠実・要件 4.1）。共有ゴーストを消してもリンク項目のみが消える
/// （元の emo2 実物・deps DLL は別リンクが残るため無傷）。
///
/// フォールバック（`copy`）は、`dst` が `src` と別ボリューム上でハードリンク不可な環境
/// （`target` を別ドライブへ向けた等）でも組立が成立するための保険である。根は既定で
/// ワークツリーの `target\test-roots` の下（repo と同一ドライブ）なのでハードリンク経路が使われ、飽和回帰は生じない。
fn materialize_file(from: &Path, to: &Path) {
    if std::fs::hard_link(from, to).is_ok() {
        return;
    }
    // 別ボリューム等でハードリンク不可なら実コピーへフォールバック（機能的等価・忠実度同等）。
    std::fs::copy(from, to).unwrap_or_else(|e| {
        panic!(
            "materialize {} -> {} に失敗: {e}",
            from.display(),
            to.display()
        )
    });
}

/// emo2 実物 shell 資産のアンカー（検体の窓口から得る）。
///
/// 窓口が返すのは借用なので、`SampleRoot` を **プロセス寿命で保持**する。段 ③ で `Drop` が
/// 展開した複製を消すため、関数内の一時値にすると借用の元がその場で消える。
fn emo2_shell_master_src() -> PathBuf {
    use std::sync::LazyLock;

    use sample_ghost_kit::SampleRoot;

    static EMO2: LazyLock<SampleRoot> =
        LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

    EMO2.folder().join("shell").join("master")
}

/// 与えられた `root` の直下へテストゴーストの実体（descript／shell／DLL）を配置する
/// （[`shared_test_ghost`] の唯一の組立手続き・`OnceLock` が直列にする）。
fn materialize_test_ghost_tree(root: &Path) {
    // 1. 事前削除（冪等・Idempotency——固定パスの共有 fixture を作り直す経路で効く）。
    let _ = std::fs::remove_dir_all(root);

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
    materialize_file(
        &built_dll,
        &ghost_master.join(shiori4_testdll::DLL_FILE_NAME),
    );

    // 5. balloon は非同梱（要件 4.2）——何も配置しない。
}

/// 共有ゴーストの錠。I1・I2・組みの確かめは同じ固定の根を使うので、1 本ずつ順に持つ
/// （持つ間はほかのテストが `profile\areka` の記録を消さない）。
static SHARED_GHOST_LOCK: Mutex<()> = Mutex::new(());

/// 錠を持ったままの共有ゴースト。終了の確かめまで持つこと（`_` で捨てない）。
pub struct SharedGhost {
    _guard: MutexGuard<'static, ()>,
    root: &'static Path,
}

impl SharedGhost {
    /// 共有ゴーストの root（`resolve()`／`boot()` の ghost_root として渡す）。
    pub fn root(&self) -> &Path {
        self.root
    }
}

/// InProc テスト一族が共有する**一度だけ組み立てる**テストゴーストを、錠を取ってから返す
/// （要件 4.1〜4.3／design.md「fixture 組立と DLL locate」）。
///
/// # なぜ共有 fixture か（FS 飽和による兄弟テスト starvation の除去）
/// 組立は emo2 実物 shell/master/ の PNG ツリーの実体化＋ビルド済み cdylib の配置という重い
/// ファイル I/O を伴う。テストごとに組み立てると FS が飽和して spine 側の協調反復テストを
/// starve させるので、組立は `OnceLock` で 1 回に抑え、プロセスをまたいでも再利用する。
///
/// # 置き場と再利用
/// 根は `<ワークスペース>\target\test-roots\areka_shiori4_test_ghost_shared`（ワークツリーごと）。
/// 名前は固定で、前のプロセスが残した実体が健全なら作り直さない（[`shared_ghost_is_fresh`]）。
/// `static` は Drop されないので、この実体は意図して残る。
///
/// # 錠と記録の消去
/// 錠を取った**後で**起動記録 `ghost\master\profile\areka` を消す（毎回の初回扱い）。無いのは
/// 通し、ほかの失敗は panic する（黙って 2 回目扱いで走らせない）。錠の毒は越える。
pub fn shared_test_ghost() -> SharedGhost {
    hold(
        SHARED_GHOST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    )
}

/// [`shared_test_ghost`] の `try_lock` の形。錠が塞がっていれば `None`（記録は消さない）。
pub fn try_shared_test_ghost() -> Option<SharedGhost> {
    match SHARED_GHOST_LOCK.try_lock() {
        Ok(guard) => Some(hold(guard)),
        Err(TryLockError::Poisoned(poisoned)) => Some(hold(poisoned.into_inner())),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// 錠を持った上で起動記録を消し、持ち物にして返す。
fn hold(guard: MutexGuard<'static, ()>) -> SharedGhost {
    let root = shared_root();
    let persist_dir = root
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    match std::fs::remove_dir_all(&persist_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!(
            "共有ゴーストの記録 {} を消せない: {e}",
            persist_dir.display()
        ),
    }
    SharedGhost {
        _guard: guard,
        root,
    }
}

/// 共有ゴーストの根（一度だけ組み立てる・健全な残りは再利用する）。
fn shared_root() -> &'static Path {
    static SHARED: OnceLock<PathBuf> = OnceLock::new();
    SHARED.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("crates/areka-ghost の 2 つ上がワークスペースの根")
            .join("target")
            .join("test-roots")
            .join("areka_shiori4_test_ghost_shared");
        if !shared_ghost_is_fresh(&root) {
            materialize_test_ghost_tree(&root);
        }
        root
    })
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
    let cached_dll = root
        .join("ghost")
        .join("master")
        .join(shiori4_testdll::DLL_FILE_NAME);
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
        // (c) 親ディレクトリ名 == "deps"（単一正準位置＝deps・design.md D-1）。
        assert_eq!(
            dll.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str()),
            Some("deps"),
            "locate 戻り値の親ディレクトリは deps であること: {}",
            dll.display()
        );
    }

    // ---- 共有ゴーストの組立檻 ----
    //
    // 読み取り専用の観測 assert（要件 4.1・4.2・4.3）は read 経路の並行スレッドを 1 本に抑えるため
    // **単一のテスト**へ畳み、共有 fixture（[`super::shared_test_ghost`]）に対して実行する。

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
        let shared = super::shared_test_ghost();
        let root = shared.root();

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

    /// 錠を持つ間は、2 本目の取り手が記録を消さない（#45・I2 の消去が I1 の記録を消す競りを
    /// 1 つのスレッドの順で再現する。錠を外すと 2 本目がすぐ取れて記録を消すので赤）。
    #[test]
    fn the_shared_ghost_is_not_reset_while_another_test_holds_it() {
        let first = super::shared_test_ghost();
        let areka = first
            .root()
            .join("ghost")
            .join("master")
            .join("profile")
            .join("areka");
        std::fs::create_dir_all(&areka).expect("profile/areka を作れること");
        let record = areka.join("sylphya.toml");
        std::fs::write(&record, "[boot]\ncount = 1\n").expect("記録を書けること");

        assert!(
            super::try_shared_test_ghost().is_none(),
            "持っている間は 2 本目が取れないこと"
        );
        assert!(record.is_file(), "持っている間は記録が消えないこと");

        drop(first);
        let second = super::shared_test_ghost();
        assert!(
            !second
                .root()
                .join("ghost")
                .join("master")
                .join("profile")
                .join("areka")
                .exists(),
            "取り直すと前の記録が消えていること"
        );
    }
}

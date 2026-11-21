//! inproc_fixture.rs — InProc 実 DLL 駆動のテストゴースト組立と成果物 DLL locate（テスト支援）。
//!
//! LOCATE 部は、`cargo test --workspace` がビルドした x64 cdylib `shiori4_testdll.dll` の場所を返す
//! [`locate_built_test_dll`] を提供する。組立部は錠つきの共有ゴースト [`shared_test_ghost`] を配る。
//!
//! DLL の場所の決め方は [`shiori4_testdll::locate_built_dll`] が持つ（cargo 1.100 の新しい置き方に対応。
//! 理由と規則はその関数の説明を参照）。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};

/// `cargo test --workspace` がビルドした x64 cdylib `shiori4_testdll.dll` の場所を返す
/// （[`shiori4_testdll::locate_built_dll`] に委ねる・不在や決められないときは panic）。
pub fn locate_built_test_dll() -> PathBuf {
    shiori4_testdll::locate_built_dll()
}

/// `src` ディレクトリツリーを `dst` へ再帰的に**実体化**する（外部 crate 非依存・手書き）。
///
/// ディレクトリ構造を再現し、通常ファイルは [`materialize_file`]（ハードリンク優先・フォールバック
/// copy）で張る——emo2 実物 shell の surfaces.txt＋PNG 一式を verbatim 流用するため。
/// ハードリンク先は実物とバイト完全同一で、新規バイト列を書かないため Defender 再走査による
/// FS 飽和（spine starvation）を回避する（`materialize_file` の説明参照）。
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
/// # なぜハードリンク優先か
/// テストゴースト組立は emo2 実物 shell（65 ファイル・約 2.5MB）＋ビルド済み cdylib を配置する。
/// `std::fs::copy` は新規バイト列を書き込むため、Windows Defender 等のリアルタイム保護が生成直後のファイル
/// （特に DLL）を走査して CPU を占有し、同一 `ghost` テストバイナリ内で並列実行される spine 側の協調反復
/// ループ（`spine_e2e_test.rs`——`BOUND` の壁時計で上限を切る）の到達を遅らせて決定論緑をフレークに
/// する。ハードリンクは既存（走査済み）の実体へ新しいディレクトリ項目を張るだけで新規バイト列を書かず、
/// リンク先は実物とバイト完全同一。共有ゴーストを消してもリンク項目のみが消える（元の emo2 実物・
/// deps DLL は別リンクが残るため無傷）。
///
/// フォールバック（`copy`）は、`dst` が `src` と別ボリュームでハードリンク不可な環境でも組立が成立するための
/// 保険。根は既定でワークツリーの `target\test-roots` の下（repo と同一ドライブ）なのでハードリンク経路が使われる。
fn materialize_file(from: &Path, to: &Path) {
    if std::fs::hard_link(from, to).is_ok() {
        return;
    }
    // 別ボリューム等でハードリンク不可なら実コピーへフォールバックする。
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

/// 共有ゴーストの `ghost/master/descript.txt` の中身（組立と鮮度の判定が同じ文を使う）。
fn ghost_descript() -> String {
    format!(
        "charset,UTF-8\nname,Shiori4TestGhost\nshiori,{}\nseriko.defaultsurfacedirectoryname,master\n",
        shiori4_testdll::DLL_FILE_NAME
    )
}

/// 与えられた `root` の直下へテストゴーストの実体（descript／shell／DLL）を配置する
/// （[`shared_test_ghost`] の唯一の組立手続き・`OnceLock` が直列にする）。
fn materialize_test_ghost_tree(root: &Path) {
    // 事前削除（冪等——固定パスの共有 fixture を作り直す経路で効く）。
    let _ = std::fs::remove_dir_all(root);

    // ghost/master/descript.txt（`shiori,` 行はテスト DLL を指す）。
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master)
        .unwrap_or_else(|e| panic!("create ghost/master に失敗: {e}"));
    std::fs::write(ghost_master.join("descript.txt"), ghost_descript())
        .unwrap_or_else(|e| panic!("write ghost descript.txt に失敗: {e}"));

    // emo2 実物 shell/master/ を再帰的に実体化（ハードリンク優先・自作最小 shell を作らない）。
    let shell_src = emo2_shell_master_src();
    assert!(
        shell_src.is_dir(),
        "emo2 実物 shell/master/ が見つからない（アンカー規約を確認）: {}",
        shell_src.display()
    );
    copy_dir_recursive(&shell_src, &root.join("shell").join("master"));

    // ビルド済み DLL を ghost/master/ へ配置（`mount.shiori.dir.join(file)`）。ハードリンク優先
    // （フォールバック copy）で、Defender の再走査による FS 飽和＝spine starvation を避ける。
    let built_dll = locate_built_test_dll();
    materialize_file(
        &built_dll,
        &ghost_master.join(shiori4_testdll::DLL_FILE_NAME),
    );

    // balloon は非同梱——何も配置しない。
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

/// InProc テスト一族が共有する**一度だけ組み立てる**テストゴーストを、錠を取ってから返す。
///
/// # なぜ共有 fixture か
/// 組立は emo2 実物 shell/master/ の PNG ツリーの実体化＋ビルド済み cdylib の配置という重いファイル I/O を
/// 伴う。テストごとに組み立てると FS が飽和して spine 側の協調反復テストを starve させるので、組立は
/// `OnceLock` で 1 回に抑え、プロセスをまたいでも再利用する。
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
/// - `shell/master/surfaces.txt` が実在（emo2 実物 shell 流用の証跡）。
/// - `ghost/master/<DLL>` が実在（DLL 物理配置の証跡）。
/// - `balloon/` が非在（balloon 非同梱の証跡）。
/// - `ghost/master/descript.txt` のバイトが [`ghost_descript`] と同じ（書き換えや古い組立を拾わない）。
/// - キャッシュ DLL が**現行ビルド済み DLL より古くない**（再ビルド陳腐化の検出）。ビルド済み DLL の
///   再ビルドは新 inode 生成で mtime を進めるため、旧ハードリンクを掴んだキャッシュはここで陳腐と判定され作り直される。
///
/// mtime 取得不能などの疑わしいケースは**安全側**（false＝作り直す）へ倒す。
fn shared_ghost_is_fresh(root: &Path) -> bool {
    let surfaces = root.join("shell").join("master").join("surfaces.txt");
    let cached_dll = root
        .join("ghost")
        .join("master")
        .join(shiori4_testdll::DLL_FILE_NAME);
    let balloon = root.join("balloon");
    let descript = root.join("ghost").join("master").join("descript.txt");

    if !surfaces.is_file() || !cached_dll.is_file() || balloon.exists() {
        return false;
    }
    if std::fs::read(&descript).ok().as_deref() != Some(ghost_descript().as_bytes()) {
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

/// このテストゴーストの LOCATE のテスト。
mod tests {
    /// `locate_built_test_dll()` は (a) 実在し (b) ファイル名が [`shiori4_testdll::DLL_FILE_NAME`] で
    /// (c) cargo の置き方どおり親ディレクトリ名が `deps`（cargo 1.99 まで）か `out`（cargo 1.100 から）の
    /// パスを返すこと（決め方は `shiori4_testdll::locate_built_dll` の説明）。
    #[test]
    fn locate_returns_existing_built_dll() {
        let dll = super::locate_built_test_dll();
        assert!(dll.exists(), "{}", dll.display());
        assert_eq!(
            dll.file_name().and_then(|s| s.to_str()),
            Some(shiori4_testdll::DLL_FILE_NAME)
        );
        let parent = dll
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str());
        assert!(
            matches!(parent, Some("deps" | "out")),
            "locate 戻り値の親ディレクトリは deps か out であること: {}",
            dll.display()
        );
    }

    // ---- 共有ゴーストの組立 ----
    //
    // 読み取り専用の観測 assert は read 経路の並行スレッドを 1 本に抑えるため単一のテストへ畳み、
    // 共有 fixture（[`super::shared_test_ghost`]）に対して実行する。

    use areka_parsers::charset::DefaultEncoding;
    use areka_parsers::package;

    /// 共有テストゴースト（[`super::shared_test_ghost`]）が次の 4 観点をすべて満たすこと。read 経路の並行
    /// スレッドを 1 本へ抑えるため単一テストへ畳み込む（FS 飽和による spine starvation 回避）:
    /// - **実マウント解決**: `package::resolve_with_shell` が Ok を返し、`shiori,` 行がテスト DLL 名
    ///   （== [`shiori4_testdll::DLL_FILE_NAME`]）を指す（boot が内部で通すのと同一契約）。
    /// - **DLL 物理配置**: ビルド済み DLL が `ghost/master/shiori4_testdll.dll` に在る。
    /// - **balloon 非同梱**: `balloon` ディレクトリを同梱しない。
    /// - **emo2 実物 shell 流用**: `shell/master/surfaces.txt` と入れ子 PNG（`purple/0/base1.png`）が実在
    ///   ＝自作最小 shell でなく実物の入れ子構造まで流用している。
    #[test]
    fn assembled_shared_ghost_satisfies_mount_dll_shell_and_no_balloon() {
        let shared = super::shared_test_ghost();
        let root = shared.root();

        // 実マウント解決が Ok＝完全な解決可能ゴースト。
        let mount = package::resolve_with_shell(root, DefaultEncoding::Utf8, None)
            .expect("組み立てた共有ゴーストは実マウント解決を成功裏に通過すべき（shell dir 込み）");
        // `shiori,` 行がテスト DLL 名（== 契約定数）を指す。
        assert_eq!(
            mount.shiori.file,
            Some(shiori4_testdll::DLL_FILE_NAME.to_string()),
            "mount.shiori.file は組立時に書いたテスト DLL 名を指すべき"
        );

        // ビルド済み DLL が ghost/master/ へ物理配置されている。
        let dll = root
            .join("ghost")
            .join("master")
            .join(shiori4_testdll::DLL_FILE_NAME);
        assert!(
            dll.is_file(),
            "ビルド済みテスト DLL が ghost/master/ へ物理配置されているべき: {}",
            dll.display()
        );

        // balloon 資産は非同梱（`resolve()` は balloon を消費しない）。
        let balloon = root.join("balloon");
        assert!(
            !balloon.exists(),
            "balloon 資産は非同梱であるべき（要件 4.2）: {}",
            balloon.display()
        );

        // emo2 実物 shell 流用（surfaces.txt＋入れ子 PNG まで）。
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

    /// descript.txt の中身が組立の文と違えば、残った共有ゴーストを再利用しない。
    #[test]
    fn a_shared_ghost_with_a_changed_descript_is_not_fresh() {
        let shared = super::shared_test_ghost();
        let descript = shared
            .root()
            .join("ghost")
            .join("master")
            .join("descript.txt");
        assert!(
            super::shared_ghost_is_fresh(shared.root()),
            "組んだ直後は健全"
        );

        std::fs::write(&descript, "charset,UTF-8\nname,Other\n").expect("descript を書けること");
        let stale = super::shared_ghost_is_fresh(shared.root());
        std::fs::write(&descript, super::ghost_descript()).expect("descript を戻せること");

        assert!(!stale, "descript が違えば作り直す側へ倒すこと");
        assert!(super::shared_ghost_is_fresh(shared.root()), "戻せば健全");
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

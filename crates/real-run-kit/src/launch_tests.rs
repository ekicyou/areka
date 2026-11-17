//! `launch` の兄弟のテスト。子を起こさずに、組んだ `Command` の環境と引数を値で確かめる
//! （開発者の環境変数を試験のプロセスに置かずに 1.5 を確かめる）。

use super::*;
use std::collections::HashMap;

/// 組んだ `Command` の環境の指定（外す側は `None`）。
fn envs(cmd: &Command) -> HashMap<String, Option<String>> {
    cmd.get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect()
}

fn args(cmd: &Command) -> Vec<String> {
    cmd.get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn set(e: &HashMap<String, Option<String>>, key: &str) -> String {
    e.get(key)
        .unwrap_or_else(|| panic!("{key} が指定されていない: {e:?}"))
        .clone()
        .unwrap_or_else(|| panic!("{key} が外す側にある: {e:?}"))
}

fn plain(exe: &str) -> Command {
    let p = |s: &str| std::path::absolute(s).unwrap();
    RealRun::new(exe).child_command(&[], &p("r"), &p("p"), 4321)
}

#[test]
fn removes_the_user_root_and_sets_the_kit_defaults() {
    let cmd = plain("areka.exe");
    let e = envs(&cmd);
    // 書ける根は外したまま（1.5）。ほかの 3 つは窓口の既定で上書き（受け継がない）。
    assert_eq!(e.get("AREKA_USER_ROOT"), Some(&None));
    assert_eq!(
        set(&e, "AREKA_ROOT"),
        abs(Path::new("r")).display().to_string()
    );
    assert_eq!(
        set(&e, "AREKA_PROFILE_DIR"),
        abs(Path::new("p")).display().to_string()
    );
    assert_eq!(set(&e, "AREKA_MCP_PORT"), "4321");
    assert_eq!(set(&e, "AREKA_NO_ALERT"), "1");
    assert_eq!(set(&e, "NO_COLOR"), "1");
    // 開発者の環境の PASTA_DEBUG=1 を受け継ぐと、ほかの pasta が握る 9276 とぶつかって LOAD に失敗する。
    assert_eq!(set(&e, "PASTA_DEBUG"), "0");
    let safety = Limits::default().safety_exit.as_millis().to_string();
    assert_eq!(set(&e, "AREKA_APP_SMOKE_EXIT_MS"), safety);
    assert_eq!(set(&e, "RUST_LOG"), "info");
    assert!(args(&cmd).is_empty());
}

#[test]
fn logical_settings_then_caller_env_wins_last() {
    let run = RealRun::new("areka.exe")
        .rust_log("warn")
        .safety_exit(Duration::from_millis(1500))
        .deadline(Duration::from_secs(7))
        .env("NO_COLOR", "0")
        .env("AREKA_USER_ROOT", "u")
        .env("AREKA_MCP_PORT", "9")
        .env("EXTRA", "x");
    assert_eq!(run.limits.deadline, Duration::from_secs(7));
    let cmd = run.child_command(&[], Path::new("r"), Path::new("p"), 4321);
    let e = envs(&cmd);
    assert_eq!(set(&e, "AREKA_APP_SMOKE_EXIT_MS"), "1500");
    assert_eq!(
        set(&e, "RUST_LOG"),
        "warn,areka_mcp::server=info,areka::app_exit=info"
    );
    assert_eq!(set(&e, "NO_COLOR"), "0");
    assert_eq!(set(&e, "AREKA_USER_ROOT"), "u");
    assert_eq!(set(&e, "AREKA_MCP_PORT"), "9");
    assert_eq!(set(&e, "EXTRA"), "x");
}

/// 10: 最後に積まれた `AREKA_MCP_PORT` が番号として読めなければ、`.mcp_port(n)` を案内して落ちる。
#[test]
#[should_panic(expected = "`.mcp_port(n)`")]
fn an_unreadable_mcp_port_override_panics_with_the_mcp_port_hint() {
    RealRun::new("areka.exe")
        .mcp_port(4321)
        .env("AREKA_MCP_PORT", "9900")
        .env("areka_mcp_port", "x")
        .port();
}

#[test]
fn positional_args_come_first_then_caller_args() {
    let run = RealRun::new("areka.exe").arg("--x").arg("y");
    let g = abs(Path::new("g"));
    let b = abs(Path::new("b"));
    let cmd = run.child_command(&[&g, &b], Path::new("r"), Path::new("p"), 1);
    let want = [g.display().to_string(), b.display().to_string()];
    assert_eq!(
        args(&cmd),
        [&want[..], &["--x".into(), "y".into()]].concat()
    );
}

#[test]
fn relative_paths_become_absolute_without_verbatim_prefix() {
    let cmd = RealRun::new(r"rel\areka.exe").child_command(
        &[Path::new(r"rel\ghost")],
        Path::new(r"rel\root"),
        Path::new(r"rel\..\profile"),
        1,
    );
    let e = envs(&cmd);
    let mut paths = vec![
        cmd.get_program().to_string_lossy().into_owned(),
        set(&e, "AREKA_ROOT"),
        set(&e, "AREKA_PROFILE_DIR"),
    ];
    paths.extend(args(&cmd));
    for p in paths {
        assert!(Path::new(&p).is_absolute(), "絶対でない: {p}");
        assert!(!p.starts_with(r"\\?\"), "\\\\?\\ が付いた: {p}");
        assert!(!p.contains(r"\..\"), "正規化されていない: {p}");
    }
}

/// 4 通りの位置の引数と根（1.1・1.4）。検体は本物の写しを取る。
#[test]
fn four_launch_variants_resolve_positional_args_and_root() {
    let dirs = RunDirs::create(Some("emo2"));
    let sample = dirs.sample().unwrap();
    let emo2 = sample.folder().to_path_buf();
    let kaku = sample.balloon("emo2-kakukaku").unwrap().to_path_buf();
    let sample_root = sample.root().to_path_buf();
    let empty = dirs.root().to_path_buf();

    let r = resolve(
        &Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        },
        &dirs,
    );
    assert_eq!(
        (r.ghost, r.balloon, r.root),
        (Some(emo2.clone()), Some(kaku), empty.clone())
    );

    let r = resolve(
        &Launch::Sample {
            name: "emo2".into(),
            balloon: None,
        },
        &dirs,
    );
    assert_eq!(
        (r.ghost, r.balloon, r.root),
        (Some(emo2), None, empty.clone())
    );

    let r = resolve(
        &Launch::SampleRoot {
            name: "emo2".into(),
        },
        &dirs,
    );
    assert_eq!((r.ghost, r.balloon, r.root), (None, None, sample_root));

    let r = resolve(&Launch::Root("x".into()), &dirs);
    assert_eq!(
        (r.ghost, r.balloon, r.root),
        (None, None, abs(Path::new("x")))
    );

    let r = resolve(&Launch::default(), &dirs);
    assert_eq!((r.ghost, r.balloon, r.root), (None, None, empty));
}

#[test]
fn sample_name_is_used_for_the_copy() {
    assert_eq!(sample_name(&Launch::Empty), None);
    assert_eq!(sample_name(&Launch::Root("x".into())), None);
    let s = Launch::SampleRoot {
        name: "emo2".into(),
    };
    assert_eq!(sample_name(&s), Some("emo2"));
    let s = Launch::Sample {
        name: "claudia".into(),
        balloon: None,
    };
    assert_eq!(sample_name(&s), Some("claudia"));
}

#[test]
fn i686_target_is_two_levels_above_areka_exe() {
    let exe = abs(Path::new(r"t\debug\areka.exe"));
    assert_eq!(i686_target(&exe), abs(Path::new(r"t\i686-pc-windows-msvc")));
}

/// 記録の段の足し方の表（D-4・4.4）。
#[test]
fn compose_rust_log_table() {
    const S: &str = ",areka_mcp::server=info";
    const X: &str = ",areka::app_exit=info";
    let table: &[(&str, String)] = &[
        // 全体が warn 以下で、どちらの出所も親も名指していない → 両方足す。
        (
            "warn,areka::boot_config=info",
            format!("warn,areka::boot_config=info{S}{X}"),
        ),
        ("error", format!("error{S}{X}")),
        ("off", format!("off{S}{X}")),
        // 全体が info 以上 → 足さない（呼び手の debug・trace を下げない）。
        ("info,kanade=trace", "info,kanade=trace".into()),
        ("trace", "trace".into()),
        ("debug", "debug".into()),
        ("INFO", "INFO".into()),
        // 数字の段は出所の名ではない（3 以上は info 以上・2 以下は warn 以下）。
        ("3", "3".into()),
        ("2", format!("2{S}{X}")),
        // 空・未設定 → 既定の info。
        ("", "info".into()),
        ("  ", "info".into()),
        // 親を名指せばその出所には足さない（もう一方は全体の段＝無し＝off で決まる）。
        // EnvFilter の名指しは素の前方一致なので、`areka` は `areka_mcp::server` も覆う。
        ("areka=debug", "areka=debug".into()),
        ("areka_mcp=debug", format!("areka_mcp=debug{X}")),
        (
            "warn,areka=debug,areka_mcp=trace",
            "warn,areka=debug,areka_mcp=trace".into(),
        ),
        // 出所そのものを名指していれば足さない（段が info より細かくても）。
        (
            "warn,areka::app_exit=trace",
            format!("warn,areka::app_exit=trace{S}"),
        ),
        (
            "warn,areka_mcp::server=off",
            format!("warn,areka_mcp::server=off{X}"),
        ),
        // `::` の切れ目が無くても前方一致すれば名指し（EnvFilter と同じ規則）。
        (
            "warn,areka_m=debug,areka::app=debug",
            "warn,areka_m=debug,areka::app=debug".into(),
        ),
        ("warn,areka_m=trace", format!("warn,areka_m=trace{X}")),
        // 前方一致しない名前は名指しではない。
        (
            "warn,areka_mcp::client=debug,areka::app_exits=debug",
            format!("warn,areka_mcp::client=debug,areka::app_exits=debug{S}{X}"),
        ),
        // 段の無い出所の指定も名指し。
        ("warn,areka", "warn,areka".into()),
    ];
    for (base, want) in table {
        assert_eq!(&compose_rust_log(base), want, "base={base:?}");
    }
}

/// 26・D2: `RELIED_TARGETS` の各項が、窓口の頼る行を本物に出しているモジュールのファイルを名指す。
/// `<crate>::<段…>` を `crates/<crate の _ を - に>/src/<段>.rs`（無ければ `<段>/mod.rs`）へ写し、
/// ファイルが在る・頼る行の語が在る・`target:` の上書きが無い、を読むだけで確かめる（本物は起こさない）。
/// 順は `RELIED_TARGETS` と同じ（項を書き換えても照らす語は付いてくる）。
#[test]
fn relied_targets_name_the_modules_that_emit_the_relied_lines() {
    const WORDS: &[&[&str]] = &[
        &[r#""MCP: 待受を始めた"#],
        &[r#"event = "app_exit""#, r#""os_close_request""#],
    ];
    // zip は短い方で黙って止まるので、項と語の表の数が揃っていることを先に確かめる。
    assert_eq!(
        RELIED_TARGETS.len(),
        WORDS.len(),
        "RELIED_TARGETS の項と照らす語の表の数が違う"
    );
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for (target, &words) in RELIED_TARGETS.iter().zip(WORDS) {
        let mut parts = target.split("::");
        let krate = parts.next().unwrap().replace('_', "-");
        let module: PathBuf = parts.collect();
        let src = crates.join(krate).join("src");
        let file = [
            src.join(&module).with_extension("rs"),
            src.join(&module).join("mod.rs"),
        ]
        .into_iter()
        .find(|f| f.is_file())
        .unwrap_or_else(|| {
            panic!(
                "出所 {target:?} のファイルが無い（{}）。出所を動かしたなら launch.rs の RELIED_TARGETS を直す",
                src.join(&module).display()
            )
        });
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{} を読めない: {e}", file.display()));
        for word in words {
            assert!(
                text.contains(word),
                "出所 {target:?} の {} に頼る行の語 {word} が無い。行を動かしたなら launch.rs の RELIED_TARGETS を直す",
                file.display()
            );
        }
        assert!(
            !text.contains("target:"),
            "出所 {target:?} の {} が target: で出所を上書きしている。launch.rs の RELIED_TARGETS を直す",
            file.display()
        );
    }
}

/// 無い実行ファイルの場所（作らない・`target\` の下の名前だけ）。隣に仲介も無いフォルダ。
fn missing_exe() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/real-run-kit-missing-exe/debug/no-such-areka.exe")
}

// 1.10: 仲介を揃える前に、実行ファイルの場所と理由で失敗させる。
#[test]
#[should_panic(expected = "no-such-areka.exe（ファイルが無い）")]
fn missing_exe_panics_with_path_and_reason_before_helper() {
    RealRun::new(missing_exe()).spawn();
}

#[test]
#[should_panic(expected = "no-such-areka.exe（ファイルが無い）")]
fn missing_exe_panics_with_path_and_reason_without_helper() {
    RealRun::new(missing_exe()).without_host32_helper().spawn();
}

use {
    crate::{ast::Protocol, parser::parse},
    error_reporter::Report,
    regex::Regex,
    std::{
        path::Path,
        process::{Command, Stdio},
    },
    walkdir::WalkDir,
};

#[derive(Debug)]
pub(crate) struct Repo {
    pub(crate) name: &'static str,
    pub(crate) url: String,
    pub(crate) protocols: Vec<Protocol>,
}

#[derive(Default)]
struct Config {
    dir: &'static str,
    exclude: Option<Regex>,
}

pub(crate) fn collect() -> Vec<Repo> {
    let configs = [
        Config {
            dir: "cosmic-protocols",
            ..Default::default()
        },
        Config {
            dir: "external",
            ..Default::default()
        },
        Config {
            dir: "hyprland-protocols",
            ..Default::default()
        },
        Config {
            dir: "jay-protocols",
            ..Default::default()
        },
        Config {
            dir: "plasma-wayland-protocols",
            ..Default::default()
        },
        Config {
            dir: "river",
            exclude: Some(Regex::new("^protocol/upstream/.*").unwrap()),
            ..Default::default()
        },
        Config {
            dir: "treeland-protocols",
            ..Default::default()
        },
        Config {
            dir: "wayland",
            exclude: Some(
                Regex::new(
                    r#"(?x)
                       ^tests/.*|
                       ^protocol/tests\.xml
                    "#,
                )
                .unwrap(),
            ),
            ..Default::default()
        },
        Config {
            dir: "wayland-protocols",
            ..Default::default()
        },
        Config {
            dir: "weston",
            ..Default::default()
        },
        Config {
            dir: "wlr-protocols",
            ..Default::default()
        },
    ];
    let repos_dir = Path::new("repos");
    let mut repos = vec![];
    for config in configs {
        let repo_dir = repos_dir.join(config.dir);
        let Some(url) = get_url(&repo_dir) else {
            continue;
        };
        let mut protocols = vec![];
        let dir = repos_dir.join(config.dir);
        for file in WalkDir::new(&dir) {
            let file = match file {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Could not read {}: {}", dir.display(), Report::new(e));
                    continue;
                }
            };
            let file = file.path();
            let rel_path = file.strip_prefix(&dir).unwrap();
            let Some(path) = rel_path.to_str() else {
                eprintln!("File name {rel_path:?} is not UTF-8");
                continue;
            };
            if !path.ends_with(".xml") {
                continue;
            }
            if let Some(e) = &config.exclude
                && e.is_match(path)
            {
                continue;
            }
            let contents = match std::fs::read(file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Could not read {}: {}", file.display(), Report::new(e));
                    continue;
                }
            };
            let p = match parse(rel_path, &contents) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Could not parse {}: {}", file.display(), Report::new(e));
                    continue;
                }
            };
            protocols.extend(p);
        }
        protocols.sort_by(|p1, p2| p1.name.cmp(&p2.name));
        repos.push(Repo {
            name: config.dir,
            url,
            protocols,
        });
    }
    repos
}

fn get_url(repo_dir: &Path) -> Option<String> {
    let child = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .stdout(Stdio::piped())
        .spawn();
    let child = match child {
        Ok(o) => o,
        Err(e) => {
            eprintln!(
                "Could not run git in {}: {}",
                repo_dir.display(),
                Report::new(e),
            );
            return None;
        }
    };
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("could not wait for git to exit: {}", Report::new(e));
            return None;
        }
    };
    if !output.status.success() {
        eprintln!("git failed while running in {}", repo_dir.display());
        return None;
    }
    match String::from_utf8(output.stdout) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!(
                "git remote in {} is not UTF-8: {}",
                repo_dir.display(),
                Report::new(e),
            );
            None
        }
    }
}

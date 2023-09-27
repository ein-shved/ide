use ide::Config;
use std::env;
use std::io;

fn main() -> io::Result<()> {
    let cfg = Config::new();
    let projects = cfg.get_projects()?;

    if let Some(name) = env::args().skip(1).next() {
        cfg.exec_from(projects, &name)
    } else {
        cfg.run_ui(projects)
    }
}

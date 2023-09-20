use ide::{Config, ui};
use std::env;
use std::io;

fn main() -> io::Result<()> {
    let cfg = Config::new();
    let mut projects = cfg.get_projects()?;

    if let Some(name) = env::args().skip(1).next() {
        let proj = projects
            .find(|proj| proj.name == name)
            .expect(&format!("No project {name} found"));
        Err(cfg.exec(&proj))
    } else {
        ui::new(projects).run()
    }
}

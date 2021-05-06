use std::env;
use minigrep::{Config, Error, run};

fn main() -> Result<(), Error> {
    let cfg = Config::from_args(env::args())?;
    run(cfg)
}

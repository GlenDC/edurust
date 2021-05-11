use minigrep::{run, Config, Error};
use std::env;

fn main() -> Result<(), Error> {
    let cfg = Config::from_args(env::args())?;
    run(cfg)
}

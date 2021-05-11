use std::env;

use crate::error::Error;

pub struct Config {
    query: String,
    filename: String,
    case_insensitive: bool,
}

impl Config {
    pub fn from_args(mut args: env::Args) -> Result<Config, Error> {
        // skip program name
        args.next();

        // read pos args
        let query = args.next().ok_or(Error::MissingArg("query"))?;
        let filename = args.next().ok_or(Error::MissingArg("filename"))?;

        // read env args
        let case_insensitive = env::var("CASE_INSENSITIVE")
            .map(|v| {
                vec!["1", "true", "ok"]
                    .iter()
                    .any(|t| v.to_lowercase() == t.to_lowercase())
            })
            .unwrap_or(false);

        Ok(Config {
            query,
            filename,
            case_insensitive,
        })
    }

    pub fn filename(&self) -> &str {
        self.filename.as_str()
    }

    pub fn query(&self) -> &str {
        self.query.as_str()
    }

    pub fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }
}

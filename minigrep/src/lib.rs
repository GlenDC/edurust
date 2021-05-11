use std::fs;

mod config;
mod error;

pub use config::Config;
pub use error::Error;

pub fn run(cfg: Config) -> Result<(), Error> {
    // read file
    let contents = fs::read_to_string(cfg.filename())?;

    // define search func
    let search = if cfg.case_insensitive() {
        search_case_insensitive
    } else {
        search
    };

    // search the query for each read line
    let mut lines_found = 0;
    for line in search(cfg.query(), &contents) {
        println!("{}", line);
        lines_found += 1;
    }

    // ensure we return an error if nothing was found
    if lines_found > 0 {
        Ok(())
    } else {
        Err(Error::NoResults)
    }
}

pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let query = query.to_lowercase();
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_result() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search_case_insensitive(query, contents)
        );
    }
}

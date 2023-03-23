use std::{fs, net::SocketAddr, str::FromStr, time::Duration};

use eyre::{Context, ContextCompat};

/// Read config from `{config dir}/rsummary.kdl`
pub(crate) fn config() -> eyre::Result<Config> {
    let path = dirs::config_dir()
        .wrap_err("Failed to obtain config directory")?
        .join("rsummary.kdl");

    let conf = fs::read_to_string(&path).wrap_err("Failed to read config")?;

    let conf = knuffel::parse(&path.to_string_lossy(), &conf).wrap_err("Failed to parse config")?;

    Ok(conf)
}

#[derive(Debug, knuffel::Decode)]
pub(crate) struct Config {
    #[knuffel(child, unwrap(argument, str))]
    pub addr: SocketAddr,
    #[knuffel(child, unwrap(argument, str), default)]
    pub cache_timeout: CacheTimeout,
    #[knuffel(children(name = "allow"), default)]
    pub allowed: Vec<Allowed>,
}

#[derive(Debug, PartialEq, Eq, knuffel::Decode)]
pub(crate) struct Allowed {
    #[knuffel(argument)]
    pub user: String,
    #[knuffel(argument)]
    pub repo: String,
}

#[derive(Debug)]
pub(crate) struct CacheTimeout(pub Duration);

impl Default for CacheTimeout {
    fn default() -> Self {
        CacheTimeout(Duration::from_secs(60 * 5))
    }
}

impl FromStr for CacheTimeout {
    type Err = eyre::Report;

    fn from_str(s: &str) -> eyre::Result<Self> {
        let seconds = s
            .strip_suffix('s')
            .ok_or_else(|| eyre::eyre!("Missing `s` suffix"))?
            .parse::<u64>()
            .wrap_err("Failed to parse duration as an integer")?;

        Ok(CacheTimeout(Duration::from_secs(seconds)))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::{Allowed, Config};

    #[test]
    fn parse_full_config() {
        let conf = r#"
addr "0.0.0.0:3000"
cache-timeout "12s"
allow "rust-lang" "rust"
allow "WaffleLapkin" "t"
        "#;
        let conf = knuffel::parse("conf.kdl", conf).unwrap();
        let Config {
            addr,
            allowed,
            cache_timeout,
        } = conf;

        assert_eq!(addr, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(cache_timeout.0, Duration::from_secs(12));
        assert_eq!(
            allowed,
            vec![
                Allowed {
                    user: "rust-lang".to_owned(),
                    repo: "rust".to_owned()
                },
                Allowed {
                    user: "WaffleLapkin".to_owned(),
                    repo: "t".to_owned()
                }
            ]
        );
    }

    #[test]
    fn parse_minimal_config() {
        let conf = r#"
addr "0.0.0.0:3000"
        "#;
        let conf = knuffel::parse("conf.kdl", conf).unwrap();
        let Config {
            addr,
            allowed,
            cache_timeout,
        } = conf;

        assert_eq!(addr, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(cache_timeout.0, Duration::from_secs(60 * 5));
        assert_eq!(allowed, vec![]);
    }
}

use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLink {
    Ratacat,
    Tx { hash: String },
    Account { id: String },
    Block { height: u64 },
    OpenPath { path: String },
    Session { id: String, read_only: bool },
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid scheme")]
    Scheme,
    #[error("missing host/path")]
    Missing,
    #[error("invalid number")]
    Num,
}

impl FromStr for DeepLink {
    type Err = ParseError;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let url = url::Url::parse(raw).map_err(|_| ParseError::Scheme)?;
        if url.scheme() != "near" {
            return Err(ParseError::Scheme);
        }

        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        let path = url.path().trim_start_matches('/');

        if host == "ratacat" || path.starts_with("ratacat") {
            return Ok(DeepLink::Ratacat);
        }

        if host == "tx" {
            if path.is_empty() {
                return Err(ParseError::Missing);
            }
            return Ok(DeepLink::Tx {
                hash: path.to_string(),
            });
        }

        if host == "account" {
            if path.is_empty() {
                return Err(ParseError::Missing);
            }
            return Ok(DeepLink::Account {
                id: path.to_string(),
            });
        }

        if host == "block" {
            if path.is_empty() {
                return Err(ParseError::Missing);
            }
            let h = path.parse::<u64>().map_err(|_| ParseError::Num)?;
            return Ok(DeepLink::Block { height: h });
        }

        if host == "open" {
            if path.starts_with("session/") {
                let id = path.trim_start_matches("session/").to_string();
                if id.is_empty() {
                    return Err(ParseError::Missing);
                }
                let ro = url
                    .query_pairs()
                    .any(|(k, v)| k == "readOnly" && (v == "1" || v.eq_ignore_ascii_case("true")));
                return Ok(DeepLink::Session { id, read_only: ro });
            }
            if let Some(p) = url
                .query_pairs()
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.to_string())
            {
                if !p.starts_with('/') {
                    return Err(ParseError::Missing);
                }
                return Ok(DeepLink::OpenPath { path: p });
            }
            return Err(ParseError::Missing);
        }

        Err(ParseError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ok_ratacat() {
        assert_eq!(
            "near://ratacat".parse::<DeepLink>().unwrap(),
            DeepLink::Ratacat
        );
    }
    #[test]
    fn ok_tx() {
        match "near://tx/DEADBEEF".parse::<DeepLink>().unwrap() {
            DeepLink::Tx { hash } => assert_eq!(hash, "DEADBEEF"),
            _ => panic!(),
        }
    }
    #[test]
    fn ok_account() {
        match "near://account/foo.near".parse::<DeepLink>().unwrap() {
            DeepLink::Account { id } => assert_eq!(id, "foo.near"),
            _ => panic!(),
        }
    }
    #[test]
    fn ok_block() {
        match "near://block/42".parse::<DeepLink>().unwrap() {
            DeepLink::Block { height } => assert_eq!(height, 42),
            _ => panic!(),
        }
    }
    #[test]
    fn ok_open_path() {
        match "near://open?path=/tx/abc".parse::<DeepLink>().unwrap() {
            DeepLink::OpenPath { path } => assert_eq!(path, "/tx/abc"),
            _ => panic!(),
        }
    }
    #[test]
    fn ok_session() {
        match "near://open/session/123?readOnly=1"
            .parse::<DeepLink>()
            .unwrap()
        {
            DeepLink::Session { id, read_only } => {
                assert_eq!(id, "123");
                assert!(read_only);
            }
            _ => panic!(),
        }
    }
    #[test]
    fn bad_scheme() {
        assert!("http://x".parse::<DeepLink>().is_err());
        assert!("myapp://tx/abc".parse::<DeepLink>().is_err());
    }
}

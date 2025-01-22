use crate::error::RoadsterResult;
use serde_derive::{Deserialize, Serialize};
use std::net::SocketAddr;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Address {
    pub scheme: String,
    pub host: String,
    pub port: u32,
}

impl Address {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn url_with_scheme(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.port)
    }

    pub fn socket_addr(&self) -> RoadsterResult<SocketAddr> {
        let addr = self.url().parse()?;
        Ok(addr)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::service::common::address::Address;
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use url::Url;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        scheme = "http"
        host = "localhost"
        port = 1234
        "#
    )]
    #[case(
        r#"
        scheme = "https"
        host = "[::]"
        port = 3000
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn url_with_scheme(_case: TestCase, #[case] address: &str) {
        let addr: Address = toml::from_str(address).unwrap();

        let url: Result<Url, _> = addr.url_with_scheme().parse();

        assert_debug_snapshot!(url);
    }
}

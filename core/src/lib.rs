use config::{Config as CConfig, ConfigError, File, FileFormat};
use http_body_util::Empty;
use hyper::{body::Bytes, Request, StatusCode};
use hyper_tls::HttpsConnector;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct WebsiteInfo {
    name: String,
    base_url: String,
    user_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    websites: Vec<WebsiteInfo>,
}

impl Default for Config {
    fn default() -> Self {
        Self { websites: vec![] }
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = ConfigError;
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let builder =
            CConfig::builder().add_source(File::new(value.to_str().unwrap(), FileFormat::Json));

        Ok(builder.build()?.try_deserialize()?)
    }
}

#[derive(Debug)]
pub struct AccountChecker {
    config: Config,
}

#[derive(Clone, Debug)]
pub enum CheckStatus {
    Available,
    NotFound,
    Other(u16),
}

#[derive(Clone, Debug)]
pub struct CheckResult<'a> {
    info: &'a WebsiteInfo,
    status: CheckStatus,
}

#[derive(Debug)]
pub struct CrabtiveError(&'static str);

const FIREFOX_USER_AGENT: &'static str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:101.0) Gecko/20100101 Firefox/101.0";

async fn fetch_status(url: hyper::Uri) -> Result<StatusCode, CrabtiveError> {
    let https = HttpsConnector::new();
    let client = Client::builder(TokioExecutor::new()).build::<_, Empty<Bytes>>(https);

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::USER_AGENT, FIREFOX_USER_AGENT)
        .body::<Empty<Bytes>>(Empty::new())
        .unwrap();

    let Ok(res) = client.request(req).await else {
        return Err(CrabtiveError("Can't send request!"));
    };

    Ok(res.status())
}

fn parse_user_url<'a>(plain_format: &'a str, username: &'a str) -> Result<String, CrabtiveError> {
    let splited: Vec<&str> = plain_format.split("{}").collect();
    let Some(url) = splited.get(0) else {
        return Err(CrabtiveError("Can't parse the url!"));
    };

    Ok(format!("{}{}", url, username))
}

impl<'a> AccountChecker {
    async fn check_account(
        website: &'a WebsiteInfo,
        username: &'a str,
    ) -> Result<CheckResult<'a>, CrabtiveError> {
        let status_code = fetch_status(
            parse_user_url(&website.user_url, username)?
                .parse()
                .unwrap(),
        )
        .await?;

        Ok(CheckResult {
            info: website,
            status: CheckStatus::from(status_code),
        })
    }

    // TODO: make it parallel
    pub async fn check_accounts(
        &'a self,
        username: &'a str,
    ) -> Result<Vec<CheckResult<'a>>, CrabtiveError> {
        let mut result = vec![];
        for web in &self.config.websites {
            result.push(Self::check_account(web, username).await?);
        }
        Ok(result)
    }
}

impl From<Config> for AccountChecker {
    fn from(value: Config) -> Self {
        Self { config: value }
    }
}

impl From<StatusCode> for CheckStatus {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::OK => CheckStatus::Available,
            StatusCode::NOT_FOUND => CheckStatus::NotFound,

            status => CheckStatus::Other(status.as_u16()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn test_config_from_pathbuf() {
        let config = Config::try_from(PathBuf::from_str("./test_config.json").unwrap()).unwrap();
        assert_eq!(config.websites.len(), 1);
    }

    #[tokio::test]
    async fn test_account_checker() {
        let config = Config::try_from(PathBuf::from_str("./test_config.json").unwrap()).unwrap();
        assert_eq!(config.websites.len(), 1);

        let checker = AccountChecker::from(config);
        let result = checker.check_accounts("nyzd").await.unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0].status, CheckStatus::Available));
    }

    #[test]
    fn test_parse_user_url() {
        let url = parse_user_url("https://github.com/user/{}", "username").unwrap();

        assert_eq!(url, String::from("https://github.com/user/username"));
    }
}
